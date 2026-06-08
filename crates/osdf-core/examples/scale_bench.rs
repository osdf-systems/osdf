//! Scaling throughput benchmark — **one profile per run**.
//!
//! ```text
//! cargo run --release -p osdf-core --example scale_bench -- \
//!   --profile full --objects 500 --bytes 65536 --threads 1 --seconds 10
//!
//! cargo run --release -p osdf-core --example scale_bench -- \
//!   --profile fast --objects 10 --bytes 1024 --threads 1 --seconds 10
//!
//! cargo run --release -p osdf-core --example scale_bench -- \
//!   --profile parsed --objects 500 --bytes 65536 --threads 8 --seconds 10
//! ```

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use osdf_core::constants::{MAX_ENTRIES, MAX_UNCOMPRESSED_BYTES};
use osdf_core::{
    commit_revision, create_package, generate_signing_key, parse_package,
    verify_package_bytes, verify_package_bytes_fast, verify_parsed_package_fast, write_package,
    CommitOptions, CreateOptions, FastVerifyResult, PackageContainer, VerificationProfile,
    VerificationStatus, VerifierConfig,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BenchProfile {
    Full,
    Fast,
    Parsed,
}

impl BenchProfile {
    fn verification_profile(self) -> VerificationProfile {
        match self {
            Self::Full => VerificationProfile::CoreJsonPortableFull,
            Self::Fast => VerificationProfile::CoreJsonPortableFast,
            Self::Parsed => VerificationProfile::CoreJsonParsedFast,
        }
    }

    fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "full" => Ok(Self::Full),
            "fast" => Ok(Self::Fast),
            "parsed" => Ok(Self::Parsed),
            other => Err(format!(
                "unknown profile `{other}` (expected full, fast, or parsed)"
            )),
        }
    }
}

