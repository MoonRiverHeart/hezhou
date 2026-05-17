# Build Mono version of UI C# scripts
# Requires Mono SDK installed (mcs compiler)

param(
    [string]$Configuration = "Release"
)

$ScriptsDir = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
$OutputDir = "$ScriptsDir/bin/Mono/$Configuration/net8.0"

if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

$McsPath = "C:\Program Files\Mono\bin\mcs.bat"

if (-not (Test-Path $McsPath)) {
    Write-Host "[Error] Mono compiler not found at $McsPath" -ForegroundColor Red
    exit 1
}

$SourceFiles = @(
    "$ScriptsDir/UIScript.cs",
    "$ScriptsDir/ButtonClickTest.cs"
)

foreach ($file in $SourceFiles) {
    Write-Host "[Info] Source: $file"
    if (-not (Test-Path $file)) {
        Write-Host "[Error] Source file not found: $file" -ForegroundColor Red
        exit 1
    }
}

$Timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$AssemblyName = "UIScript_$Timestamp"

$OutputDll = "$OutputDir/$AssemblyName.dll"

$SourceList = $SourceFiles -join " "

$Output = & $McsPath `
    -target:library `
    -out:"$OutputDll" `
    $SourceFiles `
    -define:MONO `
    2>&1

if ($LASTEXITCODE -eq 0) {
    Write-Host "[Success] $AssemblyName.dll compiled to $OutputDir" -ForegroundColor Green
    Write-Host "  DLL size: $((Get-Item $OutputDll).Length) bytes"
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