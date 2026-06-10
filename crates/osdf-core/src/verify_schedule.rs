//! Adaptive verification scheduling: pick profile and thread count from hardware and package shape.
//!
//! Parallelism speeds **many evaluations** (batch ingress, cached revalidation). A single
//! portable ingest verify remains one logical job; threads help when you run several in parallel.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::identity::VerifierConfig;
use crate::manifest::parse_manifest;
use crate::report::VerificationStatus;
use crate::verify::verify_package_bytes;
use crate::verify_fast::{parse_package, verify_package_bytes_fast, verify_parsed_package_fast};
use crate::verify_profile::{FastVerifyResult, ParsedPackage, VerificationProfile};

/// Detected or configured host capacity used for scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardwareSnapshot {
    pub logical_cores: usize,
    /// Best-effort physical core estimate (hyper-thread pairs assumed when unknown).
    pub physical_cores: usize,
    /// Memory budget for concurrent verify workers (bytes).
    pub usable_memory_bytes: u64,
}

impl HardwareSnapshot {
    /// Build a snapshot from the running host.
    ///
    /// `usable_memory_bytes` uses a conservative default when OS memory is unavailable.
    pub fn detect() -> Self {
        let logical_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .max(1);
        let physical_cores = estimate_physical_cores(logical_cores);
        Self {
            logical_cores,
            physical_cores,
            usable_memory_bytes: detect_usable_memory_bytes(),
        }
    }

    pub fn with_usable_memory_bytes(mut self, bytes: u64) -> Self {
        self.usable_memory_bytes = bytes.max(256 * 1024 * 1024);
        self
    }
}

/// Cheap package shape metrics (requires one ZIP parse + manifest read).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackageStats {
    pub archive_bytes: u64,
    pub object_count: usize,
    pub declared_payload_bytes: u64,
    pub cost_tier: PackageCostTier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageCostTier {
    Small,
    Medium,
    Large,
}

impl PackageCostTier {
    pub fn from_stats(archive_bytes: u64, object_count: usize, declared_payload_bytes: u64) -> Self {
        if archive_bytes <= 256 * 1024 && object_count <= 32 && declared_payload_bytes <= 256 * 1024
        {
            Self::Small
        } else if archive_bytes <= 8 * 1024 * 1024
            && object_count <= 256
            && declared_payload_bytes <= 64 * 1024 * 1024
        {
            Self::Medium
        } else {
            Self::Large
        }
    }
}

/// Why verification is running (drives profile and thread defaults).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyIntent {
    /// Full forensic report for one ingest.
    ForensicReport,
    /// Allow/deny on newly received bytes.
    GatewayIngest,
    /// Policy re-check on an already parsed capsule.
    CachedRevalidation,
    /// Maximize aggregate eval throughput (benchmark / batch hot path).
    MaxThroughput,
}

/// Recommended profile, worker count, and a short human-readable rationale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyPlan {
    pub profile: VerificationProfile,
    pub threads: usize,
    pub reason: String,
}

impl VerifyPlan {
    /// Heuristic plan from hardware, package stats, and intent (no timing probe).
    pub fn recommend(
        hardware: &HardwareSnapshot,
        stats: &PackageStats,
        intent: VerifyIntent,
    ) -> Self {
        let profile = profile_for_intent(intent, stats);
        let threads = threads_for_profile(hardware, stats, profile, intent);
        let reason = format!(
            "{intent:?} tier={:?} logical={} physical={} archive={} objects={} payload={}",
            stats.cost_tier,
            hardware.logical_cores,
            hardware.physical_cores,
            stats.archive_bytes,
            stats.object_count,
            stats.declared_payload_bytes,
        );
        Self {
            profile,
            threads,
            reason,
        }
    }

    /// Refine [`Self::threads`] with a short parallel probe (120 ms per candidate).
    pub fn with_thread_probe(
        mut self,
        hardware: &HardwareSnapshot,
        stats: &PackageStats,
        bytes: &[u8],
        config: &VerifierConfig,
    ) -> Self {
        let candidates = thread_candidates(hardware, stats, self.profile);
        if candidates.len() <= 1 {
            return self;
        }

        let best = probe_best_threads(self.profile, bytes, config, &candidates, Duration::from_millis(120));
        if best > 1 {
            self.reason = format!("{}; probe selected {best} threads", self.reason);
            self.threads = best;
        }
        self
    }
}

pub fn peek_package_stats(bytes: &[u8]) -> Result<PackageStats, crate::verify_profile::FastFailCode> {
    let parsed = parse_package(bytes)?;
    stats_from_parsed(&parsed)
}

