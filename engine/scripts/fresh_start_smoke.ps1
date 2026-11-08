# Hezhou 1.0 fresh-start 冒烟：模板 → 工程 → Build Settings → Player dry-run
#
# 模拟"非团队开发者从模板开始"的最小无 GUI 流程：
#   1. 复制 engine/templates/<template> 到临时目录；
#   2. 占位替换（{{PROJECT_NAME}} / {{VERSION}} / {{ENGINE_VERSION}}）；
#   3. 校验 template.json 文件清单、project.json、build.settings.json；
#   4. 生成 Player build-config + legacy cook manifest，调用
#      `hezhou-player-build --plan-out --target both` 做 Player dry-run；
#   5. 输出 JSON + 人读报告。
#
# 用法：
#   pwsh -File engine/scripts/fresh_start_smoke.ps1
#   pwsh -File engine/scripts/fresh_start_smoke.ps1 -Template all -OutDir test-results/fresh-start-smoke
#   pwsh -File engine/scripts/fresh_start_smoke.ps1 -SkipBuild -Offline
#
# 退出码：0 通过；1 模板/工程/Player dry-run 失败；2 环境/构建失败。

#Requires -Version 7.0

[CmdletBinding()]
param(
    # 模板：all / empty-3d / action-sandbox。
    [ValidateSet('all', 'empty-3d', 'action-sandbox')]
    [string]$Template = 'all',

    # 输出目录（默认 test-results/fresh-start-smoke）。
    [string]$OutDir = '',

    # 复用已有 hezhou-player-build 可执行文件，不重新构建。
    [switch]$SkipBuild,

    # 不访问网络（cargo --offline）。
    [switch]$Offline,

    # 构建 profile。
    [ValidateSet('debug', 'release')]
    [string]$Profile = 'debug'
)

$ErrorActionPreference = 'Stop'

# ---------------------------------------------------------------------------
# 路径与基础工具
# ---------------------------------------------------------------------------
$scriptsDir = $PSScriptRoot
$engineRoot = Split-Path -Parent $scriptsDir
$repoRoot = Split-Path -Parent $engineRoot

if ([string]::IsNullOrWhiteSpace($OutDir)) {
    $OutDir = Join-Path $repoRoot 'test-results/fresh-start-smoke'
}
$OutDir = [System.IO.Path]::GetFullPath($OutDir)
New-Item -ItemType Directory -Force $OutDir | Out-Null

$profileDir = if ($Profile -eq 'release') { 'release' } else { 'debug' }
$exeSuffix = if ($IsWindows) { '.exe' } else { '' }
$builder = Join-Path $engineRoot "target/$profileDir/hezhou-player-build$exeSuffix"
$hostTarget = if ($IsWindows) { 'windows' } else { 'linux' }

function Write-Utf8([string]$Path, [string]$Text) {
    $parent = Split-Path -Parent $Path
    if ($parent) { New-Item -ItemType Directory -Force $parent | Out-Null }
    [System.IO.File]::WriteAllText($Path, $Text, [System.Text.UTF8Encoding]::new($false))
}

function Get-WorkspaceVersion {
    $cargo = Join-Path $engineRoot 'Cargo.toml'
    $text = Get-Content -LiteralPath $cargo -Raw
    $match = [regex]::Match($text, '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"')
    if (-not $match.Success) { throw "无法从 $cargo 解析 workspace.version" }
    return $match.Groups[1].Value
}

function Invoke-Process {
    param(
        [string]$FilePath,
        [string[]]$Arguments,
        [string]$WorkingDirectory
    )
    Push-Location -LiteralPath $WorkingDirectory
    try {
        $output = & $FilePath @Arguments 2>&1 | ForEach-Object { "$_" }
        $code = $LASTEXITCODE
    } finally {
        Pop-Location
    }
    return [pscustomobject]@{
        exit_code = $code
        output    = ($output -join "`n")
    }
}

