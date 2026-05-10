# hdc_debug.ps1
# 鸿蒙 HDC 远程调试脚本
# 功能：远程连接、传输 HAP、安装/卸载应用、查看日志

param(
    [Parameter(Mandatory=$true)]
    [string]$DeviceIP,              # 设备 IP 地址，如 192.168.1.100
    
    [int]$Port = 5555,              # HDC 端口（默认 5555）
    
    [string]$HapFile = "",          # HAP 文件路径（可选，默认自动查找）
    
    [string]$BundleName = "com.example.myapplication",  # 应用包名
    
    [ValidateSet("connect", "install", "uninstall", "run", "log", "status", "all")]
    [string]$Action = "all",        # 操作类型
    
    [switch]$Reinstall              # 强制重新安装（先卸载）
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$HarmonyDir = Split-Path -Parent $ScriptDir
$ToolsDir = "C:\Users\94023\Documents\commandline-tools-windows-x64\command-line-tools"
$HdcExe = "$ToolsDir\sdk\default\openharmony\toolchains\hdc.exe"

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
        # 优先选择未签名的调试版本
        $debugHap = $hapFiles | Where-Object { $_.Name -match "unsigned|debug" } | Select-Object -First 1
        if ($debugHap) { return $debugHap.FullName }
        return ($hapFiles | Select-Object -First 1).FullName
    }
    
    return $null
}

function Hdc-Connect {
    Write-Host "`n========== HDC 远程连接 ==========`n" -ForegroundColor Cyan
    
    $target = "$DeviceIP:$Port"
    
    Write-Host "连接目标: $target" -ForegroundColor Yellow
    
    # 先尝试断开已有连接
    & $HdcExe disconnect $target 2>&1 | Out-Null
    
    # 连接设备
    $result = & $HdcExe connect $target 2>&1
    
    if ($result -match "connect successfully|already connected") {
        Write-Host "连接成功: $target" -ForegroundColor Green
        
        # 显示设备信息
        Write-Host "`n设备信息:" -ForegroundColor Yellow
        & $HdcExe -t $target list targets 2>&1 | ForEach-Object { Write-Host "  $_" }
        
        return $true
    } else {
        Write-Host "连接失败: $result" -ForegroundColor Red
        Write-Host "请确认:" -ForegroundColor Yellow
        Write-Host "  1. 设备和电脑在同一网络"
        Write-Host "  2. 设备已开启 HDC 远程调试（设置 → 开发者选项 → HDC调试）"
        Write-Host "  3. 设备 IP 地址正确"
        return $false
    }
}

function Hdc-Install {
    Write-Host "`n========== 安装 HAP ==========`n" -ForegroundColor Cyan
    
    $hapPath = Find-HapFile
    if (-not $hapPath) {
        Write-Host "错误: 未找到 HAP 文件" -ForegroundColor Red
        Write-Host "请先运行: .\build_harmony.ps1" -ForegroundColor Yellow
        return $false
    }
    
    Write-Host "HAP 文件: $hapPath" -ForegroundColor Cyan
    Write-Host "文件大小: $([math]::Round((Get-Item $hapPath).Length / 1MB, 2)) MB" -ForegroundColor Gray
    
    $target = "$DeviceIP:$Port"
    
    # 传输 HAP 到设备
    Write-Host "`n传输中..." -ForegroundColor Yellow
    $remotePath = "/data/local/tmp/app.hap"
    
    $result = & $HdcExe -t $target file send $hapPath $remotePath 2>&1
    
    if ($result -match "error|fail") {
        Write-Host "传输失败: $result" -ForegroundColor Red
        return $false
    }
    
    Write-Host "传输成功" -ForegroundColor Green
    
    # 如果指定重新安装，先卸载
    if ($Reinstall) {
        Write-Host "`n卸载旧版本..." -ForegroundColor Yellow
        & $HdcExe -t $target uninstall $BundleName 2>&1 | Out-Null
        Start-Sleep -Seconds 1
    }
    
    # 安装 HAP
    Write-Host "`n安装中..." -ForegroundColor Yellow
    $result = & $HdcExe -t $target install $remotePath 2>&1
    
    if ($result -match "success|Success") {
        Write-Host "安装成功" -ForegroundColor Green
        return $true
    } else {
        Write-Host "安装失败: $result" -ForegroundColor Red
        
        # 常见错误提示
        if ($result -match "signature") {
            Write-Host "提示: HAP 未签名或签名无效" -ForegroundColor Yellow
            Write-Host "请在 DevEco Studio 配置签名或使用调试签名" -ForegroundColor Yellow
        }
        
        return $false
    }
}

function Hdc-Uninstall {
    Write-Host "`n========== 卸载应用 ==========`n" -ForegroundColor Cyan
    
    $target = "$DeviceIP:$Port"
    
    Write-Host "卸载: $BundleName" -ForegroundColor Yellow
    $result = & $HdcExe -t $target uninstall $BundleName 2>&1
    
    if ($result -match "success") {
        Write-Host "卸载成功" -ForegroundColor Green
    } else {
        Write-Host "卸载失败或应用不存在: $result" -ForegroundColor Yellow
    }
}

function Hdc-Run {
    Write-Host "`n========== 启动应用 ==========`n" -ForegroundColor Cyan
    
    $target = "$DeviceIP:$Port"
    $ability = "$BundleName/EntryAbility"
    
    Write-Host "启动: $ability" -ForegroundColor Yellow
    $result = & $HdcExe -t $target shell aa start -a EntryAbility -b $BundleName 2>&1
    
    if ($result -match "success|start successfully") {
        Write-Host "启动成功" -ForegroundColor Green
    } else {
        Write-Host "启动结果: $result" -ForegroundColor Yellow
    }
}

function Hdc-Log {
    Write-Host "`n========== 查看日志 ==========`n" -ForegroundColor Cyan
    
    $target = "$DeviceIP:$Port"
    
    Write-Host "实时日志 (Ctrl+C 停止):" -ForegroundColor Yellow
    Write-Host "----------------------------------------" -ForegroundColor Gray
    
    & $HdcExe -t $target shell hilog -T Rust 2>&1
}

function Hdc-Status {
    Write-Host "`n========== 设备状态 ==========`n" -ForegroundColor Cyan
    
    $target = "$DeviceIP:$Port"
    
    Write-Host "连接设备:" -ForegroundColor Yellow
    & $HdcExe list targets 2>&1 | ForEach-Object { Write-Host "  $_" }
    
    Write-Host "`n已安装应用:" -ForegroundColor Yellow
    & $HdcExe -t $target shell bm dump -n $BundleName 2>&1 | 
        Where-Object { $_ -match "name|version|module" } | 
        ForEach-Object { Write-Host "  $_" }
}

# ========== 主流程 ==========

Write-Host "`n========== HDC 远程调试工具 ==========`n" -ForegroundColor Cyan

if (-not (Test-Hdc)) { exit 1 }

switch ($Action) {
    "connect" {
        Hdc-Connect
    }
    "install" {
        if (Hdc-Connect) { Hdc-Install }
    }
    "uninstall" {
        if (Hdc-Connect) { Hdc-Uninstall }
    }
    "run" {
        if (Hdc-Connect) { Hdc-Run }
    }
    "log" {
        if (Hdc-Connect) { Hdc-Log }
    }
    "status" {
        if (Hdc-Connect) { Hdc-Status }
    }
    "all" {
        if (Hdc-Connect) {
            if (Hdc-Install) {
                Hdc-Run
            }
        }
    }
}

Write-Host "`n========== 完成 ==========`n" -ForegroundColor Cyan