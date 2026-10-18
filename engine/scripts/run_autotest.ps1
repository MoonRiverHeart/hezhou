param(
    [ValidateSet("random", "sequential", "stress", "intent", "visual")]
    [string]$Mode = "random",
    [int]$Steps = 50,
    [double]$Duration = 15,
    [double]$Interval = 0.2,
    [int]$Seed = 42,
    [string]$ReportDir = "autotest",
    [switch]$Build
)

$ErrorActionPreference = "Stop"
$EngineDir = Split-Path $PSScriptRoot -Parent
$Exe = Join-Path $EngineDir "target\release\mono_editor_demo.exe"

Push-Location $EngineDir
try {
    if ($Build) {
        cargo build -j 1 --release --features mono
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    }

    if (-not (Test-Path $Exe)) {
        throw "找不到 $Exe，请先运行: cargo build -j 1 --release --features mono"
    }

    if (Test-Path $ReportDir) {
        Remove-Item $ReportDir -Recurse -Force
    }

    $env:HEZHOU_AUTOTEST = "1"
    $env:HEZHOU_AUTOTEST_MODE = $Mode
    $env:HEZHOU_AUTOTEST_STEPS = $Steps.ToString()
    $env:HEZHOU_AUTOTEST_DURATION = $Duration.ToString()
    $env:HEZHOU_AUTOTEST_INTERVAL = $Interval.ToString()
    $env:HEZHOU_AUTOTEST_SEED = $Seed.ToString()
    $env:HEZHOU_AUTOTEST_DIR = $ReportDir
    $env:HEZHOU_AUTOTEST_SCREENSHOT_EVERY = "10"

    Write-Host "[AutoTest] mode=$Mode steps=$Steps duration=$Duration interval=$Interval seed=$Seed report=$ReportDir" -ForegroundColor Cyan
    & $Exe --auto-test
    $code = $LASTEXITCODE

    $summary = Join-Path $ReportDir "summary.txt"
    if (Test-Path $summary) {
        Get-Content $summary
    } else {
        Write-Host "[AutoTest] 未生成 summary.txt（可能启动/编译失败）" -ForegroundColor Red
        exit 2
    }

    if ($code -ne 0) {
        Write-Host "[AutoTest] FAILED (exit=$code)" -ForegroundColor Red
        exit $code
    }

    Write-Host "[AutoTest] PASS: $ReportDir\report.json" -ForegroundColor Green
    exit 0
}
finally {
    Pop-Location
    Remove-Item Env:HEZHOU_AUTOTEST, Env:HEZHOU_AUTOTEST_MODE, Env:HEZHOU_AUTOTEST_STEPS, Env:HEZHOU_AUTOTEST_DURATION, Env:HEZHOU_AUTOTEST_INTERVAL, Env:HEZHOU_AUTOTEST_SEED, Env:HEZHOU_AUTOTEST_DIR, Env:HEZHOU_AUTOTEST_SCREENSHOT_EVERY -ErrorAction SilentlyContinue
}