# ---------------------------------------------------------------------------
# 构建 Player 构建器
# ---------------------------------------------------------------------------
$warnings = [System.Collections.Generic.List[string]]::new()
$version = Get-WorkspaceVersion

if (-not (Test-Path -LiteralPath $builder)) {
    if ($SkipBuild) {
        Write-Host "错误：-SkipBuild 但找不到 Player 构建器: $builder" -ForegroundColor Red
        exit 2
    }
    $cargoArgs = @('build', '-p', 'hezhou-player', '--bin', 'hezhou-player-build')
    if ($Profile -eq 'release') { $cargoArgs += '--release' }
    if ($Offline) { $cargoArgs += '--offline' }
    Write-Host "构建 hezhou-player-build（$Profile）..."
    $build = Invoke-Process -FilePath 'cargo' -Arguments $cargoArgs -WorkingDirectory $engineRoot
    if ($build.exit_code -ne 0) {
        Write-Host "hezhou-player-build 构建失败（exit $($build.exit_code)）" -ForegroundColor Red
        Write-Host $build.output
        exit 2
    }
}

# ---------------------------------------------------------------------------
# 单个模板冒烟
# ---------------------------------------------------------------------------
function Test-Template {
    param([string]$TemplateId)

    $result = [ordered]@{
        template         = $TemplateId
        status           = 'pass'
        project_dir      = ''
        player_plan_dir  = ''
        checks           = [ordered]@{}
        plans            = @()
        warnings         = @()
        errors           = @()
    }
    $templateDir = Join-Path $engineRoot "templates/$TemplateId"
    if (-not (Test-Path -LiteralPath (Join-Path $templateDir 'template.json'))) {
        $result.status = 'fail'
        $result.errors += "模板不存在或缺少 template.json: $templateDir"
        return [pscustomobject]$result
    }

    $projectDir = Join-Path $OutDir "$TemplateId/project"
    $playerDir = Join-Path $OutDir "$TemplateId/player"
    if (Test-Path -LiteralPath $projectDir) { Remove-Item -LiteralPath $projectDir -Recurse -Force }
    if (Test-Path -LiteralPath $playerDir) { Remove-Item -LiteralPath $playerDir -Recurse -Force }
    New-Item -ItemType Directory -Force $playerDir | Out-Null
    Copy-Item -LiteralPath $templateDir -Destination $projectDir -Recurse -Force
    $result.project_dir = $projectDir

    # 1) 占位替换（仅文本文件）。
    $productName = switch ($TemplateId) {
        'empty-3d' { 'Smoke Empty 3D' }
        'action-sandbox' { 'Smoke Action Sandbox' }
        default { "Smoke $TemplateId" }
    }
    $textExtensions = @('.json', '.md', '.cs', '.scene', '.txt', '.settings', '.yml', '.yaml')
    $replaced = 0
    Get-ChildItem -LiteralPath $projectDir -Recurse -File | ForEach-Object {
        if ($textExtensions -notcontains $_.Extension.ToLowerInvariant()) { return }
        $text = Get-Content -LiteralPath $_.FullName -Raw
        $new = $text.
            Replace('{{PROJECT_NAME}}', $productName).
            Replace('{{VERSION}}', $version).
            Replace('{{ENGINE_VERSION}}', $version)
        if ($new -ne $text) {
            Write-Utf8 $_.FullName $new
            $replaced++
        }
    }
    $result.checks['placeholder_replacement'] = [ordered]@{
        status = 'pass'
        files  = $replaced
        tokens = @('{{PROJECT_NAME}}', '{{VERSION}}', '{{ENGINE_VERSION}}')
    }

    # 2) 不应残留占位符。
    $leftover = @()
    Get-ChildItem -LiteralPath $projectDir -Recurse -File | ForEach-Object {
        if ($textExtensions -notcontains $_.Extension.ToLowerInvariant()) { return }
        $text = Get-Content -LiteralPath $_.FullName -Raw
        if ($text.Contains('{{')) { $leftover += $_.FullName }
    }
    if ($leftover.Count -gt 0) {
        $result.checks['placeholder_leftover'] = [ordered]@{
            status = 'fail'
            files  = $leftover
        }
        $result.errors += "模板占位符未替换: $($leftover -join ', ')"
    } else {
        $result.checks['placeholder_leftover'] = [ordered]@{ status = 'pass'; files = @() }
    }

    # 3) template.json 文件清单。
    $templateJson = Get-Content -LiteralPath (Join-Path $projectDir 'template.json') -Raw | ConvertFrom-Json
    $missingFiles = @()
    foreach ($relative in @($templateJson.files)) {
        $candidate = Join-Path $projectDir (($relative -replace '/', [IO.Path]::DirectorySeparatorChar))
        if (-not (Test-Path -LiteralPath $candidate)) { $missingFiles += $relative }
    }
    if ($missingFiles.Count -gt 0) {
        $result.checks['template_files'] = [ordered]@{ status = 'fail'; missing = $missingFiles }
        $result.errors += "模板文件缺失: $($missingFiles -join ', ')"
    } else {
        $result.checks['template_files'] = [ordered]@{
            status = 'pass'
            count  = @($templateJson.files).Count
        }
    }

    # 4) project.json 解析与字段。
    try {
        $project = Get-Content -LiteralPath (Join-Path $projectDir 'project.json') -Raw | ConvertFrom-Json
        $projectIssues = @()
        if ([string]::IsNullOrWhiteSpace($project.name)) { $projectIssues += 'name 为空' }
        if ($project.name -ne $productName) { $projectIssues += "name 未替换: $($project.name)" }
        if ($null -eq $project.settings) { $projectIssues += 'settings 缺失' }
        if ($project.settings.game_width -le 0 -or $project.settings.game_height -le 0) {
            $projectIssues += '窗口尺寸非法'
        }
        if ($project.settings.fps -le 0) { $projectIssues += 'fps 非法' }
        if ($projectIssues.Count -gt 0) {
            $result.checks['project_json'] = [ordered]@{ status = 'fail'; issues = $projectIssues }
            $result.errors += "project.json: $($projectIssues -join '; ')"
        } else {
            $result.checks['project_json'] = [ordered]@{
                status = 'pass'
                name   = $project.name
                width  = $project.settings.game_width
                height = $project.settings.game_height
                fps    = $project.settings.fps
            }
        }
    } catch {
        $result.checks['project_json'] = [ordered]@{ status = 'fail'; error = $_.Exception.Message }
        $result.errors += "project.json 解析失败: $($_.Exception.Message)"
    }

    # 5) build.settings.json 解析与 Build Settings 校验（不依赖 GUI）。
    try {
        $settings = Get-Content -LiteralPath (Join-Path $projectDir 'build.settings.json') -Raw | ConvertFrom-Json
        $settingIssues = @()
        if ([string]::IsNullOrWhiteSpace($settings.product_name)) { $settingIssues += 'product_name 为空' }
        if ($settings.product_name -ne $productName) { $settingIssues += "product_name 未替换: $($settings.product_name)" }
        if ($settings.version -notmatch '^\d+\.\d+\.\d+$') { $settingIssues += "version 非 semver: $($settings.version)" }
        if ($settings.executable_name -notmatch '^[A-Za-z0-9_-]+$') { $settingIssues += "executable_name 非法: $($settings.executable_name)" }
        if ([string]::IsNullOrWhiteSpace($settings.start_scene)) { $settingIssues += 'start_scene 为空' }
        if (@($settings.targets).Count -eq 0) { $settingIssues += 'targets 为空' }
        foreach ($target in @('windows', 'linux')) {
            if (@($settings.targets) -notcontains $target) { $settingIssues += "targets 缺少 $target" }
        }
        if ($settings.window.width -le 0 -or $settings.window.height -le 0) { $settingIssues += 'window 尺寸非法' }
        $startScene = Join-Path $projectDir ("assets/" + (($settings.start_scene -replace '\\', '/')))
        if (-not (Test-Path -LiteralPath $startScene)) { $settingIssues += "start_scene 文件不存在: $($settings.start_scene)" }
        if ($settingIssues.Count -gt 0) {
            $result.checks['build_settings'] = [ordered]@{ status = 'fail'; issues = $settingIssues }
            $result.errors += "build.settings.json: $($settingIssues -join '; ')"
        } else {
            $result.checks['build_settings'] = [ordered]@{
                status          = 'pass'
                product_name    = $settings.product_name
                executable_name = $settings.executable_name
                start_scene     = $settings.start_scene
                version         = $settings.version
                targets         = @($settings.targets)
                window          = [ordered]@{
                    width  = $settings.window.width
                    height = $settings.window.height
                    mode   = $settings.window.mode
                }
            }
        }
    } catch {
        $result.checks['build_settings'] = [ordered]@{ status = 'fail'; error = $_.Exception.Message }
        $settings = $null
        $result.errors += "build.settings.json 解析失败: $($_.Exception.Message)"
    }

    # 6) Player dry-run：把 build.settings.json 转成 PlayerBuildConfig，并生成 legacy cook 清单。
    if ($null -ne $settings -and $result.errors.Count -eq 0) {
        $buildConfig = [ordered]@{
            product_name    = $settings.product_name
            executable_name = $settings.executable_name
            start_scene     = $settings.start_scene
            version         = $settings.version
            target          = $hostTarget
            quality         = $settings.quality
            window          = [ordered]@{
                title  = $settings.product_name
                width  = $settings.window.width
                height = $settings.window.height
                mode   = 'windowed'
            }
            log_dir         = if ($settings.log_dir) { $settings.log_dir } else { 'logs' }
            required_font   = $null
        }
        $buildConfigPath = Join-Path $playerDir 'build-config.json'
        Write-Utf8 $buildConfigPath (($buildConfig | ConvertTo-Json -Depth 8) + "`n")

        $sceneRelative = ($settings.start_scene -replace '\\', '/')
        $sceneFile = Join-Path $projectDir ("assets/" + $sceneRelative)
        $cook = [ordered]@{
            assets = @(
                [ordered]@{
                    path       = $sceneRelative
                    kind       = 'scene'
                    size_bytes = if (Test-Path -LiteralPath $sceneFile) { (Get-Item -LiteralPath $sceneFile).Length } else { 0 }
                }
            )
        }
        $cookPath = Join-Path $playerDir 'cook-manifest.json'
        Write-Utf8 $cookPath (($cook | ConvertTo-Json -Depth 6) + "`n")

        $planDir = Join-Path $playerDir 'plan'
        $builderArgs = @(
            '--config', $buildConfigPath,
            '--cook', $cookPath,
            '--plan-out', $planDir,
            '--target', 'both',
            '--json'
        )
        $dry = Invoke-Process -FilePath $builder -Arguments $builderArgs -WorkingDirectory $playerDir
        $result.player_plan_dir = $planDir
        $planWindows = Join-Path $planDir 'windows/plan.json'
        $planLinux = Join-Path $planDir 'linux/plan.json'
        if ($dry.exit_code -ne 0 -or -not (Test-Path $planWindows) -or -not (Test-Path $planLinux)) {
            $result.checks['player_dry_run'] = [ordered]@{
                status    = 'fail'
                exit_code = $dry.exit_code
                output    = $dry.output
                plan_dir  = $planDir
            }
            $result.errors += "Player dry-run 失败（exit $($dry.exit_code)）"
        } else {
            $plans = @()
            foreach ($platform in @('windows', 'linux')) {
                $manifestPath = Join-Path $planDir "$platform/manifest.json"
                if (Test-Path $manifestPath) {
                    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
                    $plans += [ordered]@{
                        target         = $platform
                        manifest       = $manifestPath
                        format_version = $manifest.format_version
                        product_name   = $manifest.product_name
                        start_scene    = $manifest.start_scene
                    }
                }
            }
            $result.plans = $plans
            $result.checks['player_dry_run'] = [ordered]@{
                status    = 'pass'
                exit_code = $dry.exit_code
                plan_dir  = $planDir
                platforms = @('windows', 'linux')
                assets    = 1
            }
        }
    }

    if ($result.errors.Count -gt 0) { $result.status = 'fail' }
    return [pscustomobject]$result
}

