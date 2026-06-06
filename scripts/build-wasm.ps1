$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

Write-Host "Building OSDF WebAssembly verifier..."
wasm-pack build crates/osdf-wasm `
    --target web `
    --release `
    --out-dir ../../web/pkg

Write-Host "WASM build complete: web/pkg"