#[derive(Debug, Clone)]
struct Config {
    profile: BenchProfile,
    object_count: usize,
    object_bytes: usize,
    threads: usize,
    seconds: f64,
    warmup: usize,
    write_path: Option<PathBuf>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("scale_bench: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = parse_args()?;
    validate_config(&config)?;

    eprintln!(
        "Profile: {} (ZIP parse each eval: {})",
        config.profile.verification_profile().label(),
        config.profile.verification_profile().parses_zip()
    );
    eprintln!(
        "Building signed package: {} objects × {} bytes payload …",
        config.object_count, config.object_bytes
    );

    let build_start = Instant::now();
    let container = build_scaled_package(config.object_count, config.object_bytes)?;
    let bytes = container.to_bytes().map_err(|err| err.to_string())?;
    let build_ms = build_start.elapsed().as_secs_f64() * 1000.0;

    let payload_bytes = estimate_uncompressed_bytes(config.object_count, config.object_bytes);
    eprintln!(
        "Package ready: {:.2} MB compressed on wire, ~{:.2} MB declared payload (+ metadata), built in {build_ms:.0} ms",
        bytes.len() as f64 / (1024.0 * 1024.0),
        payload_bytes as f64 / (1024.0 * 1024.0),
    );

    if let Some(path) = &config.write_path {
        write_package(&container, path).map_err(|err| err.to_string())?;
        eprintln!("Wrote fixture to {}", path.display());
    }

    sanity_check(&config.profile, &bytes)?;

    if config.profile == BenchProfile::Parsed {
        let parsed = parse_package(&bytes).map_err(|code| format!("parse failed: {code:?}"))?;
        eprintln!("\n--- Parsed-container fast revalidation (ZIP parsed once) ---");
        let single = bench_parsed_single(&parsed, payload_bytes, config.warmup, config.seconds)?;
        print_stats("1 thread", &single);
        if config.threads > 1 {
            let parallel = bench_parsed_parallel(
                &parsed,
                payload_bytes,
                config.threads,
                config.warmup,
                config.seconds,
            )?;
            print_stats(&format!("{} threads", config.threads), &parallel);
        }
        return Ok(());
    }

    eprintln!("\n--- Single-thread ---");
    let single = bench_bytes_single(&config.profile, &bytes, payload_bytes, config.warmup, config.seconds)?;
    print_stats("1 thread", &single);

    if config.threads > 1 {
        eprintln!("\n--- Parallel aggregate throughput ---");
        let parallel = bench_bytes_parallel(
            &config.profile,
            &bytes,
            payload_bytes,
            config.threads,
            config.warmup,
            config.seconds,
        )?;
        print_stats(&format!("{} threads", config.threads), &parallel);
        eprintln!(
            "Per-thread efficiency: {:.1}%",
            (parallel.evals_per_sec / config.threads as f64 / single.evals_per_sec) * 100.0
        );
    }

    Ok(())
}

fn sanity_check(profile: &BenchProfile, bytes: &[u8]) -> Result<(), String> {
    let verifier = VerifierConfig::default();
    match profile {
        BenchProfile::Full => {
            if verify_package_bytes(bytes).overall != VerificationStatus::Pass {
                return Err("sanity: full verify failed".to_string());
            }
        }
        BenchProfile::Fast | BenchProfile::Parsed => {
            if verify_package_bytes_fast(bytes, &verifier) != FastVerifyResult::Pass {
                return Err("sanity: fast verify failed".to_string());
            }
        }
    }
    Ok(())
}

struct BenchStats {
    runs: u64,
    elapsed: Duration,
    evals_per_sec: f64,
    ms_per_eval: f64,
    mb_hashed_per_sec: f64,
}

fn print_stats(label: &str, stats: &BenchStats) {
    println!("{label}:");
    println!("  runs:        {}", stats.runs);
    println!("  elapsed:     {:.2} s", stats.elapsed.as_secs_f64());
    println!("  evals/sec:   {:.0}", stats.evals_per_sec);
    println!("  ms/eval:     {:.3}", stats.ms_per_eval);
    if stats.mb_hashed_per_sec >= 100.0 {
        println!(
            "  payload MB/s (declared objects only): {:.1}",
            stats.mb_hashed_per_sec
        );
    }
}

fn bench_bytes_single(
    profile: &BenchProfile,
    bytes: &[u8],
    payload_bytes: u64,
    warmup: usize,
    seconds: f64,
) -> Result<BenchStats, String> {
    let config = VerifierConfig::default();
    for _ in 0..warmup {
        run_profile_once(profile, bytes, &config);
    }
    let deadline = Instant::now() + Duration::from_secs_f64(seconds);
    let mut runs = 0u64;
    while Instant::now() < deadline {
        if run_profile_pass(profile, bytes, &config) {
            runs += 1;
        }
    }
    Ok(stats_from_runs(runs, Duration::from_secs_f64(seconds), payload_bytes))
}

fn bench_bytes_parallel(
    profile: &BenchProfile,
    bytes: &[u8],
    payload_bytes: u64,
    threads: usize,
    warmup: usize,
    seconds: f64,
) -> Result<BenchStats, String> {
    let shared = Arc::new(bytes.to_vec());
    let config = Arc::new(VerifierConfig::default());
    for _ in 0..warmup {
        run_profile_once(profile, &shared, &config);
    }
    let counter = AtomicU64::new(0);
    let deadline = Instant::now() + Duration::from_secs_f64(seconds);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            let shared = Arc::clone(&shared);
            let config = Arc::clone(&config);
            let counter = &counter;
            scope.spawn(move || {
                while Instant::now() < deadline {
                    if run_profile_pass(profile, &shared, &config) {
                        counter.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
        }
    });
    Ok(stats_from_runs(
        counter.load(Ordering::Relaxed),
        Duration::from_secs_f64(seconds),
        payload_bytes,
    ))
}

fn bench_parsed_single(
    parsed: &osdf_core::ParsedPackage,
    payload_bytes: u64,
    warmup: usize,
    seconds: f64,
) -> Result<BenchStats, String> {
    let config = VerifierConfig::default();
    for _ in 0..warmup {
        let _ = verify_parsed_package_fast(parsed, &config);
    }
    let deadline = Instant::now() + Duration::from_secs_f64(seconds);
    let mut runs = 0u64;
    while Instant::now() < deadline {
        if verify_parsed_package_fast(parsed, &config) == FastVerifyResult::Pass {
            runs += 1;
        }
    }
    Ok(stats_from_runs(runs, Duration::from_secs_f64(seconds), payload_bytes))
}

fn bench_parsed_parallel(
    parsed: &osdf_core::ParsedPackage,
    payload_bytes: u64,
    threads: usize,
    warmup: usize,
    seconds: f64,
) -> Result<BenchStats, String> {
    let shared = Arc::new(parsed.clone());
    let config = Arc::new(VerifierConfig::default());
    for _ in 0..warmup {
        let _ = verify_parsed_package_fast(&shared, &config);
    }
    let counter = AtomicU64::new(0);
    let deadline = Instant::now() + Duration::from_secs_f64(seconds);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            let shared = Arc::clone(&shared);
            let config = Arc::clone(&config);
            let counter = &counter;
            scope.spawn(move || {
                while Instant::now() < deadline {
                    if verify_parsed_package_fast(&shared, &config) == FastVerifyResult::Pass {
                        counter.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
        }
    });
    Ok(stats_from_runs(
        counter.load(Ordering::Relaxed),
        Duration::from_secs_f64(seconds),
        payload_bytes,
    ))
}

fn run_profile_once(profile: &BenchProfile, bytes: &[u8], config: &VerifierConfig) {
    let _ = run_profile_pass(profile, bytes, config);
}

fn run_profile_pass(profile: &BenchProfile, bytes: &[u8], config: &VerifierConfig) -> bool {
    match profile {
        BenchProfile::Full => verify_package_bytes(bytes).overall == VerificationStatus::Pass,
        BenchProfile::Fast => verify_package_bytes_fast(bytes, config) == FastVerifyResult::Pass,
        BenchProfile::Parsed => unreachable!("parsed profile uses bench_parsed_*"),
    }
}

fn stats_from_runs(runs: u64, elapsed: Duration, payload_bytes: u64) -> BenchStats {
    let secs = elapsed.as_secs_f64();
    let evals_per_sec = runs as f64 / secs;
    BenchStats {
        runs,
        elapsed,
        evals_per_sec,
        ms_per_eval: (secs * 1000.0) / runs.max(1) as f64,
        mb_hashed_per_sec: (payload_bytes as f64 * evals_per_sec) / (1024.0 * 1024.0),
    }
}

fn payload_chunk(object_bytes: usize, index: usize) -> Vec<u8> {
    let mut out = vec![0u8; object_bytes];
    let mut state = (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for byte in &mut out {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *byte = state as u8;
    }
    out
}

fn build_scaled_package(
    object_count: usize,
    object_bytes: usize,
) -> Result<PackageContainer, String> {
    let signing_key = generate_signing_key();
    let mut container = create_package(CreateOptions {
        title: "Scale benchmark".to_string(),
        signing_key: Some(signing_key.clone()),
        commit: false,
        ..Default::default()
    })
    .map_err(|err| err.to_string())?;

    let payload = payload_chunk(object_bytes, 0);
    for index in 0..object_count {
        let chunk = if index == 0 {
            payload.clone()
        } else {
            payload_chunk(object_bytes, index)
        };
        container
            .insert(format!("payload/chunk-{index:06}.bin"), chunk)
            .map_err(|err| err.to_string())?;
    }

    commit_revision(
        &mut container,
        CommitOptions {
            signing_key,
            signer_key_reference: None,
        },
    )
    .map_err(|err| err.to_string())?;

    Ok(container)
}

fn estimate_uncompressed_bytes(object_count: usize, object_bytes: usize) -> u64 {
    object_count as u64 * object_bytes as u64
}

fn validate_config(config: &Config) -> Result<(), String> {
    if config.object_count == 0 {
        return Err("--objects must be >= 1".to_string());
    }
    if config.object_bytes == 0 {
        return Err("--bytes must be >= 1".to_string());
    }
    if config.threads == 0 {
        return Err("--threads must be >= 1".to_string());
    }
    if config.seconds <= 0.0 {
        return Err("--seconds must be > 0".to_string());
    }

    let reserved_entries = 8;
    if config.object_count + reserved_entries > MAX_ENTRIES {
        return Err(format!(
            "object count {} exceeds format limit (max {} declared objects)",
            config.object_count,
            MAX_ENTRIES.saturating_sub(reserved_entries)
        ));
    }

    let payload = estimate_uncompressed_bytes(config.object_count, config.object_bytes);
    if payload > MAX_UNCOMPRESSED_BYTES {
        return Err(format!(
            "declared payload {payload} bytes exceeds MAX_UNCOMPRESSED_BYTES ({MAX_UNCOMPRESSED_BYTES})"
        ));
    }

    Ok(())
}

fn parse_args() -> Result<Config, String> {
    let mut profile = BenchProfile::Full;
    let mut object_count = None;
    let mut object_bytes = None;
    let mut threads = 1usize;
    let mut seconds = 10.0f64;
    let mut warmup = 20usize;
    let mut write_path = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--profile" => {
                profile = BenchProfile::from_str(&parse_string(args.next(), "--profile")?)?;
            }
            "--objects" => {
                object_count = Some(parse_usize(args.next(), "--objects")?);
            }
            "--bytes" => {
                object_bytes = Some(parse_usize(args.next(), "--bytes")?);
            }
            "--threads" => {
                threads = parse_usize(args.next(), "--threads")?;
            }
            "--seconds" => {
                seconds = parse_f64(args.next(), "--seconds")?;
            }
            "--warmup" => {
                warmup = parse_usize(args.next(), "--warmup")?;
            }
            "--write" => {
                write_path = Some(PathBuf::from(parse_string(args.next(), "--write")?));
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument `{other}` (try --help)")),
        }
    }

    Ok(Config {
        profile,
        object_count: object_count.ok_or("missing required --objects")?,
        object_bytes: object_bytes.ok_or("missing required --bytes")?,
        threads,
        seconds,
        warmup,
        write_path,
    })
}

fn parse_usize(value: Option<String>, flag: &str) -> Result<usize, String> {
    value
        .ok_or_else(|| format!("{flag} requires a value"))?
        .parse()
        .map_err(|_| format!("{flag} must be a positive integer"))
}

fn parse_f64(value: Option<String>, flag: &str) -> Result<f64, String> {
    value
        .ok_or_else(|| format!("{flag} requires a value"))?
        .parse()
        .map_err(|_| format!("{flag} must be a number"))
}

fn parse_string(value: Option<String>, flag: &str) -> Result<String, String> {
    value.ok_or_else(|| format!("{flag} requires a value"))
}

fn print_help() {
    eprintln!(
        r#"scale_bench — verify throughput by explicit profile

Profiles (benchmark separately; do not mix labels):
  full    ZIP + full forensic VerificationReport (portable ingest baseline)
  fast    ZIP + compact FastVerifyResult (gateway allow/deny)
  parsed  parse ZIP once, then fast revalidation only (hot-path / cached inspect)

Usage:
  cargo run --release -p osdf-core --example scale_bench -- [OPTIONS]

Required:
  --objects N     Number of payload/chunk-*.bin objects
  --bytes N       Uncompressed bytes per payload object

Optional:
  --profile NAME  full | fast | parsed (default: full)
  --threads N     Parallel workers (default: 1)
  --seconds N     Duration per mode (default: 10)
  --warmup N      Warmup iterations (default: 20)
  --write PATH    Save generated .osdf fixture
  -h, --help

Examples:
  ... --profile full  --objects 500 --bytes 65536 --threads 1 --seconds 10
  ... --profile fast  --objects 10  --bytes 1024  --threads 1 --seconds 10
  ... --profile parsed --objects 500 --bytes 65536 --threads 8 --seconds 10
"#
    );
}
