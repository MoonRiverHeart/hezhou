# build_harmony.ps1

cd C:\Users\94023\Documents\vscode\hezhou\demo\rust

$OhosSdk = "C:\Users\94023\Documents\commandline-tools-windows-x64\command-line-tools\sdk\default\openharmony\native"

Write-Host "开始编译鸿蒙 Rust 动态库..." -ForegroundColor Green

# 设置 RUSTFLAGS
$env:RUSTFLAGS = "-C linker=$OhosSdk\llvm\bin\clang++.exe -C link-arg=--target=aarch64-linux-ohos -C link-arg=--sysroot=$OhosSdk\sysroot"

# 编译
Write-Host "正在编译..." -ForegroundColor Cyan
cargo +nightly build -Zbuild-std --target aarch64-unknown-linux-ohos --release --lib

# 检查结果
$OutputFile = "target\aarch64-unknown-linux-ohos\release\libcsharptorust_lib.so"
if (Test-Path $OutputFile) {
    Write-Host "编译成功！" -ForegroundColor Green
    Write-Host "产物: $OutputFile" -ForegroundColor Yellow
    Get-Item $OutputFile | Format-List Name, Length
} else {
    Write-Host "编译失败" -ForegroundColor Red
}