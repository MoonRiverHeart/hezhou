= Hezhou Editor Packaging

Windows MSI (WiX v4)
--------------------
- Config: `installer/windows/HezhouEditor.wxs`
- Builder: `installer/windows/build_msi.ps1`
- Output: `dist/HezhouEditor-<version>.msi`

Linux DEB
---------
- Config/builder: `installer/linux/build_deb.sh`, `installer/linux/build_deb.py`
- Output: `dist/hezhou-editor_<version>_amd64.deb`
- The Python builder writes a standard ar/deb archive directly, so `dpkg-deb`
  is not required on the build host.
- `data.tar.gz` contains `/opt/hezhou/editor`, `/usr/bin/hezhou-editor`,
  desktop entry and icon.

CI
--
`.github/workflows/ci.yml` has `package-windows` and `package-linux` jobs,
triggered manually via `workflow_dispatch`, producing downloadable artifacts.

Notes
-----
- The editor resolves `scripts/`, `assets/`, `fonts/` relative to its working
  directory; the MSI shortcut and the Linux launcher set that directory.
- Mono runtime + mcs are runtime/development dependencies for C# scripting.
- Builds require the matching platform toolchain; MSI cannot be produced on
  Linux and DEB cannot be produced with Windows binaries.