pub fn stats_from_parsed(parsed: &ParsedPackage) -> Result<PackageStats, crate::verify_profile::FastFailCode> {
    let manifest = parse_manifest(&parsed.container)
        .map_err(|_| crate::verify_profile::FastFailCode::ManifestParseFailure)?;
    let declared_payload_bytes: u64 = manifest.objects.iter().map(|o| o.bytes).sum();
    let object_count = manifest.objects.len();
    let cost_tier =
        PackageCostTier::from_stats(parsed.archive_bytes, object_count, declared_payload_bytes);
    Ok(PackageStats {
        archive_bytes: parsed.archive_bytes,
        object_count,
        declared_payload_bytes,
        cost_tier,
    })
}

fn profile_for_intent(intent: VerifyIntent, stats: &PackageStats) -> VerificationProfile {
    match intent {
        VerifyIntent::ForensicReport => VerificationProfile::CoreJsonPortableFull,
        VerifyIntent::GatewayIngest => VerificationProfile::CoreJsonPortableFast,
        VerifyIntent::CachedRevalidation => VerificationProfile::CoreJsonParsedFast,
        VerifyIntent::MaxThroughput => match stats.cost_tier {
            PackageCostTier::Large => VerificationProfile::CoreJsonPortableFast,
            _ => VerificationProfile::CoreJsonParsedFast,
        },
    }
}

fn threads_for_profile(
    hardware: &HardwareSnapshot,
    stats: &PackageStats,
    profile: VerificationProfile,
    intent: VerifyIntent,
) -> usize {
    let max_by_ram = max_concurrent_by_memory(hardware, stats, profile);
    let core_cap = match (profile, stats.cost_tier, intent) {
        (VerificationProfile::CoreJsonParsedFast, _, _) => hardware.physical_cores,
        (_, PackageCostTier::Small, _) => hardware.logical_cores,
        (_, PackageCostTier::Medium, _) => hardware.physical_cores,
        (VerificationProfile::CoreJsonPortableFull, PackageCostTier::Large, _) => 1,
        (_, PackageCostTier::Large, VerifyIntent::ForensicReport) => 1,
        (_, PackageCostTier::Large, _) => hardware.physical_cores.div_ceil(2).clamp(1, 4),
    };
    core_cap.min(max_by_ram).max(1)
}

fn max_concurrent_by_memory(
    hardware: &HardwareSnapshot,
    stats: &PackageStats,
    profile: VerificationProfile,
) -> usize {
    let mem_per_job = estimate_memory_per_job(stats, profile);
    let ram_budget = (hardware.usable_memory_bytes as f64 * 0.5) as u64;
    std::cmp::max(1, (ram_budget / mem_per_job.max(1)) as usize)
}

fn estimate_memory_per_job(stats: &PackageStats, profile: VerificationProfile) -> u64 {
    let wire = (stats.archive_bytes as f64 * 1.5) as u64;
    let manifest = stats.object_count as u64 * 4096;
    let base = wire.saturating_add(manifest).max(512 * 1024);
    match profile {
        VerificationProfile::CoreJsonParsedFast => base / 4,
        VerificationProfile::CoreJsonPortableFull => base.saturating_mul(2),
        _ => base,
    }
}

fn thread_candidates(
    hardware: &HardwareSnapshot,
    stats: &PackageStats,
    profile: VerificationProfile,
) -> Vec<usize> {
    let max_threads = threads_for_profile(
        hardware,
        stats,
        profile,
        VerifyIntent::MaxThroughput,
    );
    let mut out = Vec::new();
    let mut n = 1usize;
    while n <= max_threads {
        out.push(n);
        n *= 2;
    }
    if out.last().copied() != Some(max_threads) && max_threads > 1 {
        out.push(max_threads);
    }
    out.sort_unstable();
    out.dedup();
    if profile == VerificationProfile::CoreJsonPortableFull && stats.cost_tier == PackageCostTier::Large {
        out.retain(|&t| t <= 2);
    }
    out
}

fn probe_best_threads(
    profile: VerificationProfile,
    bytes: &[u8],
    config: &VerifierConfig,
    candidates: &[usize],
    probe_duration: Duration,
) -> usize {
    let mut best = candidates.first().copied().unwrap_or(1);
    let mut best_rate = 0.0f64;

    for &threads in candidates {
        let rate = measure_parallel_rate(profile, bytes, config, threads, probe_duration);
        if rate > best_rate {
            best_rate = rate;
            best = threads;
        }
    }
    best
}

