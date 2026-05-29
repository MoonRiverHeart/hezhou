# Platform Abstraction

GLFW and OpenHarmony platform backends. cdylib crate for standalone FFI export.

## Overview
PlatformManager selects backend at init (GLFW or Harmony via feature gates). Exposes `platform_*` extern "C" FFI functions for window creation, event polling, time queries. GLFW backend is primary (Windows/Linux), Harmony backend for OpenHarmony port.

## Where To Look
| Task | File | Notes |
|------|------|-------|
| Fix GLFW window/input | `glfw_backend.rs` | 426 lines, GLFW init + window + key conversion |
| Fix Harmony window | `harmony_backend.rs` | 299 lines, OH_NativeWindow integration |
| Fix event types | `event.rs` | PlatformEvent enum (Key, Mouse, Char, Scroll, Resize) |
| Add platform backend | `traits.rs` + new backend file | Implement Platform trait, add feature gate |
| Fix key mapping | `glfw_backend.rs` convert_glfw_key | GLFW key code → engine KeyCode enum |

## Key Architecture
```
PlatformBackend enum {
    GLFW(GLFWPlatform),      // feature = "glfw"
    Harmony(HarmonyPlatform), // feature = "harmony"
}
PlatformManager {
    backend: Option<PlatformBackend>,
    event_queue: Arc<Mutex<Vec<PlatformEvent>>>,
}
```
- Platform trait: init(), create_window(), poll_events(), is_running(), get_time(), shutdown()
- WindowHandle: opaque pointer to native window
- KeyCode: Left=45, Right=46, Up=47, Down=48, ESC=39

## Conventions
- FFI naming: `platform_manager_create/destroy`, `platform_init_glfw/harmony`, `platform_create_window`, `platform_poll_events` (platform_ prefix)
- Feature gates: `glfw` (default on Windows/Linux), `harmony` (OpenHarmony builds)
- FFI functions take `*mut PlatformManager` as first arg (raw pointer, null-checked)
- Events buffered in event_queue (Arc<Mutex<Vec>>), polled via poll_events()

## Anti-Patterns
- NEVER assume GLFW is always available — check feature gates and PlatformBackend variant
- NEVER call platform FFI functions with null manager pointer — all functions null-check