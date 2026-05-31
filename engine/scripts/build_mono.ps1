# Hezhou引擎 C#脚本统一编译脚本
# 合并了 build_mono.ps1 + build_mono_ui.ps1 + build_ui_mono.ps1
# 用法:
#   powershell -ExecutionPolicy Bypass -File build_mono.ps1          # 编译所有C#脚本（默认）
#   powershell -ExecutionPolicy Bypass -File build_mono.ps1 -UiOnly  # 只编译UI相关脚本
#   powershell -ExecutionPolicy Bypass -File build_mono.ps1 -All     # 编译所有（等同于默认）
#
# 统一使用PATH中的mcs编译器（不硬编码路径）
# 输出统一到 scripts/bin/Mono/Release/net8.0/

param(
    [string]$Configuration = "Release",
    [switch]$UiOnly = $false,
    [switch]$All = $false
)

$ErrorActionPreference = "Continue"

# 路径配置
$ScriptsDir = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
$OutputDir = "$ScriptsDir/bin/Mono/$Configuration/net8.0"

# 创建输出目录
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

# ============================================================
# 检测mcs编译器（优先PATH，fallback硬编码路径）
# ============================================================
$McsPath = $null

# 优先从PATH中查找mcs
try {
    $mcsVersion = & mcs --version 2>&1
    if ($LASTEXITCODE -eq 0) {
        $McsPath = "mcs"
        Write-Host "[信息] 使用PATH中的mcs: $mcsVersion" -ForegroundColor DarkGray
    }
} catch {
    # PATH中没有mcs，尝试硬编码路径
}

if (-not $McsPath) {
    $HardcodedMcs = "C:\Program Files\Mono\bin\mcs.bat"
    if (Test-Path $HardcodedMcs) {
        $McsPath = $HardcodedMcs
        Write-Host "[信息] 使用硬编码路径的mcs: $HardcodedMcs" -ForegroundColor DarkGray
    } else {
        Write-Host "[错误] Mono编译器未找到！" -ForegroundColor Red
        Write-Host "  请安装Mono SDK: https://www.mono-project.com/download/stable/" -ForegroundColor Red
        Write-Host "  或将mcs添加到PATH环境变量" -ForegroundColor Red
        exit 1
    }
}

# ============================================================
# 定义编译文件列表
# ============================================================

# 所有C#脚本文件（完整列表）
$AllSourceFiles = @(
    "$ScriptsDir/RotationScript.cs",
    "$ScriptsDir/UI.cs",
    "$ScriptsDir/UI.Widgets.cs",
    "$ScriptsDir/UI.ComplexWidgets.cs",
    "$ScriptsDir/UI.NewWidgets.cs",
    "$ScriptsDir/UI.Scene.cs",
    "$ScriptsDir/UI.Pipeline.cs",
    "$ScriptsDir/UI.AssetProject.cs",
    "$ScriptsDir/DFX.cs",
    "$ScriptsDir/EditorScript.cs",
    "$ScriptsDir/EditorScript.State.cs",
    "$ScriptsDir/EditorScript.View.cs",
    "$ScriptsDir/EditorScript.Presenter.cs",
    "$ScriptsDir/AssetProjectTest.cs",
    "$ScriptsDir/UITestRunner.cs",
    "$ScriptsDir/ExposeAttribute.cs",
    "$ScriptsDir/IScriptEntity.cs",
    "$ScriptsDir/RotatingEntity.cs",
    "$ScriptsDir/TestRunner.cs"
)

# UI相关脚本子集（用于快速编译UI模块）
$UiSourceFiles = @(
    "$ScriptsDir/UI.cs",
    "$ScriptsDir/UI.Widgets.cs",
    "$ScriptsDir/UI.ComplexWidgets.cs",
    "$ScriptsDir/UI.NewWidgets.cs",
    "$ScriptsDir/UI.Scene.cs",
    "$ScriptsDir/UI.Pipeline.cs",
    "$ScriptsDir/UI.AssetProject.cs",
    "$ScriptsDir/DFX.cs"
)

# ============================================================
# 选择编译目标
# ============================================================
if ($UiOnly -and -not $All) {
    $SourceFiles = $UiSourceFiles
    $AssemblyPrefix = "UIScript"
    Write-Host "[模式] UI脚本编译 (UiOnly)" -ForegroundColor Cyan
} else {
    $SourceFiles = $AllSourceFiles
    $AssemblyPrefix = "RotationScript"
    Write-Host "[模式] 全量脚本编译 (All)" -ForegroundColor Cyan
}

# ============================================================
# 验证源文件存在
# ============================================================
Write-Host "[信息] 编译文件列表:" -ForegroundColor DarkGray
$MissingFiles = @()
foreach ($file in $SourceFiles) {
    $FileName = Split-Path -Leaf $file
    if (Test-Path $file) {
        Write-Host "  ✓ $FileName" -ForegroundColor DarkGray
    } else {
        Write-Host "  ✗ $FileName (缺失)" -ForegroundColor Red
        $MissingFiles += $file
    }
}

if ($MissingFiles.Count -gt 0) {
    Write-Host "[错误] 缺失源文件:" -ForegroundColor Red
    foreach ($f in $MissingFiles) {
        Write-Host "  $f" -ForegroundColor Red
    }
    exit 1
}

# ============================================================
# 编译
# ============================================================
# 生成唯一assembly名称（绕过Mono缓存）
$Timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$AssemblyName = "$AssemblyPrefix_$Timestamp"

$OutputDll = "$OutputDir/$AssemblyName.dll"

Write-Host ""
Write-Host "[编译] $AssemblyName → $OutputDir" -ForegroundColor Green

$Output = & $McsPath `
    -target:library `
    -out:"$OutputDll" `
    $SourceFiles `
    -define:MONO `
    2>&1

if ($LASTEXITCODE -eq 0) {
    $DllSize = (Get-Item $OutputDll).Length
    Write-Host "[成功] $AssemblyName.dll 编译完成" -ForegroundColor Green
    Write-Host "  DLL大小: $DllSize 字节 ($([math]::Round($DllSize / 1024, 2)) KB)" -ForegroundColor DarkGray
    # 输出assembly名称供Rust热重载使用
    Write-Host "AssemblyName:$AssemblyName"
} else {
    Write-Host "[错误] 编译失败:" -ForegroundColor Red
    if ($Output -is [System.Array]) {
        $Output | ForEach-Object { Write-Host $_ -ForegroundColor Red }
    } else {
        Write-Host $Output -ForegroundColor Red
    }
    exit 1
}