fn measure_parallel_rate(
    profile: VerificationProfile,
    bytes: &[u8],
    config: &VerifierConfig,
    threads: usize,
    duration: Duration,
) -> f64 {
    if threads == 1 {
        return measure_single_rate(profile, bytes, config, duration);
    }

    let shared_bytes = Arc::new(bytes.to_vec());
    let shared_config = Arc::new(config.clone());
    let parsed = if profile == VerificationProfile::CoreJsonParsedFast {
        Some(Arc::new(parse_package(bytes).expect("probe package must parse")))
    } else {
        None
    };

    for _ in 0..3 {
        let _ = run_eval(profile, &shared_bytes, &shared_config, parsed.as_deref());
    }

    let counter = AtomicU64::new(0);
    let deadline = Instant::now() + duration;
    std::thread::scope(|scope| {
        for _ in 0..threads {
            let shared_bytes = Arc::clone(&shared_bytes);
            let shared_config = Arc::clone(&shared_config);
            let parsed = parsed.as_ref().map(Arc::clone);
            let counter = &counter;
            scope.spawn(move || {
                while Instant::now() < deadline {
                    if run_eval(profile, &shared_bytes, &shared_config, parsed.as_deref()) {
                        counter.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
        }
    });
    counter.load(Ordering::Relaxed) as f64 / duration.as_secs_f64()
}

fn measure_single_rate(
    profile: VerificationProfile,
    bytes: &[u8],
    config: &VerifierConfig,
    duration: Duration,
) -> f64 {
    let parsed = if profile == VerificationProfile::CoreJsonParsedFast {
        Some(parse_package(bytes).expect("probe package must parse"))
    } else {
        None
    };
    for _ in 0..3 {
        let _ = run_eval(profile, bytes, config, parsed.as_ref());
    }
    let deadline = Instant::now() + duration;
    let mut runs = 0u64;
    while Instant::now() < deadline {
        if run_eval(profile, bytes, config, parsed.as_ref()) {
            runs += 1;
        }
    }
    runs as f64 / duration.as_secs_f64()
}

fn run_eval(
    profile: VerificationProfile,
    bytes: &[u8],
    config: &VerifierConfig,
    parsed: Option<&ParsedPackage>,
) -> bool {
    match profile {
        VerificationProfile::CoreJsonPortableFull => {
            verify_package_bytes(bytes).overall == VerificationStatus::Pass
        }
        VerificationProfile::CoreJsonPortableFast => {
            verify_package_bytes_fast(bytes, config) == FastVerifyResult::Pass
        }
        VerificationProfile::CoreJsonParsedFast => {
            if let Some(parsed) = parsed {
                verify_parsed_package_fast(parsed, config) == FastVerifyResult::Pass
            } else {
                false
            }
        }
        _ => false,
    }
}

fn estimate_physical_cores(logical: usize) -> usize {
    if logical <= 1 {
        1
    } else if logical.is_multiple_of(2) {
        (logical / 2).max(1)
    } else {
        logical
    }
}

fn detect_usable_memory_bytes() -> u64 {
    // Conservative default when OS memory is not queried (8 GiB budget).
    8_u64 * 1024 * 1024 * 1024
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hw(logical: usize, mem_gib: u64) -> HardwareSnapshot {
        HardwareSnapshot {
            logical_cores: logical,
            physical_cores: (logical / 2).max(1),
            usable_memory_bytes: mem_gib * 1024 * 1024 * 1024,
        }
    }

    #[test]
    fn small_package_allows_more_threads() {
        let stats = PackageStats {
            archive_bytes: 64 * 1024,
            object_count: 10,
            declared_payload_bytes: 10 * 1024,
            cost_tier: PackageCostTier::Small,
        };
        let plan = VerifyPlan::recommend(&hw(24, 32), &stats, VerifyIntent::MaxThroughput);
        assert_eq!(plan.profile, VerificationProfile::CoreJsonParsedFast);
        assert!(plan.threads > 1);
    }

    #[test]
    fn large_forensic_stays_single_threaded() {
        let stats = PackageStats {
            archive_bytes: 32 * 1024 * 1024,
            object_count: 500,
            declared_payload_bytes: 32 * 1024 * 1024,
            cost_tier: PackageCostTier::Large,
        };
        let plan = VerifyPlan::recommend(&hw(24, 32), &stats, VerifyIntent::ForensicReport);
        assert_eq!(plan.profile, VerificationProfile::CoreJsonPortableFull);
        assert_eq!(plan.threads, 1);
    }

    #[test]
    fn memory_cap_limits_workers() {
        let stats = PackageStats {
            archive_bytes: 64 * 1024 * 1024,
            object_count: 100,
            declared_payload_bytes: 64 * 1024 * 1024,
            cost_tier: PackageCostTier::Large,
        };
        let tight = HardwareSnapshot {
            logical_cores: 24,
            physical_cores: 12,
            usable_memory_bytes: 128 * 1024 * 1024,
        };
        let plan = VerifyPlan::recommend(&tight, &stats, VerifyIntent::GatewayIngest);
        assert_eq!(plan.threads, 1);
    }

    #[test]
    fn cost_tier_boundaries() {
        assert_eq!(
            PackageCostTier::from_stats(128 * 1024, 10, 100 * 1024),
            PackageCostTier::Small
        );
        assert_eq!(
            PackageCostTier::from_stats(1024 * 1024, 100, 8 * 1024 * 1024),
            PackageCostTier::Medium
        );
        assert_eq!(
            PackageCostTier::from_stats(16 * 1024 * 1024, 400, 32 * 1024 * 1024),
            PackageCostTier::Large
        );
    }
}
