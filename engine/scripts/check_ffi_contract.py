#!/usr/bin/env python3
"""检查 Rust FfiContext、C# FfiContext、编辑器装配代码三方字段是否一致。

这个检查不依赖 Mono/UI，可在 CI 的接口测试阶段先跑，避免新增 FFI 时
Rust 侧加了字段但 C# 委托/编辑器 transmute 漏掉，运行时才出现空指针。
"""

from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
RUST_CONTEXT = ROOT / "engine" / "scripting" / "src" / "ffi_context" / "mod.rs"
CSHARP_CONTEXT = ROOT / "engine" / "scripts" / "UI.cs"
EDITOR_CONTEXT = ROOT / "engine" / "examples" / "src" / "editor" / "mod.rs"


def fields_between(text: str, start_pattern: str, field_pattern: str) -> list[str]:
    match = re.search(start_pattern, text)
    if not match:
        raise SystemExit(f"未找到起始标记: {start_pattern}")
    start = match.end()
    depth = 1
    i = start
    while i < len(text) and depth > 0:
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
        i += 1
    if depth != 0:
        raise SystemExit(f"结构体花括号不匹配: {start_pattern}")
    body = text[start : i - 1]
    return re.findall(field_pattern, body, re.MULTILINE)


def main() -> int:
    rust = fields_between(
        RUST_CONTEXT.read_text(encoding="utf-8"),
        r"pub\s+struct\s+FfiContext\s*\{",
        r"^\s*pub\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*:",
    )
    csharp = set(
        fields_between(
            CSHARP_CONTEXT.read_text(encoding="utf-8"),
            r"public\s+struct\s+FfiContext\s*\{",
            r"^\s*public\s+IntPtr\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*;",
        )
    )
    editor = set(
        fields_between(
            EDITOR_CONTEXT.read_text(encoding="utf-8"),
            r"let\s+ffi_ctx\s*=\s*FfiContext\s*\{",
            r"^\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*:(?!:)",
        )
    )

    rust_set = set(rust)
    errors: list[str] = []

    duplicate = len(rust) - len(rust_set)
    if duplicate:
        errors.append(f"Rust FfiContext 存在 {duplicate} 个重复字段")
    if len(set(csharp)) != len(csharp):
        errors.append("C# FfiContext 存在重复字段")
    if len(editor) != len(rust_set):
        errors.append(f"编辑器装配字段数({len(editor)}) != Rust 字段数({len(rust_set)})")

    missing_cs = sorted(rust_set - csharp)
    extra_cs = sorted(csharp - rust_set)
    missing_editor = sorted(rust_set - editor)
    extra_editor = sorted(editor - rust_set)

    if missing_cs:
        errors.append("C# 缺少字段: " + ", ".join(missing_cs))
    if extra_cs:
        errors.append("C# 多出字段: " + ", ".join(extra_cs))
    if missing_editor:
        errors.append("编辑器装配缺少字段: " + ", ".join(missing_editor))
    if extra_editor:
        errors.append("编辑器装配多出字段: " + ", ".join(extra_editor))

    print(f"Rust FfiContext: {len(rust_set)}")
    print(f"C# FfiContext: {len(csharp)}")
    print(f"Editor assembly: {len(editor)}")

    if errors:
        print("\nFFI CONTRACT FAILED")
        for error in errors:
            print(" - " + error)
        return 1

    print("FFI contract OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
