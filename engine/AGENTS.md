# Hezhou Engine Knowledge Base

**Generated:** 2026-05-24
**Commit:** 4839913
**Branch:** main

## Overview
Cross-platform game engine: Rust core + C# scripting (Mono JIT/NativeAOT). Vulkan rendering, custom UI system with 17 widget types, ECS Scene, asset library, project file system. Inverted dependency: core depends on scripting (for FfiContext types).

## Structure
```
engine/
├── core/           # ECS Scene, Transform, AssetLibrary, Project (see core/src/AGENTS.md)
├── ui/             # Widget system, Canvas, FontAtlas, EventDispatcher (see ui/src/AGENTS.md)
│   └── widgets/    # 17 widget types (see widgets/AGENTS.md)
│   └── thunk/      # Callback dispatch (see ui/src/AGENTS.md)
│   └── ffi/        # FFI bridge layer (see ui/src/AGENTS.md)
├── rhi-vulkan/     # Vulkan renderer (see rhi-vulkan/src/AGENTS.md)
├── scripting/      # Mono JIT executor, FfiContext (see scripting/src/AGENTS.md)
│   └── ffi_context/ # 270-field FfiContext struct (see scripting/src/ffi_context/AGENTS.md)
├── scripts/        # C# source: EditorScript.cs, UI.cs (see scripts/AGENTS.md)
├── dfx/            # Logging, crash, trace system
├── examples/src/   # 15 demo programs (see examples/src/AGENTS.md)
│   └── editor/     # Main editor module (see examples/src/editor/AGENTS.md)
├── shaders/        # rotation.vert/frag + ui shaders (manually compiled to .spv)
└── fonts/          # HarmonyOS_Sans_SC.ttf (bundled)
```

## Where To Look
| Task | Location | Notes |
|------|----------|-------|
| Add new widget | `ui/src/widgets/` + `ui/src/ffi/` + `ui/src/thunk/` + `scripting/src/ffi_context/` + `scripts/UI.cs` | 4 Rust files + 2 C# files + FfiContext (16-step checklist) |
| Fix rendering | `rhi-vulkan/src/ui_vulkan_renderer.rs` | 4094 lines, game+UI+outline passes |
| Fix UI layout | `ui/src/widget_tree.rs` measure_and_layout | Add type-specific layout in match |
| Fix callback deadlock | `ui/src/thunk/` | queue_callback + flush_pending_callbacks, NEVER trigger inside lock |
| Add FFI function | `ui/src/ffi/` + `scripting/src/ffi_context/` + `scripts/UI.cs` | Type alias + FfiContext field + C# delegate |
| Edit C# editor | `scripts/EditorScript.View.cs` + `.Presenter.cs` + `.State.cs` | Partial classes, 4 files |
| Hot reload | `examples/src/editor/hot_reload.rs` | build_mono.ps1 → mcs → executor.reload() |
| Entity/Scene | `core/src/ecs/scene.rs` + `core/src/ecs/scene_ffi_impl.rs` | ScriptBinding, entity_names |
| Asset library | `core/src/asset_library.rs` + `core/src/asset_ffi.rs` | 5 primitives + textures + materials |
| Project files | `core/src/project.rs` | JSON format, create/load/save |

## Architecture Notes
- **Inverted dependency**: core → scripting (core needs FfiContext types from scripting). Scripting is the leaf crate.
- **4 mid-level crates produce cdylib**: scripting, ui, dfx, harmony — needed for standalone FFI export model
- **FfiContext**: 270-field flat #[repr(C)] struct of function pointers, assembled in editor/mod.rs with transmute
- **C# has NO Main() methods**: All scripts are DLL assemblies, entry via static Initialize() called from Rust
- **Shader compilation**: NO automated pipeline. `.spv` files manually compiled via glslc and committed to repo
- **Mono build**: 3 inconsistent PowerShell scripts (build_mono.ps1, build_mono_ui.ps1, build_ui_mono.ps1)

