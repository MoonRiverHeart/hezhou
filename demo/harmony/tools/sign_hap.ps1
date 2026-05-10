param(
    [string]$HapPath = "",
    [string]$OutputPath = "",
    [string]$BundleName = "com.example.myapplication"
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$HarmonyRoot = Split-Path -Parent $ScriptDir
$SdkRoot = $env:DEVECO_SDK_HOME
if (-not $SdkRoot) {
    $SdkRoot = "C:\Users\94023\Documents\commandline-tools-windows-x64\command-line-tools\sdk"
}

$JavaHome = $env:JAVA_HOME
if (-not $JavaHome) {
    $JavaHome = "C:\Program Files\Java\jdk-25.0.3"
}

$ToolchainsLib = Join-Path $SdkRoot "default\openharmony\toolchains\lib"
$HapSignTool = Join-Path $ToolchainsLib "hap-sign-tool.jar"
$OpenHarmonyP12 = Join-Path $ToolchainsLib "OpenHarmony.p12"
$ProfileTemplate = Join-Path $ToolchainsLib "UnsgnedDebugProfileTemplate.json"
$ProfileDebugPem = Join-Path $ToolchainsLib "OpenHarmonyProfileDebug.pem"

$KeystorePwd = "123456"
$ProfileKeyAlias = "openharmony application profile debug"
$RootCaAlias = "openharmony application root ca"
$SubCaAlias = "openharmony application ca"

if (-not $HapPath) {
    $HapPath = Join-Path $HarmonyRoot "entry\build\default\outputs\default\entry-default-unsigned.hap"
}

if (-not (Test-Path $HapPath)) {
    Write-Error "HAP not found: $HapPath"
    exit 1
}

if (-not $OutputPath) {
    $OutputPath = $HapPath -replace "-unsigned\.hap$", "-signed.hap"
}

Write-Host "========== HAP Signing Script ==========" -ForegroundColor Cyan

$TempDir = Join-Path $env:TEMP "hap_sign_$(Get-Date -Format 'yyyyMMddHHmmss')"
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

try {
    Write-Host "`nStep 1: Create app keystore..." -ForegroundColor Yellow
    
    $AppKeystore = Join-Path $TempDir "app-keystore.p12"
    $AppKeyAlias = "app-key-$BundleName"
    
    $genKeyArgs = @(
        "generate-keypair",
        "-keyAlias", $AppKeyAlias,
        "-keyPwd", $KeystorePwd,
        "-keyAlg", "ECC",
        "-keySize", "NIST-P-256",
        "-keystoreFile", $AppKeystore,
        "-keystorePwd", $KeystorePwd
    )
    
    $env:JAVA_HOME = $JavaHome
    & java -jar $HapSignTool @genKeyArgs 2>&1 | ForEach-Object { Write-Host "  $_" }
    
    Write-Host "`nStep 2: Export CA certificates..." -ForegroundColor Yellow
    
    $RootCaCer = Join-Path $TempDir "root_ca.cer"
    $SubCaCer = Join-Path $TempDir "sub_ca.cer"
    
    @(
        @{Alias = $RootCaAlias; File = $RootCaCer},
        @{Alias = $SubCaAlias; File = $SubCaCer}
    ) | ForEach-Object {
        $exportArgs = @(
            "-exportcert",
            "-alias", $_.Alias,
            "-keystore", $OpenHarmonyP12,
            "-storepass", $KeystorePwd,
            "-file", $_.File,
            "-rfc"
        )
        & "$JavaHome\bin\keytool.exe" @exportArgs 2>&1 | Out-Null
    }
    
    Write-Host "  Root CA: $RootCaCer"
    Write-Host "  Sub CA: $SubCaCer"
    
    Write-Host "`nStep 3: Generate app certificate chain..." -ForegroundColor Yellow
    
    $AppCertChain = Join-Path $TempDir "app-cert-chain.cer"
    
    $genCertArgs = @(
        "generate-app-cert",
        "-keyAlias", $AppKeyAlias,
        "-keyPwd", $KeystorePwd,
        "-issuer", "C=CN,O=OpenHarmony,OU=OpenHarmony Team,CN=OpenHarmony Application CA",
        "-issuerKeyAlias", $SubCaAlias,
        "-issuerKeyPwd", $KeystorePwd,
        "-issuerKeystoreFile", $OpenHarmonyP12,
        "-issuerKeystorePwd", $KeystorePwd,
        "-subject", "C=CN,O=Test,OU=Test Team,CN=$BundleName",
        "-validity", "365",
        "-signAlg", "SHA256withECDSA",
        "-rootCaCertFile", $RootCaCer,
        "-subCaCertFile", $SubCaCer,
        "-keystoreFile", $AppKeystore,
        "-keystorePwd", $KeystorePwd,
        "-outForm", "certChain",
        "-outFile", $AppCertChain
    )
    
    & java -jar $HapSignTool @genCertArgs 2>&1 | ForEach-Object { Write-Host "  $_" }
    
    Write-Host "`nStep 4: Generate debug profile..." -ForegroundColor Yellow
    
    $ProfileJson = Join-Path $TempDir "profile.json"
    $ProfileP7b = Join-Path $TempDir "profile.p7b"
    
    $template = Get-Content $ProfileTemplate -Raw | ConvertFrom-Json
    $template.'bundle-info'.'bundle-name' = $BundleName
    
    $uuid = [guid]::NewGuid().ToString()
    $template.uuid = $uuid
    
    $now = [int](Get-Date -UFormat %s)
    $template.validity.'not-before' = $now
    $template.validity.'not-after' = $now + (10 * 365 * 24 * 60 * 60)
    
    $template | ConvertTo-Json -Depth 10 | Set-Content $ProfileJson -Encoding UTF8
    
    Write-Host "`nStep 5: Sign profile..." -ForegroundColor Yellow
    
    $signProfileArgs = @(
        "sign-profile",
        "-mode", "localSign",
        "-keyAlias", $ProfileKeyAlias,
        "-keyPwd", $KeystorePwd,
        "-inFile", $ProfileJson,
        "-keystoreFile", $OpenHarmonyP12,
        "-keystorePwd", $KeystorePwd,
        "-outFile", $ProfileP7b,
        "-signAlg", "SHA256withECDSA",
        "-profileCertFile", $ProfileDebugPem
    )
    
    & java -jar $HapSignTool @signProfileArgs 2>&1 | ForEach-Object { Write-Host "  $_" }
    
    Write-Host "`nStep 6: Sign HAP..." -ForegroundColor Yellow
    
    $signHapArgs = @(
        "sign-app",
        "-mode", "localSign",
        "-keyAlias", $AppKeyAlias,
        "-keyPwd", $KeystorePwd,
        "-inFile", $HapPath,
        "-outFile", $OutputPath,
        "-keystoreFile", $AppKeystore,
        "-keystorePwd", $KeystorePwd,
        "-signAlg", "SHA256withECDSA",
        "-appCertFile", $AppCertChain,
        "-profileFile", $ProfileP7b,
        "-profileSigned", "1",
        "-compatibleVersion", "8"
    )
    
    & java -jar $HapSignTool @signHapArgs 2>&1 | ForEach-Object { Write-Host "  $_" }
    
    if (Test-Path $OutputPath) {
        $fileSize = (Get-Item $OutputPath).Length
        Write-Host "`n========== Sign Success ==========" -ForegroundColor Green
        Write-Host "  Signed HAP: $OutputPath" -ForegroundColor Green
        Write-Host "  Size: $fileSize bytes" -ForegroundColor Green
    } else {
        Write-Error "Failed to sign HAP"
        exit 1
    }
}
finally {
    if (Test-Path $TempDir) {
        Remove-Item -Path $TempDir -Recurse -Force
    }
}