# build_windows.ps1
Write-Host "=== 编译 Windows 版本 (使用短路径) ===" -ForegroundColor Green

# 短路径（已验证）
$WinsdkShort = "C:\PROGRA~2\WI3CF2~1\10\Lib\100261~1.0\um\x64"
$MonoLib = "C:\PROGRA~1\Mono\lib"  # Program Files 的短路径

# 设置 RUSTFLAGS
$env:RUSTFLAGS = @(
    "-C", "link-arg=/LIBPATH:$WinsdkShort",
    "-C", "link-arg=advapi32.lib",
    "-C", "link-arg=bcrypt.lib",
    "-C", "link-arg=user32.lib",
    "-C", "link-arg=shell32.lib",
    "-C", "link-arg=ole32.lib",
    "-C", "link-arg=oleaut32.lib",
    "-C", "link-arg=/LIBPATH:$MonoLib"
) -join " "

Write-Host "RUSTFLAGS: $env:RUSTFLAGS" -ForegroundColor Cyan

# 编译
Write-Host "正在编译..." -ForegroundColor Cyan
cargo build --bin test_app

if ($LASTEXITCODE -eq 0) {
    Write-Host "`n✅ 编译成功！" -ForegroundColor Green
    Write-Host "产物: target\debug\test_app.exe" -ForegroundColor Yellow
} else {
    Write-Host "`n❌ 编译失败" -ForegroundColor Red
}