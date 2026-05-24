# Vulkan Renderer

RHI Vulkan implementation for game engine rendering.

## Overview
Dual-pass rendering: Game Pass (offscreen) + UI Pass (font atlas), composited to swapchain. Outline pass for entity selection.

## Where To Look
| Task | File | Notes |
|------|------|-------|
| Add render feature | `ui_vulkan_renderer.rs` | 3649 lines, central renderer |
| Fix game rendering | `rotation_renderer.rs` or `mono_rotation_renderer.rs` | Entity mesh rendering |
| Fix UI rendering | `ui_renderer.rs` | Font texture + MSDF rendering |
| Fix pipeline config | `renderer.rs` | Pipeline creation, render pass setup |
| Fix depth/blend | `ui_vulkan_renderer.rs:757-768` | Depth stencil + alpha blend state |

## Key Architecture
```
Render Flow:
1. Game Render Pass (offscreen framebuffer)
   → game_pipeline (entity meshes)
   → outline_pipeline (selection highlight, CULL_FRONT + scale>1)
   → FXAA pass (anti-aliasing)

2. UI Render Pass (swapchain)
   → ui_pipeline (font atlas texture)
   → preview_pipeline (game texture -> PreviewWindow)
   → Merge: swapchain present
```

## Conventions
- All pipelines MUST use `p_dynamic_state` for VIEWPORT+SCISSOR (runtime resize)
- Outline: `depth_write_enable: FALSE`, `blend_enable: TRUE`, `CULL_FRONT` + scale>1
- Game preview extent set via `SetGamePreviewExtent(w, h)` — must match PreviewWindow size
- Push constants must match shader struct exactly (size + field order)

## Anti-Patterns
- NEVER create pipeline without `p_dynamic_state` for VIEWPORT+SCISSOR
- NEVER forget depth attachment (D32_SFLOAT) in game_render_pass
- NEVER hardcode viewport size — use `cmd_set_viewport` at render time
- NEVER mismatch push constant layout with shader struct
- NEVER destroy FBO resources without device_wait_idle() first (causes device lost)

## Key Files
| File | Lines | Role |
|------|-------|------|
| `ui_vulkan_renderer.rs` | 4098 | Central renderer — init, game pass, UI pass, outline pass, FBO management |
| `ui_renderer.rs` | 559 | Font atlas texture + MSDF rendering |
| `renderer.rs` | 607 | Pipeline creation, render pass setup, swapchain management |
| `rotation_renderer.rs` | 815 | Entity mesh rendering (rotation demo) |
| `mono_rotation_renderer.rs` | 750 | Entity mesh rendering (Mono demo) |