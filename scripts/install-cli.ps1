# Install or refresh the osdf CLI on the user PATH.
#
# Usage:
#   .\scripts\install-cli.ps1                 # build release + install
#   .\scripts\install-cli.ps1 -CopyOnly -BinaryPath target\release\osdf.exe

[CmdletBinding()]
param(
    [switch]$CopyOnly,
    [string]$BinaryPath,
    [switch]$SkipPathUpdate
)

$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$ProjectTargetDir = Join-Path $ProjectRoot "target"
$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\osdf\bin"
$InstallExe = Join-Path $InstallDir "osdf.exe"

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

if (-not $CopyOnly) {
    Push-Location $ProjectRoot
    try {
        # Always build into the workspace target directory so installs are not
        # accidentally copied from a stale or sandbox-only cargo target.
        $env:CARGO_TARGET_DIR = $ProjectTargetDir

        Write-Host "Building osdf-cli (release)..." -ForegroundColor Cyan
        cargo build --release -p osdf-cli
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
    }
    $BinaryPath = Join-Path $ProjectTargetDir "release\osdf.exe"
}

if (-not $BinaryPath) {
    throw "BinaryPath is required when -CopyOnly is set."
}

if (-not (Test-Path -LiteralPath $BinaryPath)) {
    throw "Built CLI not found at $BinaryPath. Run without -CopyOnly to build first."
}

$BinaryPath = Resolve-Path -LiteralPath $BinaryPath

Copy-Item -LiteralPath $BinaryPath -Destination $InstallExe -Force
Write-Host "Installed $InstallExe" -ForegroundColor Green

if (-not $SkipPathUpdate) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ([string]::IsNullOrWhiteSpace($userPath)) {
        $userPath = ""
    }

    $pathEntries = $userPath -split ";" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    if ($pathEntries -notcontains $InstallDir) {
        $updatedPath = if ($userPath.Trim().Length -eq 0) {
            $InstallDir
        } else {
            "$userPath;$InstallDir"
        }
        [Environment]::SetEnvironmentVariable("Path", $updatedPath, "User")
        Write-Host "Added to user PATH: $InstallDir" -ForegroundColor Green
        Write-Host "Open a new terminal (or restart Cursor) for PATH changes to apply everywhere." -ForegroundColor Yellow
    } else {
        Write-Host "User PATH already contains: $InstallDir" -ForegroundColor DarkGray
    }

    if ($env:Path -notlike "*$InstallDir*") {
        $env:Path = "$env:Path;$InstallDir"
    }
}

& $InstallExe --version
Write-Host ""
& $InstallExe verify --help
