Hezhou Editor — Debian package
==============================

Build prerequisites on Debian/Ubuntu:
- Rust toolchain
- Mono (`mono-devel`, provides `mcs` and runtime)
- Vulkan/GLFW/X11 development packages for building the editor:
  `libvulkan-dev libglfw3-dev libx11-dev libxrandr-dev libxi-dev libxcursor-dev libxinerama-dev pkg-config`
- python3

Build:

```bash
# from repository root
bash installer/linux/build_deb.sh
# or reuse an existing release binary
bash installer/linux/build_deb.sh --skip-build
```

Output: `dist/hezhou-editor-<version>-amd64.deb`

Install:

```bash
sudo apt install ./dist/hezhou-editor-0.1.0-amd64.deb
hezhou-editor
```

The package installs to `/opt/hezhou/editor`, adds `/usr/bin/hezhou-editor`
and a desktop entry. Runtime dependencies include Vulkan, GLFW, Mono runtime
and Mono C# compiler.
