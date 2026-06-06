#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
web_root="$repo_root/web"

if [[ ! -f "$web_root/index.html" ]]; then
  echo "Missing web/index.html. Serve the web/ directory, not web/verifier/ or web/pkg/." >&2
  exit 1
fi

echo "Serving OSDF verifier from: $web_root"
echo "Open: http://localhost:8080/"
echo

cd "$web_root"
python -m http.server 8080
