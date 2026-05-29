# Render Engine

High-level rendering engine for HarmonyOS/OpenHarmony port. FFI bridge to native window surface.

## Overview
RenderEngine manages Camera + RenderSurface (EGL/OpenHarmony native window). 9 modules: Camera, Color, Mesh, Material, Texture, Surface, Renderer, plus FFI bridge. Uses hezhou_core math + hezhou_harmony OH_NativeWindow.

## Where To Look
| Task | File | Notes |
|------|------|-------|
| Fix camera | `camera.rs` | Camera struct with projection + view matrices |
| Fix surface init | `surface.rs` | RenderSurface::create(window, w, h) — EGL context |
| Fix rendering loop | `renderer.rs` | 66 lines — minimal renderer stub |
| Fix mesh data | `mesh.rs` | Mesh struct for vertex/index data |
| Add FFI function | `ffi.rs` | 239 lines — render_* extern "C" FFI bridge |

## Key Architecture
```
RenderEngine {
    surface: Option<RenderSurface>,
    camera: Option<Camera>,
    clear_color: Color,
    mesh_count: u64,
}
```
- init_surface(window, w, h) → creates EGL surface from OH_NativeWindow
- begin_frame/end_frame → make_current/present
- Camera: view + projection matrices, position + target

## Conventions
- FFI naming: `render_*` prefix (render_engine_create, render_init_surface, etc.)
- RenderEngine takes `*mut OH_NativeWindow` — HarmonyOS-specific, NOT GLFW
- Camera IDs: u64 (matches entity ID convention from core)
- Color: RGBA f32 struct, Color::black() default

## Anti-Patterns
- NEVER use RenderEngine with GLFW — it's HarmonyOS-specific (uses OH_NativeWindow)
- NEVER call init_surface with null window pointer