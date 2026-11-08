# Hezhou 0.3 性能基线 / 门禁（headless）
#
# 构建并运行 examples 里的 perf_smoke 基准，输出 JSON 报告并按内置阈值门禁。
# 详细指标与阈值说明见 docs/PERF_BASELINE_0_3.md。
#
# 用法（在仓库任意目录）：
#   pwsh -File engine/scripts/perf_baseline.ps1                     # release，CI 门禁
#   pwsh -File engine/scripts/perf_baseline.ps1 -Profile debug -Quick
#   pwsh -File engine/scripts/perf_baseline.ps1 -BaselineJson test-results/perf-baseline/report.json
#
# 退出码：0 全部通过；非 0 表示超阈值/运行失败/回归超限。

#Requires -Version 7.0

[CmdletBinding()]
param(
    # 构建 profile：release 用于 CI 门禁；debug 用于本地快速测量。
    [ValidateSet('release', 'debug')]
    [string]$Profile = 'release',

    # 报告输出路径（相对仓库根目录或绝对路径）；默认 test-results/perf-baseline/report.json。
    [string]$OutPath = '',

    # 阈值缩放倍率（>0 时覆盖内置阈值；调试阈值噪声时使用）。
    [double]$ThresholdScale = 0,

    # 测量轮数覆盖（默认由 perf_smoke 决定）。
    [int]$Rounds = 0,

    # 快速模式：缩减工作量，仅用于本地冒烟（不作为基线）。
    [switch]$Quick,

    # 不访问网络（本地/离线环境使用）。
    [switch]$Offline,

    # 可选：与历史报告 JSON 比较，任一指标超过 -MaxRegression 倍视为回归。
    [string]$BaselineJson = '',

    # 相对基线的允许回归倍率。
    [double]$MaxRegression = 1.5
)

$ErrorActionPreference = 'Stop'

$engineRoot = Split-Path -Parent $PSScriptRoot
$repoRoot = Split-Path -Parent $engineRoot

function Resolve-RepoPath([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path)) { return $null }
    if ([System.IO.Path]::IsPathRooted($Path)) { return $Path }
    return (Join-Path $repoRoot $Path)
}

$reportPath = Resolve-RepoPath $OutPath
if (-not $reportPath) {
    # release 用规范名 report.json（CI artifact）；debug 单独命名，避免覆盖 release 基线。
    $defaultName = if ($Profile -eq 'release') { 'report.json' } else { 'report-debug.json' }
    $reportPath = Join-Path $repoRoot 'test-results' 'perf-baseline' $defaultName
}
$reportDir = Split-Path -Parent $reportPath
New-Item -ItemType Directory -Force $reportDir | Out-Null

# 给报告注入 git commit（CI 优先使用 GITHUB_SHA）。
if (-not $env:GITHUB_SHA) {
    $git = Get-Command git -ErrorAction SilentlyContinue
    if ($git) {
        try {
            $commit = (& git -C $repoRoot rev-parse HEAD 2>$null)
            if ($LASTEXITCODE -eq 0 -and $commit) {
                $env:GIT_COMMIT = ($commit | Select-Object -First 1).Trim()
            }
        } catch {
            Write-Verbose "读取 git commit 失败（忽略）: $_"
        }
    }
}

$cargoArgs = @('run')
if ($Profile -eq 'release') { $cargoArgs += '--release' }
$cargoArgs += @(
    '-p', 'hezhou-examples', '--bin', 'perf_smoke',
    # headless：不启用 examples 的 mono feature，避免依赖 Mono/GPU 环境。
    '--no-default-features'
)
# CI 需要联网拉取依赖；本地可用 -Offline 走缓存。
if ($Offline) { $cargoArgs += '--offline' }
$cargoArgs += @(
    '--', '--json', $reportPath,
    '--profile', $(if ($Profile -eq 'release') { 'ci' } else { 'local' })
)
if ($ThresholdScale -gt 0) { $cargoArgs += @('--threshold-scale', "$ThresholdScale") }
if ($Rounds -gt 0) { $cargoArgs += @('--rounds', "$Rounds") }
if ($Quick) { $cargoArgs += '--quick' }

