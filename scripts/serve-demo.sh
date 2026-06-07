#!/usr/bin/env bash
# Serve gateway + browser verifier from repo root (port 8081).
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
port=8081
wasm_file="$repo_root/web/pkg/osdf_wasm_bg.wasm"

if [[ ! -f "$repo_root/gateway/index.html" ]]; then
  echo "Missing gateway/index.html. Run this script from the osdf repository." >&2
  exit 1
fi

if [[ ! -f "$wasm_file" ]]; then
  echo "WASM bundle not found. Building..."
  "$repo_root/scripts/build-wasm.sh"
fi

if command -v lsof >/dev/null 2>&1; then
  if lsof -Pi ":$port" -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo "Port $port is already in use. Stop the other server (Ctrl+C), then retry." >&2
    exit 1
  fi
fi

echo "Serving OSDF demo from: $repo_root"
echo "Gateway:  http://localhost:$port/gateway/"
echo "Verifier: http://localhost:$port/web/"
echo

cd "$repo_root"
if command -v python3 >/dev/null 2>&1; then
  python3 -m http.server "$port"
else
  python -m http.server "$port"
fi
