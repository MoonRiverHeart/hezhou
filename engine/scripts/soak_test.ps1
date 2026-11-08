# Hezhou 1.0 长时间稳定性 soak 测试（headless）
#
# 运行 examples 里的 soak_smoke：固定 tick 驱动 ECS/物理/动画/Player-ish 循环，
# 采样 RSS 与每 tick 耗时，输出 JSON 并做内存/帧时间门禁。
# 详细指标与阈值见 docs/SOAK_TEST_1_0.md。
#
# 用法（在仓库任意目录）：
#   pwsh -File engine/scripts/soak_test.ps1                     # smoke：60s release
#   pwsh -File engine/scripts/soak_test.ps1 -Mode long          # 300s
#   pwsh -File engine/scripts/soak_test.ps1 -Mode soak          # 1800s（30 分钟）
#   pwsh -File engine/scripts/soak_test.ps1 -Quick              # CI 快速模式
#   pwsh -File engine/scripts/soak_test.ps1 -Seconds 120 -Profile debug
#
# 退出码：0 通过；1 超阈值；2 构建/报告错误；3 超时被终止。

#Requires -Version 7.0

[CmdletBinding()]
param(
    # 预设模式：smoke/ci=60s，long=300s，soak=1800s。
    [ValidateSet('smoke', 'ci', 'long', 'soak')]
    [string]$Mode = 'smoke',

    # 测量时长（秒）；>0 时覆盖 Mode。
    [double]$Seconds = 0,

    # 预热时长（秒）；<0 时使用默认（quick=1s，否则 2s）。
    [double]$Warmup = -1,

    # 目标 tick rate（Hz）。
    [double]$TickRate = 60,

    # RSS 采样间隔（秒）。
    [double]$MemoryInterval = 1,

    # 构建 profile。
    [ValidateSet('release', 'debug')]
    [string]$Profile = 'release',

    # 报告输出路径（相对仓库根目录或绝对路径）。
    [string]$OutPath = '',

    # 快速模式：缩减 ECS/物理/动画工作量（CI quick soak 使用；时长仍按 Mode）。
    [switch]$Quick,

    # 不访问网络（本地/离线环境使用）。
    [switch]$Offline,

    # 超时秒数；<=0 时 = 预热 + 时长 + 180s。
    [int]$TimeoutSeconds = 0,

    # 阈值的显式覆盖（<0 表示使用 profile 默认值）。
    [double]$MaxGrowthPercent = -1,
    [double]$MaxGrowthMb = -1,
    [double]$MinGrowthMb = -1,
    [double]$MaxRssMb = -1,
    [double]$MaxAvgFrameMs = -1,
    [double]$MaxP95FrameMs = -1
)

$ErrorActionPreference = 'Stop'

$engineRoot = Split-Path -Parent $PSScriptRoot
$repoRoot = Split-Path -Parent $engineRoot

function Resolve-RepoPath([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path)) { return $null }
    if ([System.IO.Path]::IsPathRooted($Path)) { return $Path }
    return (Join-Path $repoRoot $Path)
}

# ---- 时长 / 默认值 ----
if ($Seconds -le 0) {
    $Seconds = switch ($Mode) {
        'smoke' { 60 }
        'ci' { 60 }
        'long' { 300 }
        'soak' { 1800 }
        default { 60 }
    }
}
if ($Warmup -lt 0) {
    $Warmup = if ($Quick) { 1 } else { 2 }
}
$timeout = if ($TimeoutSeconds -gt 0) { $TimeoutSeconds } else { [int][Math]::Ceiling($Warmup + $Seconds + 180) }

# ---- 报告路径 ----
$reportPath = Resolve-RepoPath $OutPath
if (-not $reportPath) {
    $defaultName = if ($Profile -eq 'release') { 'report.json' } else { 'report-debug.json' }
    $reportPath = Join-Path $repoRoot 'test-results' 'soak' $defaultName
}
$reportDir = Split-Path -Parent $reportPath
New-Item -ItemType Directory -Force $reportDir | Out-Null

# ---- git commit 注入 ----
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

# ---- 构建 ----
$cargoArgs = @('build', '-p', 'hezhou-examples', '--bin', 'soak_smoke', '--no-default-features')
if ($Profile -eq 'release') { $cargoArgs += '--release' }
if ($Offline) { $cargoArgs += '--offline' }

Write-Host '== Hezhou soak test =='
Write-Host "mode    : $Mode（${Seconds}s, warmup ${Warmup}s, tick ${TickRate}Hz, quick=$($Quick.IsPresent)）"
Write-Host "profile : $Profile"
Write-Host "report  : $reportPath"
Write-Host "timeout : ${timeout}s"
Write-Host "command : cargo $($cargoArgs -join ' ')"

Push-Location $engineRoot
try {
    & cargo @cargoArgs
    $buildExit = $LASTEXITCODE
} finally {
    Pop-Location
}
if ($buildExit -ne 0) {
    Write-Host "soak_smoke 构建失败（exit $buildExit）"
    exit 2
}

