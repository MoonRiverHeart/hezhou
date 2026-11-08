# Hezhou 1.0 诊断包导出（用户报障 / 稳定性支持）
#
# 采集引擎日志、崩溃/DFX 报告、系统信息、工程与 Player 摘要，做隐私脱敏后
# 打包为单个 zip + summary JSON + 人可读报告。不会上传任何内容，全部写到本地目录。
#
# 用法（在仓库根目录或任意目录）：
#   pwsh -File engine/scripts/diagnostics.ps1 `
#       -OutDir C:\temp\hezhou-diag `
#       -ProjectRoot D:\MyGame `
#       -PlayerRoot D:\MyGame\dist\player\windows
#
# 自检（构造临时 fixture，验证脱敏/排除/zip；CI 使用）：
#   pwsh -File engine/scripts/diagnostics.ps1 -SelfTest
#
# 退出码：0 成功；1 自检失败；2 采集/打包失败。

#Requires -Version 7.0

[CmdletBinding()]
param(
    # 输出目录；省略时使用仓库根目录 test-results/diagnostics/hezhou-diagnostics-<timestamp>。
    [string]$OutDir = '',

    # Hezhou 工程根目录（含 project.json / build.settings.json / assets/）。
    [string]$ProjectRoot = '',

    # Player 包根目录（含 manifest.json）。
    [string]$PlayerRoot = '',

    # 额外日志目录（可多次传入数组）。
    [string[]]$LogDirs = @(),

    # 额外要包含的文件/目录（仍走敏感文件过滤、大小限制与脱敏）。
    [string[]]$IncludePaths = @(),

    # 环境变量白名单（只记录这些变量的值，且仍会脱敏）。
    [string[]]$EnvAllowlist = @(
        'OS', 'PROCESSOR_ARCHITECTURE', 'NUMBER_OF_PROCESSORS',
        'CI', 'GITHUB_SHA', 'GITHUB_RUN_ID', 'GITHUB_WORKSPACE',
        'RUST_VERSION', 'MONO_VERSION', 'HEZHOU_MCP'
    ),

    # 单个文件最大字节数（文本文件超出则保留尾部）。
    [int]$MaxFileMB = 10,

    # 诊断包内容总大小上限（MB）。
    [int]$MaxTotalMB = 50,

    # 每个目录最多收集的文件数。
    [int]$MaxFilesPerDir = 300,

    # 工程/Player 资产索引最多条目数。
    [int]$MaxAssetEntries = 5000,

    # 不采集系统信息（自检/离线环境可用）。
    [switch]$SkipSystemInfo,

    # 自检模式：构造临时 fixture 并断言 zip 内容/脱敏/排除。
    [switch]$SelfTest,

    # 保留 staging 目录（调试用）。
    [switch]$KeepStaging
)

$ErrorActionPreference = 'Stop'

# ===========================================================================
# 常量 / 全局状态
# ===========================================================================

$script:ToolName = 'hezhou-diagnostics'
$script:FormatVersion = 1
$script:Warnings = [System.Collections.Generic.List[string]]::new()
$script:Excluded = [System.Collections.Generic.List[object]]::new()
$script:Files = [System.Collections.Generic.List[object]]::new()
$script:RedactionStats = [ordered]@{ home = 0; user = 0; secret = 0; path = 0 }
$script:TotalBytes = 0L
$script:MaxFileBytes = [Math]::Max(0, $MaxFileMB) * 1MB
$script:MaxTotalBytes = [Math]::Max(1, $MaxTotalMB) * 1MB
$script:MaxFilesPerDir = [Math]::Max(1, $MaxFilesPerDir)
$script:MaxAssetEntries = [Math]::Max(1, $MaxAssetEntries)
$script:SkipSystemInfo = $SkipSystemInfo.IsPresent

$script:TextExtensions = @(
    '.log', '.txt', '.json', '.trace', '.csv', '.md', '.yml', '.yaml',
    '.xml', '.ini', '.cfg', '.conf', '.scene', '.settings'
)

# 敏感/禁止收集的文件模式（文件名或完整路径匹配；命中即不收集内容）。
$script:SensitiveFilePatterns = @(
    '(?i)\.(pfx|p12|cer|crt|key|pem|jks|keystore|snk|kdbx|ovpn|gpg|asc)$',
    '(?i)(^|[\\/])(id_rsa|id_ed25519|id_ecdsa|known_hosts)(\.pub)?$',
    '(?i)(^|[\\/])(token|tokens|secret|secrets|credential|credentials|password|passwords|auth)([._-][^\\/]*)?$',
    '(?i)\.(env|env\..*|npmrc|pypirc|netrc|git-credentials|htpasswd)$',
    '(?i)(^|[\\/])\.git([\\/]|$)',
    '(?i)(^|[\\/])(node_modules|target|\.ssh|\.gnupg|\.aws|\.azure)([\\/]|$)',
    # 崩溃转储可能包含完整内存（含 token/密码），默认只记录元数据、收集内容。
    '(?i)\.(dmp|dump|mdmp)$'
)

# ===========================================================================
# 基础工具函数
# ===========================================================================

function Write-Utf8 {
    param([string]$Path, [string]$Text)
    $parent = Split-Path -Parent $Path
    if ($parent) { New-Item -ItemType Directory -Force $parent | Out-Null }
    # 统一 LF + UTF-8 no BOM，便于跨平台 diff。
    $normalized = $Text -replace "`r`n", "`n"
    [System.IO.File]::WriteAllText($Path, $normalized, [System.Text.UTF8Encoding]::new($false))
}

function Add-Warning {
    param([string]$Message)
    $script:Warnings.Add($Message) | Out-Null
    Write-Verbose "警告: $Message"
}

function Add-Excluded {
    param([string]$Path, [string]$Reason)
    $script:Excluded.Add([pscustomobject]@{
            path   = $Path
            reason = $Reason
        }) | Out-Null
}