# ---------------------------------------------------------------------------
# 主流程
# ---------------------------------------------------------------------------
$templates = if ($Template -eq 'all') { @('empty-3d', 'action-sandbox') } else { @($Template) }
$results = @()
foreach ($id in $templates) {
    Write-Host "== fresh-start smoke: $id =="
    try {
        $result = Test-Template -TemplateId $id
    } catch {
        $result = [pscustomobject]@{
            template        = $id
            status          = 'fail'
            project_dir     = ''
            player_plan_dir = ''
            checks          = [ordered]@{}
            plans           = @()
            warnings        = @()
            errors          = @("异常: $($_.Exception.Message)")
        }
    }
    $results += $result
    $color = if ($result.status -eq 'pass') { 'Green' } else { 'Red' }
    Write-Host ("[{0}] {1}" -f $result.status.ToUpperInvariant(), $id) -ForegroundColor $color
    foreach ($errorItem in @($result.errors)) { Write-Host "  - $errorItem" -ForegroundColor Red }
    foreach ($plan in @($result.plans)) {
        Write-Host ("  plan: {0} ({1})" -f $plan.manifest, $plan.target)
    }
}

$overall = if (@($results | Where-Object { $_.status -ne 'pass' }).Count -eq 0) { 'pass' } else { 'fail' }
$report = [ordered]@{
    format_version   = 1
    tool             = 'fresh-start-smoke'
    generated_at_utc = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
    engine_version   = $version
    profile          = $Profile
    skip_build       = $SkipBuild.IsPresent
    offline          = $Offline.IsPresent
    host_target      = $hostTarget
    overall_status   = $overall
    templates        = $results
    warnings         = @($warnings)
}
$reportPath = Join-Path $OutDir 'smoke-report.json'
Write-Utf8 $reportPath (($report | ConvertTo-Json -Depth 12) + "`n")

