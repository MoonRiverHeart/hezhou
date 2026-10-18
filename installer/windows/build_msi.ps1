#requires -Version 5
param(
    [string]$Version = "",
    [switch]$SkipBuild,
    [string]$StageDir = ""
)
$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$Engine = Join-Path $Root "engine"
$Dist = Join-Path $Root "dist"
$Stage = if ($StageDir) { $StageDir } else { Join-Path $Root "installer\stage\windows" }

if (-not $Version) {
    $cargo = Get-Content (Join-Path $Engine "Cargo.toml") -Raw
    $m = [regex]::Match($cargo, '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"')
    $Version = if ($m.Success) { $m.Groups[1].Value } else { "0.1.0" }
}

if (-not $SkipBuild) {
    Push-Location $Engine
    try {
        cargo build -j 1 --release --features mono
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    } finally { Pop-Location }

    $monoBuild = Join-Path $Engine "scripts\build_mono.ps1"
    if (Get-Command mcs -ErrorAction SilentlyContinue) {
        & powershell -ExecutionPolicy Bypass -File $monoBuild
        if ($LASTEXITCODE -ne 0) { throw "C# build failed" }
    } else {
        Write-Warning "mcs not found; EditorScript.dll will not be refreshed"
    }
}

if (Test-Path $Stage) { Remove-Item $Stage -Recurse -Force }
New-Item -ItemType Directory -Force $Stage | Out-Null

Copy-Item (Join-Path $Engine "target\release\mono_editor_demo.exe") $Stage
Copy-Item (Join-Path $Engine "scripts") (Join-Path $Stage "scripts") -Recurse
Copy-Item (Join-Path $Engine "assets") (Join-Path $Stage "assets") -Recurse
Copy-Item (Join-Path $Engine "fonts") (Join-Path $Stage "fonts") -Recurse
Copy-Item (Join-Path $Engine "FIXES.md") (Join-Path $Stage "CHANGELOG.md") -ErrorAction SilentlyContinue

# 只保留部署所需的 C# 运行文件，去掉测试/中间产物
$scriptsStage = Join-Path $Stage "scripts"
Get-ChildItem $scriptsStage -Recurse -Directory | Where-Object {
    $_.FullName -match '\\bin\\Mono\\Release' -or $_.FullName -match '\\bin\\Mono\\Debug'
} | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
Get-ChildItem $scriptsStage -Recurse -File -Include *.pdb | Remove-Item -Force -ErrorAction SilentlyContinue

$wix = Get-Command wix -ErrorAction SilentlyContinue
if (-not $wix) {
    Write-Error "WiX not found. Install with: dotnet tool install --global wix --version 5.*"
    exit 2
}

New-Item -ItemType Directory -Force $Dist | Out-Null
$msi = Join-Path $Dist "HezhouEditor-$Version.msi"
Remove-Item $msi -Force -ErrorAction SilentlyContinue
& $wix.Source build (Join-Path $PSScriptRoot "HezhouEditor.wxs") `
    -d StageDir="$Stage" -d Version="$Version" -o "$msi"
if ($LASTEXITCODE -ne 0 -or -not (Test-Path $msi)) { throw "WiX build failed" }
Write-Host "MSI: $msi"
