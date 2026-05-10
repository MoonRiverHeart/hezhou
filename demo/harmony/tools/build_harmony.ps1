# build_harmony.ps1
# 鸿蒙应用完整构建脚本（命令行方式）

param(
    [string]$JavaHome = "",
    [switch]$SkipRust = $false,
    [switch]$Clean = $false
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$HarmonyDir = Split-Path -Parent $ScriptDir
$RustDir = Join-Path (Split-Path -Parent $HarmonyDir) "rust"
$ToolsDir = "C:\Users\94023\Documents\commandline-tools-windows-x64\command-line-tools"
$OhosSdk = "$ToolsDir\sdk\default\openharmony\native"
$SdkRoot = "$ToolsDir\sdk"

function Find-JavaHome {
    if ($JavaHome -ne "") {
        if (Test-Path "$JavaHome\bin\java.exe") { return $JavaHome }
        if (Test-Path "$JavaHome\java.exe") { return Split-Path -Parent $JavaHome }
    }
    
    if ($env:JAVA_HOME -ne "") {
        if (Test-Path "$env:JAVA_HOME\bin\java.exe") { return $env:JAVA_HOME }
    }
    
    $javaInstallDir = Get-ChildItem "C:\Program Files\Java" -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match "jdk" } |
        Where-Object { Test-Path "$($_.FullName)\bin\java.exe" } |
        Sort-Object Name -Descending |
        Select-Object -First 1
    
    if ($javaInstallDir) { return $javaInstallDir.FullName }
    
    $commonPaths = @(
        "C:\Program Files\Java\jdk-25",
        "C:\Program Files\Java\jdk-25.0.3",
        "C:\Program Files\Java\jdk-21",
        "C:\Program Files\Java\jdk-17"
    )
    
    foreach ($path in $commonPaths) {
        if (Test-Path "$path\bin\java.exe") { return $path }
    }
    
    return $null
}

Write-Host "`n========== 鸿蒙应用构建脚本 ==========`n" -ForegroundColor Cyan

# ========== 步骤 0：检测环境 ==========
Write-Host "步骤 0: 检测环境..." -ForegroundColor Yellow

# 检测 Java
$javaHome = Find-JavaHome
if ($javaHome) {
    Write-Host "  JDK: $javaHome" -ForegroundColor Green
    $env:JAVA_HOME = $javaHome
} else {
    Write-Host "  错误: 未找到 JDK" -ForegroundColor Red
    exit 1
}

# 检测 SDK
if (Test-Path $SdkRoot) {
    Write-Host "  SDK: $SdkRoot" -ForegroundColor Green
    $env:DEVECO_SDK_HOME = $SdkRoot
} else {
    Write-Host "  错误: SDK 目录不存在: $SdkRoot" -ForegroundColor Red
    exit 1
}

Write-Host "  Java 版本:" -ForegroundColor Gray
& "$env:JAVA_HOME\bin\java.exe" -version 2>&1 | ForEach-Object { Write-Host "    $_" }

# ========== 步骤 1：停止守护进程 ==========
Write-Host "`n步骤 1: 停止 hvigor 守护进程..." -ForegroundColor Yellow
Push-Location $HarmonyDir

$NodeExe = "$ToolsDir\tool\node\node.exe"
$HvigorwJs = "$ToolsDir\hvigor\bin\hvigorw.js"

if ((Test-Path $NodeExe) -and (Test-Path $HvigorwJs)) {
    & $NodeExe $HvigorwJs --stop-daemon 2>&1 | Out-Null
    Write-Host "  已停止" -ForegroundColor Green
}

Pop-Location

# ========== 步骤 2：编译 Rust SO ==========
if (-not $SkipRust) {
    Write-Host "`n步骤 2: 编译 Rust SO..." -ForegroundColor Yellow
    Push-Location $RustDir
    
    $env:RUSTFLAGS = "-C linker=$OhosSdk\llvm\bin\clang++.exe -C link-arg=--target=aarch64-linux-ohos -C link-arg=--sysroot=$OhosSdk\sysroot"
    
    cargo +nightly build -Zbuild-std --target aarch64-unknown-linux-ohos --release --lib 2>&1 | 
        Where-Object { $_ -match "error|warning|Finished" } | 
        ForEach-Object { Write-Host "    $_" }
    
    $SoFile = "target\aarch64-unknown-linux-ohos\release\libcsharptorust_lib.so"
    if (Test-Path $SoFile) {
        $DestDir = "$HarmonyDir\entry\src\main\libs\arm64-v8a"
        if (-not (Test-Path $DestDir)) { New-Item -ItemType Directory -Path $DestDir -Force | Out-Null }
        Copy-Item -Path $SoFile -Destination "$DestDir\libcsharptorust_lib.so" -Force
        Write-Host "  成功: $DestDir\libcsharptorust_lib.so" -ForegroundColor Green
    } else {
        Write-Host "  错误: Rust 编译失败" -ForegroundColor Red
        Pop-Location
        exit 1
    }
    
    Pop-Location
} else {
    Write-Host "`n步骤 2: 跳过 Rust 编译" -ForegroundColor Yellow
}

# ========== 步骤 3：清理缓存 ==========
if ($Clean) {
    Write-Host "`n步骤 3: 清理缓存..." -ForegroundColor Yellow
    Push-Location $HarmonyDir
    foreach ($dir in @("entry\.cxx", "entry\build", ".hvigor\cache")) {
        if (Test-Path $dir) { Remove-Item -Path $dir -Recurse -Force -ErrorAction SilentlyContinue }
    }
    Pop-Location
}

# ========== 步骤 4：编译 HAP ==========
Write-Host "`n步骤 4: 编译 Harmony HAP..." -ForegroundColor Yellow
Push-Location $HarmonyDir

# 确保 SDK 环境变量在编译前设置
$env:JAVA_HOME = $javaHome
$env:DEVECO_SDK_HOME = $SdkRoot

Write-Host "  环境变量:" -ForegroundColor Gray
Write-Host "    JAVA_HOME = $env:JAVA_HOME" -ForegroundColor Gray
Write-Host "    DEVECO_SDK_HOME = $env:DEVECO_SDK_HOME" -ForegroundColor Gray

& $NodeExe $HvigorwJs assembleHap --mode module -p product=default 2>&1 | 
    ForEach-Object { 
        if ($_ -match "ERROR|BUILD FAILED") { Write-Host "    $_" -ForegroundColor Red }
        elseif ($_ -match "Finished|BUILD SUCCESS") { Write-Host "    $_" -ForegroundColor Green }
        elseif ($_ -match "WARN") { Write-Host "    $_" -ForegroundColor Yellow }
        else { Write-Host "    $_" -ForegroundColor Gray }
    }

if ($LASTEXITCODE -eq 0) {
    Write-Host "`n========== 构建成功！ ==========`n" -ForegroundColor Green
    Get-ChildItem "$HarmonyDir\entry\build\default\outputs" -Filter "*.hap" -Recurse | 
        ForEach-Object { Write-Host "  HAP: $($_.FullName)" -ForegroundColor Cyan }
} else {
    Write-Host "`n========== 构建失败 ==========`n" -ForegroundColor Red
    Write-Host "  日志: $HarmonyDir\.hvigor\outputs\build-logs\build.log" -ForegroundColor Yellow
}

Pop-Location