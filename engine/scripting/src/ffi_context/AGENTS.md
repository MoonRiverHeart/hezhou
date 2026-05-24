# FfiContext Bridge

270-field flat function pointer table bridging Rust ↔ C#.

## Overview
#[repr(C)] struct with ~270 pub fields of function pointer type aliases. C# receives as IntPtr → Marshal.PtrToStructure<FfiContext>. Adding one FFI function requires 6+ file changes.

## Structure
```
scripting/src/ffi_context/
├── mod.rs           # FfiContext struct (~270 fields) + set/get_ffi_context_ptr globals
├── types.rs         # Common type aliases (WidgetTreeHandle, ScenePtr, etc.)
├── ui_system.rs     # System-level FFI types (create_widget_tree, get_font_atlas, draw_rect, etc.)
├── widget_basic.rs  # Basic widget FFI types (Button, Label, Panel, VStack, HStack, List)
├── widget_complex.rs # Complex widget FFI types (Checkbox, Dropdown, InputField, Slider)
├── widget_new.rs    # New widget FFI types (TabWidget, TreeView, PopupMenu, GridView, Dialog, FileBrowser)
├── scene.rs         # Scene FFI types (create_entity, destroy, transform, script_binding)
├── renderer.rs      # Renderer FFI types (game_preview_extent, outline entity)
├── property.rs      # Property FFI types (set/get position/rotation/scale)
├── thunk.rs         # Thunk callback registration types (register_xxx_thunk_ptr)
└── asset_project.rs # Asset/Project FFI types (texture, material, project CRUD)
```

## Where To Look
| Task | File | Notes |
|------|------|-------|
| Add FFI type alias | Appropriate sub-file by domain | e.g. widget_new.rs for TabWidget |
| Add FfiContext field | `mod.rs` FfiContext struct | Must match type alias name exactly |
| Fix C# FfiContext | `scripts/UI.cs` | Add IntPtr field + delegate definition |
| Fix FFI assignment | `examples/src/editor/ffi_impl.rs` | transmute(fn_ptr as *const c_void) |

## Adding New FFI Function (Checklist)
1. Type alias in appropriate sub-file (e.g. `widget_new.rs`): `pub type CreateCheckboxFn = extern "C" fn(WidgetTreeHandle, u64, f32, f32) -> u64;`
2. Field in FfiContext struct in `mod.rs`: `pub ui_create_checkbox: CreateCheckboxFn,`
3. IntPtr field in C# `UI.cs` FfiContext: `public IntPtr ui_create_checkbox;`
4. C# delegate in `UI.cs`: `[UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate ulong CreateCheckboxDelegate(...);`
5. Assignment in `examples/src/editor/ffi_impl.rs`: `ui_create_checkbox: unsafe { std::mem::transmute(ui_ffi::ui_create_checkbox as *const c_void) },`
6. C# wrapper method in `UI.NewWidgets.cs` or appropriate partial class

## Conventions
- All FfiContext fields are pub function pointers (no nested structs)
- Type aliases use `Fn` suffix: `CreateXxxFn`, `SetYyyFn`, `GetYyyFn`
- C# FfiContext struct must match Rust field names exactly (snake_case IntPtr fields)
- transmute pattern: `unsafe { std::mem::transmute(fn_ptr as *const c_void) }`

## Anti-Patterns
- NEVER skip FfiContext initialization for new FFI functions (Rust AND C# both required)
- NEVER define delegates or FfiContext fields outside UI.cs main file