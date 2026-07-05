# Reproducible CLI benchmarks: OSDF vs optional GPG (PGP) vs OpenTDF (TDF).
#
# Compares (all available bars land in ONE hyperfine export):
#   - OSDF full verify                          (always)
#   - GPG detached verify                       (optional: gpg on PATH)
#   - OpenTDF structural manifest read (offline) (default: golden .tdf fixture present)
#   - OpenTDF decrypt (otdfctl + platform)      (optional: otdfctl on PATH + $env:OTDF_PLATFORM)
#
# These are trust-model comparisons, NOT byte-for-byte equivalence. See docs/benchmarks.md.
# Requires: hyperfine, release osdf CLI. Optional: gpg, otdfctl + OpenTDF platform.
# Run from repo root: .\scripts\run-benchmarks.ps1

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

$outDir = Join-Path $repoRoot "docs\assets\benchmarks"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$benchDir = Join-Path $repoRoot "benchmarks"
New-Item -ItemType Directory -Force -Path $benchDir | Out-Null

function Find-OsdfCli {
    foreach ($path in @(
        (Join-Path $repoRoot "target\release\osdf.exe"),
        (Join-Path $repoRoot "target\debug\osdf.exe")
    )) {
        if (Test-Path $path) { return $path }
    }
    return $null
}

# Quote a single token so hyperfine (which runs each command string through a shell)
# survives paths that contain spaces.
function Quote-Token([string]$value) { '"' + $value + '"' }

Write-Host "Building release CLI..."
cargo build --release -p osdf-cli -q

$fixture = Join-Path $repoRoot "fixtures\valid\valid-committed.osdf"
if (-not (Test-Path $fixture)) {
    Write-Host "Generating fixtures..."
    cargo test -p osdf-core --test generate_fixtures write_fixtures -- --ignored -q
}

$osdf = Find-OsdfCli
if (-not $osdf) { throw "osdf CLI not found after build." }

$payload = Join-Path $benchDir "payload.bin"
$sig = Join-Path $benchDir "payload.bin.sig"

# --- PGP (GPG) comparison fixture: local ephemeral key, detached signature ---
$gpg = Get-Command gpg -ErrorAction SilentlyContinue
if ($gpg) {
    $bytes = New-Object byte[] 65536
    [System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($bytes)
    [System.IO.File]::WriteAllBytes($payload, $bytes)

    $batch = Join-Path $benchDir "gpg-batch.txt"
    @"
%no-protection
Key-Type: eddsa
Key-Curve: ed25519
Name-Real: OSDF Benchmark
Name-Email: benchmark@osdf.local
Expire-Date: 0
"@ | Set-Content -Path $batch -Encoding ascii

    $null = gpg --list-keys benchmark@osdf.local 2>$null
    if ($LASTEXITCODE -ne 0) {
        gpg --batch --generate-key $batch
    }
    gpg --batch --yes --armor --detach-sign --local-user benchmark@osdf.local -o $sig $payload
} else {
    Write-Host "Skipping GPG comparison (gpg not on PATH)."
}

$hyperfine = Get-Command hyperfine -ErrorAction SilentlyContinue
if (-not $hyperfine) {
    Write-Host "Install hyperfine: https://github.com/sharkdp/hyperfine"
    Write-Host "Running scale_bench sample instead..."
    cargo run --release -p osdf-core --example scale_bench -- --profile fast --objects 10 --bytes 1024 --threads 1 --seconds 5
    exit 0
}

# --- OpenTDF structural reader: tiny in-repo helper (uses zip + serde_json only;
#     does NOT depend on unzip/jq being installed on Windows) ---
Write-Host "Building OpenTDF structural reader example..."
cargo build --release -p osdf-core --example tdf_manifest_read -q
$tdfReader = Join-Path $repoRoot "target\release\examples\tdf_manifest_read.exe"

# Pick a committed golden TDF fixture (small preferred; big is optional via fetch script).
$tdfFixture = $null
foreach ($candidate in @(
    (Join-Path $repoRoot "fixtures\benchmarks\opentdf\small-java-4.3.0-e0f8caf.tdf"),
    (Join-Path $repoRoot "fixtures\benchmarks\opentdf\big-java-4.3.0-e0f8caf.tdf")
)) {
    if (Test-Path $candidate) { $tdfFixture = $candidate; break }
}

# --- Assemble the hyperfine command list. Only available bars are added so a
#     machine with just hyperfine + osdf still works. Each bar gets a clear -n label. ---
$names = @()
$commands = @()

$names += "OSDF full verify"
$commands += "$(Quote-Token $osdf) verify $(Quote-Token $fixture)"

if ($gpg -and (Test-Path $sig)) {
    $names += "GPG detached verify"
    $commands += "gpg --batch --verify $(Quote-Token $sig) $(Quote-Token $payload)"
} else {
    Write-Host "Skipping GPG comparison (gpg not installed or signing failed)."
}

if ((Test-Path $tdfReader) -and $tdfFixture) {
    $names += "OpenTDF structural manifest read (offline)"
    $commands += "$(Quote-Token $tdfReader) $(Quote-Token $tdfFixture)"
} else {
    Write-Host "Skipping OpenTDF structural read (no golden .tdf fixture; run .\scripts\fetch-opentdf-fixtures.ps1)."
}

# Optional true-decrypt bar: requires otdfctl AND a configured platform endpoint.
$otdfctl = Get-Command otdfctl -ErrorAction SilentlyContinue
if ($otdfctl -and $env:OTDF_PLATFORM -and $tdfFixture) {
    $extra = $env:OTDF_DECRYPT_ARGS  # e.g. "--tls-no-verify --with-client-creds-file creds.json"
    $decrypt = "otdfctl decrypt --host $(Quote-Token $env:OTDF_PLATFORM)"
    if ($extra) { $decrypt += " $extra" }
    $decrypt += " -o `"$([System.IO.Path]::GetTempPath())otdf-out.bin`" $(Quote-Token $tdfFixture)"
    $names += "OpenTDF decrypt (otdfctl + platform)"
    $commands += $decrypt
} else {
    Write-Host "Skipping OpenTDF decrypt (needs otdfctl on PATH and `$env:OTDF_PLATFORM set)."
}

$hyperfineOut = Join-Path $outDir "hyperfine-results.json"
$summaryMd = Join-Path $outDir "hyperfine-summary.md"

$hyperfineArgs = @("--export-json", $hyperfineOut, "--export-markdown", $summaryMd, "--warmup", "5", "--min-runs", "10")
for ($i = 0; $i -lt $commands.Count; $i++) {
    $hyperfineArgs += "-n"
    $hyperfineArgs += $names[$i]
    $hyperfineArgs += $commands[$i]
}
& hyperfine @hyperfineArgs

Write-Host ""
Write-Host "Wrote $summaryMd"
Write-Host "Wrote $hyperfineOut"
Write-Host "Criterion: cargo bench -p osdf-core --bench verify_throughput"
