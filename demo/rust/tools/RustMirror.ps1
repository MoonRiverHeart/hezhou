param(
    [ValidateSet("set", "clear", "status", "help")]
    [string]$Action = "set"
)

function Set-RustMirror {
    $env:RUSTUP_DIST_SERVER = "https://rsproxy.cn"
    $env:RUSTUP_UPDATE_ROOT = "https://rsproxy.cn/rustup"
    Write-Host "[OK] Rust mirror enabled: rsproxy.cn" -ForegroundColor Green
}

function Clear-RustMirror {
    Remove-Item Env:\RUSTUP_DIST_SERVER -ErrorAction SilentlyContinue
    Remove-Item Env:\RUSTUP_UPDATE_ROOT -ErrorAction SilentlyContinue
    Write-Host "[OK] Rust mirror cleared" -ForegroundColor Green
}

function Get-RustMirrorStatus {
    if ($env:RUSTUP_DIST_SERVER) {
        Write-Host "Status: Mirror enabled" -ForegroundColor Green
        Write-Host "  RUSTUP_DIST_SERVER: $env:RUSTUP_DIST_SERVER"
        Write-Host "  RUSTUP_UPDATE_ROOT: $env:RUSTUP_UPDATE_ROOT"
    } else {
        Write-Host "Status: No mirror (official source)" -ForegroundColor Gray
    }
}

function Show-Help {
    Write-Host ""
    Write-Host "Rust Mirror Script Usage:" -ForegroundColor Yellow
    Write-Host "  .\RustMirror.ps1 set     - Enable mirror (default)"
    Write-Host "  .\RustMirror.ps1 clear   - Disable mirror"
    Write-Host "  .\RustMirror.ps1 status  - Show status"
    Write-Host "  .\RustMirror.ps1 help    - Show this help"
    Write-Host ""
}

# 执行对应的操作
switch ($Action) {
    "set"    { Set-RustMirror }
    "clear"  { Clear-RustMirror }
    "status" { Get-RustMirrorStatus }
    "help"   { Show-Help }
    default  { Set-RustMirror }
}

# 如果只是查看状态，不额外提示
if ($Action -ne "status" -and $Action -ne "help") {
    Write-Host "`nNow you can run: rustup update" -ForegroundColor Yellow
}