function Get-TimestampUtc {
    [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss')
}

function Get-TimestampIso {
    [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
}

function Get-ShortId {
    [guid]::NewGuid().ToString('N').Substring(0, 8)
}

function Resolve-RepoRoot {
    # script 位于 <repo>/engine/scripts/diagnostics.ps1；安装/复制到别处时回退当前目录。
    $scriptsDir = $PSScriptRoot
    $engineDir = Split-Path -Parent $scriptsDir
    $maybeRepo = Split-Path -Parent $engineDir
    if ((Split-Path -Leaf $scriptsDir) -eq 'scripts' -and
        (Split-Path -Leaf $engineDir) -eq 'engine' -and
        (Test-Path (Join-Path $maybeRepo 'engine'))) {
        return $maybeRepo
    }
    return (Get-Location).Path
}

$RepoRoot = Resolve-RepoRoot
$EngineRoot = if (Test-Path (Join-Path $RepoRoot 'engine')) {
    Join-Path $RepoRoot 'engine'
} else {
    Join-Path $RepoRoot 'engine'
}

function Get-HomeDir {
    $homeDir = [Environment]::GetFolderPath('UserProfile')
    if ([string]::IsNullOrWhiteSpace($homeDir)) { $homeDir = $env:HOME }
    if ([string]::IsNullOrWhiteSpace($homeDir)) { $homeDir = $env:USERPROFILE }
    return $homeDir
}

function Get-UserName {
    $userName = [Environment]::UserName
    if ([string]::IsNullOrWhiteSpace($userName)) { $userName = $env:USERNAME }
    return $userName
}

function Get-MachineName {
    try { return [Environment]::MachineName } catch { return 'unknown-host' }
}

function Get-RedactionPairs {
    param([string]$ProjectRootPath, [string]$PlayerRootPath)

    $pairs = [System.Collections.Generic.List[object]]::new()
    $homeDir = Get-HomeDir
    $userName = Get-UserName
    $machine = Get-MachineName

    $add = {
        param([string]$Value, [string]$Token)
        if (-not [string]::IsNullOrWhiteSpace($Value)) {
            $pairs.Add([pscustomobject]@{ value = $Value; token = $Token }) | Out-Null
        }
    }

    # 长路径优先替换（避免家目录先被用户名替换后路径形态破坏）。
    & $add $ProjectRootPath '<project_root>'
    & $add $PlayerRootPath '<player_root>'
    & $add $RepoRoot '<hezhou_root>'
    & $add $homeDir '<user>'
    & $add ([Environment]::GetFolderPath('UserProfile')) '<user>'
    & $add $env:USERPROFILE '<user>'
    & $add $env:HOME '<user>'
    & $add $machine '<computer>'
    & $add $userName '<user>'
    if ($env:USERDOMAIN -and $userName) {
        & $add ("{0}\{1}" -f $env:USERDOMAIN, $userName) '<user>'
    }

    # 去重 + 长值优先，保证替换顺序稳定。
    $seen = @{}
    $unique = @()
    foreach ($pair in ($pairs | Sort-Object { $_.value.Length } -Descending)) {
        $key = $pair.value.ToLowerInvariant()
        if ($seen.ContainsKey($key)) { continue }
        $seen[$key] = $true
        $unique += $pair
    }
    return , $unique
}

function Invoke-Redaction {
    param(
        [string]$Text,
        [object[]]$Pairs
    )
    if ([string]::IsNullOrEmpty($Text)) { return $Text }
    $result = $Text

    foreach ($pair in $Pairs) {
        $pattern = [regex]::Escape($pair.value)
        $matches = [regex]::Matches($result, $pattern, [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
        if ($matches.Count -gt 0) {
            $script:RedactionStats.path += $matches.Count
            $result = [regex]::Replace(
                $result, $pattern, $pair.token,
                [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
        }
    }

    # 通用家目录形态（防止路径以未收录形式出现）。
    $homePatterns = @(
        '(?i)[A-Za-z]:\\Users\\[^\\\s"''<>]+',
        '(?i)/home/[^/\s"''<>]+',
        '(?i)/Users/[^/\s"''<>]+',
        '(?i)%USERPROFILE%',
        '(?i)%LOCALAPPDATA%',
        '(?i)%APPDATA%'
    )
    foreach ($pattern in $homePatterns) {
        $matches = [regex]::Matches($result, $pattern)
        if ($matches.Count -gt 0) {
            $script:RedactionStats.home += $matches.Count
            $result = [regex]::Replace($result, $pattern, '<user>')
        }
    }

    # 常见密钥/令牌形态。
    $secretPatterns = @(
        @{ pattern = '(?i)((?:password|passwd|pwd|secret|token|api[_-]?key|apikey|access[_-]?key|client[_-]?secret|private[_-]?key)\s*[:=]\s*)("?[^\s"''`,;]{3,}"?)'; replacement = '$1<REDACTED>' },
        @{ pattern = '(?i)(\bBearer\s+)[A-Za-z0-9\-._~+/]{8,}=*'; replacement = '$1<REDACTED>' },
        @{ pattern = '(?i)(\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{16,})'; replacement = '<REDACTED>' },
        @{ pattern = '(?i)(\bsk-[A-Za-z0-9]{16,})'; replacement = '<REDACTED>' },
        @{ pattern = '(?i)(\bAKIA[0-9A-Z]{16}\b)'; replacement = '<REDACTED>' },
        @{ pattern = '(?is)(-----BEGIN [A-Z ]*PRIVATE KEY-----).*?(-----END [A-Z ]*PRIVATE KEY-----)'; replacement = '[REDACTED PRIVATE KEY]' }
    )
    foreach ($rule in $secretPatterns) {
        $matches = [regex]::Matches($result, $rule.pattern)
        if ($matches.Count -gt 0) {
            $script:RedactionStats.secret += $matches.Count
            $result = [regex]::Replace($result, $rule.pattern, $rule.replacement)
        }
    }
    return $result
}

function Test-SensitivePath {
    param([string]$Path)
    $normalized = $Path -replace '\\', '/'
    foreach ($pattern in $script:SensitiveFilePatterns) {
        if ([regex]::IsMatch($normalized, $pattern)) {
            return "匹配敏感文件规则: $pattern"
        }
    }
    return $null
}

function Convert-ToArchiveRelative {
    param([string]$Prefix, [string]$Relative)
    $clean = ($Relative -replace '\\', '/').TrimStart('/')
    if ([string]::IsNullOrWhiteSpace($clean)) { return $Prefix }
    $segments = $clean -split '/'
    $safeSegments = foreach ($segment in $segments) {
        $safe = $segment -replace '[<>:"|?*#\x00-\x1f]', '_'
        if ([string]::IsNullOrWhiteSpace($safe)) { '_' } else { $safe }
    }
    return (($Prefix.TrimEnd('/') + '/' + ($safeSegments -join '/')).TrimStart('/'))
}

function Add-FileRecord {
    param(
        [string]$ArchivePath,
        [long]$Bytes,
        [string]$Category,
        [bool]$Truncated,
        [bool]$Redacted
    )
    $script:Files.Add([pscustomobject]@{
            archive_path = $ArchivePath
            bytes        = $Bytes
            category     = $Category
            truncated    = $Truncated
            redacted     = $Redacted
        }) | Out-Null
}

function Copy-SafeFile {
    param(
        [string]$SourcePath,
        [string]$ArchivePath,
        [string]$Category,
        [object[]]$Pairs,
        [string]$StagingDir,
        [switch]$TextOnly
    )

    $reason = Test-SensitivePath $SourcePath
    if ($reason) {
        Add-Excluded $SourcePath $reason
        return $false
    }
    if (-not (Test-Path -LiteralPath $SourcePath -PathType Leaf)) {
        Add-Warning "文件不存在，跳过: $SourcePath"
        return $false
    }
    $item = Get-Item -LiteralPath $SourcePath -ErrorAction Stop
    if ($script:TotalBytes -ge $script:MaxTotalBytes) {
        Add-Excluded $SourcePath '超过诊断包总大小上限，未收集'
        Add-Warning "已达到总大小上限（$MaxTotalMB MB），跳过剩余文件: $SourcePath"
        return $false
    }

    $extension = $item.Extension.ToLowerInvariant()
    $isText = ($script:TextExtensions -contains $extension)
    if ($TextOnly -and -not $isText) {
        Add-Excluded $SourcePath '非文本文件（额外路径只收集文本内容）'
        return $false
    }
    if (-not $isText) {
        # 二进制内容不进入诊断包（避免携带 DLL/证书/转储）；仅记录元数据。
        Add-Excluded $SourcePath '二进制/非文本内容不收集（仅元数据）'
        return $false
    }

    $truncated = $false
    $bytes = [System.IO.File]::ReadAllBytes($SourcePath)
    if ($bytes.LongLength -gt $script:MaxFileBytes) {
        $truncated = $true
        $readLength = [int]$script:MaxFileBytes
        $tail = New-Object byte[] $readLength
        [Array]::Copy($bytes, $bytes.Length - $readLength, $tail, 0, $readLength)
        $bytes = $tail
    }

    $text = [System.Text.Encoding]::UTF8.GetString($bytes)
    $redactedText = Invoke-Redaction -Text $text -Pairs $Pairs

    $target = Join-Path $StagingDir $ArchivePath
    New-Item -ItemType Directory -Force (Split-Path -Parent $target) | Out-Null
    [System.IO.File]::WriteAllText($target, $redactedText, [System.Text.UTF8Encoding]::new($false))

    $script:TotalBytes += $redactedText.Length
    Add-FileRecord -ArchivePath $ArchivePath -Bytes $redactedText.Length -Category $Category -Truncated $truncated -Redacted $true
    return $true
}

function Resolve-FullPath {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) { return $Path }
    try {
        return (Get-Item -LiteralPath $Path -ErrorAction Stop).FullName
    } catch {
        return $Path
    }
}

function Get-RelativeFilePath {
    param([string]$BaseDir, [string]$FullPath)
    $base = $BaseDir.TrimEnd('\', '/')
    $relative = $FullPath.Substring($base.Length)
    return $relative.TrimStart('\', '/')
}

function Collect-TextTree {
    param(
        [string]$Label,
        [string]$SourceDir,
        [string]$ArchivePrefix,
        [string]$Category,
        [object[]]$Pairs,
        [string]$StagingDir,
        [int]$MaxDepth = 3
    )

    $result = [ordered]@{ label = $Label; source = ''; found = $false; files = 0; bytes = 0; truncated = 0 }
    if ([string]::IsNullOrWhiteSpace($SourceDir) -or -not (Test-Path -LiteralPath $SourceDir -PathType Container)) {
        Add-Warning "日志/目录不存在: $Label -> $SourceDir"
        return [pscustomobject]$result
    }
    # 统一成绝对路径：Get-ChildItem 的 FullName 是绝对路径，relative 源目录会算错相对路径。
    $SourceDir = (Resolve-FullPath $SourceDir)
    $result.source = $SourceDir
    $result.found = $true

    try {
        $files = Get-ChildItem -LiteralPath $SourceDir -Recurse -File -ErrorAction SilentlyContinue |
            Where-Object {
                $relative = Get-RelativeFilePath -BaseDir $SourceDir -FullPath $_.FullName
                ($relative -split '[\\/]').Count -le ($MaxDepth + 1) -and
                ($script:TextExtensions -contains $_.Extension.ToLowerInvariant()) -and
                -not ($_.FullName -match '[\\/](node_modules|target|\.git)[\\/]')
            } |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First $script:MaxFilesPerDir

        foreach ($file in $files) {
            $relative = Get-RelativeFilePath -BaseDir $SourceDir -FullPath $file.FullName
            $archive = Convert-ToArchiveRelative -Prefix $ArchivePrefix -Relative $relative
            $sizeBefore = $script:TotalBytes
            if (Copy-SafeFile -SourcePath $file.FullName -ArchivePath $archive -Category $Category -Pairs $Pairs -StagingDir $StagingDir) {
                $result.files++
                $result.bytes += ($script:TotalBytes - $sizeBefore)
            }
        }
    } catch {
        Add-Warning "收集目录失败 $SourceDir : $($_.Exception.Message)"
    }
    return [pscustomobject]$result
}

# ===========================================================================
# 系统 / 环境信息
# ===========================================================================

function Invoke-CapturedCommand {
    param([string[]]$CommandLine, [int]$MaxLines = 60)
    try {
        $output = & $CommandLine[0] @($CommandLine | Select-Object -Skip 1) 2>&1 |
            ForEach-Object { "$_" } |
            Select-Object -First $MaxLines
        return ($output -join "`n")
    } catch {
        return "unavailable: $($_.Exception.Message)"
    }
}

function Get-EngineVersion {
    $cargoToml = Join-Path $EngineRoot 'Cargo.toml'
    if (-not (Test-Path -LiteralPath $cargoToml)) { return $null }
    $lines = Get-Content -LiteralPath $cargoToml -ErrorAction SilentlyContinue
    $inWorkspacePackage = $false
    foreach ($line in $lines) {
        $trimmed = $line.Trim()
        if ($trimmed -match '^\[workspace\.package\]$') { $inWorkspacePackage = $true; continue }
        if ($trimmed -match '^\[') { $inWorkspacePackage = $false; continue }
        if ($inWorkspacePackage -and $trimmed -match '^version\s*=\s*"([^"]+)"') {
            return $Matches[1]
        }
    }
    return $null
}

function Get-GitInfo {
    $git = Get-Command git -ErrorAction SilentlyContinue
    if (-not $git -or -not (Test-Path (Join-Path $RepoRoot '.git'))) {
        return [ordered]@{ available = $false }
    }
    $commit = (& git -C $RepoRoot rev-parse HEAD 2>$null | Select-Object -First 1)
    $branch = (& git -C $RepoRoot rev-parse --abbrev-ref HEAD 2>$null | Select-Object -First 1)
    $status = (& git -C $RepoRoot status --porcelain 2>$null)
    return [ordered]@{
        available = $true
        commit    = if ($commit) { $commit.Trim() } else { $null }
        branch    = if ($branch) { $branch.Trim() } else { $null }
        dirty     = [bool]($status -and $status.Count -gt 0)
    }
}

function Get-SystemSnapshot {
    $snapshot = [ordered]@{
        os             = [Environment]::OSVersion.VersionString
        os_platform    = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
        arch           = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
        powershell     = $PSVersionTable.PSVersion.ToString()
        is_windows     = $IsWindows
        engine_version = Get-EngineVersion
        git            = Get-GitInfo
        cpu            = @()
        memory_gb      = $null
        gpu            = @()
        dotnet         = $null
    }

    if ($IsWindows) {
        try {
            $os = Get-CimInstance Win32_OperatingSystem -ErrorAction Stop
            $snapshot.os = "$($os.Caption) $($os.Version) (build $($os.BuildNumber))"
            $snapshot.memory_gb = [math]::Round([double]$os.TotalVisibleMemorySize / 1MB, 2)
        } catch {
            Add-Warning "读取 Win32_OperatingSystem 失败: $($_.Exception.Message)"
        }
        try {
            $cpus = Get-CimInstance Win32_Processor -ErrorAction Stop
            $snapshot.cpu = @($cpus | ForEach-Object {
                    [ordered]@{
                        name  = $_.Name
                        cores = $_.NumberOfCores
                        logical_processors = $_.NumberOfLogicalProcessors
                    }
                })
        } catch {
            Add-Warning "读取 Win32_Processor 失败: $($_.Exception.Message)"
        }
        try {
            $gpus = Get-CimInstance Win32_VideoController -ErrorAction Stop
            $snapshot.gpu = @($gpus | ForEach-Object {
                    [ordered]@{ name = $_.Name; driver = $_.DriverVersion }
                })
        } catch {
            Add-Warning "读取 Win32_VideoController 失败: $($_.Exception.Message)"
        }
    } else {
        $snapshot.os_platform = (Invoke-CapturedCommand @('uname', '-a') -MaxLines 1)
        # Linux CPU / 内存
        if (Test-Path '/proc/cpuinfo') {
            try {
                $model = (Get-Content '/proc/cpuinfo' | Where-Object { $_ -match '^model name' } | Select-Object -First 1)
                if ($model) { $model = ($model -split ':', 2)[1].Trim() }
                $logical = (Get-Content '/proc/cpuinfo' | Where-Object { $_ -match '^processor' }).Count
                $snapshot.cpu = @([ordered]@{ name = $model; logical_processors = $logical })
            } catch { Add-Warning "读取 /proc/cpuinfo 失败: $($_.Exception.Message)" }
        }
        if (Test-Path '/proc/meminfo') {
            try {
                $line = Get-Content '/proc/meminfo' | Where-Object { $_ -match '^MemTotal:' } | Select-Object -First 1
                if ($line -match '(\d+)') {
                    $snapshot.memory_gb = [math]::Round(([double]$Matches[1]) / 1MB, 2)
                }
            } catch { Add-Warning "读取 /proc/meminfo 失败: $($_.Exception.Message)" }
        }
        $lspci = Get-Command lspci -ErrorAction SilentlyContinue
        if ($lspci) {
            try {
                $gpuLines = & lspci 2>$null | Where-Object { $_ -match 'VGA|3D|Display' }
                $snapshot.gpu = @($gpuLines | ForEach-Object { [ordered]@{ name = $_ } })
            } catch { Add-Warning "lspci 读取 GPU 失败: $($_.Exception.Message)" }
        }
    }
    if (Get-Command dotnet -ErrorAction SilentlyContinue) {
        $snapshot.dotnet = (Invoke-CapturedCommand @('dotnet', '--version') -MaxLines 1)
    }
    return $snapshot
}

function Collect-SystemInfo {
    param(
        [string]$StagingDir,
        [object[]]$Pairs,
        [string[]]$EnvAllowlistNames
    )
    $systemDir = Join-Path $StagingDir 'system'
    New-Item -ItemType Directory -Force $systemDir | Out-Null

    $snapshot = Get-SystemSnapshot
    $commands = [ordered]@{
        git    = Invoke-CapturedCommand @('git', '--version')
        rustc  = Invoke-CapturedCommand @('rustc', '--version')
        cargo  = Invoke-CapturedCommand @('cargo', '--version')
        mono   = Invoke-CapturedCommand @('mono', '--version')
        python = Invoke-CapturedCommand @('python', '--version')
        pwsh   = Invoke-CapturedCommand @('pwsh', '-NoProfile', '-Command', '$PSVersionTable.PSVersion.ToString()')
    }
    if (Get-Command vulkaninfo -ErrorAction SilentlyContinue) {
        $commands.vulkaninfo = Invoke-CapturedCommand @('vulkaninfo', '--summary') -MaxLines 80
    } else {
        $commands.vulkaninfo = 'unavailable: vulkaninfo not found'
    }

    $envValues = [ordered]@{}
    foreach ($name in $EnvAllowlistNames) {
        try {
            $value = [Environment]::GetEnvironmentVariable($name)
            if ($null -ne $value) {
                # 变量名像密钥时连值都不记录（白名单也可能被误配成 token）。
                if ($name -match '(?i)(token|secret|password|passwd|key|auth)') {
                    $script:RedactionStats.secret++
                    $envValues[$name] = '<REDACTED>'
                } else {
                    $envValues[$name] = $value
                }
            }
        } catch { Add-Warning "读取环境变量 $name 失败: $($_.Exception.Message)" }
    }

    $payload = [ordered]@{
        generated_at_utc = Get-TimestampIso
        system           = $snapshot
        commands         = $commands
        environment      = $envValues
        process          = [ordered]@{
            pid           = $PID
            clr_version   = [System.Environment]::Version.ToString()
            culture       = [System.Globalization.CultureInfo]::CurrentCulture.Name
        }
    }
    $json = $payload | ConvertTo-Json -Depth 8
    $json = Invoke-Redaction -Text $json -Pairs $Pairs
    Write-Utf8 (Join-Path $systemDir 'system.json') $json
    Add-FileRecord -ArchivePath 'system/system.json' -Bytes $json.Length -Category 'system' -Truncated $false -Redacted $true
}

# ===========================================================================
# 日志 / 崩溃
# ===========================================================================

function Get-DefaultLogCandidates {
    param([string]$ProjectRootPath, [string]$PlayerRootPath)

    $candidates = [System.Collections.Generic.List[object]]::new()
    $add = {
        param([string]$Label, [string]$Path)
        if (-not [string]::IsNullOrWhiteSpace($Path)) {
            $candidates.Add([pscustomobject]@{ label = $Label; path = $Path }) | Out-Null
        }
    }

    & $add 'engine-repo-logs' (Join-Path $RepoRoot 'logs')
    & $add 'engine-workspace-logs' (Join-Path $EngineRoot 'logs')

    if ($IsWindows) {
        if ($env:LOCALAPPDATA) {
            & $add 'editor-userdata-logs' (Join-Path $env:LOCALAPPDATA 'Hezhou/Editor/logs')
            & $add 'editor-userdata' (Join-Path $env:LOCALAPPDATA 'Hezhou/Editor')
            $playerDataRoot = Join-Path $env:LOCALAPPDATA 'Hezhou'
            if (Test-Path $playerDataRoot) {
                Get-ChildItem -LiteralPath $playerDataRoot -Directory -ErrorAction SilentlyContinue |
                    ForEach-Object { & $add ("player-logs-" + $_.Name) (Join-Path $_.FullName 'logs') }
            }
        }
    } else {
        $dataHome = $env:XDG_DATA_HOME
        if (-not $dataHome) { $dataHome = Join-Path (Get-HomeDir) '.local/share' }
        & $add 'editor-userdata-logs' (Join-Path $dataHome 'hezhou/editor/logs')
        & $add 'editor-userdata' (Join-Path $dataHome 'hezhou/editor')
        $playerDataRoot = Join-Path $dataHome 'hezhou'
        if (Test-Path $playerDataRoot) {
            Get-ChildItem -LiteralPath $playerDataRoot -Directory -ErrorAction SilentlyContinue |
                ForEach-Object { & $add ("player-logs-" + $_.Name) (Join-Path $_.FullName 'logs') }
        }
    }

    if ($ProjectRootPath) { & $add 'project-logs' (Join-Path $ProjectRootPath 'logs') }
    if ($PlayerRootPath) { & $add 'player-package-logs' (Join-Path $PlayerRootPath 'logs') }

    return ($candidates | Sort-Object { $_.label } | Select-Object -Unique -Property label, path)
}

function Collect-CrashFiles {
    param(
        [string]$StagingDir,
        [object[]]$Pairs,
        [string[]]$CandidateDirs
    )
    $crashDir = Join-Path $StagingDir 'crashes'
    New-Item -ItemType Directory -Force $crashDir | Out-Null
    $collected = 0
    $dumps = [System.Collections.Generic.List[object]]::new()
    $seen = @{}

    foreach ($dir in $CandidateDirs) {
        if ([string]::IsNullOrWhiteSpace($dir) -or -not (Test-Path -LiteralPath $dir -PathType Container)) { continue }
        $dir = Resolve-FullPath $dir
        try {
            Get-ChildItem -LiteralPath $dir -File -ErrorAction SilentlyContinue |
                Where-Object {
                    $_.Name -match '(?i)(crash|dfx|startup-error|panic)' -or
                    $_.Extension -match '(?i)^\.(dmp|dump|mdmp)$'
                } |
                ForEach-Object {
                    $key = $_.FullName.ToLowerInvariant()
                    if ($seen.ContainsKey($key)) { return }
                    $seen[$key] = $true
                    if ($_.Extension -match '(?i)^\.(dmp|dump|mdmp)$') {
                        $dumps.Add([pscustomobject]@{ name = $_.Name; bytes = $_.Length; mtime = $_.LastWriteTime.ToString('o') }) | Out-Null
                        Add-Excluded $_.FullName '崩溃转储可能含完整内存（token/密码），仅记录元数据'
                        return
                    }
                    $archive = Convert-ToArchiveRelative -Prefix 'crashes' -Relative $_.Name
                    if (Copy-SafeFile -SourcePath $_.FullName -ArchivePath $archive -Category 'crashes' -Pairs $Pairs -StagingDir $StagingDir) {
                        $collected++
                    }
                }
        } catch {
            Add-Warning "收集崩溃文件失败 $dir : $($_.Exception.Message)"
        }
    }
    if ($dumps.Count -gt 0) {
        $dumpJson = $dumps | ConvertTo-Json -Depth 4
        Write-Utf8 (Join-Path $crashDir 'excluded-dumps.json') (Invoke-Redaction $dumpJson $Pairs)
    }
    return [pscustomobject]@{ files = $collected; dumps_excluded = $dumps.Count }
}

function New-ErrorScan {
    param([string]$StagingDir, [object[]]$Pairs)
    $lines = [System.Collections.Generic.List[string]]::new()
    $lines.Add('# 诊断包错误/异常扫描（从已脱敏日志中提取，最多 50 条）') | Out-Null
    $pattern = '(?i)\b(error|fatal|panic|exception|crash|failed)\b'
    $scanned = 0
    $logsDir = Join-Path $StagingDir 'logs'
    if (Test-Path $logsDir) {
        Get-ChildItem -LiteralPath $logsDir -Recurse -File -ErrorAction SilentlyContinue |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 30 |
            ForEach-Object {
                $scanned++
                $content = Get-Content -LiteralPath $_.FullName -ErrorAction SilentlyContinue -TotalCount 5000
                foreach ($line in $content) {
                    if ($line -match $pattern) {
                        if ($lines.Count -le 50) {
                            $lines.Add(("[{0}] {1}" -f $_.Name, $line.Trim().Substring(0, [Math]::Min(300, $line.Trim().Length)))) | Out-Null
                        }
                    }
                }
            }
    }
    $text = Invoke-Redaction ($lines -join "`n") $Pairs
    $crashDir = Join-Path $StagingDir 'crashes'
    New-Item -ItemType Directory -Force $crashDir | Out-Null
    Write-Utf8 (Join-Path $crashDir 'error-scan.txt') $text
    Add-FileRecord -ArchivePath 'crashes/error-scan.txt' -Bytes $text.Length -Category 'crashes' -Truncated $false -Redacted $true
    return [pscustomobject]@{ scanned_logs = $scanned; matched_lines = [Math]::Max(0, $lines.Count - 1) }
}

# ===========================================================================
# 工程 / Player 摘要
# ===========================================================================

function Get-SafeJsonFile {
    param([string]$Path)
    try {
        return Get-Content -LiteralPath $Path -Raw -ErrorAction Stop | ConvertFrom-Json
    } catch {
        Add-Warning "解析 JSON 失败 $Path : $($_.Exception.Message)"
        return $null
    }
}

function Add-FileIndex {
    param(
        [string]$SourceDir,
        [string]$ArchivePath,
        [string]$StagingDir,
        [object[]]$Pairs,
        [string]$Label,
        [string]$Category = 'project'
    )
    $entries = [System.Collections.Generic.List[object]]::new()
    $SourceDir = Resolve-FullPath $SourceDir
    if (-not (Test-Path -LiteralPath $SourceDir -PathType Container)) {
        return [pscustomobject]@{ label = $Label; dir = $SourceDir; exists = $false; files = 0; bytes = 0 }
    }
    try {
        $files = Get-ChildItem -LiteralPath $SourceDir -Recurse -File -ErrorAction SilentlyContinue |
            Select-Object -First $script:MaxAssetEntries
        foreach ($file in $files) {
            # 敏感文件（证书/令牌/转储等）连文件名都不进入索引。
            if (Test-SensitivePath $file.FullName) { continue }
            $entries.Add([ordered]@{
                    path  = ((Get-RelativeFilePath -BaseDir $SourceDir -FullPath $file.FullName) -replace '\\', '/')
                    bytes = $file.Length
                    ext   = $file.Extension.ToLowerInvariant()
                }) | Out-Null
        }
    } catch {
        Add-Warning "索引目录失败 $SourceDir : $($_.Exception.Message)"
    }
    $payload = [ordered]@{
        label = $Label
        files = $entries
    }
    $json = Invoke-Redaction ($payload | ConvertTo-Json -Depth 6) $Pairs
    $target = Join-Path $StagingDir $ArchivePath
    New-Item -ItemType Directory -Force (Split-Path -Parent $target) | Out-Null
    Write-Utf8 $target $json
    Add-FileRecord -ArchivePath $ArchivePath -Bytes $json.Length -Category $Category -Truncated $false -Redacted $true
    return [pscustomobject]@{
        label = $Label
        dir   = $SourceDir
        exists = $true
        files = $entries.Count
        bytes = ($entries | ForEach-Object { [long]$_['bytes'] } | Measure-Object -Sum).Sum
    }
}

function Collect-ProjectSummary {
    param([string]$ProjectRootPath, [string]$StagingDir, [object[]]$Pairs)

    $result = [ordered]@{
        provided = [bool]$ProjectRootPath
        found    = $false
        summary  = $null
        assets   = $null
        cook     = $null
    }
    if ([string]::IsNullOrWhiteSpace($ProjectRootPath)) {
        return [pscustomobject]$result
    }
    if (-not (Test-Path -LiteralPath $ProjectRootPath -PathType Container)) {
        Add-Warning "工程目录不存在: $ProjectRootPath"
        return [pscustomobject]$result
    }
    $result.found = $true
    $ProjectRootPath = Resolve-FullPath $ProjectRootPath
    $projectDir = Join-Path $StagingDir 'project'
    New-Item -ItemType Directory -Force $projectDir | Out-Null

    $projectJsonPath = Join-Path $ProjectRootPath 'project.json'
    $buildSettingsPath = Join-Path $ProjectRootPath 'build.settings.json'
    $project = if (Test-Path $projectJsonPath) { Get-SafeJsonFile $projectJsonPath } else { $null }
    $build = if (Test-Path $buildSettingsPath) { Get-SafeJsonFile $buildSettingsPath } else { $null }

    if ($project) {
        $summary = [ordered]@{
            name           = $project.name
            version        = $project.version
            created_at     = $project.created_at
            modified_at    = $project.modified_at
            entity_count   = @($project.entities).Count
            asset_count    = @($project.assets).Count
            script_count   = @($project.scripts).Count
            game_settings  = $project.settings
            has_path       = -not [string]::IsNullOrWhiteSpace($project.path)
        }
        $result.summary = $summary
        Copy-SafeFile -SourcePath $projectJsonPath -ArchivePath 'project/project.json' -Category 'project' -Pairs $Pairs -StagingDir $StagingDir | Out-Null
    } else {
        Add-Warning "未找到/无法解析 project.json: $ProjectRootPath"
    }
    if ($build) {
        $buildSummary = [ordered]@{
            product_name  = $build.product_name
            version       = $build.version
            start_scene   = $build.start_scene
            quality       = $build.quality
            window        = $build.window
            targets       = $build.targets
            log_dir       = $build.log_dir
            required_font = $build.required_font
        }
        $result.summary = [ordered]@{ project = $result.summary; build = $buildSummary }
        Copy-SafeFile -SourcePath $buildSettingsPath -ArchivePath 'project/build.settings.json' -Category 'project' -Pairs $Pairs -StagingDir $StagingDir | Out-Null
    } else {
        Add-Warning "未找到/无法解析 build.settings.json: $ProjectRootPath"
    }

    $result.assets = Add-FileIndex -SourceDir (Join-Path $ProjectRootPath 'assets') `
        -ArchivePath 'project/assets-index.json' -StagingDir $StagingDir -Pairs $Pairs -Label 'assets'

    $cookCandidates = @(
        (Join-Path $ProjectRootPath 'cook_manifest.json'),
        (Join-Path $ProjectRootPath 'cooked/cook_manifest.json'),
        (Join-Path $ProjectRootPath 'build/cook_manifest.json')
    )
    foreach ($cookPath in $cookCandidates) {
        if (Test-Path -LiteralPath $cookPath) {
            if (Copy-SafeFile -SourcePath $cookPath -ArchivePath 'project/cook_manifest.json' -Category 'project' -Pairs $Pairs -StagingDir $StagingDir) {
                $cook = Get-SafeJsonFile $cookPath
                if ($cook) {
                    $result.cook = [ordered]@{
                        format_version = $cook.format_version
                        roots          = @($cook.roots).Count
                        entries        = @($cook.entries).Count
                    }
                }
            }
            break
        }
    }
    return [pscustomobject]$result
}

function Collect-PlayerSummary {
    param([string]$PlayerRootPath, [string]$StagingDir, [object[]]$Pairs)

    $result = [ordered]@{
        provided       = [bool]$PlayerRootPath
        found          = $false
        manifest_valid = $false
        summary        = $null
        scripts        = $null
        assets         = $null
        runtime        = $null
    }
    if ([string]::IsNullOrWhiteSpace($PlayerRootPath)) {
        return [pscustomobject]$result
    }
    if (-not (Test-Path -LiteralPath $PlayerRootPath -PathType Container)) {
        Add-Warning "Player 目录不存在: $PlayerRootPath"
        return [pscustomobject]$result
    }
    $result.found = $true
    $PlayerRootPath = Resolve-FullPath $PlayerRootPath
    $playerDir = Join-Path $StagingDir 'player'
    New-Item -ItemType Directory -Force $playerDir | Out-Null

    $manifestPath = Join-Path $PlayerRootPath 'manifest.json'
    if (Test-Path -LiteralPath $manifestPath) {
        $manifest = Get-SafeJsonFile $manifestPath
        if ($manifest) {
            $result.manifest_valid = $true
            $result.summary = [ordered]@{
                format_version = $manifest.format_version
                engine_version = $manifest.engine_version
                product_name   = $manifest.product_name
                version        = $manifest.version
                start_scene    = $manifest.start_scene
                target         = $manifest.target
                quality        = $manifest.quality
                window         = $manifest.window
                font           = $manifest.font
                scripts        = @($manifest.scripts).Count
                assets         = @($manifest.assets).Count
                built_by       = $manifest.built_by
            }
        } else {
            Add-Warning "manifest.json 无法解析: $manifestPath"
        }
        Copy-SafeFile -SourcePath $manifestPath -ArchivePath 'player/manifest.json' -Category 'player' -Pairs $Pairs -StagingDir $StagingDir | Out-Null
    } else {
        Add-Warning "Player 根目录缺少 manifest.json: $PlayerRootPath"
    }

    $result.scripts = Add-FileIndex -SourceDir (Join-Path $PlayerRootPath 'scripts') `
        -ArchivePath 'player/scripts-index.json' -StagingDir $StagingDir -Pairs $Pairs -Label 'scripts' -Category 'player'
    $result.assets = Add-FileIndex -SourceDir (Join-Path $PlayerRootPath 'assets') `
        -ArchivePath 'player/assets-index.json' -StagingDir $StagingDir -Pairs $Pairs -Label 'assets' -Category 'player'
    $result.runtime = Add-FileIndex -SourceDir (Join-Path $PlayerRootPath 'runtime') `
        -ArchivePath 'player/runtime-index.json' -StagingDir $StagingDir -Pairs $Pairs -Label 'runtime' -Category 'player'

    $buildConfig = Join-Path $PlayerRootPath 'build-config.json'
    if (Test-Path -LiteralPath $buildConfig) {
        Copy-SafeFile -SourcePath $buildConfig -ArchivePath 'player/build-config.json' -Category 'player' -Pairs $Pairs -StagingDir $StagingDir | Out-Null
    }
    return [pscustomobject]$result
}

# ===========================================================================
# 主采集流程
# ===========================================================================

function Invoke-DiagnosticsCollection {
    param(
        [string]$TargetOutDir,
        [string]$ProjectRootPath,
        [string]$PlayerRootPath,
        [string[]]$ExtraLogDirs,
        [string[]]$ExtraIncludePaths,
        [string[]]$EnvAllowlistNames,
        [switch]$IsSelfTest
    )

    $timestamp = Get-TimestampUtc
    if ([string]::IsNullOrWhiteSpace($TargetOutDir)) {
        $TargetOutDir = Join-Path $RepoRoot "test-results/diagnostics/hezhou-diagnostics-$timestamp"
    }
    New-Item -ItemType Directory -Force $TargetOutDir | Out-Null
    $TargetOutDir = Resolve-FullPath $TargetOutDir
    $ProjectRootPath = Resolve-FullPath $ProjectRootPath
    $PlayerRootPath = Resolve-FullPath $PlayerRootPath

    # 每次采集重置统计（同一进程内多次调用时状态隔离）。
    $script:Warnings.Clear()
    $script:Excluded.Clear()
    $script:Files.Clear()
    $script:TotalBytes = 0L
    $script:RedactionStats = [ordered]@{ home = 0; user = 0; secret = 0; path = 0 }

    $staging = Join-Path $TargetOutDir "staging-$timestamp-$(Get-ShortId)"
    New-Item -ItemType Directory -Force $staging | Out-Null
    $pairs = Get-RedactionPairs -ProjectRootPath $ProjectRootPath -PlayerRootPath $PlayerRootPath

    if (-not $script:SkipSystemInfo) {
        try { Collect-SystemInfo -StagingDir $staging -Pairs $pairs -EnvAllowlistNames $EnvAllowlistNames }
        catch { Add-Warning "系统信息采集失败: $($_.Exception.Message)" }
    }

    # 日志目录：默认候选 + 参数 + 工程/Player 日志。
    # 自检模式只允许 fixture 提供的目录，避免读到开发机真实日志/用户数据。
    $logCandidates = @()
    if (-not $IsSelfTest) {
        $logCandidates = @(Get-DefaultLogCandidates -ProjectRootPath $ProjectRootPath -PlayerRootPath $PlayerRootPath)
    }
    # 工程/Player 自带的 logs 目录始终纳入（自检也走同一路径逻辑）。
    if ($ProjectRootPath) {
        $logCandidates += [pscustomobject]@{ label = 'project-logs'; path = (Join-Path $ProjectRootPath 'logs') }
    }
    if ($PlayerRootPath) {
        $logCandidates += [pscustomobject]@{ label = 'player-package-logs'; path = (Join-Path $PlayerRootPath 'logs') }
    }
    $extraIndex = 0
    foreach ($dir in $ExtraLogDirs) {
        $extraIndex++
        if (-not [string]::IsNullOrWhiteSpace($dir)) {
            $logCandidates += [pscustomobject]@{ label = "user-log-$extraIndex"; path = $dir }
        }
    }
    $logSummaries = @()
    $seenLogDirs = @{}
    foreach ($candidate in $logCandidates) {
        if ([string]::IsNullOrWhiteSpace($candidate.path)) { continue }
        $key = $candidate.path.ToLowerInvariant()
        if ($seenLogDirs.ContainsKey($key)) { continue }
        $seenLogDirs[$key] = $true
        $logSummaries += Collect-TextTree -Label $candidate.label -SourceDir $candidate.path `
            -ArchivePrefix ("logs/" + ($candidate.label -replace '[^A-Za-z0-9_.-]', '_')) `
            -Category 'logs' -Pairs $pairs -StagingDir $staging -MaxDepth 2
    }

    # 崩溃/DFX 报告：仓库根、各日志目录、工程/Player 根目录。
    # 自检模式只扫描 fixture 提供的目录。
    $crashCandidates = @()
    if (-not $IsSelfTest) { $crashCandidates = @($RepoRoot, (Get-Location).Path) }
    foreach ($candidate in $logCandidates) {
        if ($candidate.path -and (Test-Path -LiteralPath $candidate.path -PathType Container)) {
            $crashCandidates += $candidate.path
            $crashCandidates += (Split-Path -Parent $candidate.path)
        }
    }
    if ($ProjectRootPath -and (Test-Path $ProjectRootPath)) { $crashCandidates += $ProjectRootPath }
    if ($PlayerRootPath -and (Test-Path $PlayerRootPath)) { $crashCandidates += $PlayerRootPath }
    $crashSummaries = @()
    foreach ($dir in ($crashCandidates | Select-Object -Unique)) {
        $crashSummaries += Collect-CrashFiles -StagingDir $staging -Pairs $pairs -CandidateDirs @($dir)
    }
    $errorScan = New-ErrorScan -StagingDir $staging -Pairs $pairs

    $projectSummary = $null
    try { $projectSummary = Collect-ProjectSummary -ProjectRootPath $ProjectRootPath -StagingDir $staging -Pairs $pairs }
    catch { Add-Warning "工程摘要失败: $($_.Exception.Message)" }

    $playerSummary = $null
    try { $playerSummary = Collect-PlayerSummary -PlayerRootPath $PlayerRootPath -StagingDir $staging -Pairs $pairs }
    catch { Add-Warning "Player 摘要失败: $($_.Exception.Message)" }

    # 额外包含路径。
    $extraSummaries = @()
    $extraIndex = 0
    foreach ($includePath in $ExtraIncludePaths) {
        $extraIndex++
        if ([string]::IsNullOrWhiteSpace($includePath)) { continue }
        if (-not (Test-Path -LiteralPath $includePath)) {
            Add-Warning "IncludePaths 不存在，跳过: $includePath"
            continue
        }
        $includePath = Resolve-FullPath $includePath
        if ((Get-Item -LiteralPath $includePath).PSIsContainer) {
            $extraSummaries += Collect-TextTree -Label "extra-$extraIndex" -SourceDir $includePath `
                -ArchivePrefix ("extra/" + "path$extraIndex") -Category 'extra' -Pairs $pairs -StagingDir $staging -MaxDepth 3
        } else {
            $item = Get-Item -LiteralPath $includePath
            $archive = "extra/" + ($item.Name -replace '[^A-Za-z0-9_.-]', '_')
            $ok = Copy-SafeFile -SourcePath $item.FullName -ArchivePath $archive -Category 'extra' -Pairs $pairs -StagingDir $staging -TextOnly
            $extraSummaries += [pscustomobject]@{ label = "extra-file-$extraIndex"; files = $(if ($ok) { 1 } else { 0 }); bytes = $item.Length }
        }
    }

    # ---- 汇总 ----
    $logFiles = ($logSummaries | Measure-Object -Property files -Sum).Sum
    $logBytes = ($logSummaries | Measure-Object -Property bytes -Sum).Sum
    $crashFiles = ($crashSummaries | Measure-Object -Property files -Sum).Sum
    $crashDumps = ($crashSummaries | Measure-Object -Property dumps_excluded -Sum).Sum
    $zipName = "hezhou-diagnostics-$timestamp.zip"

    $safePairs = $pairs
    $summary = [ordered]@{
        format_version      = $script:FormatVersion
        tool                = $script:ToolName
        tool_version        = '1.0'
        generated_at_utc    = Get-TimestampIso
        host                = [ordered]@{
            os          = ([System.Runtime.InteropServices.RuntimeInformation]::OSDescription)
            arch        = ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString())
            user        = '<user>'
            computer    = '<computer>'
            powershell  = $PSVersionTable.PSVersion.ToString()
        }
        engine_version      = Get-EngineVersion
        git                 = Get-GitInfo
        sources             = [ordered]@{
            logs    = [ordered]@{
                candidates = $logSummaries
                files      = $logFiles
                bytes      = $logBytes
            }
            crashes = [ordered]@{
                candidates     = $crashSummaries
                files          = $crashFiles
                dumps_excluded = $crashDumps
                error_scan     = $errorScan
            }
            project = $projectSummary
            player  = $playerSummary
            extra   = $extraSummaries
        }
        redaction           = [ordered]@{
            home_like_replacements = $script:RedactionStats.home
            path_replacements      = $script:RedactionStats.path
            secret_replacements    = $script:RedactionStats.secret
            excluded_files         = @($script:Excluded)
        }
        limits              = [ordered]@{
            max_file_mb       = $MaxFileMB
            max_total_mb      = $MaxTotalMB
            max_files_per_dir = $MaxFilesPerDir
            max_asset_entries = $MaxAssetEntries
            collected_bytes   = $script:TotalBytes
        }
        files               = @($script:Files)
        warnings            = @($script:Warnings)
        self_test           = $IsSelfTest.IsPresent
    }
    $summaryJson = Invoke-Redaction ($summary | ConvertTo-Json -Depth 10) $safePairs
    Write-Utf8 (Join-Path $staging 'summary.json') $summaryJson
    Add-FileRecord -ArchivePath 'summary.json' -Bytes $summaryJson.Length -Category 'metadata' -Truncated $false -Redacted $true

    # 人可读报告
    $reportLines = @(
        'Hezhou 诊断包（请检查后再外发）',
        ('生成时间(UTC): {0}' -f $summary.generated_at_utc),
        ('引擎版本: {0}' -f $(if ($summary.engine_version) { $summary.engine_version } else { 'unknown' })),
        ('git: {0}' -f $(if ($summary.git.available) { "$($summary.git.branch)@$($summary.git.commit) dirty=$($summary.git.dirty)" } else { 'unavailable' })),
        ('系统: {0} / {1}' -f $summary.host.os, $summary.host.arch),
        ('日志: {0} 个文件 / {1:N0} 字节' -f $logFiles, $logBytes),
        ('崩溃/DFX: {0} 个文件；转储未收集 {1} 个' -f $crashFiles, $crashDumps),
        ('工程: 提供={0} 找到={1}' -f $projectSummary.provided, $projectSummary.found),
        ('Player: 提供={0} 找到={1} manifest有效={2}' -f $playerSummary.provided, $playerSummary.found, $playerSummary.manifest_valid),
        ('脱敏: 家目录类={0} 路径={1} 密钥类={2}；排除文件={3}' -f $script:RedactionStats.home, $script:RedactionStats.path, $script:RedactionStats.secret, $script:Excluded.Count),
        ('诊断包大小上限: {0} MB（当前内容 {1:N2} MB）' -f $MaxTotalMB, ($script:TotalBytes / 1MB)),
        '',
        '注意：诊断包仍可能包含日志中的业务信息；外发前请人工检查。',
        ''
    )
    if ($script:Warnings.Count -gt 0) {
        $reportLines += '警告:'
        foreach ($warning in $script:Warnings) { $reportLines += "  - $warning" }
    }
    $reportText = $reportLines -join "`n"
    $reportText = Invoke-Redaction -Text $reportText -Pairs $safePairs
    Write-Utf8 (Join-Path $staging 'REPORT.txt') $reportText

    # ---- 打包 ----
    $zipPath = Join-Path $TargetOutDir $zipName
    if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
    try {
        Compress-Archive -Path (Join-Path $staging '*') -DestinationPath $zipPath -CompressionLevel Optimal -Force
    } catch {
        throw "创建 zip 失败: $($_.Exception.Message)"
    }
    $zipBytes = (Get-Item -LiteralPath $zipPath).Length
    $summaryOut = Join-Path $TargetOutDir 'summary.json'
    Copy-Item -LiteralPath (Join-Path $staging 'summary.json') -Destination $summaryOut -Force
    Copy-Item -LiteralPath (Join-Path $staging 'REPORT.txt') -Destination (Join-Path $TargetOutDir 'REPORT.txt') -Force

    if (-not $KeepStaging) {
        Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue
    }

    return [pscustomobject]@{
        out_dir      = $TargetOutDir
        zip_path     = $zipPath
        zip_bytes    = $zipBytes
        summary_path = $summaryOut
        report_path  = (Join-Path $TargetOutDir 'REPORT.txt')
        summary      = $summaryJson | ConvertFrom-Json
        staging      = $(if ($KeepStaging) { $staging } else { $null })
    }
}

# ===========================================================================
# 自检（fixture）
# ===========================================================================

function Write-FixtureFile {
    param([string]$Path, [string]$Content)
    New-Item -ItemType Directory -Force (Split-Path -Parent $Path) | Out-Null
    [System.IO.File]::WriteAllText($Path, $Content, [System.Text.UTF8Encoding]::new($false))
}

function Invoke-DiagnosticsSelfTest {
    param([string]$BaseOutDir)

    $root = if ([string]::IsNullOrWhiteSpace($BaseOutDir)) {
        Join-Path $RepoRoot 'test-results/diagnostics/selftest'
    } else { $BaseOutDir }
    if (Test-Path $root) { Remove-Item -LiteralPath $root -Recurse -Force }

    $fixture = Join-Path $root 'fixture'
    $homeDir = Get-HomeDir
    $userName = Get-UserName
    $homeMarker = if ($homeDir) { $homeDir } else { '/home/example' }
    # JSON 字符串里的 Windows 路径需要把反斜杠转义成 `\\`。
    $homeJson = $homeMarker -replace '\\', '\\\\'
    $userMarker = if ($userName) { $userName } else { 'example' }

    # 日志：包含家目录、用户名、密码/token/Bearer 等敏感内容
    Write-FixtureFile (Join-Path $fixture 'logs/hezhou_2026-11-08.log') @"
[INFO] 启动 at $homeMarker\project
[WARN] user=$userMarker
[ERROR] password=hunter2-in-log
[ERROR] token=SECRET_TOKEN_IN_LOG
[ERROR] Authorization: Bearer abc.def.ghi
[INFO] normal line ok
"@
    Write-FixtureFile (Join-Path $fixture 'logs/token.txt') 'token=SHOULD_NOT_BE_COLLECTED'
    Write-FixtureFile (Join-Path $fixture 'logs/notes.md') '测试日志 $userMarker'
    Write-FixtureFile (Join-Path $fixture 'crashes/crash_report.txt') @"
=== Crash Report ===
Time: 2026-11-08T00:00:00Z
Message: panic at $homeMarker\game
password=crashed-secret
"@
    [System.IO.File]::WriteAllBytes((Join-Path (New-Item -ItemType Directory -Force (Join-Path $fixture 'crashes')).FullName 'memory.dmp'), [byte[]](1..32))

    # 工程
    Write-FixtureFile (Join-Path $fixture 'project/project.json') @"
{
  "version": "1.0",
  "name": "FixtureGame",
  "path": "$homeJson\\project",
  "created_at": "2026-01-01T00:00:00Z",
  "modified_at": "2026-01-01T00:00:00Z",
  "settings": { "game_width": 1280, "game_height": 720, "fps": 60 },
  "entities": [ { "id": 1 } ],
  "assets": [],
  "scripts": []
}
"@
    Write-FixtureFile (Join-Path $fixture 'project/build.settings.json') @'
{
  "product_name": "FixtureGame",
  "executable_name": "fixture-game",
  "start_scene": "scenes/main.scene",
  "quality": "high",
  "window": { "width": 1280, "height": 720, "mode": "windowed" },
  "targets": ["windows", "linux"],
  "version": "1.0.0",
  "log_dir": "logs",
  "required_font": null
}
'@
    Write-FixtureFile (Join-Path $fixture 'project/assets/scenes/main.scene') '{"entities":[]}'
    Write-FixtureFile (Join-Path $fixture 'project/assets/data.json') '{"api_key":"FIXTURE_API_KEY_SECRET"}'
    Write-FixtureFile (Join-Path $fixture 'project/assets/models/hero.bin') 'binary-placeholder'
    Write-FixtureFile (Join-Path $fixture 'project/credentials.env') 'TOKEN=NEVER_COLLECT'
    [System.IO.File]::WriteAllBytes((Join-Path (New-Item -ItemType Directory -Force (Join-Path $fixture 'project/assets')).FullName 'client.pfx'), [byte[]](1..16))

    # Player 包
    Write-FixtureFile (Join-Path $fixture 'player/manifest.json') @'
{
  "format_version": 1,
  "engine_version": "0.3.0",
  "product_name": "FixtureGame",
  "version": "1.0.0",
  "start_scene": "scenes/main.scene",
  "target": "windows",
  "quality": "high",
  "window": { "title": "FixtureGame", "width": 1280, "height": 720, "mode": "windowed" },
  "paths": { "assets": "assets", "scripts": "scripts", "runtime": "runtime", "logs": "logs" },
  "font": null,
  "scripts": ["Game.dll"],
  "assets": [],
  "built_by": "fixture"
}
'@
    Write-FixtureFile (Join-Path $fixture 'player/logs/player.log') "启动 $homeMarker\player; user=$userMarker; token=PLAYER_TOKEN_SECRET"
    Write-FixtureFile (Join-Path $fixture 'player/logs/startup-error.log') 'startup failed: password=STARTUP_SECRET'
    Write-FixtureFile (Join-Path $fixture 'player/build-config.json') '{"product_name":"FixtureGame"}'
    [System.IO.File]::WriteAllBytes((Join-Path (New-Item -ItemType Directory -Force (Join-Path $fixture 'player/scripts')).FullName 'Game.dll'), [byte[]](1..16))
    [System.IO.File]::WriteAllBytes((Join-Path (New-Item -ItemType Directory -Force (Join-Path $fixture 'player/runtime/mono')).FullName 'mono.dll'), [byte[]](1..16))
    Write-FixtureFile (Join-Path $fixture 'player/certs/client.pfx') 'certificate-placeholder'

    # 额外 include 路径
    Write-FixtureFile (Join-Path $fixture 'extra/notes.log') "extra log $homeMarker user=$userMarker"
    Write-FixtureFile (Join-Path $fixture 'extra/id_rsa') 'PRIVATE KEY SHOULD NOT BE COLLECTED'

    $packageOut = Join-Path $root 'package'
    $result = Invoke-DiagnosticsCollection `
        -TargetOutDir $packageOut `
        -ProjectRootPath (Join-Path $fixture 'project') `
        -PlayerRootPath (Join-Path $fixture 'player') `
        -ExtraLogDirs @((Join-Path $fixture 'logs'), (Join-Path $fixture 'crashes')) `
        -ExtraIncludePaths @((Join-Path $fixture 'extra')) `
        -EnvAllowlistNames @('CI', 'GITHUB_SHA') `
        -IsSelfTest

    # ---- 断言 ----
    function Assert-True {
        param([bool]$Condition, [string]$Message)
        if (-not $Condition) { throw "自检失败: $Message" }
    }
    function Assert-NoSecret {
        param([string]$Text, [string]$Path)
        $lower = $Text.ToLowerInvariant()
        foreach ($needle in @(
                'hunter2-in-log', 'secret_token_in_log', 'abc.def.ghi',
                'fixture_api_key_secret', 'never_collect', 'player_token_secret',
                'startup_secret', 'crashed-secret', 'private key should not be collected'
            )) {
            Assert-True (-not $lower.Contains($needle)) "脱敏失败：$Path 中仍包含 $needle"
        }
        if ($homeMarker -and $Text.Contains($homeMarker)) {
            throw "自检失败：$Path 仍包含家目录 $homeMarker"
        }
        if ($userMarker.Length -ge 3 -and $Text.Contains($userMarker)) {
            throw "自检失败：$Path 仍包含用户名 $userMarker"
        }
    }

    Assert-True (Test-Path $result.zip_path) 'zip 未生成'
    Assert-True (Test-Path $result.summary_path) 'summary.json 未生成'
    Assert-True (Test-Path $result.report_path) 'REPORT.txt 未生成'
    Assert-True ($result.zip_bytes -gt 0) 'zip 为空'

    $checkDir = Join-Path $root 'check'
    if (Test-Path $checkDir) { Remove-Item $checkDir -Recurse -Force }
    Expand-Archive -LiteralPath $result.zip_path -DestinationPath $checkDir -Force

    $required = @(
        'summary.json', 'REPORT.txt',
        'logs/user-log-1/hezhou_2026-11-08.log',
        'crashes/crash_report.txt',
        'project/project.json',
        'project/build.settings.json',
        'project/assets-index.json',
        'player/manifest.json',
        'logs/player-package-logs/player.log',
        'logs/player-package-logs/startup-error.log',
        'extra/path1/notes.log'
    )
    foreach ($relative in $required) {
        $candidate = Join-Path $checkDir ($relative -replace '/', [IO.Path]::DirectorySeparatorChar)
        Assert-True (Test-Path -LiteralPath $candidate) "缺少预期文件: $relative"
    }

    # 敏感文件必须完全不存在于 zip 中。
    $forbidden = @(
        'token.txt', 'credentials.env', 'client.pfx', 'memory.dmp',
        'id_rsa', 'mono.dll', 'Game.dll'
    )
    $allZipFiles = Get-ChildItem -LiteralPath $checkDir -Recurse -File
    foreach ($file in $allZipFiles) {
        foreach ($name in $forbidden) {
            Assert-True (-not ($file.Name -ieq $name)) "敏感/二进制文件被错误收集: $($file.FullName)"
        }
    }

    # 脱敏断言：扫描 zip 中所有文本文件。
    foreach ($file in ($allZipFiles | Where-Object { $script:TextExtensions -contains $_.Extension.ToLowerInvariant() })) {
        $text = Get-Content -LiteralPath $file.FullName -Raw -ErrorAction SilentlyContinue
        if ($null -ne $text) { Assert-NoSecret $text $file.FullName }
    }
    Assert-True ((Get-Content -LiteralPath (Join-Path $checkDir 'summary.json') -Raw) -match '"self_test"\s*:\s*true') 'summary.json 未标记 self_test'
    $parsedSummary = Get-Content -LiteralPath (Join-Path $checkDir 'summary.json') -Raw | ConvertFrom-Json
    Assert-True ($parsedSummary.sources.project.found -eq $true) '工程摘要未标记 found'
    Assert-True ($parsedSummary.sources.player.manifest_valid -eq $true) 'Player manifest 未校验通过'
    Assert-True (@($parsedSummary.redaction.excluded_files).Count -gt 0) '未记录任何被排除文件'

    # 坏路径安全：不存在的工程/Player/日志目录不应抛异常。
    $badOut = Join-Path $root 'bad-paths-package'
    $bad = Invoke-DiagnosticsCollection `
        -TargetOutDir $badOut `
        -ProjectRootPath (Join-Path $root 'missing-project') `
        -PlayerRootPath (Join-Path $root 'missing-player') `
        -ExtraLogDirs @((Join-Path $root 'missing-logs')) `
        -ExtraIncludePaths @((Join-Path $root 'missing-extra')) `
        -EnvAllowlistNames @() `
        -IsSelfTest
    Assert-True (Test-Path $bad.zip_path) '坏路径场景未生成 zip（应安全降级）'

    Write-Host 'SELFTEST PASS'
    Write-Host ("fixture : {0}" -f $fixture)
    Write-Host ("package : {0}" -f $packageOut)
    Write-Host ("zip     : {0} ({1:N0} bytes)" -f $result.zip_path, $result.zip_bytes)
    Write-Host ("summary : {0}" -f $result.summary_path)
    Write-Host ("bad-path: {0}" -f $bad.zip_path)
}

# ===========================================================================
# 入口
# ===========================================================================

try {
    if ($SelfTest) {
        Invoke-DiagnosticsSelfTest -BaseOutDir $OutDir
        exit 0
    }

    $result = Invoke-DiagnosticsCollection `
        -TargetOutDir $OutDir `
        -ProjectRootPath $ProjectRoot `
        -PlayerRootPath $PlayerRoot `
        -ExtraLogDirs $LogDirs `
        -ExtraIncludePaths $IncludePaths `
        -EnvAllowlistNames $EnvAllowlist

    Write-Host '== Hezhou 诊断包已生成 =='
    Write-Host ("out_dir : {0}" -f $result.out_dir)
    Write-Host ("zip     : {0} ({1:N2} MB)" -f $result.zip_path, ($result.zip_bytes / 1MB))
    Write-Host ("summary : {0}" -f $result.summary_path)
    Write-Host ("report  : {0}" -f $result.report_path)
    Write-Host ''
    Write-Host '摘要：'
    Get-Content -LiteralPath $result.report_path | ForEach-Object { Write-Host "  $_" }
    exit 0
} catch {
    Write-Host ''
    Write-Host ("诊断包生成失败: {0}" -f $_.Exception.Message)
    Write-Host $_.ScriptStackTrace
    exit 2
}
