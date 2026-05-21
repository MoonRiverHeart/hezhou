# Hezhou Engine Knowledge Base

**Generated:** 2026-05-21
**Commit:** 0fc1498
**Branch:** main

## Overview
Cross-platform game engine: Rust core + C# scripting (Mono JIT/NativeAOT). Vulkan rendering, custom UI system with 16 widget types, ECS Scene, asset library, project file system.

## Structure
```
engine/
├── core/           # ECS Scene, Transform, AssetLibrary, Project
├── ui/             # Widget system, Canvas, FontAtlas, EventDispatcher
│   └── widgets/    # 16 widget types (see widgets/AGENTS.md)
├── rhi-vulkan/     # Vulkan renderer (see rhi-vulkan/src/AGENTS.md)
├── scripting/      # Mono JIT executor, FfiContext (see scripting/src/AGENTS.md)
├── scripts/        # C# source: EditorScript.cs, UI.cs (see scripts/AGENTS.md)
├── dfx/            # Logging, crash, trace system
├── examples/src/   # 6 demo programs (mono_editor_demo main)
├── shaders/        # rotation.vert/frag
└── fonts/          # HarmonyOS_Sans_SC.ttf (bundled)
```

## Where To Look
| Task | Location | Notes |
|------|----------|-------|
| Add new widget | `ui/src/widgets/` + `ui/src/ffi.rs` + `ui/src/thunk_manager.rs` | 3-layer: Rust widget → FFI → C# wrapper |
| Fix rendering | `rhi-vulkan/src/ui_vulkan_renderer.rs` | 3649 lines, game+UI+outline passes |
| Fix UI layout | `ui/src/widget_tree.rs` measure_and_layout | Add type-specific layout in match |
| Fix callback deadlock | `ui/src/thunk_manager.rs` | Use queue_callback, never trigger inside lock |
| Add FFI function | `ui/src/ffi.rs` + `scripting/src/ffi_context.rs` + C# UI.cs | Add to FfiContext struct too |
| Edit C# editor | `scripts/EditorScript.cs` | 1693 lines, CreateEditorLayout() |
| Hot reload | `examples/src/mono_editor_demo.rs` | build_mono.ps1 → mcs → executor.reload() |
| Entity/Scene | `core/src/ecs/scene.rs` + `core/src/scene_ffi.rs` | ScriptBinding, entity_names |
| Asset library | `core/src/asset_library.rs` + `core/src/asset_ffi.rs` | 5 primitives + textures + materials |
| Project files | `core/src/project.rs` | JSON format, create/load/save |

## Anti-Patterns (THIS PROJECT)
- **NEVER** call `create_font_atlas()` directly — use `get_font_atlas()` (OnceLock cached)
- **NEVER** trigger thunk callbacks inside `WidgetTree.lock()` — use `queue_callback()` + `flush_pending_callbacks()` 
- **NEVER** use `as any` or `@ts-ignore` in any language
- **NEVER** call `calculate_size()` inside `add_item()` — defer to `show()` or `measure()`
- **NEVER** use Button for menu items — use PopupMenu with shortcuts and separators
- **NEVER** use static Labels for tree structures — use TreeView/TreeNode
- **NEVER** use VStack+Label for asset lists — use GridView

## Unique Styles
- **FFI naming**: `ui_create_xxx`, `ui_xxx_set_yyy`, `ui_xxx_get_yyy` (snake_case)
- **C# naming**: PascalCase (CreateXxx, SetYyy, GetYyy)
- **Widget 3-layer**: Rust widget trait → FFI extern "C" → C# static wrapper
- **Thunk callbacks**: All via `queue_callback(PendingCallback::...)` + `flush_pending_callbacks()`
- **Render layers**: Background(0) Content(1) Popup(2) Overlay(3) — menus use layer=2
- **InputField focus**: Global `FOCUSED_INPUT_FIELD`, draw() syncs `is_focused` each frame
- **GameState**: Editing(0), Running(1), Paused(2) — orange/blue/green preview border
- **KeyCode**: Left=45, Right=46, Up=47, Down=48, ESC=39
- **ContentScale**: DPI-aware (1.5 at 144 DPI), all sizes multiplied

## Commands
```bash
cd engine
cargo build                                          # Build all crates
cargo run --bin mono_editor_demo --features mono --release  # Game Editor
cargo run --bin mono_triangle_demo --features mono   # Hot reload demo

cd engine/scripts
powershell -ExecutionPolicy Bypass -File build_mono.ps1  # Compile C# (mcs)

# Debug UI tree: Ctrl+Shift+D in editor
```

## Notes
- Mono/.NET 8 incompatibility: .NET 8 DLLs → 0 methods in Mono. Use `mcs` compiler.
- `mono_class_get_method_from_name` returns null → iterate `mono_class_get_methods` + match name
- Standalone release: dfx function pointers via FfiContext (exe doesn't export symbols)
- FontAtlas first init ~0.23s (OnceLock), subsequent calls instant
- Editor startup: FontAtlas init + C# compile + Mono load ≈ 2-3s