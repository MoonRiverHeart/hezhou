# Editor Module

Main game editor logic shared across demo binaries.

## Overview
5-line `mono_editor_demo.rs` shim delegates to `editor::run()`. 7 `static mut` globals for state sharing across sub-modules. FfiContext assembly (270 transmute calls).

## Structure
```
examples/src/editor/
├── mod.rs           # 529 lines — editor::run(), FfiContext assembly, main loop
├── ffi_impl.rs      # UI FFI function implementations (draw_rect, draw_text, etc.)
├── scene_ffi_impl.rs # 548 lines — Scene FFI (entity CRUD, transform)
├── dfx_init.rs      # DFX logger initialization
└── hot_reload.rs    # Hot reload: compile → unload → reload → restore bindings
```

## Where To Look
| Task | File | Notes |
|------|------|-------|
| Fix editor startup | `mod.rs` editor::run() | FfiContext assembly → MonoUIExecutor → C# Initialize |
| Fix FFI binding | `ffi_impl.rs` | UI draw functions called from C# via FfiContext |
| Fix entity/script | `scene_ffi_impl.rs` | Scene FFI + ScriptBinding save/restore |
| Fix hot reload | `hot_reload.rs` | mcs compile → MonoExecutor.reload() → ResetAll |
| Fix C# compilation | `hot_reload.rs` compile_editor_script() | Hardcoded mcs.bat path, duplicates build_mono.ps1 |

## Key Globals (static mut — soundness hazard)
```
EXECUTOR: Option<MonoUIExecutor>
RENDERER: Option<UIVulkanRenderer>
SCENE: Option<Scene>
FFI_PTR: Option<*mut FfiContext>
STATUS_TEXT_CALLBACK: Option<UpdateStatusTextFn>
HOT_RELOAD_COMPLETE_CALLBACK: Option<HotReloadCompleteFn>
SAVED_BINDINGS: Option<HashMap<u64, ScriptBinding>>
```
- All accessed via `unsafe { GLOBAL.take() }` / `unsafe { GLOBAL.get() }` patterns
- Soundness: undefined behavior if accessed from multiple threads (currently single-threaded)

## FfiContext Assembly Pattern
```rust
let ffi_ctx = FfiContext {
    ui_create_button: unsafe { std::mem::transmute(ui_ffi::ui_create_button as *const c_void) },
    // ... ~270 fields, each transmuted from FFI function pointer
};
```
- Duplicated in mono_ui_thunk_demo.rs (same pattern, different subset of functions)
- High friction: adding one FFI function requires touching 6+ locations across Rust and C#

## Conventions
- All FFI function pointer assignments use `std::mem::transmute(fn_ptr as *const c_void)`
- Hot reload sequence: save_bindings → compile → unload → reload → restore_bindings → ResetAll
- C# entry: `EditorScript.Initialize(IntPtr contextPtr)` — receives FfiContext, creates all UI
- Main loop: process_events → draw_frame → (optional hot_reload check)