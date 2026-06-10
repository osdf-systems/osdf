//! Criterion throughput benchmarks for OSDF verification profiles.
//!
//! Run: `cargo bench -p osdf-core --bench verify_throughput`

use std::path::PathBuf;
use std::sync::Arc;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use osdf_core::{
    create_package, generate_signing_key, parse_package, verify_package_bytes,
    verify_package_bytes_fast, verify_parsed_package_fast, CreateOptions, FastVerifyResult,
    VerifierConfig, VerificationStatus,
};

fn fixture_bytes() -> Vec<u8> {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/valid/valid-committed.osdf");
    if fixture.is_file() {
        return std::fs::read(fixture).expect("read fixture");
    }

    let signing_key = generate_signing_key();
    let container = create_package(CreateOptions {
        title: "Bench fixture".to_string(),
        signing_key: Some(signing_key),
        commit: true,
        ..Default::default()
    })
    .expect("create bench package");
    container.to_bytes().expect("serialize bench package")
}

fn bench_profiles(c: &mut Criterion) {
    let bytes = fixture_bytes();
    let config = VerifierConfig::default();
    let parsed = parse_package(&bytes).expect("parse bench package");

    let mut group = c.benchmark_group("verify_profile");
    group.throughput(Throughput::Bytes(bytes.len() as u64));

    group.bench_function("full_report", |b| {
        b.iter(|| {
            let report = verify_package_bytes(black_box(&bytes));
            assert_eq!(report.overall, VerificationStatus::Pass);
        });
    });

    group.bench_function("portable_fast", |b| {
        b.iter(|| {
            let result = verify_package_bytes_fast(black_box(&bytes), black_box(&config));
            assert_eq!(result, FastVerifyResult::Pass);
        });
    });

    group.bench_function("parsed_fast", |b| {
        b.iter(|| {
            let result = verify_parsed_package_fast(black_box(&parsed), black_box(&config));
            assert_eq!(result, FastVerifyResult::Pass);
        });
    });

    group.finish();
}

fn bench_parallel_parsed(c: &mut Criterion) {
    let bytes = fixture_bytes();
    let config = Arc::new(VerifierConfig::default());
    let parsed = Arc::new(parse_package(&bytes).expect("parse bench package"));

    let mut group = c.benchmark_group("parsed_parallel");
    for threads in [1usize, 2, 4, 8] {
        group.bench_with_input(BenchmarkId::new("threads", threads), &threads, |b, &threads| {
            b.iter(|| {
                std::thread::scope(|scope| {
                    for _ in 0..threads {
                        let parsed = Arc::clone(&parsed);
                        let config = Arc::clone(&config);
                        scope.spawn(move || {
                            for _ in 0..50 {
                                let result =
                                    verify_parsed_package_fast(&parsed, config.as_ref());
                                assert_eq!(result, FastVerifyResult::Pass);
                            }
                        });
                    }
                });
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_profiles, bench_parallel_parsed);
criterion_main!(benches);
