# Download OpenTDF golden TDF fixtures for local benchmarks.
$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$dest = Join-Path $repoRoot "fixtures\benchmarks\opentdf"
$base = "https://raw.githubusercontent.com/opentdf/tests/main/xtest/golden"

New-Item -ItemType Directory -Force -Path $dest | Out-Null

function Fetch-File($name) {
    $url = "$base/$name"
    $out = Join-Path $dest $name
    Write-Host "Fetching $name ..."
    Invoke-WebRequest -Uri $url -OutFile $out
}

Fetch-File "small-java-4.3.0-e0f8caf.tdf"
Fetch-File "big-java-4.3.0-e0f8caf.tdf"

$b64 = @"
TDFMAQ9rYXMuZXhhbXBsZS5jb22ANQABHWthcy5leGFtcGxlLmNvbS9wb2xpY3kvYWJjZGVmYaoGjXbC
DfOlY3YzmGKfUjBy0IbUTUvmbiV04TvDLMcCKkzceqfvy6YDwZg/h3LvHRDoLg1ABvS93ZJ4eTVmcwPo
sz9EmnOSdxPUpKK05elFLi8FNDOdNZEb36Fe4Ys62wAAK1DknPqraRhSJhstY2CDGsvV8gP77xf5Rr7+
x57lEZugkjM7LA7qy54vjcg=
"@
$ntdf = Join-Path $dest "spec-nosign.ntdf"
[IO.File]::WriteAllBytes($ntdf, [Convert]::FromBase64String(($b64 -replace '\s','')))
Write-Host "Wrote spec-nosign.ntdf"
Write-Host "Done. See fixtures/benchmarks/opentdf/README.md"
