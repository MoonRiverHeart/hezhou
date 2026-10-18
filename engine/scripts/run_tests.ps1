param(
    [switch]$Offline,
    [switch]$SkipBuild,
    [switch]$SkipStress
)

$ErrorActionPreference = "Stop"
$EngineDir = Split-Path $PSScriptRoot -Parent
$ResultsDir = Join-Path $EngineDir "test-results"

Push-Location $EngineDir
try {
    New-Item -ItemType Directory -Path $ResultsDir -Force | Out-Null

    # glfw-sys 的 CMake/MSBuild 在 Debug 并行构建下会偶发文件锁失败。
    $env:CMAKE_BUILD_PARALLEL_LEVEL = "1"

    Write-Host "=== [1/5] Rust unit/integration tests ===" -ForegroundColor Cyan
    # glfw-sys 的 MSBuild 在并行构建下不稳定，统一使用单线程。
    if ($Offline) {
        & cargo test -j 1 -p hezhou-core -p hezhou-dfx -p hezhou-ui --offline
    } else {
        & cargo test -j 1 -p hezhou-core -p hezhou-dfx -p hezhou-ui
    }
    if ($LASTEXITCODE -ne 0) { throw "cargo test failed" }

    Write-Host "=== [2/5] Fuzz smoke (deterministic) ===" -ForegroundColor Cyan
    if ($Offline) {
        & cargo test -j 1 -p hezhou-core --test project_fuzz_smoke --offline
    } else {
        & cargo test -j 1 -p hezhou-core --test project_fuzz_smoke
    }
    if ($LASTEXITCODE -ne 0) { throw "fuzz smoke failed" }

    Write-Host "=== [3/5] FFI contract check ===" -ForegroundColor Cyan
    & python "scripts/check_ffi_contract.py"
    if ($LASTEXITCODE -ne 0) { throw "FFI contract check failed" }

    if (-not $SkipBuild) {
        Write-Host "=== [4/5] Build C# scripts ===" -ForegroundColor Cyan
        & powershell -ExecutionPolicy Bypass -File "scripts/build_mono.ps1"
        if ($LASTEXITCODE -ne 0) { throw "C# build failed" }
    } else {
        Write-Host "=== [4/5] C# build skipped ===" -ForegroundColor DarkGray
    }

    Write-Host "=== [5/5] End-to-end automated click tests ===" -ForegroundColor Cyan
    & powershell -ExecutionPolicy Bypass -File "scripts/run_autotest.ps1" -Mode random -Steps 30 -Duration 10 -Interval 0.2 -Seed 42 -ReportDir "test-results/random"
    if ($LASTEXITCODE -ne 0) { throw "random autotest failed" }
    & python "scripts/check_autotest_report.py" "test-results/random"
    if ($LASTEXITCODE -ne 0) { throw "random report validation failed" }

    & powershell -ExecutionPolicy Bypass -File "scripts/run_autotest.ps1" -Mode intent -ReportDir "test-results/intent" -Duration 1 -Steps 1
    if ($LASTEXITCODE -ne 0) { throw "intent autotest failed" }
    & python "scripts/check_autotest_report.py" "test-results/intent"
    if ($LASTEXITCODE -ne 0) { throw "intent report validation failed" }

    if (-not $SkipStress) {
        & powershell -ExecutionPolicy Bypass -File "scripts/run_autotest.ps1" -Mode stress -Duration 8 -Interval 0.1 -Seed 7 -ReportDir "test-results/stress"
        if ($LASTEXITCODE -ne 0) { throw "stress autotest failed" }
        & python "scripts/check_autotest_report.py" "test-results/stress"
        if ($LASTEXITCODE -ne 0) { throw "stress report validation failed" }
    }

    Write-Host "=== ALL TESTS PASSED ===" -ForegroundColor Green
    Write-Host "Reports: test-results/random, test-results/intent, test-results/stress"
    exit 0
}
catch {
    Write-Host "=== TEST FAILED: $($_.Exception.Message) ===" -ForegroundColor Red
    exit 1
}
finally {
    Pop-Location
}
