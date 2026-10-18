Hezhou Editor
=============

Windows build prerequisites:
- Rust toolchain
- Mono (mcs) if C# scripts need recompilation
- WiX Toolset v4+: `dotnet tool install --global wix --version 5.*`

Build:

```powershell
# from repository root
powershell -ExecutionPolicy Bypass -File installer/windows/build_msi.ps1
```

Options:
- `-SkipBuild`: reuse `engine/target/release/mono_editor_demo.exe`
- `-Version 0.1.0`: override package version

Output: `dist/HezhouEditor-<version>.msi`

The MSI installs to `%ProgramFiles%\Hezhou`, with a Start Menu shortcut.

Runtime dependencies bundled when available at build time:
- `vulkan-1.dll` (Vulkan loader)
- `vcruntime140.dll`, `vcruntime140_1.dll`, `msvcp140.dll` (MSVC runtime)
- `runtime/mono/` (Mono runtime DLLs, `mcs`, 4.5 assemblies and config)

The editor prepends `runtime/mono/bin` to PATH and sets `MONO_PATH`/
`MONO_CFG_DIR` before initializing Mono, so a system Mono install is not required
when the bundle is present. A Vulkan-capable GPU driver is still required.