$targetDir = if ($Profile -eq 'release') { 'release' } else { 'debug' }
$exeName = if ($env:OS -eq 'Windows_NT') { 'soak_smoke.exe' } else { 'soak_smoke' }
$exePath = Join-Path $engineRoot "target/$targetDir/$exeName"
if (-not (Test-Path $exePath)) {
    Write-Host "找不到 soak_smoke 可执行文件: $exePath"
    exit 2
}

$runArgs = @(
    '--seconds', "$Seconds",
    '--warmup', "$Warmup",
    '--tick-rate', "$TickRate",
    '--memory-interval', "$MemoryInterval",
    '--json', $reportPath,
    '--profile', $(if ($Profile -eq 'release') { 'ci' } else { 'local' })
)
if ($Quick) { $runArgs += '--quick' }
if ($MaxGrowthPercent -ge 0) { $runArgs += @('--max-growth-percent', "$MaxGrowthPercent") }
if ($MaxGrowthMb -ge 0) { $runArgs += @('--max-growth-mb', "$MaxGrowthMb") }
if ($MinGrowthMb -ge 0) { $runArgs += @('--min-growth-mb', "$MinGrowthMb") }
if ($MaxRssMb -ge 0) { $runArgs += @('--max-rss-mb', "$MaxRssMb") }
if ($MaxAvgFrameMs -ge 0) { $runArgs += @('--max-avg-frame-ms', "$MaxAvgFrameMs") }
if ($MaxP95FrameMs -ge 0) { $runArgs += @('--max-p95-frame-ms', "$MaxP95FrameMs") }

# 清理旧报告：避免超时/失败时误读上一次的 report.json。
Remove-Item $reportPath -ErrorAction SilentlyContinue

$quotedArgs = ($runArgs | ForEach-Object {
        if ($_ -match '\s') { '"{0}"' -f $_ } else { $_ }
    }) -join ' '

# ---- 带超时保护地运行 ----
$process = Start-Process -FilePath $exePath -ArgumentList $quotedArgs -NoNewWindow -PassThru
$timeoutMs = [int]($timeout * 1000)
if (-not $process.WaitForExit($timeoutMs)) {
    try { $process.Kill() } catch { Write-Verbose "终止超时进程失败: $_" }
    Write-Host "soak 超时（${timeout}s），已终止。报告可能不完整: $reportPath"
    exit 3
}
$exitCode = $process.ExitCode

# ---- 摘要 ----
if (Test-Path $reportPath) {
    $report = Get-Content $reportPath -Raw | ConvertFrom-Json
    Write-Host ''
    Write-Host '--- soak report ---'
    Write-Host ("build={0} profile={1} os={2} arch={3} cores={4} duration={5:N1}s ticks={6}" -f `
            $report.machine.build_profile, $report.profile, $report.machine.os, `
            $report.machine.arch, $report.machine.cpu_cores, `
            [double]$report.duration_actual_s, $report.ticks)
    Write-Host ("frame: avg={0:N3}ms p95={1:N3}ms max={2:N3}ms overruns={3}" -f `
            [double]$report.frame.avg_ms, [double]$report.frame.p95_ms, `
            [double]$report.frame.max_ms, $report.frame.overruns)
    if ($report.memory) {
        Write-Host ("memory: source={0} start={1:N2}MB end={2:N2}MB peak={3:N2}MB growth={4:N2}MB ({5:N2}%) samples={6}" -f `
                $report.memory.source, [double]$report.memory.start_mb, `
                [double]$report.memory.end_mb, [double]$report.memory.peak_mb, `
                [double]$report.memory.growth_mb, [double]$report.memory.growth_percent, `
                $report.memory.samples.Count)
    } else {
        Write-Host 'memory: unavailable'
    }

    # GitHub Actions job summary（本地运行时不存在该变量，自然跳过）。
    if ($env:GITHUB_STEP_SUMMARY) {
        $lines = @(
            '## Hezhou soak test',
            '',
            ("- mode: ``{0}`` / duration: {1}s / build: ``{2}``" -f $Mode, $Seconds, $Profile),
            ("- OS: ``{0}`` / arch: ``{1}`` / ticks: {2}" -f $report.machine.os, $report.machine.arch, $report.ticks),
            '',
            '| 指标 | 值 |',
            '|---|---|',
            ("| 平均 tick | {0:N3} ms |" -f [double]$report.frame.avg_ms),
            ("| p95 tick | {0:N3} ms |" -f [double]$report.frame.p95_ms),
            ("| 峰值 RSS | {0:N2} MB |" -f $(if ($report.memory) { [double]$report.memory.peak_mb } else { 0 })),
            ("| 内存增长 | {0:N2} MB ({1:N2}%) |" -f $(if ($report.memory) { [double]$report.memory.growth_mb } else { 0 }), $(if ($report.memory) { [double]$report.memory.growth_percent } else { 0 })),
            ("| 结果 | {0} |" -f $(if ($report.passed) { 'pass' } else { 'fail' })),
            '',
            ("report: ``{0}``" -f $reportPath)
        )
        Add-Content -Path $env:GITHUB_STEP_SUMMARY -Value ($lines -join "`n")
    }
}

if ($exitCode -eq 0) {
    Write-Host ''
    Write-Host "soak PASS（报告: $reportPath）"
} else {
    Write-Host ''
    Write-Host "soak FAIL（exit $exitCode，报告: $reportPath）"
}
exit $exitCode
