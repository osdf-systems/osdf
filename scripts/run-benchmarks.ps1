# Reproducible CLI benchmarks: OSDF vs optional GPG detached verify.
# Requires: hyperfine, release osdf CLI, optional gpg
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

$commands = @(
    @("$osdf", "verify", $fixture)
)
$gpg = Get-Command gpg -ErrorAction SilentlyContinue
if ($gpg) {
    $bytes = New-Object byte[] 65536
    [System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($bytes)
    [System.IO.File]::WriteAllBytes($payload, $bytes)

    $batch = Join-Path $benchDir "gpg-batch.txt"
    @"
%no-protection
Key-Type: Ed25519
Key-Curve: Ed25519
Name-Real: OSDF Benchmark
Name-Email: benchmark@osdf.local
Expire-Date: 0
"@ | Set-Content -Path $batch -Encoding ascii

    $null = gpg --list-keys benchmark@osdf.local 2>$null
    if ($LASTEXITCODE -ne 0) {
        gpg --batch --generate-key $batch
    }
    gpg --batch --yes --armor --detach-sign --local-user benchmark@osdf.local -o $sig $payload
    $commands += ,@("gpg", "--batch", "--verify", $sig, $payload)
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

$hyperfineOut = Join-Path $outDir "hyperfine-results.json"
$summaryMd = Join-Path $outDir "hyperfine-summary.md"

$hyperfineArgs = @("--export-json", $hyperfineOut, "--export-markdown", $summaryMd, "--warmup", "5", "--min-runs", "10")
foreach ($cmd in $commands) {
    $hyperfineArgs += $cmd
}
& hyperfine @hyperfineArgs

Write-Host ""
Write-Host "Wrote $summaryMd"
Write-Host "Criterion: cargo bench -p osdf-core --bench verify_throughput"