Write-Host "== Hezhou perf baseline =="
Write-Host "profile : $Profile"
Write-Host "report  : $reportPath"
Write-Host "command : cargo $($cargoArgs -join ' ')"

Push-Location $engineRoot
try {
    & cargo @cargoArgs
    $cargoExit = $LASTEXITCODE
} finally {
    Pop-Location
}

if ($cargoExit -ne 0) {
    Write-Host "perf_smoke 门禁失败（exit $cargoExit）。报告: $reportPath"
    exit $cargoExit
}

if (-not (Test-Path $reportPath)) {
    Write-Host "perf_smoke 未生成报告: $reportPath"
    exit 2
}

$report = Get-Content $reportPath -Raw | ConvertFrom-Json
Write-Host ''
Write-Host '--- perf report ---'
Write-Host ("build={0} gate={1} os={2} arch={3} cores={4}" -f `
        $report.machine.build_profile, $report.gate_profile, $report.machine.os, `
        $report.machine.arch, $report.machine.cpu_cores)

# GitHub Actions job summary（本地运行时不存在该变量，自然跳过）。
$summaryLines = @(
    '## Hezhou perf baseline',
    '',
    ("- build: ``{0}`` / gate: ``{1}`` / os: ``{2}`` / cores: {3}" -f `
            $report.machine.build_profile, $report.gate_profile, `
            $report.machine.os, $report.machine.cpu_cores),
    '',
    '| 指标 | 中位耗时 (ms) | 阈值 (ms) | 结果 |',
    '|---|---:|---:|---|'
)
foreach ($benchmark in $report.benchmarks) {
    $status = if ($benchmark.passed) { 'pass' } else { 'fail' }
    Write-Host ("[{0}] {1,-32} median {2,10:N3} ms  threshold {3,10:N3} ms  ({4})" -f `
            $status, $benchmark.name, [double]$benchmark.median_ms, `
            [double]$benchmark.threshold_ms, $benchmark.workload)
    $summaryLines += ("| {0} | {1:N3} | {2:N3} | {3} |" -f `
            $benchmark.name, [double]$benchmark.median_ms, `
            [double]$benchmark.threshold_ms, $status)
}
if ($env:GITHUB_STEP_SUMMARY) {
    $summaryLines += ''
    $summaryLines += ("report: ``{0}``" -f $reportPath)
    Add-Content -Path $env:GITHUB_STEP_SUMMARY -Value ($summaryLines -join "`n")
}

# 可选：与历史基线对比（同一台机器/同一构建 profile 才有意义）。
if ($BaselineJson) {
    $baselinePath = Resolve-RepoPath $BaselineJson
    if (-not (Test-Path $baselinePath)) {
        Write-Host "baseline 报告不存在: $baselinePath"
        exit 2
    }
    $baseline = Get-Content $baselinePath -Raw | ConvertFrom-Json
    $regressions = 0
    Write-Host ''
    Write-Host '--- baseline compare (-MaxRegression ' -NoNewline
    Write-Host ("{0}) ---" -f $MaxRegression)
    foreach ($current in $report.benchmarks) {
        $previous = $baseline.benchmarks | Where-Object { $_.name -eq $current.name } | Select-Object -First 1
        if (-not $previous) {
            Write-Host ("[skip] {0}: baseline 中没有该项" -f $current.name)
            continue
        }
        $old = [double]$previous.median_ms
        $new = [double]$current.median_ms
        $ratio = if ($old -gt 0) { $new / $old } else { 0.0 }
        $over = $ratio -gt $MaxRegression
        if ($over) { $regressions++ }
        Write-Host ("[{0}] {1,-32} {2,10:N3} -> {3,10:N3} ms  x{4:N2}" -f `
                $(if ($over) { 'slow' } else { 'ok' }), $current.name, $old, $new, $ratio)
    }
    if ($regressions -gt 0) {
        Write-Host "性能回归: $regressions 项超过 x$MaxRegression"
        exit 1
    }
}

Write-Host ''
Write-Host "perf baseline PASS（报告: $reportPath）"
exit 0
