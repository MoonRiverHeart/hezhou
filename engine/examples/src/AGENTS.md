# Demo Programs

15 binary targets (5 feature-gated) + shared editor module.

## Overview
15 demo binaries demonstrating engine features. Main editor (mono_editor_demo) requires `mono` feature. Shared editor/ sub-module contains FfiContext assembly and editor logic.

## Binary Targets
| Binary | Feature Gate | Purpose |
|--------|-------------|---------|
| engine_demo | none | ECS + scripting callbacks |
| glfw_demo | none | GLFW window + engine |
| triangle_demo | none | Vulkan triangle |
| vulkan_triangle_demo | none | Vulkan init-only |
| vulkan_render_demo | none | Full Vulkan 100-frame loop |
| ui_vulkan_demo | none | UI + Vulkan + screenshot |
| rotation_demo | native-aot | NativeAOT thunk calls |
| mono_rotation_demo | mono | Console Mono JIT rotation |
| mono_hot_reload_test | mono | Automated hot reload test |
| mono_triangle_demo | mono | Vulkan + Mono + hot reload |
| mono_ui_demo | mono | Mono UI demo |
| mono_ui_thunk_demo | mono | Thunk FFI + Mono UI |
| **mono_editor_demo** | mono | **THE MAIN EDITOR** — 5-line shim → editor::run() |
| trace_viewer_demo | none | Trace JSON viewer (741 lines) |
| screenshot_demo | none | Screenshot capture (S key) |

## Where To Look
| Task | File | Notes |
|------|------|-------|
| Fix editor | `editor/mod.rs` | 529 lines — FfiContext assembly + main loop |
| Fix FFI impl | `editor/ffi_impl.rs` | UI draw functions for C# |
| Fix hot reload | `editor/hot_reload.rs` | mcs compile → unload → reload |
| Fix scene FFI | `editor/scene_ffi_impl.rs` | Entity CRUD + ScriptBinding |
| Fix trace viewer | `trace_viewer_demo.rs` | 727 lines — JSON loading + rendering |

## Conventions
- Feature gates: `mono` for Mono JIT demos, `native-aot` for NativeAOT demo
- mono_editor_demo.rs is 5-line shim: `fn main() { editor::run() }`
- editor/ sub-module shared across demos (unusual for binary crate)
- build.rs links shell32 on Windows (same as platform crate)