param(
    [string]$ScriptDir = "scripts",
    [string]$OutputDir = "scripts/bin/Mono",
    [string]$ScriptName = "UIScript"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $ScriptDir)) {
    Write-Error "Script directory not found: $ScriptDir"
    exit 1
}

$timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
$outputDll = "$OutputDir/${ScriptName}_${timestamp}.dll"

if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

$mcsPath = "mcs"
try {
    & $mcsPath --version | Out-Null
} catch {
    Write-Error "Mono compiler (mcs) not found. Please install Mono SDK."
    Write-Host "Download from: https://www.mono-project.com/download/stable/"
    exit 1
}

$scriptsToCompile = @(
    "$ScriptDir/$ScriptName.cs",
    "$ScriptDir/DFX.cs",
    "$ScriptDir/UI.cs"
)

foreach ($script in $scriptsToCompile) {
    if (-not (Test-Path $script)) {
        Write-Error "Script file not found: $script"
        exit 1
    }
}

$mcsArgs = @(
    "-target:library",
    "-out:$outputDll",
    "-unsafe"
) + $scriptsToCompile

Write-Host "[Info] Compiling: $ScriptName"
Write-Host "[Info] Output: $outputDll"

& $mcsPath $mcsArgs 2>&1

if ($LASTEXITCODE -ne 0) {
    Write-Error "Compilation failed with exit code: $LASTEXITCODE"
    exit 1
}

if (-not (Test-Path $outputDll)) {
    Write-Error "Output DLL not created: $outputDll"
    exit 1
}

Write-Host "[Success] $outputDll compiled"
Write-Output $outputDll