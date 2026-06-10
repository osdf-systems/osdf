#!/usr/bin/env bash
# Reproducible CLI benchmarks: OSDF vs optional GPG detached verify.
# Requires: hyperfine (https://github.com/sharkdp/hyperfine), release osdf CLI, optional gpg
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

out_dir="$repo_root/docs/assets/benchmarks"
mkdir -p "$out_dir"
bench_dir="$repo_root/benchmarks"
mkdir -p "$bench_dir"

find_osdf() {
  for candidate in "$repo_root/target/release/osdf" "$repo_root/target/debug/osdf"; do
    if [[ -x "$candidate" ]]; then
      echo "$candidate"
      return 0
    fi
  done
  return 1
}

echo "Building release CLI..."
cargo build --release -p osdf-cli -q

if [[ ! -f "$repo_root/fixtures/valid/valid-committed.osdf" ]]; then
  echo "Generating fixtures..."
  cargo test -p osdf-core --test generate_fixtures write_fixtures -- --ignored -q
fi

osdf="$(find_osdf)"
fixture="$repo_root/fixtures/valid/valid-committed.osdf"
payload="$bench_dir/payload.bin"
sig="$bench_dir/payload.bin.sig"

# PGP comparison fixture (local ephemeral key)
if command -v gpg >/dev/null 2>&1; then
  head -c 65536 /dev/urandom >"$payload"
  if ! gpg --list-keys benchmark@osdf.local >/dev/null 2>&1; then
    cat >"$bench_dir/gpg-batch.txt" <<'EOF'
%no-protection
Key-Type: Ed25519
Key-Curve: Ed25519
Name-Real: OSDF Benchmark
Name-Email: benchmark@osdf.local
Expire-Date: 0
EOF
    gpg --batch --generate-key "$bench_dir/gpg-batch.txt"
  fi
  gpg --batch --yes --armor --detach-sign --local-user benchmark@osdf.local -o "$sig" "$payload"
fi

commands=()
labels=()

commands+=("$osdf verify $fixture")
labels+=("OSDF full verify")

if command -v gpg >/dev/null 2>&1 && [[ -f "$sig" ]]; then
  commands+=("gpg --batch --verify $sig $payload")
  labels+=("GPG detached verify")
else
  echo "Skipping GPG comparison (gpg not installed or keygen failed)."
fi

hyperfine_out="$out_dir/hyperfine-results.json"
summary_md="$out_dir/hyperfine-summary.md"

hyperfine "${commands[@]}" \
  --export-json "$hyperfine_out" \
  --export-markdown "$summary_md" \
  --warmup 5 \
  --min-runs 10 \
  --shell bash

echo ""
echo "Wrote $summary_md"
echo "Wrote $hyperfine_out"
echo "Criterion HTML: target/criterion/report/index.html (after cargo bench)"

if command -v cargo >/dev/null 2>&1; then
  echo "Running Criterion bench (quick sample)..."
  cargo bench -p osdf-core --bench verify_throughput -- --sample-size 10 -q || true
fi
