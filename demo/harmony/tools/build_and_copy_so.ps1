# build_and_copy_so.ps1
# 编译 Rust 鸿蒙 SO 并自动复制到 Harmony 项目目录

$RustDir = "C:\Users\94023\Documents\vscode\hezhou\demo\rust"
$HarmonyLibDir = "C:\Users\94023\Documents\vscode\hezhou\demo\harmony\entry\src\main\libs\arm64-v8a"

Push-Location $RustDir

Write-Host "开始交叉编译 Rust 鸿蒙 SO..." -ForegroundColor Green
$env:RUSTFLAGS = "-C linker=C:\Users\94023\Documents\commandline-tools-windows-x64\command-line-tools\sdk\default\openharmony\native\llvm\bin\clang++.exe -C link-arg=--target=aarch64-linux-ohos -C link-arg=--sysroot=C:\Users\94023\Documents\commandline-tools-windows-x64\command-line-tools\sdk\default\openharmony\native\sysroot"
cargo +nightly build -Zbuild-std --target aarch64-unknown-linux-ohos --release --lib

$SoFile = "target\aarch64-unknown-linux-ohos\release\libcsharptorust_lib.so"
if (Test-Path $SoFile) {
    Write-Host "编译成功，正在复制到 Harmony 项目..." -ForegroundColor Yellow
    Copy-Item -Path $SoFile -Destination $HarmonyLibDir -Force
    Write-Host "已复制到: $HarmonyLibDir\libcsharptorust_lib.so" -ForegroundColor Green
} else {
    Write-Host "编译失败，未找到 SO 文件" -ForegroundColor Red
}

Pop-Location
