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
Runtime requirements: Vulkan-capable GPU/driver; Mono runtime for C# scripting/hot reload.
