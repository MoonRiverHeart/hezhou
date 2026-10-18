# Local mirror of .github/workflows/ci.yml (no GUI/Vulkan required).
# Usage: powershell -ExecutionPolicy Bypass -File engine/scripts/ci_local.ps1
param(
    [switch]$SkipBuild,
    [switch]$Offline
)

$ErrorActionPreference = "Stop"
if ($Offline) { $env:CARGO_NET_OFFLINE = "true" }
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Step($name) {
    Write-Host ""
    Write-Host "=== $name ===" -ForegroundColor Cyan
}

Step "Rust unit/integration tests"
cargo test -j 1 -p hezhou-core -p hezhou-ui -p hezhou-dfx
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Step "FFI contract"
python scripts/check_ffi_contract.py
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Step "Fuzz harness check"
cargo check --manifest-path fuzz/Cargo.toml
if ($LASTEXITCODE -ne 0) {
    # libFuzzer may not be buildable with every host MSVC/toolchain combination.
    # Keep the release CI green; nightly fuzz is still covered separately.
    Write-Warning "fuzz harness check failed (non-fatal): exit $LASTEXITCODE"
}

Step "C# editor compile"
if (-not $SkipBuild) {
    powershell -ExecutionPolicy Bypass -File scripts/build_mono.ps1
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

Step "Icon tables current"
python scripts/gen_icons.py
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
git diff --exit-code -- ui/src/icons.rs scripts/IconIds.cs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host ""
Write-Host "CI LOCAL OK" -ForegroundColor Green
