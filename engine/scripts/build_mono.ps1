# Build Mono version of C# scripts
# Requires Mono SDK installed (mcs compiler)

param(
    [string]$Configuration = "Release"
)

# Fix: $PSScriptRoot may be empty in some contexts
$ScriptsDir = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
$OutputDir = "$ScriptsDir/bin/Mono/$Configuration/net8.0"

# Create output directory
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

$McsPath = "C:\Program Files\Mono\bin\mcs.bat"

if (-not (Test-Path $McsPath)) {
    Write-Host "[Error] Mono compiler not found at $McsPath" -ForegroundColor Red
    exit 1
}

$SourceFile = "$ScriptsDir/RotationScript.cs"
$UiFile = "$ScriptsDir/UI.cs"
Write-Host "[Info] Compiling: $SourceFile"
Write-Host "[Info] Including: $UiFile"

if (-not (Test-Path $SourceFile)) {
    Write-Host "[Error] Source file not found: $SourceFile" -ForegroundColor Red
    exit 1
}

if (-not (Test-Path $UiFile)) {
    Write-Host "[Error] UI file not found: $UiFile" -ForegroundColor Red
    exit 1
}

# Generate unique assembly name to bypass Mono's cache
$Timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$AssemblyName = "RotationScript_$Timestamp"

# Compile with Mono compiler - assembly name comes from output file name
$OutputDll = "$OutputDir/$AssemblyName.dll"
$Output = & $McsPath `
    -target:library `
    -out:"$OutputDll" `
    $SourceFile `
    $UiFile `
    -define:MONO `
    2>&1

if ($LASTEXITCODE -eq 0) {
    Write-Host "[Success] $AssemblyName.dll compiled to $OutputDir" -ForegroundColor Green
    Write-Host "  DLL size: $((Get-Item $OutputDll).Length) bytes"
    # Output the assembly name for Rust to use
    Write-Host "AssemblyName:$AssemblyName"
} else {
    Write-Host "[Error] Compilation failed:" -ForegroundColor Red
    if ($Output -is [System.Array]) {
        $Output | ForEach-Object { Write-Host $_ -ForegroundColor Red }
    } else {
        Write-Host $Output -ForegroundColor Red
    }
    exit 1
}
