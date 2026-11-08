# Hezhou 1.0 发布门禁（release check）
#
# 一键检查版本/文档/模板/插件/包 registry/FFI/workspace/关键测试/capability/
# perf/soak/Player fixture/fresh-start 冒烟，输出 JSON + 人读摘要；失败退出 1。
#
# 用法：
#   pwsh -File engine/scripts/release_check.ps1                    # 完整检查（release benchmark/soak 60s）
#   pwsh -File engine/scripts/release_check.ps1 -Quick -Offline    # CI / 本地快速预检
#   pwsh -File engine/scripts/release_check.ps1 -SkipBuild         # 复用已有构建产物
#   pwsh -File engine/scripts/release_check.ps1 -PlayerRoot dist/player/windows
#
# 说明：
#   - 不改能力 crate/editor 源码；只读检查 + 调用现有脚本；
#   - 1.0_RELEASE_CHECKLIST 中无法自动化的项（安装包/干净机器/交互冒烟/
#     30 分钟 soak/Player host/发布签字等）输出到 manual_items；
#   - 退出码：0 通过（允许 warning/skip）；1 有 fail；2 脚本/环境错误。

#Requires -Version 7.0

[CmdletBinding()]
param(
    # 不访问网络（cargo --offline，子脚本传 -Offline）。
    [switch]$Offline,

    # 快速模式：debug 构建/测试、10s soak、缩短 perf。
    [switch]$Quick,

    # 跳过 workspace 构建检查（复用已有 target 产物）。
    [switch]$SkipBuild,

    # 输出目录；默认 test-results/release-check。
    [string]$OutDir = '',

    # 已存在的 Player 包根目录（含 manifest.json）；省略时构建 minimal fixture 并 validate。
    [string]$PlayerRoot = ''
)

$ErrorActionPreference = 'Stop'

$scriptsDir = $PSScriptRoot
$engineRoot = Split-Path -Parent $scriptsDir
$repoRoot = Split-Path -Parent $engineRoot

if ([string]::IsNullOrWhiteSpace($OutDir)) {
    $OutDir = Join-Path $repoRoot 'test-results/release-check'
}
$OutDir = [System.IO.Path]::GetFullPath($OutDir)
$reportsDir = Join-Path $OutDir 'reports'
$logsDir = Join-Path $OutDir 'logs'
New-Item -ItemType Directory -Force $OutDir, $reportsDir, $logsDir | Out-Null
if ($Offline) { $env:CARGO_NET_OFFLINE = 'true' }

$script:Checks = [System.Collections.Generic.List[object]]::new()
$script:Warnings = [System.Collections.Generic.List[string]]::new()
$script:Started = [DateTime]::UtcNow
$script:Version = ''
$script:HostTarget = if ($IsWindows) { 'windows' } elseif ($IsLinux) { 'linux' } else { 'unsupported' }
$script:ProfileDir = if ($Quick) { 'debug' } else { 'release' }
$script:ExeSuffix = if ($IsWindows) { '.exe' } else { '' }
$script:Pwsh = (Get-Command pwsh -ErrorAction SilentlyContinue).Source
if (-not $script:Pwsh) { $script:Pwsh = Join-Path $PSHOME 'pwsh' }

# ---------------------------------------------------------------------------
# 基础工具
# ---------------------------------------------------------------------------
function Write-Utf8 {
    param([string]$Path, [string]$Text)
    $parent = Split-Path -Parent $Path
    if ($parent) { New-Item -ItemType Directory -Force $parent | Out-Null }
    [System.IO.File]::WriteAllText($Path, $Text, [System.Text.UTF8Encoding]::new($false))
}

function Add-Warning {
    param([string]$Message)
    $script:Warnings.Add($Message) | Out-Null
    Write-Host "  ! $Message" -ForegroundColor Yellow
}

function Add-Check {
    param(
        [string]$Id,
        [string]$Name,
        [string]$Category,
        [string[]]$Checklist = @(),
        [scriptblock]$Body
    )
    $check = [pscustomobject]@{
        id          = $Id
        name        = $Name
        category    = $Category
        status      = 'pass'
        detail      = ''
        error       = ''
        report      = ''
        command     = ''
        checklist   = $Checklist
        duration_ms = 0
    }
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        & $Body $check
    } catch {
        $check.status = 'fail'
        $check.error = $_.Exception.Message
    } finally {
        $watch.Stop()
        $check.duration_ms = [int]$watch.ElapsedMilliseconds
        $script:Checks.Add($check) | Out-Null
    }
    $color = switch ($check.status) {
        'pass' { 'Green' }
        'fail' { 'Red' }
        'skip' { 'Yellow' }
        default { 'White' }
    }
    Write-Host ("[{0}] {1} ({2} ms) {3}" -f $check.status.ToUpperInvariant(), $check.id, $check.duration_ms, $check.detail) -ForegroundColor $color
    if ($check.error) { Write-Host "      $($check.error)" -ForegroundColor Red }
}

function Invoke-External {
    param(
        [string]$FilePath,
        [string[]]$Arguments,
        [string]$WorkingDirectory,
        [string]$LogPath = ''
    )
    Push-Location -LiteralPath $WorkingDirectory
    try {
        $lines = & $FilePath @Arguments 2>&1 | ForEach-Object { "$_" }
        $code = $LASTEXITCODE
    } catch {
        $code = 127
        $lines = @("执行失败: $($_.Exception.Message)")
    } finally {
        Pop-Location
    }
    $text = ($lines -join "`n")
    if ($LogPath) { Write-Utf8 $LogPath $text }
    return [pscustomobject]@{ exit_code = $code; output = $text }
}

