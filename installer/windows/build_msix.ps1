param(
    [string]$Version = "",
    [switch]$SkipBuild,
    [string]$CertificatePath = "",
    [string]$CertificatePassword = ""
)
$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$Engine = Join-Path $Root "engine"
$Dist = Join-Path $Root "dist"
$Stage = Join-Path $Root "installer\stage\msix"
$App = $Stage
if (-not $Version) {
    $cargo = Get-Content (Join-Path $Engine "Cargo.toml") -Raw
    $m = [regex]::Match($cargo, '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"')
    $Version = if ($m.Success) { $m.Groups[1].Value } else { "0.1.0" }
}
if (-not $SkipBuild) {
    Push-Location $Engine
    try { cargo build -j 1 --release --features mono; if ($LASTEXITCODE -ne 0) { throw "cargo build failed" } } finally { Pop-Location }
    if (Get-Command mcs -ErrorAction SilentlyContinue) {
        & powershell -ExecutionPolicy Bypass -File (Join-Path $Engine "scripts\build_mono.ps1")
        if ($LASTEXITCODE -ne 0) { throw "C# build failed" }
    }
}
if (Test-Path $Stage) { Remove-Item $Stage -Recurse -Force }
New-Item -ItemType Directory -Force $App | Out-Null
Copy-Item (Join-Path $Engine "target\release\mono_editor_demo.exe") $App
Copy-Item (Join-Path $Engine "target\release\hezhou_launcher.exe") $App
$scriptsStage = Join-Path $App "scripts"; New-Item -ItemType Directory -Force $scriptsStage | Out-Null
Copy-Item (Join-Path $Engine "scripts\*.cs") $scriptsStage
$dll = Join-Path $Engine "scripts\bin\Mono\EditorScript.dll"
if (Test-Path $dll) { New-Item -ItemType Directory -Force (Join-Path $scriptsStage "bin\Mono") | Out-Null; Copy-Item $dll (Join-Path $scriptsStage "bin\Mono\EditorScript.dll") }
Copy-Item (Join-Path $Engine "assets") (Join-Path $App "assets") -Recurse
$fonts = Join-Path $Engine "fonts"
if (Test-Path $fonts) { Copy-Item $fonts (Join-Path $App "fonts") -Recurse }
foreach ($dllName in @("vulkan-1.dll","vcruntime140.dll","vcruntime140_1.dll","msvcp140.dll")) {
    $src = Join-Path $env:WINDIR "System32\$dllName"
    if (Test-Path $src) { Copy-Item $src (Join-Path $App $dllName) }
}
$monoRoot = if ($env:MONO_HOME) { $env:MONO_HOME } elseif (Test-Path "C:\Program Files\Mono") { "C:\Program Files\Mono" } else { "" }
if ($monoRoot) {
    $runtime = Join-Path $App "runtime\mono"; New-Item -ItemType Directory -Force $runtime | Out-Null
    foreach ($part in @("bin","etc")) { Copy-Item (Join-Path $monoRoot $part) (Join-Path $runtime $part) -Recurse }
    New-Item -ItemType Directory -Force (Join-Path $runtime "lib\mono") | Out-Null
    foreach ($asm in @("4.5","4.5-api","4.0-api")) { $src=Join-Path $monoRoot "lib\mono\$asm"; if (Test-Path $src) { Copy-Item $src (Join-Path $runtime "lib\mono\$asm") -Recurse } }
}
Copy-Item (Join-Path $PSScriptRoot "msix\AppxManifest.xml") $Stage
Copy-Item (Join-Path $PSScriptRoot "msix\PackageAssets") $Stage -Recurse
New-Item -ItemType Directory -Force $Dist | Out-Null
$sdkRoot = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\bin"
$makeappx = Get-ChildItem $sdkRoot -Recurse -Filter makeappx.exe -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match '\\x64\\' } | Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
if (-not $makeappx) { throw "makeappx.exe not found under $sdkRoot" }
$msix = Join-Path $Dist "HezhouEditor-$Version.msix"
Remove-Item $msix -Force -ErrorAction SilentlyContinue
& $makeappx pack /d $Stage /p $msix /o
if ($LASTEXITCODE -ne 0) { throw "MakeAppx failed" }
if ($CertificatePath) {
    $signtool = Get-ChildItem $sdkRoot -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match '\\x64\\' } | Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
    & $signtool sign /fd SHA256 /f $CertificatePath /p $CertificatePassword $msix
    if ($LASTEXITCODE -ne 0) { throw "signtool failed" }
}
Write-Host "MSIX: $msix"
