#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

echo "Building OSDF WebAssembly verifier..."

wasm-pack build crates/osdf-wasm \
  --target web \
  --release \
  --out-dir ../../web/pkg

echo "WASM build complete: web/pkg"