function Resolve-RepoPath {
    param([string]$Path)
    if ([System.IO.Path]::IsPathRooted($Path)) { return $Path }
    return (Join-Path $repoRoot $Path)
}

function Get-LastLine {
    param([string]$Text)
    $lines = @($Text -split "`n" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    if ($lines.Count -eq 0) { return '' }
    return $lines[-1].Trim()
}

function Invoke-Cargo {
    param(
        [string[]]$Arguments,
        [string]$LogPath
    )
    $argsWithOffline = @($Arguments)
    if ($Offline -and $argsWithOffline -notcontains '--offline') {
        # cargo 子命令均支持 --offline；放在子命令参数后、`--` 之前。
        $separator = [Array]::IndexOf($argsWithOffline, '--')
        if ($separator -gt 0) {
            $before = @($argsWithOffline[0..($separator - 1)])
            $after = @($argsWithOffline[$separator..($argsWithOffline.Count - 1)])
            $argsWithOffline = $before + @('--offline') + $after
        } elseif ($separator -eq 0) {
            $argsWithOffline = @('--offline') + $argsWithOffline
        } else {
            $argsWithOffline += '--offline'
        }
    }
    return Invoke-External -FilePath 'cargo' -Arguments $argsWithOffline -WorkingDirectory $engineRoot -LogPath $LogPath
}

function Invoke-PwshScript {
    param(
        [string]$ScriptPath,
        [string[]]$Arguments,
        [string]$LogPath
    )
    return Invoke-External -FilePath $script:Pwsh `
        -Arguments (@('-NoProfile', '-File', $ScriptPath) + $Arguments) `
        -WorkingDirectory $repoRoot -LogPath $LogPath
}

function Invoke-Python {
    param([string[]]$Arguments, [string]$LogPath)
    $python = $null
    foreach ($candidate in @('python', 'python3')) {
        $command = Get-Command $candidate -ErrorAction SilentlyContinue
        if ($command) { $python = $command.Source; break }
    }
    if (-not $python) { throw '找不到 python/python3（FFI contract 检查需要）' }
    return Invoke-External -FilePath $python -Arguments $Arguments -WorkingDirectory $engineRoot -LogPath $LogPath
}

function Get-CheckById {
    param([string]$Id)
    return $script:Checks | Where-Object { $_.id -eq $Id } | Select-Object -First 1
}

# ---------------------------------------------------------------------------
# 版本解析
# ---------------------------------------------------------------------------
$cargoPath = Join-Path $engineRoot 'Cargo.toml'
$cargoText = Get-Content -LiteralPath $cargoPath -Raw
$versionMatch = [regex]::Match($cargoText, '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"')
if (-not $versionMatch.Success) {
    Write-Host "错误：无法从 $cargoPath 解析 workspace.version" -ForegroundColor Red
    exit 2
}
$script:Version = $versionMatch.Groups[1].Value

# 运行环境。
$gitCommit = ''
$gitBranch = ''
if (Get-Command git -ErrorAction SilentlyContinue) {
    $gitCommit = (& git -C $repoRoot rev-parse HEAD 2>$null | Select-Object -First 1)
    $gitBranch = (& git -C $repoRoot rev-parse --abbrev-ref HEAD 2>$null | Select-Object -First 1)
    if ($gitCommit) { $gitCommit = $gitCommit.Trim() }
    if ($gitBranch) { $gitBranch = $gitBranch.Trim() }
}
$cargoVersion = (Invoke-External -FilePath 'cargo' -Arguments @('--version') -WorkingDirectory $engineRoot).output

Write-Host '== Hezhou 1.0 release check =='
Write-Host ("version : {0}" -f $script:Version)
Write-Host ("mode    : quick={0} offline={1} skipBuild={2}" -f $Quick, $Offline, $SkipBuild)
Write-Host ("out     : {0}" -f $OutDir)
Write-Host ''

# ---------------------------------------------------------------------------
# 1. 版本一致性
# ---------------------------------------------------------------------------
Add-Check -Id 'version.workspace' -Name 'Cargo workspace 版本' -Category 'version' -Checklist @('§1 版本与元数据') -Body {
    param($c)
    $c.detail = "$($script:Version)（engine/Cargo.toml）"
}

Add-Check -Id 'version.installers' -Name '安装包/发布说明版本一致性' -Category 'version' -Checklist @('§1 版本与元数据', '§3 双平台安装包') -Body {
    param($c)
    $issues = @()
    $details = @()

    $debSh = Join-Path $repoRoot 'installer/linux/build_deb.sh'
    $debPy = Join-Path $repoRoot 'installer/linux/build_deb.py'
    if (-not (Test-Path $debSh)) { $issues += '缺少 installer/linux/build_deb.sh' }
    else {
        $text = Get-Content -LiteralPath $debSh -Raw
        $m = [regex]::Match($text, 'VERSION="\$\{VERSION:-([^}]+)\}"')
        if (-not $m.Success) { $issues += 'build_deb.sh 未找到默认 VERSION' }
        elseif ($m.Groups[1].Value -ne $script:Version) {
            $issues += "build_deb.sh 默认版本 $($m.Groups[1].Value) != $($script:Version)"
        } else { $details += "deb.sh=$($m.Groups[1].Value)" }
    }
    if (-not (Test-Path $debPy)) { $issues += '缺少 installer/linux/build_deb.py' }
    else {
        $text = Get-Content -LiteralPath $debPy -Raw
        $m = [regex]::Match($text, '--version",\s*default="([^"]+)"')
        if (-not $m.Success) { $issues += 'build_deb.py 未找到默认 --version' }
        elseif ($m.Groups[1].Value -ne $script:Version) {
            $issues += "build_deb.py 默认版本 $($m.Groups[1].Value) != $($script:Version)"
        } else { $details += "deb.py=$($m.Groups[1].Value)" }
    }

    $wxs = Join-Path $repoRoot 'installer/windows/HezhouEditor.wxs'
    if (-not (Test-Path $wxs)) { $issues += '缺少 installer/windows/HezhouEditor.wxs' }
    elseif (-not (Get-Content -LiteralPath $wxs -Raw).Contains('Version="$(var.Version)"')) {
        $issues += 'HezhouEditor.wxs 未使用 $(var.Version) 参数'
    } else { $details += 'msi=$(var.Version)' }

    $msixManifest = Join-Path $repoRoot 'installer/windows/msix/AppxManifest.xml'
    $msixBuild = Join-Path $repoRoot 'installer/windows/build_msix.ps1'
    if (-not (Test-Path $msixManifest)) { $issues += '缺少 MSIX AppxManifest.xml' }
    if (-not (Test-Path $msixBuild)) { $issues += '缺少 build_msix.ps1' }
    if ((Test-Path $msixManifest) -and (Test-Path $msixBuild)) {
        $manifestText = Get-Content -LiteralPath $msixManifest -Raw
        $m = [regex]::Match($manifestText, '<Identity[^>]*Version="([^"]+)"')
        if (-not $m.Success) { $issues += 'AppxManifest.xml 缺失 Identity Version' }
        else {
            $templateVersion = $m.Groups[1].Value
            $buildText = Get-Content -LiteralPath $msixBuild -Raw
            if (-not $buildText.Contains('workspace\.package')) {
                $issues += 'build_msix.ps1 未从 Cargo workspace 读取版本'
            }
            $base = ($templateVersion -split '\.')[0..2] -join '.'
            if ($base -ne $script:Version) {
                Add-Warning "MSIX 模板 Version=$templateVersion 不是 $($script:Version).0（build_msix.ps1 构建时会按 Cargo 版本覆盖）"
                $details += "msix(模板)=$templateVersion→构建时覆盖"
            } else { $details += "msix=$templateVersion" }
        }
    }

    if ($issues.Count -gt 0) {
        $c.status = 'fail'
        $c.error = $issues -join '; '
    } else {
        $c.detail = $details -join ', '
    }
}

Add-Check -Id 'version.release_notes' -Name '当前版本 RELEASE_NOTES' -Category 'version' -Checklist @('§1 版本与元数据') -Body {
    param($c)
    $notes = Join-Path $repoRoot "RELEASE_NOTES_$($script:Version).md"
    if (-not (Test-Path $notes)) {
        $c.status = 'fail'
        $c.error = "缺少 RELEASE_NOTES_$($script:Version).md"
        return
    }
    $text = Get-Content -LiteralPath $notes -Raw
    if (-not $text.Contains($script:Version)) {
        $c.status = 'fail'
        $c.error = "RELEASE_NOTES_$($script:Version).md 中未出现版本号 $($script:Version)"
        return
    }
    $c.detail = "RELEASE_NOTES_$($script:Version).md"
    $c.report = $notes
}

# ---------------------------------------------------------------------------
# 2. 必需文件 / 模板 / 插件 / 包 registry / .gitignore
# ---------------------------------------------------------------------------
$requiredFiles = @(
    @{ path = 'README.md'; kind = 'readme' },
    @{ path = 'docs/GETTING_STARTED_1_0.md'; kind = 'doc' },
    @{ path = 'docs/PLATFORM_SUPPORT_1_0.md'; kind = 'doc' },
    @{ path = 'docs/API_STABILITY_1_0.md'; kind = 'doc' },
    @{ path = 'docs/1.0_RELEASE_CHECKLIST.md'; kind = 'doc' },
    @{ path = 'docs/DIAGNOSTICS_1_0.md'; kind = 'doc' },
    @{ path = 'docs/SOAK_TEST_1_0.md'; kind = 'doc' },
    @{ path = 'docs/PERF_BASELINE_0_3.md'; kind = 'doc' },
    @{ path = 'docs/RELEASE_CHECK_1_0.md'; kind = 'doc' },
    @{ path = 'engine/scripts/build_mono.ps1'; kind = 'script' },
    @{ path = 'engine/scripts/perf_baseline.ps1'; kind = 'script' },
    @{ path = 'engine/scripts/soak_test.ps1'; kind = 'script' },
    @{ path = 'engine/scripts/diagnostics.ps1'; kind = 'script' },
    @{ path = 'engine/scripts/ci_local.ps1'; kind = 'script' },
    @{ path = 'engine/scripts/run_tests.ps1'; kind = 'script' },
    @{ path = 'engine/scripts/release_check.ps1'; kind = 'script' },
    @{ path = 'engine/scripts/fresh_start_smoke.ps1'; kind = 'script' },
    @{ path = 'engine/scripts/check_ffi_contract.py'; kind = 'script' },
    @{ path = 'engine/scripts/gen_icons.py'; kind = 'script' },
    @{ path = 'engine/fonts/HarmonyOS_Sans_SC.ttf'; kind = 'font' },
    @{ path = 'engine/templates/empty-3d/template.json'; kind = 'template' },
    @{ path = 'engine/templates/empty-3d/project.json'; kind = 'template' },
    @{ path = 'engine/templates/empty-3d/build.settings.json'; kind = 'template' },
    @{ path = 'engine/templates/empty-3d/assets/scenes/main.scene'; kind = 'template' },
    @{ path = 'engine/templates/action-sandbox/template.json'; kind = 'template' },
    @{ path = 'engine/templates/action-sandbox/project.json'; kind = 'template' },
    @{ path = 'engine/templates/action-sandbox/build.settings.json'; kind = 'template' },
    @{ path = 'engine/templates/action-sandbox/assets/scenes/main.scene'; kind = 'template' },
    @{ path = 'engine/templates/action-sandbox/scripts/Spin.cs'; kind = 'template' },
    @{ path = 'engine/plugin/Cargo.toml'; kind = 'plugin' },
    @{ path = 'engine/plugin/src/registry.rs'; kind = 'plugin' },
    @{ path = 'engine/plugins/sample-tools/plugin.json'; kind = 'plugin' },
    @{ path = 'engine/plugins/scene-tools/plugin.json'; kind = 'plugin' },
    @{ path = 'scripts/PluginBase.cs'; kind = 'plugin' },
    @{ path = 'scripts/NativeMethods.cs'; kind = 'plugin' },
    @{ path = 'scripts/ScriptValue.cs'; kind = 'plugin' },
    @{ path = 'engine/package/Cargo.toml'; kind = 'package' },
    @{ path = 'engine/package/src/registry.rs'; kind = 'package' },
    @{ path = 'engine/packages/registry/hezhou-hello/package.json'; kind = 'package' },
    @{ path = 'engine/packages/registry/hezhou-mathx/package.json'; kind = 'package' },
    @{ path = 'engine/packages/registry/hezhou-gameplay-kit/package.json'; kind = 'package' },
    @{ path = 'installer/windows/build_msi.ps1'; kind = 'installer' },
    @{ path = 'installer/windows/build_msix.ps1'; kind = 'installer' },
    @{ path = 'installer/linux/build_deb.sh'; kind = 'installer' },
    @{ path = 'installer/linux/build_deb.py'; kind = 'installer' }
)

Add-Check -Id 'required.files' -Name '必需文档/脚本/模板/插件/包 registry/字体' -Category 'layout' -Checklist @('§1 版本与元数据', '§6 文档与开发者体验') -Body {
    param($c)
    $missing = @()
    foreach ($entry in $requiredFiles) {
        $path = Join-Path $repoRoot $entry.path
        if (-not (Test-Path -LiteralPath $path)) { $missing += $entry.path }
    }
    if ($missing.Count -gt 0) {
        $c.status = 'fail'
        $c.error = "缺少 $($missing.Count) 个必需文件: $($missing -join ', ')"
        $c.detail = "required=$($requiredFiles.Count) missing=$($missing.Count)"
    } else {
        $c.detail = "$($requiredFiles.Count) 个文件存在"
    }
}

Add-Check -Id 'repo.gitignore_artifacts' -Name '构建产物未入库（.gitignore）' -Category 'layout' -Checklist @('§1 版本与元数据') -Body {
    param($c)
    $path = Join-Path $repoRoot '.gitignore'
    if (-not (Test-Path $path)) {
        $c.status = 'fail'
        $c.error = '缺少 .gitignore'
        return
    }
    $text = Get-Content -LiteralPath $path -Raw
    $missing = @()
    foreach ($pattern in @('dist/', 'installer/stage/', 'build-temp/', 'download/')) {
        if (-not $text.Contains($pattern)) { $missing += $pattern }
    }
    if ($missing.Count -gt 0) {
        $c.status = 'fail'
        $c.error = "缺少忽略规则: $($missing -join ', ')"
        return
    }
    if (-not $text.Contains('test-results/')) {
        Add-Warning '.gitignore 未显式忽略 test-results/（当前检查输出可能出现在 git status）'
    }
    $c.detail = 'dist/ installer/stage/ build-temp/ download/ 已忽略'
}

# ---------------------------------------------------------------------------
# 3. FFI contract
# ---------------------------------------------------------------------------
Add-Check -Id 'ffi.contract' -Name 'FFI contract（Rust/C#/装配三方一致）' -Category 'contract' -Checklist @('§2 CI/测试门禁') -Body {
    param($c)
    $log = Join-Path $reportsDir 'ffi-contract.txt'
    $result = Invoke-Python -Arguments @('scripts/check_ffi_contract.py') -LogPath $log
    $c.report = $log
    $c.command = 'python engine/scripts/check_ffi_contract.py'
    if ($result.exit_code -ne 0) {
        $c.status = 'fail'
        $c.error = "FFI contract 失败（exit $($result.exit_code)）: $(Get-LastLine $result.output)"
        return
    }
    $c.detail = Get-LastLine $result.output
}

# ---------------------------------------------------------------------------
# 4. Workspace 构建 / 关键测试
# ---------------------------------------------------------------------------
Add-Check -Id 'workspace.build' -Name 'workspace 构建/检查' -Category 'build' -Checklist @('§2 CI/测试门禁') -Body {
    param($c)
    if ($SkipBuild) {
        $c.status = 'skip'
        $c.detail = '用户指定 -SkipBuild'
        return
    }
    $log = Join-Path $reportsDir 'workspace-build.txt'
    if ($Quick) {
        $cargoArgs = @('check', '--workspace')
        $c.command = 'cargo check --workspace'
    } else {
        $cargoArgs = @('build', '--workspace')
        $c.command = 'cargo build --workspace'
    }
    $result = Invoke-Cargo -Arguments $cargoArgs -LogPath $log
    $c.report = $log
    if ($result.exit_code -ne 0) {
        $c.status = 'fail'
        $c.error = "失败（exit $($result.exit_code)）: $(Get-LastLine $result.output)"
        return
    }
    $c.detail = if ($Quick) { 'cargo check --workspace OK' } else { 'cargo build --workspace OK' }
}

Add-Check -Id 'tests.key' -Name '关键测试（core/player/net/package/plugin/physics）' -Category 'tests' -Checklist @('§2 CI/测试门禁') -Body {
    param($c)
    $log = Join-Path $reportsDir 'key-tests.txt'
    $packages = @('hezhou-core', 'hezhou-player', 'hezhou-net', 'hezhou-package', 'hezhou-plugin', 'hezhou-physics')
    $cargoArgs = @('test')
    foreach ($package in $packages) { $cargoArgs += @('-p', $package) }
    $c.command = 'cargo ' + ($cargoArgs -join ' ')
    $result = Invoke-Cargo -Arguments $cargoArgs -LogPath $log
    $c.report = $log
    if ($result.exit_code -ne 0) {
        $c.status = 'fail'
        $c.error = "关键测试失败（exit $($result.exit_code)）: $(Get-LastLine $result.output)"
        return
    }
    $testResults = @($result.output -split "`n" | Where-Object { $_ -match '^test result:' })
    $c.detail = "$($packages.Count) 个 crate 测试通过（$($testResults.Count) 个 test result 行）"
}

# ---------------------------------------------------------------------------
# 5. capability_demo
# ---------------------------------------------------------------------------
Add-Check -Id 'capability.demo' -Name 'capability_demo（headless 能力组合）' -Category 'capability' -Checklist @('§2 CI/测试门禁', '§8 已知问题') -Body {
    param($c)
    $log = Join-Path $reportsDir 'capability-demo.txt'
    $report = Join-Path $reportsDir 'capability-report.json'
    $steps = 360
    $cargoArgs = @('run', '-p', 'hezhou-examples', '--bin', 'capability_demo', '--no-default-features')
    $cargoArgs += @('--', '--steps', "$steps", '--out', $report)
    $c.command = 'cargo ' + ($cargoArgs -join ' ')
    $result = Invoke-Cargo -Arguments $cargoArgs -LogPath $log
    $c.report = $report
    if ($result.exit_code -ne 0 -or -not (Test-Path $report)) {
        $c.status = 'fail'
        $c.error = "capability_demo 失败（exit $($result.exit_code)）: $(Get-LastLine $result.output)"
        return
    }
    $summary = Get-Content -LiteralPath $report -Raw | ConvertFrom-Json
    $c.detail = "steps=$steps report=$report; $($summary.capabilities.Count) capabilities"
}

# ---------------------------------------------------------------------------
# 6. perf / soak 门禁
# ---------------------------------------------------------------------------
Add-Check -Id 'perf.quick' -Name 'perf_smoke 性能基线' -Category 'performance' -Checklist @('§5 稳定性/性能/诊断') -Body {
    param($c)
    $log = Join-Path $reportsDir 'perf-baseline.txt'
    $report = Join-Path $reportsDir 'perf-report.json'
    $profile = if ($Quick) { 'debug' } else { 'release' }
    $scriptArgs = @('-Profile', $profile, '-Offline', '-OutPath', $report)
    if ($Quick) { $scriptArgs += '-Quick' }
    $c.command = "perf_baseline.ps1 -Profile $profile$(if ($Quick) { ' -Quick' }) -Offline"
    $result = Invoke-PwshScript -ScriptPath (Join-Path $scriptsDir 'perf_baseline.ps1') -Arguments $scriptArgs -LogPath $log
    $c.report = $report
    if ($result.exit_code -ne 0) {
        $c.status = 'fail'
        $c.error = "perf 门禁失败（exit $($result.exit_code)）: $(Get-LastLine $result.output)"
        return
    }
    if (Test-Path $report) {
        $data = Get-Content -LiteralPath $report -Raw | ConvertFrom-Json
        $passed = @($data.benchmarks | Where-Object { $_.passed }).Count
        $c.detail = "$($data.benchmarks.Count) 项基准，$passed 项通过（$profile）"
    } else {
        $c.detail = "perf 脚本通过（profile=$profile，报告未找到）"
    }
}

Add-Check -Id 'soak.quick' -Name 'soak 稳定性冒烟' -Category 'performance' -Checklist @('§5 稳定性/性能/诊断') -Body {
    param($c)
    $log = Join-Path $reportsDir 'soak.txt'
    $report = Join-Path $reportsDir 'soak-report.json'
    if ($Quick) {
        $profile = 'debug'
        $seconds = 10
        $scriptArgs = @('-Quick', '-Profile', $profile, '-Offline', '-Seconds', "$seconds", '-Warmup', '1', '-OutPath', $report)
        $c.command = "soak_test.ps1 -Quick -Profile debug -Seconds ${seconds}s -Offline"
    } else {
        $profile = 'release'
        $scriptArgs = @('-Mode', 'ci', '-Quick', '-Profile', $profile, '-Offline', '-OutPath', $report)
        $c.command = 'soak_test.ps1 -Mode ci -Quick -Profile release -Offline'
    }
    $result = Invoke-PwshScript -ScriptPath (Join-Path $scriptsDir 'soak_test.ps1') -Arguments $scriptArgs -LogPath $log
    $c.report = $report
    if ($result.exit_code -ne 0) {
        $c.status = 'fail'
        $c.error = "soak 门禁失败（exit $($result.exit_code)）: $(Get-LastLine $result.output)"
        return
    }
    if (Test-Path $report) {
        $data = Get-Content -LiteralPath $report -Raw | ConvertFrom-Json
        $c.detail = "PASS（profile=$profile, mode=$(if ($Quick) { "quick-${seconds}s" } else { 'ci-60s' })）"
    } else {
        $c.detail = "soak 脚本通过（profile=$profile）"
    }
}

# ---------------------------------------------------------------------------
# 7. Player fixture / -PlayerRoot validate
# ---------------------------------------------------------------------------
Add-Check -Id 'player.validate' -Name 'Player 构建/启动校验' -Category 'player' -Checklist @('§4 Player 导出与运行') -Body {
    param($c)
    if ($script:HostTarget -eq 'unsupported') {
        $c.status = 'skip'
        $c.detail = '当前平台不在 Windows/Linux 支持范围'
        return
    }
    $playerExe = Join-Path $engineRoot "target/$($script:ProfileDir)/hezhou-player$($script:ExeSuffix)"
    $builderExe = Join-Path $engineRoot "target/$($script:ProfileDir)/hezhou-player-build$($script:ExeSuffix)"

    if ($SkipBuild -and (-not (Test-Path $playerExe) -or -not (Test-Path $builderExe))) {
        $c.status = 'skip'
        $c.detail = '用户指定 -SkipBuild 且缺少 Player 产物'
        return
    }
    if (-not $SkipBuild) {
        $buildLog = Join-Path $reportsDir 'player-build.txt'
        $buildArgs = @('build', '-p', 'hezhou-player', '--bin', 'hezhou-player', '--bin', 'hezhou-player-build')
        if ($script:ProfileDir -eq 'release') { $buildArgs += '--release' }
        $c.command = 'cargo ' + ($buildArgs -join ' ')
        $build = Invoke-Cargo -Arguments $buildArgs -LogPath $buildLog
        if ($build.exit_code -ne 0) {
            $c.status = 'fail'
            $c.error = "Player 构建失败（exit $($build.exit_code)）: $(Get-LastLine $build.output)"
            return
        }
    }

    $logDir = Join-Path $logsDir 'player'
    if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Force $logDir | Out-Null }

    if (-not [string]::IsNullOrWhiteSpace($PlayerRoot)) {
        $root = Resolve-RepoPath $PlayerRoot
        if (-not (Test-Path -LiteralPath (Join-Path $root 'manifest.json'))) {
            $c.status = 'fail'
            $c.error = "-PlayerRoot 缺少 manifest.json: $root"
            return
        }
        $c.command = "hezhou-player --validate --player-root $root"
        $validate = Invoke-External -FilePath $playerExe -Arguments @('--validate', '--player-root', $root, '--log-dir', $logDir) -WorkingDirectory $engineRoot -LogPath (Join-Path $reportsDir 'player-validate.txt')
        if ($validate.exit_code -ne 0) {
            $c.status = 'fail'
            $c.error = "Player validate 失败（exit $($validate.exit_code)）: $(Get-LastLine $validate.output)"
            return
        }
        $c.detail = "PASS（-PlayerRoot=$root）"
        $c.report = Join-Path $reportsDir 'player-validate.txt'
        return
    }

    $fixtureSource = Join-Path $engineRoot 'player/tests/fixtures/minimal'
    if (-not (Test-Path -LiteralPath $fixtureSource)) {
        $c.status = 'fail'
        $c.error = "缺少 Player fixture: $fixtureSource"
        return
    }
    $fixtureDir = Join-Path $OutDir 'player-fixture'
    if (Test-Path -LiteralPath $fixtureDir) { Remove-Item -LiteralPath $fixtureDir -Recurse -Force }
    Copy-Item -LiteralPath $fixtureSource -Destination $fixtureDir -Recurse -Force

    $configPath = Join-Path $fixtureDir 'build-config.json'
    $config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
    $config.target = $script:HostTarget
    Write-Utf8 $configPath (($config | ConvertTo-Json -Depth 8) + "`n")

    $buildOut = Join-Path $OutDir 'player-build'
    if (Test-Path -LiteralPath $buildOut) { Remove-Item -LiteralPath $buildOut -Recurse -Force }
    $builderArgs = @(
        '--config', $configPath,
        '--cook', (Join-Path $fixtureDir 'cook-manifest.json'),
        '--assets-dir', (Join-Path $fixtureDir 'assets'),
        '--scripts-dir', (Join-Path $fixtureDir 'scripts'),
        '--runtime-dir', (Join-Path $fixtureDir 'runtime'),
        '--exe', $playerExe,
        '--build-out', $buildOut
    )
    $c.command = 'hezhou-player-build ... --build-out <out>/player-build'
    $build = Invoke-External -FilePath $builderExe -Arguments $builderArgs -WorkingDirectory $engineRoot -LogPath (Join-Path $reportsDir 'player-fixture-build.txt')
    if ($build.exit_code -ne 0) {
        $c.status = 'fail'
        $c.error = "Player fixture 构建失败（exit $($build.exit_code)）: $(Get-LastLine $build.output)"
        return
    }
    $package = Join-Path $buildOut $script:HostTarget
    $executable = Join-Path $package "minimal-fixture$($script:ExeSuffix)"
    if (-not (Test-Path -LiteralPath $executable)) {
        $c.status = 'fail'
        $c.error = "缺少 Player fixture 可执行文件: $executable"
        return
    }
    $validate = Invoke-External -FilePath $executable -Arguments @('--validate', '--player-root', $package, '--log-dir', $logDir) -WorkingDirectory $package -LogPath (Join-Path $reportsDir 'player-validate.txt')
    if ($validate.exit_code -ne 0) {
        $c.status = 'fail'
        $c.error = "Player fixture validate 失败（exit $($validate.exit_code)）: $(Get-LastLine $validate.output)"
        return
    }
    $c.detail = "PASS（fixture minimal → $script:HostTarget）"
    $c.report = Join-Path $reportsDir 'player-validate.txt'
}

