# HEZHOU PROJECT KNOWLEDGE BASE

**Generated:** 2026-05-25
**Commit:** afb4f7d
**Branch:** main

## OVERVIEW
Cross-platform game engine: Rust core + C# scripting (Mono JIT/NativeAOT) + HarmonyOS port. Vulkan rendering, custom UI system (17 widget types), ECS Scene, asset library, project file system.

## STRUCTURE
```
hezhou/
├── engine/      # Rust workspace — 12 crates (see engine/AGENTS.md)
├── demo/        # Standalone interop demos (Rust↔C#, Rust↔Harmony)
│   ├── rust/    # Rust→C# via Mono JIT (edition 2024)
│   ├── csharp/  # C#→Rust via DllImport
│   ├── callback/ # C#↔Rust callbacks via csbindgen
│   └── harmony/ # OpenHarmony UIAbility + Rust libentry.so
├── release/     # Packaged editor distribution (hezhou-editor.exe + start.bat)
├── design/      # Design documents
├── docs/        # Screenshots for README
├── scripts/     # Top-level scripts (if any)
└── hezhou.sln   # VS solution — only demo/csharp (NOT the engine)
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Engine architecture | `engine/AGENTS.md` | Full engine KB, 124 lines |
| Run editor | `engine/examples/src/mono_editor_demo.rs` → `editor::run()` | `cd engine && cargo run --bin mono_editor_demo --features mono --release` |
| C# scripting | `engine/scripts/` | 24 .cs files, mcs compiler only |
| HarmonyOS port | `demo/harmony/` | EntryAbility.ets → Index.ets → libentry.so |
| Release build | `release/hezhou-editor/` | start.bat wrapper |
| Interop demo (Rust→C#) | `demo/rust/src/main.rs` | edition 2024, hardcoded Mono paths |
| Interop demo (C#→Rust) | `demo/csharp/Program.cs` | DllImport-based |

## CONVENTIONS
- Workspace Cargo.toml is in `engine/` (NOT repo root) — `cd engine` before any cargo command
- `.gitignore` excludes `*.json` and `*.png` — overly broad, blocks legitimate assets
- hezhou.sln only contains `demo/csharp/CsharpCaller.csproj` — engine is NOT in the solution
- Release: `release/hezhou-editor/start.bat` → `hezhou-editor.exe`

## ANTI-PATTERNS (THIS PROJECT)
- **NEVER** run `cargo build` from repo root — must `cd engine` first
- **NEVER** add .csproj files to hezhou.sln without checking engine .csproj files exist separately
- **NEVER** commit `.json` or `.png` files to root — `.gitignore` blocks them (needs fix)
- **NEVER** use .NET 8 DLLs with Mono — use `mcs` compiler

## COMMANDS
```bash
cd engine
cargo build                                          # Build all crates
cargo run --bin mono_editor_demo --features mono --release  # Game Editor
cargo run --bin mono_triangle_demo --features mono   # Hot reload demo
cargo run --bin trace_viewer_demo --release           # Trace viewer

cd engine/scripts
powershell -ExecutionPolicy Bypass -File build_mono.ps1  # Compile C# (mcs)
```

## NOTES
- No CI/CD pipeline — zero `.github/workflows`
- No rust-toolchain.toml — no pinned toolchain
- `demo/rust/Cargo.toml` uses edition = "2024" (very new Rust edition)
- Hardcoded absolute paths in `demo/rust/` (Mono lib path, Windows SDK paths)
- Inverted dependency: core → scripting (core needs FfiContext types)

## LANGUAGE POLICY
- **全程使用中文** — 所有思考、回答、注释、对话必须用中文
- 思考过程用中文，代码注释用中文，与用户交流用中文
- 代码本身（Rust/C#/GLSL）保持英文，因为编程语言惯例如此
- 但所有解释、诊断、计划、todo描述必须中文