$reportLines = @(
    'Hezhou 1.0 fresh-start 冒烟报告',
    ("时间(UTC): {0}" -f $report.generated_at_utc),
    ("引擎版本: {0} / 平台: {1} / profile: {2}" -f $version, $hostTarget, $Profile),
    ("结果: {0}" -f $overall.ToUpperInvariant()),
    ''
)
foreach ($result in $results) {
    $reportLines += ("[{0}] {1}" -f $result.status.ToUpperInvariant(), $result.template)
    $reportLines += ("  project: {0}" -f $result.project_dir)
    $reportLines += ("  player plan: {0}" -f $result.player_plan_dir)
    foreach ($check in $result.checks.GetEnumerator()) {
        $reportLines += ("  - {0}: {1}" -f $check.Key, $check.Value.status)
    }
    foreach ($errorItem in @($result.errors)) { $reportLines += ("  ! {0}" -f $errorItem) }
}
$reportLines += ''
$reportLines += "JSON: $reportPath"
$humanPath = Join-Path $OutDir 'REPORT.txt'
Write-Utf8 $humanPath ($reportLines -join "`n")

Write-Host ''
Write-Host '== fresh-start summary =='
Write-Host ("overall : {0}" -f $overall.ToUpperInvariant())
Write-Host ("json    : {0}" -f $reportPath)
Write-Host ("report  : {0}" -f $humanPath)

if ($env:GITHUB_STEP_SUMMARY) {
    $summary = @('## Hezhou fresh-start smoke', '')
    foreach ($result in $results) {
        $summary += ("- ``{0}``: {1} -> {2}" -f $result.template, $result.status, $result.player_plan_dir)
    }
    Add-Content -Path $env:GITHUB_STEP_SUMMARY -Value ($summary -join "`n")
}

exit $(if ($overall -eq 'pass') { 0 } else { 1 })
