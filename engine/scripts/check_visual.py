#!/usr/bin/env python3
"""视觉回归基础校验：关键 UI 截图存在、尺寸正确、不是空文件。

完整像素 diff 需要基线图；这里先做 CI 可执行的硬性检查，防止
visual mode 静默失败或截图路径错位。
"""

from __future__ import annotations

import pathlib
import struct
import sys

EXPECTED = [
    "01_rotation_script_binding.png",
    "02_console.png",
    "03_asset_search.png",
    "04_project_search.png",
    "05_multi_select.png",
    "06_theme_light.png",
    "07_theme_dark.png",
    "08_widget_gallery_dark.png",
    "09_widget_gallery_light.png",
    "10_editor_restored.png",
    "11_right_panel_max.png",
    "12_dock_restored.png",
    "13_shift_range_select.png",
    "14_command_palette.png",
    "15_palette_closed.png",
    "16_inspector_rows.png",
    "17_hierarchy_drag.png",
    "18_material_tab.png",
    "19_gizmo_translate.png",
    "20_material_assign.png",
    "21_gizmo_scale.png",
    "22_tab_reorder.png",
    "23_asset_collapsed.png",
    "24_dock_restored_2.png",
    "25_floating_panel.png",
    "26_floating_moved.png",
    "27_floating_closed.png",
    "28_texture_tab.png",
    "29_texture_docked_right.png",
    "30_texture_dock_restored.png",
]


def png_size(path: pathlib.Path) -> tuple[int, int]:
    with path.open("rb") as f:
        header = f.read(24)
    if len(header) < 24 or header[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not a PNG")
    width, height = struct.unpack(">II", header[16:24])
    return width, height


def main(root: str) -> int:
    base = pathlib.Path(root) / "visual"
    errors: list[str] = []

    for name in EXPECTED:
        path = base / name
        if not path.exists():
            errors.append(f"缺少截图: {name}")
            continue
        if path.stat().st_size < 20_000:
            errors.append(f"截图疑似空白/过小: {name} ({path.stat().st_size} bytes)")
            continue
        try:
            width, height = png_size(path)
        except ValueError as e:
            errors.append(f"截图格式错误: {name}: {e}")
            continue
        if width != 1280 or height != 720:
            errors.append(f"截图尺寸异常: {name} = {width}x{height}")
        print(f"visual OK: {name} {width}x{height} {path.stat().st_size} bytes")

    if errors:
        print("VISUAL CHECK FAILED")
        for e in errors:
            print(" - " + e)
        return 1

    print("VISUAL CHECK OK")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("usage: check_visual.py <report_dir>")
        sys.exit(2)
    sys.exit(main(sys.argv[1]))