## Anti-Patterns (THIS PROJECT)
- **NEVER** call `create_font_atlas()` directly — use `get_font_atlas()` (OnceLock cached)
- **NEVER** trigger thunk callbacks inside `WidgetTree.lock()` — use `queue_callback()` + `flush_pending_callbacks()` 
- **NEVER** use `as any` or `@ts-ignore` in any language
- **NEVER** call `calculate_size()` inside `add_item()` — defer to `show()` or `measure()`
- **NEVER** use Button for menu items — use PopupMenu with shortcuts and separators
- **NEVER** use static Labels for tree structures — use TreeView/TreeNode
- **NEVER** use VStack+Label for asset lists — use GridView
- **NEVER** store C# callback delegates as local variables — must be static fields (GC risk)
- **NEVER** skip FfiContext initialization for new FFI functions (Rust AND C# both)
- **NEVER** define delegates or FfiContext fields outside `UI.cs` main file
- **NEVER** use .NET APIs not available in Mono mcs (no LINQ advanced, no async)
- **NEVER** use .NET 8 compiled DLLs with Mono — use `mcs` compiler instead
- **NEVER** destroy/recreate widgets on state switch — use `SetWidgetVisible` instead
- **NEVER** create Vulkan pipeline without `p_dynamic_state` for VIEWPORT+SCISSOR
- **NEVER** hardcode viewport size — use `cmd_set_viewport` at render time
- **NEVER** mismatch push constant layout with shader struct

## Unique Styles
- **FFI naming**: `ui_create_xxx`, `ui_xxx_set_yyy`, `ui_xxx_get_yyy` (snake_case)
- **Thunk naming**: `ui_xxx_set_on_yyy_thunk_ptr` for per-widget, `ui_register_yyy_thunk_ptr` for global
- **C# naming**: PascalCase (CreateXxx, SetYyy, GetYyy) + Delegate suffix for types
- **Widget 3-layer**: Rust Widget trait → FFI extern "C" → C# static wrapper (4 Rust files + 2 C# files)
- **Thunk 2-phase**: `queue_callback(PendingCallback::...)` inside lock → `flush_pending_callbacks()` outside lock
- **Thunk lock-copy-release**: Trigger functions lock → copy fn ptr → drop lock → call C# (prevents deadlock)
- **Render layers**: Background(0) Content(1) Popup(2) Overlay(3) — PopupMenu=2, Dialog=3
- **InputField focus**: Global `FOCUSED_INPUT_FIELD`, draw() syncs `is_focused` each frame
- **GameState**: Editing(0), Running(1), Paused(2) — orange/blue/green preview border
- **KeyCode**: Left=45, Right=46, Up=47, Down=48, ESC=39
- **ContentScale**: DPI-aware (1.5 at 144 DPI), all visual sizes multiplied, input coords NOT scaled
- **Deferred FBO resize**: `pending_offscreen_resize` → `device_wait_idle()` → recreate at frame start
- **Widget layout**: SplitView/Dialog are parent-allocated (size from Panel), not auto-sized

## Commands
```bash
cd engine
cargo build                                          # Build all crates
cargo build --release                                 # Release build
cargo run --bin mono_editor_demo --features mono --release  # Game Editor
cargo run --bin mono_triangle_demo --features mono   # Hot reload demo
cargo run --bin trace_viewer_demo --release           # Trace viewer

cd engine/scripts
powershell -ExecutionPolicy Bypass -File build_mono.ps1  # Compile C# (mcs)

# Shader compile (manual, no automation):
glslc shaders/rotation.vert -o shaders/rotation.vert.spv
glslc shaders/rotation.frag -o shaders/rotation.frag.spv

# Debug UI tree: Ctrl+Shift+D in editor
```

## Notes
- Mono/.NET 8 incompatibility: .NET 8 DLLs → 0 methods in Mono. Use `mcs` compiler.
- `mono_class_get_method_from_name` returns null → iterate `mono_class_get_methods` + match name
- Standalone release: dfx function pointers via FfiContext (exe doesn't export symbols)
- FontAtlas first init ~0.23s (OnceLock), subsequent calls instant
- Editor startup: FontAtlas init + C# compile + Mono load ≈ 2-3s
- 3 inconsistent Mono build scripts (build_mono.ps1 hardcoded mcs path, build_mono_ui.ps1 uses PATH + -unsafe flag, build_ui_mono.ps1 hardcoded + subset) — should consolidate
- No CI/CD pipeline — zero `.github/workflows`
- No rust-toolchain.toml — no pinned toolchain
- workspace.dependencies under-used: only 3 deps (parking_lot, tokio unused, wrapped_mono) — version skew risk
- editor/mod.rs uses 7 `static mut` globals — soundness hazard (undefined behavior from multiple threads)
- `.gitignore` excludes `*.json` and `*.png` — overly broad, blocks legitimate asset files
- Shader compilation has NO automation — .spv files manually compiled via glslc and committed