# ---------------------------------------------------------------------------
# 8. fresh-start 冒烟
# ---------------------------------------------------------------------------
Add-Check -Id 'fresh_start.smoke' -Name 'fresh-start 模板→工程→Player dry-run' -Category 'template' -Checklist @('§6 文档与开发者体验', '§4 Player 导出与运行') -Body {
    param($c)
    $builder = Join-Path $engineRoot "target/$($script:ProfileDir)/hezhou-player-build$($script:ExeSuffix)"
    if ($SkipBuild -and -not (Test-Path $builder)) {
        $c.status = 'skip'
        $c.detail = '用户指定 -SkipBuild 且缺少 hezhou-player-build'
        return
    }
    $smokeOut = Join-Path $OutDir 'fresh-start'
    $log = Join-Path $reportsDir 'fresh-start.txt'
    $smokeArgs = @('-Template', 'all', '-Offline', '-Profile', $script:ProfileDir, '-OutDir', $smokeOut)
    if ($SkipBuild) { $smokeArgs += '-SkipBuild' }
    $c.command = 'fresh_start_smoke.ps1 -Template all -Offline'
    $result = Invoke-PwshScript -ScriptPath (Join-Path $scriptsDir 'fresh_start_smoke.ps1') -Arguments $smokeArgs -LogPath $log
    $json = Join-Path $smokeOut 'smoke-report.json'
    $c.report = $json
    if ($result.exit_code -ne 0 -or -not (Test-Path $json)) {
        $c.status = 'fail'
        $c.error = "fresh-start smoke 失败（exit $($result.exit_code)）: $(Get-LastLine $result.output)"
        return
    }
    $data = Get-Content -LiteralPath $json -Raw | ConvertFrom-Json
    if ($data.overall_status -ne 'pass') {
        $c.status = 'fail'
        $c.error = "fresh-start overall=$($data.overall_status)"
        return
    }
    $c.detail = "$(@($data.templates).Count) 个模板通过（Player dry-run）"
}

