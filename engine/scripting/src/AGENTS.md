# Scripting System

Mono JIT and NativeAOT script execution for game engine.

## Overview
Two script modes: Mono JIT (development, hot reload) and NativeAOT (release, performance). FfiContext bridges Rust ↔ C# via function pointer table.

## Where To Look
| Task | File | Notes |
|------|------|-------|
| Add FFI function | `ffi_context.rs` | Add type alias + field to FfiContext struct |
| Fix Mono loading | `mono_executor.rs` | wrapped_mono method lookup issues |
| Fix hot reload | `mono_editor_demo.rs` | save_bindings → unload → recompile → reload → restore |
| Fix C# callback | `ffi_context.rs` + `thunk_manager.rs` | Function pointer registration chain |

## Conventions
- FfiContext: flat struct of function pointers, passed to C# via `MonoExecutor::call_initialize_with_context`
- C# receives FfiContext as IntPtr → Marshal.PtrToStructure<FfiContext>
- Each FFI function: type alias (e.g. `CreateButtonFn = extern "C" fn(...) -> u64`) + field in FfiContext
- C# delegates match FFI signatures exactly, stored as static fields
- FfiContext initialization in `mono_editor_demo.rs`: assign each field to actual FFI function address

## Anti-Patterns
- NEVER skip adding new FFI function to FfiContext struct (Rust AND C#)
- NEVER use `mono_class_get_method_from_name` alone — iterate methods if null returned
- NEVER use .NET 8 compiled DLLs with Mono — use `mcs` compiler instead
- NEVER unload Mono assembly without saving Entity-Script bindings first