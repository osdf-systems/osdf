#!/usr/bin/env bash
# Download OpenTDF golden TDF fixtures for local benchmarks.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dest="$repo_root/fixtures/benchmarks/opentdf"
base="https://raw.githubusercontent.com/opentdf/tests/main/xtest/golden"

mkdir -p "$dest"

fetch() {
  local name="$1"
  local url="$base/$name"
  echo "Fetching $name ..."
  curl -fsSL "$url" -o "$dest/$name"
}

fetch "small-java-4.3.0-e0f8caf.tdf"
fetch "big-java-4.3.0-e0f8caf.tdf"

# NanoTDF spec vector (No Signature Example)
python3 - <<'PY' "$dest/spec-nosign.ntdf"
import base64, pathlib, sys
b64 = """
TDFMAQ9rYXMuZXhhbXBsZS5jb22ANQABHWthcy5leGFtcGxlLmNvbS9wb2xpY3kvYWJjZGVmYaoGjXbC
DfOlY3YzmGKfUjBy0IbUTUvmbiV04TvDLMcCKkzceqfvy6YDwZg/h3LvHRDoLg1ABvS93ZJ4eTVmcwPo
sz9EmnOSdxPUpKK05elFLi8FNDOdNZEb36Fe4Ys62wAAK1DknPqraRhSJhstY2CDGsvV8gP77xf5Rr7+
x57lEZugkjM7LA7qy54vjcg=
""".replace("\n", "").strip()
pathlib.Path(sys.argv[1]).write_bytes(base64.b64decode(b64))
print("Wrote", sys.argv[1])
PY

echo "Done. See fixtures/benchmarks/opentdf/README.md"
