# Serve gateway + browser verifier from repo root (no marketing/docs-site).
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$port = 8081

if (-not (Test-Path (Join-Path $repoRoot "gateway\index.html"))) {
    Write-Error "Missing gateway/index.html. Run this script from the osdf repository."
}

$existing = Get-NetTCPConnection -LocalPort $port -ErrorAction SilentlyContinue | Select-Object -First 1
if ($existing) {
    Write-Host "Port $port is already in use (PID $($existing.OwningProcess))."
    exit 1
}

Write-Host "Serving OSDF demo from: $repoRoot"
Write-Host "Gateway:  http://localhost:$port/gateway/"
Write-Host "Verifier: http://localhost:$port/web/"
Write-Host ""

Set-Location $repoRoot
python -m http.server $port
