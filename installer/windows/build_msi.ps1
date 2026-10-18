#requires -Version 5
param(
    [string]$Version = "",
    [switch]$SkipBuild,
    [string]$StageDir = "",
    [string]$MonoRoot = "",
    [switch]$NoRuntime
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
Copy-Item (Join-Path $Engine "target\release\hezhou_launcher.exe") $Stage
$scriptsStage = Join-Path $Stage "scripts"
New-Item -ItemType Directory -Force $scriptsStage | Out-Null
Copy-Item (Join-Path $Engine "scripts\*.cs") $scriptsStage
$editorDll = Join-Path $Engine "scripts\bin\Mono\EditorScript.dll"
if (Test-Path $editorDll) {
    New-Item -ItemType Directory -Force (Join-Path $scriptsStage "bin\Mono") | Out-Null
    Copy-Item $editorDll (Join-Path $scriptsStage "bin\Mono\EditorScript.dll") -Force
}
Copy-Item (Join-Path $Engine "assets") (Join-Path $Stage "assets") -Recurse
$fontDir = Join-Path $Engine "fonts"
if (Test-Path $fontDir) {
    Copy-Item $fontDir (Join-Path $Stage "fonts") -Recurse
} else {
    Write-Warning "engine/fonts not present; editor will use a system font"
}
Copy-Item (Join-Path $Engine "FIXES.md") (Join-Path $Stage "CHANGELOG.md") -ErrorAction SilentlyContinue

if (-not $NoRuntime) {
    # Vulkan loader + MSVC runtime: copy next to the executable so the MSI is
    # self-contained on machines without the Vulkan SDK/VC redist.
    foreach ($dll in @("vulkan-1.dll", "vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll")) {
        $src = Join-Path $env:WINDIR "System32\$dll"
        if (Test-Path $src) { Copy-Item $src (Join-Path $Stage $dll) -Force }
    }

    # Bundle a minimal Mono runtime (runtime DLLs, mcs, 4.5 assemblies, config).
    if (-not $MonoRoot) {
        if ($env:MONO_HOME) { $MonoRoot = $env:MONO_HOME }
        elseif (Test-Path "C:\Program Files\Mono") { $MonoRoot = "C:\Program Files\Mono" }
    }
    if ($MonoRoot -and (Test-Path $MonoRoot)) {
        $runtime = Join-Path $Stage "runtime\mono"
        New-Item -ItemType Directory -Force $runtime | Out-Null
        foreach ($part in @("bin", "etc")) {
            Copy-Item (Join-Path $MonoRoot $part) (Join-Path $runtime $part) -Recurse -Force
        }
        $libRoot = Join-Path $runtime "lib\mono"
        New-Item -ItemType Directory -Force $libRoot | Out-Null
        foreach ($asm in @("4.5", "4.5-api", "4.0-api")) {
            $src = Join-Path $MonoRoot "lib\mono\$asm"
            if (Test-Path $src) { Copy-Item $src (Join-Path $libRoot $asm) -Recurse -Force }
        }
        Write-Host "Bundled Mono runtime: $MonoRoot"
    } else {
        Write-Warning "Mono runtime not found; MSI relies on a system Mono installation"
    }
}

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