# ---------------------------------------------------------------------------
# 汇总：自动检查 + 未自动化清单 + 报告
# ---------------------------------------------------------------------------
$manualItems = @(
    [pscustomobject]@{
        id = 'version.1_0_bump'; checklist = '§1'; title = '1.0.0 版本号 bump + RELEASE_NOTES_1.0.0'
        reason = '需要人工决定发布版本并在 Cargo workspace/安装包/Player/文档同步；1.0 尚未发布'
    },
    [pscustomobject]@{
        id = 'packages.real_build'; checklist = '§3'; title = 'MSI/MSIX/DEB 实际打包、签名、干净机器安装/启动/卸载'
        reason = '需要 WiX/MakeAppx/dpkg 与干净环境；本地/CI tag 执行，release_check 仅静态核对版本与文件'
    },
    [pscustomobject]@{
        id = 'ci.tag_green'; checklist = '§2'; title = 'tag CI 全绿（rust-tests/csharp/perf/soak/package/publish）'
        reason = '需要在推送 tag v1.0.0 后观察 GitHub Actions 全链路'
    },
    [pscustomobject]@{
        id = 'player.full_host'; checklist = '§4/§8'; title = 'Player 完整游戏 host / PlayerGame / PlayerCSharpContext 端到端'
        reason = '当前默认 host 为 pending（退出码 3）；1.0 阻断项，需要功能开发/验证'
    },
    [pscustomobject]@{
        id = 'soak.30min'; checklist = '§5'; title = '1.0 构建上的 30 分钟 soak（1800s）复核'
        reason = 'release_check 默认只跑 60s（Quick=10s）；30 分钟需在本机/夜间任务执行'
    },
    [pscustomobject]@{
        id = 'editor.interactive'; checklist = '§5'; title = '编辑器 30+ 分钟交互/导出冒烟（FPS/帧时间/内存）'
        reason = '需要 GUI/Vulkan 交互，无法在 headless CI 自动完成'
    },
    [pscustomobject]@{
        id = 'diagnostics.normal'; checklist = '§5'; title = 'diagnostics.ps1 普通模式在 Windows/Linux 双平台人工验证'
        reason = 'self-test 已自动化；真实用户日志/Player 包采集仍建议双平台人工跑一次'
    },
    [pscustomobject]@{
        id = 'docs.review'; checklist = '§6'; title = 'README 文档入口、交叉链接、FAQ/已知限制与实现一致'
        reason = '文档语义/链接需要人工评审（结构存在性已自动检查）'
    },
    [pscustomobject]@{
        id = 'release.signoff'; checklist = '§9'; title = '发版签字（引擎/Player/测试/文档/发布负责人）'
        reason = '流程/责任确认，必须人工完成'
    },
    [pscustomobject]@{
        id = 'release.assets_rollback'; checklist = '§7'; title = 'Release 资产上传、回滚预案、发布后 24h 观察'
        reason = '涉及 GitHub Release 操作与线上观察'
    }
)

