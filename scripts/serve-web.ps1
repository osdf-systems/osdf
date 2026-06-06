$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$webRoot = Join-Path $repoRoot "web"
$wasmFile = Join-Path $webRoot "pkg\osdf_wasm_bg.wasm"
$port = 8080

if (-not (Test-Path (Join-Path $webRoot "index.html"))) {
    Write-Error "Missing web/index.html. Run this script from the osdf repository."
}

if (-not (Test-Path $wasmFile)) {
    Write-Host "WASM bundle not found. Building..."
    & (Join-Path $repoRoot "scripts\build-wasm.ps1")
    if (-not (Test-Path $wasmFile)) {
        Write-Error "WASM build did not produce web/pkg/osdf_wasm_bg.wasm"
    }
}

$existing = Get-NetTCPConnection -LocalPort $port -ErrorAction SilentlyContinue | Select-Object -First 1
if ($existing) {
    Write-Host "Port $port is already in use (PID $($existing.OwningProcess))."
    Write-Host "Stop the old server with Ctrl+C in that terminal, then run this script again."
    Write-Host ""
    Write-Host "If the old server used web/verifier/, it will 404 after the Phase B move to web/."
    exit 1
}

Write-Host "Serving OSDF verifier from: $webRoot"
Write-Host "Open: http://localhost:$port/"
Write-Host ""

Set-Location $webRoot
python -m http.server $port
