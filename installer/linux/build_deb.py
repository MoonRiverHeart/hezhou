#!/usr/bin/env python3
"""Build a Debian .deb for Hezhou Editor without dpkg-deb.

The artifact layout follows the standard ar(5) container:
  debian-binary
  control.tar.gz
  data.tar.gz
"""
import argparse
import gzip
import io
import os
import shutil
import stat
import tarfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ENGINE = ROOT / "engine"


def ar_member(name: str, data: bytes) -> bytes:
    header = (
        f"{name:<16}{int(time.time()):<12}{0:<6}{0:<6}{'100644':<8}{len(data):<10}`\n"
    ).encode("ascii")
    out = header + data
    if len(data) % 2:
        out += b"\n"
    return out


def add_dir(tar: tarfile.TarFile, path: Path, arcname: str):
    info = tarfile.TarInfo(arcname)
    info.type = tarfile.DIRTYPE
    info.mode = 0o755
    info.mtime = int(time.time())
    tar.addfile(info)


def add_file(tar: tarfile.TarFile, path: Path, arcname: str, mode: int = 0o644):
    data = path.read_bytes()
    info = tarfile.TarInfo(arcname)
    info.size = len(data)
    info.mode = mode
    info.mtime = int(time.time())
    tar.addfile(info, io.BytesIO(data))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--binary", default=str(ENGINE / "target/release/mono_editor_demo"))
    ap.add_argument("--version", default="0.1.0")
    ap.add_argument("--arch", default="amd64")
    ap.add_argument("--out", default=str(ROOT / "dist"))
    ap.add_argument("--mono-root", default="/usr")
    ap.add_argument("--no-bundle-mono", action="store_true")
    args = ap.parse_args()

    binary = Path(args.binary)
    if not binary.is_file():
        raise SystemExit(f"Linux binary not found: {binary}\nBuild on Linux with: cargo build --release --features mono")

    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    pkgname = f"hezhou-editor_{args.version}_{args.arch}"
    build = ROOT / "installer" / "build" / pkgname
    if build.exists():
        shutil.rmtree(build)
    app = build / "opt" / "hezhou" / "editor"
    app.mkdir(parents=True)

    # Application payload
    shutil.copy2(binary, app / "mono_editor_demo")
    script_src = ENGINE / "scripts"
    if script_src.exists():
        dst_scripts = app / "scripts"
        dst_scripts.mkdir(parents=True)
        for cs in script_src.glob("*.cs"):
            shutil.copy2(cs, dst_scripts / cs.name)
        dll = script_src / "bin" / "Mono" / "EditorScript.dll"
        if dll.exists():
            (dst_scripts / "bin" / "Mono").mkdir(parents=True)
            shutil.copy2(dll, dst_scripts / "bin" / "Mono" / "EditorScript.dll")
    for name in ("assets", "fonts"):
        src = ENGINE / name
        if src.exists():
            shutil.copytree(src, app / name)
    for p in app.rglob("*.pdb"):
        p.unlink(missing_ok=True)

    # Launcher: editor resolves assets relative to the installed editor directory
    bin_dir = build / "usr" / "bin"
    bin_dir.mkdir(parents=True)
    launcher = bin_dir / "hezhou-editor"
    launcher.write_text(
        "#!/bin/sh\n"
        "cd /opt/hezhou/editor || exit 1\n"
        "if [ -d /opt/hezhou/runtime/mono/bin ]; then\n"
        "  export PATH=/opt/hezhou/runtime/mono/bin:$PATH\n"
        "  export LD_LIBRARY_PATH=/opt/hezhou/runtime/mono/lib:${LD_LIBRARY_PATH:-}\n"
        "  export MONO_PATH=/opt/hezhou/runtime/mono/lib/mono/4.5\n"
        "  export MONO_CFG_DIR=/opt/hezhou/runtime/mono/etc\n"
        "  export MONO_GAC_PREFIX=/opt/hezhou/runtime/mono\n"
        "fi\n"
        "exec ./mono_editor_demo \"$@\"\n",
        encoding="utf-8",
    )
    launcher.chmod(0o755)

    desktop = build / "usr" / "share" / "applications" / "hezhou-editor.desktop"
    desktop.parent.mkdir(parents=True)
    desktop.write_text(
        "[Desktop Entry]\n"
        "Type=Application\n"
        "Name=Hezhou Editor\n"
        "Comment=Hezhou game editor\n"
        "Exec=hezhou-editor\n"
        "Icon=hezhou-editor\n"
        "Terminal=false\n"
        "Categories=Development;Game;\n",
        encoding="utf-8",
    )
    icon_src = ENGINE / "assets" / "icons" / "cube.svg"
    icon_dst = build / "usr" / "share" / "icons" / "hicolor" / "scalable" / "apps" / "hezhou-editor.svg"
    icon_dst.parent.mkdir(parents=True)
    if icon_src.exists():
        shutil.copy2(icon_src, icon_dst)

    bundle_mono = False
    mono_root = Path(args.mono_root)
    if not args.no_bundle_mono and (mono_root / "bin" / "mono").exists():
        runtime = build / "opt" / "hezhou" / "runtime" / "mono"
        rb = runtime / "bin"
        rl = runtime / "lib"
        rb.mkdir(parents=True)
        rl.mkdir(parents=True)
        shutil.copy2(mono_root / "bin" / "mono", rb / "mono")
        for asm in ("4.5", "4.5-api"):
            src = mono_root / "lib" / "mono" / asm
            if src.exists():
                shutil.copytree(src, runtime / "lib" / "mono" / asm)
        etc = mono_root / "etc" / "mono"
        if etc.exists():
            shutil.copytree(etc, runtime / "etc" / "mono")
        # Mono native runtime libraries (best effort; glibc/driver libs stay system)
        for libdir in (mono_root / "lib" / "x86_64-linux-gnu", mono_root / "lib64"):
            if libdir.exists():
                for so in libdir.glob("libmono*.so*"):
                    if so.is_file():
                        shutil.copy2(so, rl / so.name)
        mcs_exe = mono_root / "lib" / "mono" / "4.5" / "mcs.exe"
        if mcs_exe.exists():
            mcs = rb / "mcs"
            mcs.write_text(
                "#!/bin/sh\n"
                "exec /opt/hezhou/runtime/mono/bin/mono "
                "/opt/hezhou/runtime/mono/lib/mono/4.5/mcs.exe \"$@\"\n",
                encoding="utf-8",
            )
            mcs.chmod(0o755)
        bundle_mono = True
        print(f"Bundled Mono runtime from {mono_root}")

    debian = build / "DEBIAN"
    debian.mkdir()
    installed_kb = int(sum(p.stat().st_size for p in build.rglob("*") if p.is_file()) / 1024) + 1
    control = (
        f"Package: hezhou-editor\n"
        f"Version: {args.version}\n"
        f"Section: games\n"
        f"Priority: optional\n"
        f"Architecture: {args.arch}\n"
        f"Installed-Size: {installed_kb}\n"
        f"Depends: libc6, libvulkan1, libglfw3, libx11-6, libxrandr2, libxi6, libxcursor1, libxinerama1, libfontconfig1"
        + ("" if bundle_mono else ", mono-runtime, mono-mcs") + "\n"
        f"Maintainer: Hezhou <dev@hezhou.local>\n"
        f"Homepage: https://github.com/MoonRiverHeart/hezhou\n"
        f"Description: Hezhou game editor\n"
        f" Rust + C# game engine editor with Vulkan rendering, custom UI,\n"
        f" scene editing and Mono scripting.\n"
    )
    (debian / "control").write_text(control, encoding="utf-8")
    (debian / "postinst").write_text(
        "#!/bin/sh\nset -e\nupdate-desktop-database -q /usr/share/applications 2>/dev/null || true\ngtk-update-icon-cache -q -t -f /usr/share/icons/hicolor 2>/dev/null || true\nexit 0\n",
        encoding="utf-8",
    )
    (debian / "prerm").write_text(
        "#!/bin/sh\nset -e\nrm -f /usr/bin/hezhou-editor\nexit 0\n",
        encoding="utf-8",
    )
    for p in (debian / "postinst", debian / "prerm"):
        p.chmod(0o755)
    (debian / "copyright").write_text(
        "Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/\n"
        "Upstream-Name: hezhou\n"
        "Source: https://github.com/MoonRiverHeart/hezhou\n"
        "\nFiles: *\nCopyright: Hezhou contributors\nLicense: see upstream repository\n",
        encoding="utf-8",
    )

    # control.tar.gz
    control_tar = io.BytesIO()
    with tarfile.open(fileobj=control_tar, mode="w:gz") as tar:
        for name in ("control", "postinst", "prerm", "copyright"):
            add_file(tar, debian / name, name, 0o755 if name in ("postinst", "prerm") else 0o644)

    # data.tar.gz (paths relative to /)
    data_tar = io.BytesIO()
    with tarfile.open(fileobj=data_tar, mode="w:gz") as tar:
        for p in sorted(build.rglob("*")):
            if p.is_relative_to(debian):
                continue
            rel = p.relative_to(build).as_posix()
            if p.is_dir():
                add_dir(tar, p, rel)
            else:
                mode = 0o755 if p.name in ("mono_editor_demo", "hezhou-editor") else 0o644
                add_file(tar, p, rel, mode)

    deb = out_dir / f"hezhou-editor_{args.version}_{args.arch}.deb"
    with deb.open("wb") as f:
        f.write(b"!<arch>\n")
        f.write(ar_member("debian-binary", b"2.0\n"))
        f.write(ar_member("control.tar.gz", control_tar.getvalue()))
        f.write(ar_member("data.tar.gz", data_tar.getvalue()))
    print(f"DEB: {deb} ({deb.stat().st_size} bytes)")
    print(f"payload: {installed_kb} KiB, app dir: {app}")


if __name__ == "__main__":
    main()
