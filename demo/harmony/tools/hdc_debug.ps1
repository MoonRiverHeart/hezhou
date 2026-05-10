# hdc_debug.ps1
# 鸿蒙 HDC 远程调试脚本
# 功能：远程连接、传输 HAP、安装/卸载应用、查看日志

param(
    [Parameter(Mandatory=$true)]
    [string]$DeviceIP,              # 设备 IP，可带端口：192.168.1.6:12345
    
    [int]$Port = -1,                # HDC 端口（-1 表示从 DeviceIP 解析）
    
    [string]$HapFile = "",          # HAP 文件路径（可选）
    
    [string]$BundleName = "com.example.myapplication",
    
    [ValidateSet("connect", "send", "install", "uninstall", "run", "log", "status", "all")]
    [string]$Action = "all",
    
    [switch]$Reinstall
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$HarmonyDir = Split-Path -Parent $ScriptDir
$ToolsDir = "C:\Users\94023\Documents\commandline-tools-windows-x64\command-line-tools"
$HdcExe = "$ToolsDir\sdk\default\openharmony\toolchains\hdc.exe"

# 解析 DeviceIP 中的端口
if ($DeviceIP -match ":") {
    $parts = $DeviceIP -split ":"
    $DeviceIP = $parts[0]
    if ($Port -eq -1) {
        $Port = [int]$parts[1]
    }
}
if ($Port -eq -1) { $Port = 5555 }

$Target = "${DeviceIP}:${Port}"

function Test-Hdc {
    if (-not (Test-Path $HdcExe)) {
        Write-Host "错误: HDC 工具未找到: $HdcExe" -ForegroundColor Red
        return $false
    }
    return $true
}

function Find-HapFile {
    if ($HapFile -ne "" -and (Test-Path $HapFile)) {
        return $HapFile
    }
    
    $hapFiles = Get-ChildItem -Path "$HarmonyDir\entry\build\default\outputs" -Filter "*.hap" -Recurse -ErrorAction SilentlyContinue
    
    if ($hapFiles) {
        $debugHap = $hapFiles | Where-Object { $_.Name -match "unsigned|debug" } | Select-Object -First 1
        if ($debugHap) { return $debugHap.FullName }
        return ($hapFiles | Select-Object -First 1).FullName
    }
    
    return $null
}

function Hdc-Connect {
    Write-Host "========== HDC 远程连接 ==========`n" -ForegroundColor Cyan
    
    Write-Host "连接目标: $Target" -ForegroundColor Yellow
    
    & $HdcExe disconnect $Target 2>&1 | Out-Null
    $result = & $HdcExe connect $Target 2>&1
    
    if ($result -match "connect successfully|already connected") {
        Write-Host "连接成功: $Target" -ForegroundColor Green
        Write-Host "`n设备列表:" -ForegroundColor Yellow
        & $HdcExe list targets 2>&1 | ForEach-Object { Write-Host "  $_" }
        return $true
    } else {
        Write-Host "连接失败: $result" -ForegroundColor Red
        Write-Host "请确认:" -ForegroundColor Yellow
        Write-Host "  1. 设备和电脑在同一网络"
        Write-Host "  2. 设备已开启 HDC 远程调试"
        return $false
    }
}

function Hdc-Send {
    Write-Host "`n========== 发送 HAP 文件 ==========`n" -ForegroundColor Cyan
    
    $hapPath = Find-HapFile
    if (-not $hapPath) {
        Write-Host "错误: 未找到 HAP 文件" -ForegroundColor Red
        return $false
    }
    
    $fileSize = [math]::Round((Get-Item $hapPath).Length / 1KB, 2)
    Write-Host "HAP: $hapPath ($fileSize KB)" -ForegroundColor Cyan
    
    $remotePath = "/data/local/tmp/app.hap"
    
    Write-Host "发送到: $remotePath" -ForegroundColor Yellow
    $result = & $HdcExe -t $Target file send $hapPath $remotePath 2>&1
    
    if ($result -match "error|fail" -or $LASTEXITCODE -ne 0) {
        Write-Host "发送失败: $result" -ForegroundColor Red
        return $false
    }
    
    Write-Host "发送成功" -ForegroundColor Green
    return $true
}

function Hdc-Install {
    Write-Host "`n========== 安装 HAP ==========`n" -ForegroundColor Cyan
    
    $remotePath = "/data/local/tmp/app.hap"
    
    if ($Reinstall) {
        Write-Host "卸载旧版本..." -ForegroundColor Yellow
        & $HdcExe -t $Target uninstall $BundleName 2>&1 | Out-Null
        Start-Sleep -Seconds 1
    }
    
    Write-Host "安装中..." -ForegroundColor Yellow
    $result = & $HdcExe -t $Target install $remotePath 2>&1
    
    if ($result -match "success|Success") {
        Write-Host "安装成功" -ForegroundColor Green
        return $true
    } else {
        Write-Host "安装失败: $result" -ForegroundColor Red
        if ($result -match "signature") {
            Write-Host "提示: HAP 未签名，请在 DevEco Studio 配置签名" -ForegroundColor Yellow
        }
        return $false
    }
}

function Hdc-Uninstall {
    Write-Host "`n========== 卸载应用 ==========`n" -ForegroundColor Cyan
    Write-Host "卸载: $BundleName" -ForegroundColor Yellow
    $result = & $HdcExe -t $Target uninstall $BundleName 2>&1
    if ($result -match "success") {
        Write-Host "卸载成功" -ForegroundColor Green
    } else {
        Write-Host "卸载失败或不存在: $result" -ForegroundColor Yellow
    }
}

function Hdc-Run {
    Write-Host "`n========== 启动应用 ==========`n" -ForegroundColor Cyan
    Write-Host "启动: $BundleName" -ForegroundColor Yellow
    $result = & $HdcExe -t $Target shell aa start -a EntryAbility -b $BundleName 2>&1
    if ($result -match "success") {
        Write-Host "启动成功" -ForegroundColor Green
    } else {
        Write-Host "结果: $result" -ForegroundColor Yellow
    }
}

function Hdc-Log {
    Write-Host "`n========== 实时日志 ==========`n" -ForegroundColor Cyan
    Write-Host "按 Ctrl+C 停止`n" -ForegroundColor Yellow
    & $HdcExe -t $Target shell hilog -T Rust 2>&1
}

function Hdc-Status {
    Write-Host "`n========== 设备状态 ==========`n" -ForegroundColor Cyan
    Write-Host "连接设备:" -ForegroundColor Yellow
    & $HdcExe list targets 2>&1 | ForEach-Object { Write-Host "  $_" }
    Write-Host "`n应用信息:" -ForegroundColor Yellow
    & $HdcExe -t $Target shell bm dump -n $BundleName 2>&1 | 
        Where-Object { $_ -match "name|version" } | 
        ForEach-Object { Write-Host "  $_" }
}

# ========== 主流程 ==========

Write-Host "`n========== HDC 远程调试工具 ==========`n" -ForegroundColor Cyan
Write-Host "目标: $Target" -ForegroundColor Gray

if (-not (Test-Hdc)) { exit 1 }

switch ($Action) {
    "connect" { Hdc-Connect }
    "send" { if (Hdc-Connect) { Hdc-Send } }
    "install" { if (Hdc-Connect) { Hdc-Send; Hdc-Install } }
    "uninstall" { if (Hdc-Connect) { Hdc-Uninstall } }
    "run" { if (Hdc-Connect) { Hdc-Run } }
    "log" { if (Hdc-Connect) { Hdc-Log } }
    "status" { if (Hdc-Connect) { Hdc-Status } }
    "all" { if (Hdc-Connect) { if (Hdc-Send) { if (Hdc-Install) { Hdc-Run } } } }
}

Write-Host "`n========== 完成 ==========`n" -ForegroundColor Cyan