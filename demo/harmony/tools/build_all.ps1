# build_all.ps1
# 完整构建流程：Rust SO 编译 -> 复制到 Harmony -> hvigorw 编译 HAP

$ErrorActionPreference = "Stop"

$RustDir = "C:\Users\94023\Documents\vscode\hezhou\demo\rust"
$HarmonyDir = "C:\Users\94023\Documents\vscode\hezhou\demo\harmony"
$HarmonyLibDir = "$HarmonyDir\entry\src\main\libs\arm64-v8a"
$OhosSdk = "C:\Users\94023\Documents\commandline-tools-windows-x64\command-line-tools\sdk\default\openharmony\native"

# ========== 步骤 1：编译 Rust SO ==========
Write-Host "`n========== 步骤 1：交叉编译 Rust 鸿蒙 SO ==========" -ForegroundColor Cyan
Push-Location $RustDir

$env:RUSTFLAGS = "-C linker=$OhosSdk\llvm\bin\clang++.exe -C link-arg=--target=aarch64-linux-ohos -C link-arg=--sysroot=$OhosSdk\sysroot"
cargo +nightly build -Zbuild-std --target aarch64-unknown-linux-ohos --release --lib

$SoFile = "target\aarch64-unknown-linux-ohos\release\libcsharptorust_lib.so"
if (Test-Path $SoFile) {
    Write-Host "Rust SO 编译成功" -ForegroundColor Green
} else {
    Write-Host "Rust SO 编译失败" -ForegroundColor Red
    Pop-Location
    exit 1
}

# ========== 步骤 2：复制 SO 到 Harmony 项目 ==========
Write-Host "`n========== 步骤 2：复制 SO 到 Harmony 项目 ==========" -ForegroundColor Cyan
if (-not (Test-Path $HarmonyLibDir)) {
    New-Item -ItemType Directory -Path $HarmonyLibDir -Force | Out-Null
}
Copy-Item -Path $SoFile -Destination "$HarmonyLibDir\libcsharptorust_lib.so" -Force
Write-Host "已复制: $HarmonyLibDir\libcsharptorust_lib.so" -ForegroundColor Green

Pop-Location

# ========== 步骤 3：hvigorw 编译 HAP ==========
Write-Host "`n========== 步骤 3：hvigorw 编译 Harmony HAP ==========" -ForegroundColor Cyan
Push-Location $HarmonyDir

$NodeExe = "C:\Users\94023\Documents\commandline-tools-windows-x64\command-line-tools\tool\node\node.exe"
$HvigorwJs = "C:\Users\94023\Documents\commandline-tools-windows-x64\command-line-tools\hvigor\bin\hvigorw.js"

& $NodeExe $HvigorwJs assembleHap --mode module -p product=default

if ($LASTEXITCODE -eq 0) {
    Write-Host "`n构建成功！" -ForegroundColor Green
    $HapDir = Get-ChildItem -Path "$HarmonyDir\entry\build\default\outputs" -Filter "*.hap" -Recurse | Select-Object -First 1
    if ($HapDir) {
        Write-Host "HAP 产物: $($HapDir.FullName)" -ForegroundColor Yellow
    }
} else {
    Write-Host "`n构建失败" -ForegroundColor Red
}

Pop-Location
