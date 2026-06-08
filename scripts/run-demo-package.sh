#!/usr/bin/env bash
# Demonstration Package - narrated CLI walkthrough (Supplemental Plan section 23.1)
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

write_beat() {
  echo
  echo "=== Step $1 - $2 ==="
}

find_osdf_cli() {
  local candidate
  for candidate in "$repo_root/target/release/osdf" "$repo_root/target/debug/osdf"; do
    if [[ -x "$candidate" ]]; then
      echo "$candidate"
      return 0
    fi
  done
  return 1
}

osdf="$(find_osdf_cli || true)"
if [[ -z "$osdf" ]]; then
  echo "Building osdf CLI..."
  cargo build --release -p osdf-cli
  osdf="$(find_osdf_cli || true)"
  if [[ -z "$osdf" ]]; then
    echo "Could not locate osdf binary after build." >&2
    exit 1
  fi
fi

ledger_trust="$repo_root/fixtures/valid/ledger-trust.json"
rev1="$repo_root/fixtures/valid/valid-with-ledger-proof.osdf"
rev2="$repo_root/fixtures/valid/valid-rev2-with-ledger-proof.osdf"
tampered="$repo_root/fixtures/invalid/undeclared-object.osdf"
tax_rev1="$repo_root/fixtures/valid/taxes-template.osdf"

echo "OSDF Demonstration Package (partial - Month 1 foundation)"
echo "CLI: $osdf"
echo "Full plan: specs/demo-package.md"

for path in "$ledger_trust" "$rev1" "$rev2" "$tampered"; do
  if [[ ! -f "$path" ]]; then
    echo "Missing fixture: $path" >&2
    echo "Run: cargo test -p osdf-core --test generate_fixtures write_fixtures -- --ignored"
    exit 1
  fi
done

write_beat 1 "Create contract revision 1 (fixture stand-in)"
echo "Demo uses pre-built packages. Production demo will run: osdf create contract-rev1.osdf --commit"
if [[ -f "$tax_rev1" ]]; then
  "$osdf" inspect "$tax_rev1"
else
  "$osdf" inspect "$rev1"
fi

write_beat 2 "Sign revision 1"
echo "Signatures verified in next step (PASS = signed + chain valid)."

write_beat 3 "Submit to trusted log"
echo "Ledger proof embedded in valid-with-ledger-proof.osdf (from osdf ledger append + attach-proof)."
echo "Trust registry: fixtures/valid/ledger-trust.json"

write_beat 4 "Gateway send event [Month 2 - not built]"
echo "Planned: signed DOCUMENT_TRANSMITTED event + timeline entry."

write_beat 5 "Gateway receipt event [Month 2 - not built]"
echo "Planned: signed DOCUMENT_RECEIVED event + timeline entry."

write_beat 6 "Create revision 2"
echo "Demo package rev2 (ledger + latest revision confirmed):"
"$osdf" verify "$rev2" --ledger-config "$ledger_trust"

write_beat 7 "Tamper with a fixture"
echo "Undeclared object attack (should FAIL verification):"
"$osdf" verify "$tampered"

write_beat 8 "Quarantine tampered file [Month 2 - not built]"
echo "Planned: osdf gateway inspect -> verdict QUARANTINE (HTTP POST /v1/inspect)."
echo "Today: verification FAIL above is the crypto gate; gateway policy wrapper is next."

write_beat "9a" "Outdated but authentic (rollback detection)"
echo "Revision 1 with ledger trust - expect WARNING + OSDF_LATEST_REVISION_OUTDATED:"
"$osdf" verify "$rev1" --ledger-config "$ledger_trust"

write_beat "9b" "Forensic report export [partial]"
echo "Browser verifier: drop file at http://localhost:8081/web/ and use Export report."
echo "CLI JSON: osdf verify <file> --ledger-config ... --json"

echo
echo "--- Summary ---"
echo "WORKING NOW:"
echo "  - Signed revision chain + ledger inclusion proofs"
echo "  - Latest-revision OUTDATED vs CONFIRMED (offline registry)"
echo "  - Tamper detection (FAIL closed)"
echo "  - Browser verifier + gateway tax viewer"
echo
echo "NEXT (Month 2 - the magnet):"
echo "  - osdf gateway inspect + policy YAML + QUARANTINE verdict"
echo "  - Transmission timeline events (send / receive)"
echo
echo "See specs/demo-package.md for the full 3-month checklist."