$failures = @($script:Checks | Where-Object { $_.status -eq 'fail' })
$skipped = @($script:Checks | Where-Object { $_.status -eq 'skip' })
$passed = @($script:Checks | Where-Object { $_.status -eq 'pass' })
$overall = if ($failures.Count -gt 0) { 'fail' } else { 'pass' }
$finished = [DateTime]::UtcNow
$durationMs = [int](($finished - $script:Started).TotalMilliseconds)

$report = [ordered]@{
    format_version   = 1
    tool             = 'release-check'
    generated_at_utc = $finished.ToString('yyyy-MM-ddTHH:mm:ssZ')
    engine_version   = $script:Version
    mode             = [ordered]@{
        quick      = $Quick.IsPresent
        offline    = $Offline.IsPresent
        skip_build = $SkipBuild.IsPresent
        player_root = $PlayerRoot
    }
    environment      = [ordered]@{
        os           = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
        arch         = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
        pwsh         = $PSVersionTable.PSVersion.ToString()
        cargo        = $cargoVersion
        git_commit   = $gitCommit
        git_branch   = $gitBranch
        python       = (Get-Command python, python3 -ErrorAction SilentlyContinue | Select-Object -First 1).Source
    }
    overall_status   = $overall
    summary          = [ordered]@{
        total    = $script:Checks.Count
        passed   = $passed.Count
        failed   = $failures.Count
        skipped  = $skipped.Count
        warnings = $script:Warnings.Count
        duration_ms = $durationMs
    }
    checks           = @($script:Checks)
    manual_items     = $manualItems
    warnings         = @($script:Warnings)
    reports          = [ordered]@{
        root = $OutDir
        perf = (Join-Path $reportsDir 'perf-report.json')
        soak = (Join-Path $reportsDir 'soak-report.json')
        ffi  = (Join-Path $reportsDir 'ffi-contract.txt')
        fresh_start = (Join-Path $OutDir 'fresh-start/smoke-report.json')
        player_log = (Join-Path $logsDir 'player/player.log')
    }
}

