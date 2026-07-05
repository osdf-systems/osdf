#!/usr/bin/env bash
# Reproducible CLI benchmarks: OSDF vs optional GPG (PGP) vs OpenTDF (TDF).
#
# Compares (all available bars land in ONE hyperfine export):
#   - OSDF full verify                           (always)
#   - GPG detached verify                        (optional: gpg on PATH)
#   - OpenTDF structural manifest read (offline) (default: golden .tdf fixture present)
#   - OpenTDF decrypt (otdfctl + platform)       (optional: otdfctl on PATH + $OTDF_PLATFORM)
#
# These are trust-model comparisons, NOT byte-for-byte equivalence. See docs/benchmarks.md.
# Requires: hyperfine (https://github.com/sharkdp/hyperfine), release osdf CLI.
# Optional: gpg, otdfctl + a running OpenTDF platform.
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

# --- PGP (GPG) comparison fixture: local ephemeral key, detached signature ---
if command -v gpg >/dev/null 2>&1; then
  head -c 65536 /dev/urandom >"$payload"
  if ! gpg --list-keys benchmark@osdf.local >/dev/null 2>&1; then
    cat >"$bench_dir/gpg-batch.txt" <<'EOF'
%no-protection
Key-Type: eddsa
Key-Curve: ed25519
Name-Real: OSDF Benchmark
Name-Email: benchmark@osdf.local
Expire-Date: 0
EOF
    gpg --batch --generate-key "$bench_dir/gpg-batch.txt"
  fi
  gpg --batch --yes --armor --detach-sign --local-user benchmark@osdf.local -o "$sig" "$payload"
fi

if ! command -v hyperfine >/dev/null 2>&1; then
  echo "Install hyperfine: https://github.com/sharkdp/hyperfine"
  echo "Running scale_bench sample instead..."
  cargo run --release -p osdf-core --example scale_bench -- --profile fast --objects 10 --bytes 1024 --threads 1 --seconds 5
  exit 0
fi

# --- OpenTDF structural reader: tiny in-repo helper (uses zip + serde_json only;
#     does NOT depend on unzip/jq being installed) ---
echo "Building OpenTDF structural reader example..."
cargo build --release -p osdf-core --example tdf_manifest_read -q
tdf_reader="$repo_root/target/release/examples/tdf_manifest_read"

# Pick a committed golden TDF fixture (small preferred; big is optional via fetch script).
tdf_fixture=""
for candidate in \
  "$repo_root/fixtures/benchmarks/opentdf/small-java-4.3.0-e0f8caf.tdf" \
  "$repo_root/fixtures/benchmarks/opentdf/big-java-4.3.0-e0f8caf.tdf"; do
  if [[ -f "$candidate" ]]; then
    tdf_fixture="$candidate"
    break
  fi
done

# --- Assemble the hyperfine command list. Only available bars are added so a
#     machine with just hyperfine + osdf still works. Each bar gets a clear -n label. ---
names=()
commands=()

names+=("OSDF full verify")
commands+=("\"$osdf\" verify \"$fixture\"")

if command -v gpg >/dev/null 2>&1 && [[ -f "$sig" ]]; then
  names+=("GPG detached verify")
  commands+=("gpg --batch --verify \"$sig\" \"$payload\"")
else
  echo "Skipping GPG comparison (gpg not installed or signing failed)."
fi

if [[ -x "$tdf_reader" && -n "$tdf_fixture" ]]; then
  names+=("OpenTDF structural manifest read (offline)")
  commands+=("\"$tdf_reader\" \"$tdf_fixture\"")
else
  echo "Skipping OpenTDF structural read (no golden .tdf fixture; run ./scripts/fetch-opentdf-fixtures.sh)."
fi

# Optional true-decrypt bar: requires otdfctl AND a configured platform endpoint.
if command -v otdfctl >/dev/null 2>&1 && [[ -n "${OTDF_PLATFORM:-}" && -n "$tdf_fixture" ]]; then
  # OTDF_DECRYPT_ARGS lets you pass auth flags, e.g.
  #   export OTDF_DECRYPT_ARGS="--tls-no-verify --with-client-creds-file creds.json"
  decrypt="otdfctl decrypt --host \"$OTDF_PLATFORM\" ${OTDF_DECRYPT_ARGS:-} -o \"${TMPDIR:-/tmp}/otdf-out.bin\" \"$tdf_fixture\""
  names+=("OpenTDF decrypt (otdfctl + platform)")
  commands+=("$decrypt")
else
  echo "Skipping OpenTDF decrypt (needs otdfctl on PATH and \$OTDF_PLATFORM set)."
fi

hyperfine_out="$out_dir/hyperfine-results.json"
summary_md="$out_dir/hyperfine-summary.md"

hyperfine_args=(--export-json "$hyperfine_out" --export-markdown "$summary_md" --warmup 5 --min-runs 10 --shell bash)
for i in "${!commands[@]}"; do
  hyperfine_args+=(-n "${names[$i]}" "${commands[$i]}")
done

hyperfine "${hyperfine_args[@]}"

echo ""
echo "Wrote $summary_md"
echo "Wrote $hyperfine_out"
echo "Criterion HTML: target/criterion/report/index.html (after cargo bench)"

if command -v cargo >/dev/null 2>&1; then
  echo "Running Criterion bench (quick sample)..."
  cargo bench -p osdf-core --bench verify_throughput || true
fi
