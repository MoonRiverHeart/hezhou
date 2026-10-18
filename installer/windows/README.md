Hezhou Editor — Windows MSIX
=============================

Build prerequisites:
- Rust toolchain
- Mono 6.12 (mcs/runtime)
- Windows 10 SDK (`makeappx.exe`, `signtool.exe`), normally on windows-latest

Build unsigned:

```powershell
powershell -ExecutionPolicy Bypass -File installer/windows/build_msix.ps1
```

Build and sign with a certificate:

```powershell
powershell -ExecutionPolicy Bypass -File installer/windows/build_msix.ps1 `
  -CertificatePath C:\path\hezhou.pfx -CertificatePassword password
```

Output: `dist/HezhouEditor-<version>.msix`

Install:
1. Install the matching `.cer` into `Trusted People`
2. `Add-AppxPackage .\HezhouEditor-0.1.0.msix`

The package is a full-trust MSIX; the editor copies read-only assets to
`%LOCALAPPDATA%\Hezhou\Editor` at first launch.
