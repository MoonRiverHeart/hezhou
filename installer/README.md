= Hezhou Editor Packaging

Windows MSIX
------------
- Manifest: `installer/windows/msix/AppxManifest.xml`
- Builder: `installer/windows/build_msix.ps1`
- Output: `dist/HezhouEditor-<version>.msix`
- CI produces a self-signed certificate (`dist/HezhouEditor.cer`) alongside it.

Linux DEB
---------
- Builder: `installer/linux/build_deb.sh`, `installer/linux/build_deb.py`
- Output: `dist/hezhou-editor_<version>_amd64.deb`
- Python writes a standard ar/deb archive directly; no dpkg-deb needed.

Bundled runtime
---------------
Windows and Linux builders copy a minimal Mono runtime into `runtime/mono`
when Mono is present at build time. The Windows MSIX also bundles the Vulkan
loader and MSVC runtime DLLs next to the executable. Linux keeps Vulkan/GLFW/X11
as Debian dependencies because the GPU driver cannot be bundled.

CI
--
Manually dispatch `.github/workflows/ci.yml`, or push a `v*` tag; the
`package-windows`, `package-linux` and `publish-release` jobs build the MSIX/DEB
and upload them to the GitHub Release.
