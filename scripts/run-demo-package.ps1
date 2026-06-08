# Demonstration Package - narrated CLI walkthrough (Supplemental Plan section 23.1)
# Run from repo root: .\scripts\run-demo-package.ps1

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

function Write-Beat($n, $title) {
    Write-Host ""
    Write-Host "=== Step $n - $title ===" -ForegroundColor Cyan
}

function Find-OsdfCli {
    $candidates = @(
        (Join-Path $repoRoot "target\release\osdf.exe"),
        (Join-Path $repoRoot "target\debug\osdf.exe")
    )
    foreach ($path in $candidates) {
        if (Test-Path $path) { return $path }
    }
    return $null
}

$osdf = Find-OsdfCli
if (-not $osdf) {
    Write-Host "Building osdf CLI..." -ForegroundColor Yellow
    cargo build --release -p osdf-cli
    $osdf = Find-OsdfCli
    if (-not $osdf) { throw "Could not locate osdf.exe after build." }
}

Write-Host "OSDF Demonstration Package (partial - Month 1 foundation)" -ForegroundColor Green
Write-Host "CLI: $osdf"
Write-Host "Full plan: specs/demo-package.md"

$ledgerTrust = Join-Path $repoRoot "fixtures\valid\ledger-trust.json"
$identityTrust = Join-Path $repoRoot "fixtures\valid\identity-trust.json"
$rev1 = Join-Path $repoRoot "fixtures\valid\valid-with-ledger-proof.osdf"
$rev2 = Join-Path $repoRoot "fixtures\valid\valid-rev2-with-ledger-proof.osdf"
$tampered = Join-Path $repoRoot "fixtures\invalid\undeclared-object.osdf"
$taxRev1 = Join-Path $repoRoot "fixtures\valid\taxes-template.osdf"
$taxRev2 = Join-Path $repoRoot "fixtures\valid\Taxes.osdf"

foreach ($path in @($ledgerTrust, $rev1, $rev2, $tampered)) {
    if (-not (Test-Path $path)) {
        Write-Host "Missing fixture: $path" -ForegroundColor Red
        Write-Host "Run: cargo test -p osdf-core --test generate_fixtures write_fixtures -- --ignored"
        exit 1
    }
}

Write-Beat 1 "Create contract revision 1 (fixture stand-in)"
Write-Host "Demo uses pre-built packages. Production demo will run: osdf create contract-rev1.osdf --commit"
if (Test-Path $taxRev1) {
    & $osdf inspect $taxRev1
} else {
    & $osdf inspect $rev1
}

Write-Beat 2 "Sign revision 1"
Write-Host "Signatures verified in next step (PASS = signed + chain valid)."

Write-Beat 3 "Submit to trusted log"
Write-Host "Ledger proof embedded in valid-with-ledger-proof.osdf (from osdf ledger append + attach-proof)."
Write-Host "Trust registry: fixtures/valid/ledger-trust.json"

Write-Beat 4 "Gateway send event [Month 2 - not built]"
Write-Host "Planned: signed DOCUMENT_TRANSMITTED event + timeline entry."

Write-Beat 5 "Gateway receipt event [Month 2 - not built]"
Write-Host "Planned: signed DOCUMENT_RECEIVED event + timeline entry."

Write-Beat 6 "Create revision 2"
Write-Host "Demo package rev2 (ledger + latest revision confirmed):"
& $osdf verify $rev2 --ledger-config $ledgerTrust

Write-Beat 7 "Tamper with a fixture"
Write-Host "Undeclared object attack (should FAIL verification):"
& $osdf verify $tampered

Write-Beat 8 "Quarantine tampered file [Month 2 - not built]"
Write-Host "Planned: osdf gateway inspect -> verdict QUARANTINE (HTTP POST /v1/inspect)."
Write-Host "Today: verification FAIL above is the crypto gate; gateway policy wrapper is next."

Write-Beat "9a" "Outdated but authentic (rollback detection)"
Write-Host "Revision 1 with ledger trust - expect WARNING + OSDF_LATEST_REVISION_OUTDATED:"
& $osdf verify $rev1 --ledger-config $ledgerTrust

Write-Beat "9b" "Forensic report export [partial]"
Write-Host "Browser verifier: drop file at http://localhost:8081/web/ and use Export report."
Write-Host "CLI JSON: osdf verify <file> --ledger-config ... --json"

Write-Host ""
Write-Host "--- Summary ---" -ForegroundColor Green
Write-Host "WORKING NOW:"
Write-Host "  - Signed revision chain + ledger inclusion proofs"
Write-Host "  - Latest-revision OUTDATED vs CONFIRMED (offline registry)"
Write-Host "  - Tamper detection (FAIL closed)"
Write-Host "  - Browser verifier + gateway tax viewer"
Write-Host ""
Write-Host "NEXT (Month 2 - the magnet):"
Write-Host "  - osdf gateway inspect + policy YAML + QUARANTINE verdict"
Write-Host "  - Transmission timeline events (send / receive)"
Write-Host ""
Write-Host "See specs/demo-package.md for the full 3-month checklist."