$jsonPath = Join-Path $OutDir 'release-report.json'
Write-Utf8 $jsonPath (($report | ConvertTo-Json -Depth 12) + "`n")

$lines = @(
    '# Hezhou 1.0 发布门禁报告',
    '',
    ("- 时间(UTC): {0}" -f $report.generated_at_utc),
    ("- 版本: {0} / git: {1}@{2}" -f $script:Version, $gitBranch, $gitCommit),
    ("- 模式: quick={0} offline={1} skipBuild={2}" -f $Quick, $Offline, $SkipBuild),
    ("- 结果: **{0}**" -f $overall.ToUpperInvariant()),
    ("- 检查: {0} 通过 / {1} 失败 / {2} 跳过（共 {3}，{4:N1}s）" -f $passed.Count, $failures.Count, $skipped.Count, $script:Checks.Count, ($durationMs / 1000.0)),
    '',
    '## 自动检查',
    '',
    '| 状态 | 检查 | 类别 | 明细 |',
    '|---|---|---|---|'
)
foreach ($check in $script:Checks) {
    $detail = ($check.detail -replace '\|', '/') -replace "`n", ' '
    $lines += ("| {0} | {1} | {2} | {3} |" -f $check.status, $check.name, $check.category, $detail)
}
if ($failures.Count -gt 0) {
    $lines += ''
    $lines += '## 失败项'
    $lines += ''
    foreach ($failure in $failures) { $lines += ("- **{0}**: {1}" -f $failure.name, $failure.error) }
}
if ($script:Warnings.Count -gt 0) {
    $lines += ''
    $lines += '## 警告'
    $lines += ''
    foreach ($warning in $script:Warnings) { $lines += "- $warning" }
}
$lines += ''
$lines += '## 未自动化项（必须人工确认）'
$lines += ''
foreach ($item in $manualItems) {
    $lines += ("- [{0}] {1} — {2}" -f $item.checklist, $item.title, $item.reason)
}
$lines += ''
$lines += ("JSON: {0}" -f $jsonPath)
$humanPath = Join-Path $OutDir 'REPORT.md'
Write-Utf8 $humanPath ($lines -join "`n")

