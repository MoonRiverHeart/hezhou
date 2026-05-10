# build_harmony.ps1
# 鸿蒙应用完整构建脚本（命令行方式）
# 功能：自动检测 Java -> 编译 Rust SO -> 编译 Harmony HAP

param(
    [string]$JavaHome = "",
    [switch]$SkipRust = $false,
    [switch]$Clean = $false
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$HarmonyDir = Split-Path -Parent $ScriptDir
$RustDir = Join-Path (Split-Path -Parent $HarmonyDir) "rust"
$OhosSdk = "C:\Users\94023\Documents\commandline-tools-windows-x64\command-line-tools\sdk\default\openharmony\native"
$ToolsDir = "C:\Users\94023\Documents\commandline-tools-windows-x64\command-line-tools"

function Find-JavaHome {
    # 1. 检查参数传入
    if ($JavaHome -ne "" -and (Test-Path $JavaHome)) {
        return $JavaHome
    }
    
    # 2. 检查环境变量
    if ($env:JAVA_HOME -ne "" -and (Test-Path $env:JAVA_HOME)) {
        return $env:JAVA_HOME
    }
    
    # 3. 查找 Java 安装目录（真正的 JDK）
    $javaDirs = @()
    
    # 检查 Program Files\Java 下的 JDK
    $pfJava = Get-ChildItem "C:\Program Files\Java" -Directory -ErrorAction SilentlyContinue
    if ($pfJava) {
        $javaDirs += $pfJava | Where-Object { $_.Name -match "jdk" } | Select-Object -ExpandProperty FullName
    }
    
    # 通过 java 命令反推
    $javaCmd = Get-Command java -ErrorAction SilentlyContinue
    if ($javaCmd) {
        $javaBin = Split-Path -Parent $javaCmd.Source
        # javapath 是链接目录，需要找到真实路径
        if ($javaBin -match "javapath") {
            # 尝试读取链接目标
            $linkTarget = (Get-Item $javaCmd.Source).Target
            if ($linkTarget) {
                $realBin = Split-Path -Parent $linkTarget
                $javaDirs += Split-Path -Parent $realBin
            }
        } elseif ($javaBin.EndsWith("\bin")) {
            $javaDirs += Split-Path -Parent $javaBin
        }
    }
    
    # 验证每个路径是否是有效的 JDK（包含 bin\java.exe）
    foreach ($dir in $javaDirs) {
        $javaExe = Join-Path $dir "bin\java.exe"
        if (Test-Path $javaExe) {
            return $dir
        }
    }
    
    # 4. 检查常见安装路径
    $commonPaths = @(
        "C:\Program Files\Java\jdk-25",
        "C:\Program Files\Java\jdk-25.0.3",
        "C:\Program Files\Java\jdk-21",
        "C:\Program Files\Java\jdk-17",
        "C:\Program Files\Java\latest"
    )
    
    foreach ($path in $commonPaths) {
        if (Test-Path $path) {
            return $path
        }
    }
    
    return $null
}

Write-Host "`n========== 鸿蒙应用构建脚本 ==========`n" -ForegroundColor Cyan

# ========== 步骤 0：检测 Java 环境 ==========
Write-Host "步骤 0: 检测 Java 环境..." -ForegroundColor Yellow

$javaHome = Find-JavaHome
if ($javaHome) {
    Write-Host "  找到 Java: $javaHome" -ForegroundColor Green
    $env:JAVA_HOME = $javaHome
} else {
    Write-Host "  错误: 未找到 Java，请安装 JDK 或手动指定 -JavaHome 参数" -ForegroundColor Red
    Write-Host "  示例: .\build_harmony.ps1 -JavaHome 'C:\Program Files\Java\jdk-25'" -ForegroundColor Yellow
    exit 1
}

# 验证 Java 版本
Write-Host "  Java 版本:" -ForegroundColor Gray
& "$env:JAVA_HOME\bin\java.exe" -version 2>&1 | Select-String "version" | ForEach-Object { Write-Host "    $_" }

# ========== 步骤 1：停止旧的 hvigor 守护进程 ==========
Write-Host "`n步骤 1: 停止旧的 hvigor 守护进程..." -ForegroundColor Yellow
Push-Location $HarmonyDir

$NodeExe = "$ToolsDir\tool\node\node.exe"
$HvigorwJs = "$ToolsDir\hvigor\bin\hvigorw.js"

if ((Test-Path $NodeExe) -and (Test-Path $HvigorwJs)) {
    & $NodeExe $HvigorwJs --stop-daemon 2>&1 | Out-Null
    Write-Host "  守护进程已停止" -ForegroundColor Green
} else {
    Write-Host "  警告: hvigorw.js 未找到" -ForegroundColor Yellow
}

Pop-Location

# ========== 步骤 2：编译 Rust SO（可选）==========
if (-not $SkipRust) {
    Write-Host "`n步骤 2: 交叉编译 Rust 鸿蒙 SO..." -ForegroundColor Yellow
    
    if (-not (Test-Path $RustDir)) {
        Write-Host "  错误: Rust 项目目录不存在: $RustDir" -ForegroundColor Red
        exit 1
    }
    
    Push-Location $RustDir
    
    # 设置 RUSTFLAGS
    $env:RUSTFLAGS = "-C linker=$OhosSdk\llvm\bin\clang++.exe -C link-arg=--target=aarch64-linux-ohos -C link-arg=--sysroot=$OhosSdk\sysroot"
    
    Write-Host "  编译中..." -ForegroundColor Gray
    cargo +nightly build -Zbuild-std --target aarch64-unknown-linux-ohos --release --lib 2>&1 | 
        Where-Object { $_ -match "error|warning|Finished" } | 
        ForEach-Object { Write-Host "    $_" }
    
    $SoFile = "target\aarch64-unknown-linux-ohos\release\libcsharptorust_lib.so"
    if (Test-Path $SoFile) {
        Write-Host "  Rust SO 编译成功" -ForegroundColor Green
        
        # 复制到 Harmony 项目
        $DestDir = "$HarmonyDir\entry\src\main\libs\arm64-v8a"
        if (-not (Test-Path $DestDir)) {
            New-Item -ItemType Directory -Path $DestDir -Force | Out-Null
        }
        Copy-Item -Path $SoFile -Destination "$DestDir\libcsharptorust_lib.so" -Force
        Write-Host "  已复制到: $DestDir\libcsharptorust_lib.so" -ForegroundColor Green
    } else {
        Write-Host "  错误: Rust SO 编译失败" -ForegroundColor Red
        Pop-Location
        exit 1
    }
    
    Pop-Location
} else {
    Write-Host "`n步骤 2: 跳过 Rust 编译 (-SkipRust)" -ForegroundColor Yellow
}

# ========== 步骤 3：清理构建缓存（可选）==========
if ($Clean) {
    Write-Host "`n步骤 3: 清理构建缓存..." -ForegroundColor Yellow
    Push-Location $HarmonyDir
    
    $cacheDirs = @(
        "entry\.cxx",
        "entry\build",
        ".hvigor\cache"
    )
    
    foreach ($dir in $cacheDirs) {
        if (Test-Path $dir) {
            Remove-Item -Path $dir -Recurse -Force -ErrorAction SilentlyContinue
            Write-Host "  已删除: $dir" -ForegroundColor Green
        }
    }
    
    Pop-Location
}

# ========== 步骤 4：编译 Harmony HAP ==========
Write-Host "`n步骤 4: 编译 Harmony HAP..." -ForegroundColor Yellow
Push-Location $HarmonyDir

# 再次确保 JAVA_HOME 已设置（守护进程需要）
$env:JAVA_HOME = $javaHome

Write-Host "  编译中..." -ForegroundColor Gray
& $NodeExe $HvigorwJs assembleHap --mode module -p product=default 2>&1 | 
    ForEach-Object { 
        if ($_ -match "ERROR|BUILD FAILED") {
            Write-Host "    $_" -ForegroundColor Red
        } elseif ($_ -match "Finished|BUILD SUCCESS") {
            Write-Host "    $_" -ForegroundColor Green
        } elseif ($_ -match "WARN") {
            Write-Host "    $_" -ForegroundColor Yellow
        } else {
            Write-Host "    $_" -ForegroundColor Gray
        }
    }

if ($LASTEXITCODE -eq 0) {
    Write-Host "`n========== 构建成功！ ==========`n" -ForegroundColor Green
    
    # 查找生成的 HAP 文件
    $HapFiles = Get-ChildItem -Path "$HarmonyDir\entry\build\default\outputs" -Filter "*.hap" -Recurse -ErrorAction SilentlyContinue
    if ($HapFiles) {
        foreach ($hap in $HapFiles) {
            Write-Host "  HAP 文件: $($hap.FullName)" -ForegroundColor Cyan
            Write-Host "  文件大小: $([math]::Round($hap.Length / 1KB, 2)) KB" -ForegroundColor Gray
        }
    } else {
        Write-Host "  注意: 未找到 HAP 文件，请检查构建输出" -ForegroundColor Yellow
    }
} else {
    Write-Host "`n========== 构建失败 ==========`n" -ForegroundColor Red
    Write-Host "  请查看日志: $HarmonyDir\.hvigor\outputs\build-logs\build.log" -ForegroundColor Yellow
}

Pop-Location