Write-Host ''
Write-Host '== release check summary =='
Write-Host ("overall : {0}" -f $overall.ToUpperInvariant()) -ForegroundColor $(if ($overall -eq 'pass') { 'Green' } else { 'Red' })
Write-Host ("checks  : {0} passed, {1} failed, {2} skipped ({3:N1}s)" -f $passed.Count, $failures.Count, $skipped.Count, ($durationMs / 1000.0))
Write-Host ("json    : {0}" -f $jsonPath)
Write-Host ("report  : {0}" -f $humanPath)
Write-Host ("manual  : {0} 项未自动化检查（见报告）" -f $manualItems.Count)

if ($failures.Count -gt 0) {
    Write-Host ''
    Write-Host '失败项：' -ForegroundColor Red
    foreach ($failure in $failures) { Write-Host ("- {0}: {1}" -f $failure.name, $failure.error) -ForegroundColor Red }
}

if ($env:GITHUB_STEP_SUMMARY) {
    $summary = @('## Hezhou 1.0 release check', '')
    $summary += ("- result: **{0}**" -f $overall)
    $summary += ("- version: {0} / quick={1} offline={2}" -f $script:Version, $Quick, $Offline)
    $summary += ''
    $summary += '| 状态 | 检查 | 明细 |'
    $summary += '|---|---|---|'
    foreach ($check in $script:Checks) {
        $detail = ($check.detail -replace '\|', '/') -replace "`n", ' '
        $summary += ("| {0} | {1} | {2} |" -f $check.status, $check.name, $detail)
    }
    Add-Content -Path $env:GITHUB_STEP_SUMMARY -Value ($summary -join "`n")
}

exit $(if ($overall -eq 'pass') { 0 } else { 1 })
