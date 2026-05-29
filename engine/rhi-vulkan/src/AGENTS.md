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
- **NEVER omit UV coordinates from any vertex type** — ALL vertices (Rect, Text, Triangle, Line, RectOutline, Rect stroke) MUST be 8 floats (x,y,r,g,b,a,u,v) matching pipeline stride 32. Missing UV causes cumulative misalignment in vertex buffer, making alternating Text batches read from wrong offsets → alternating invisible text.
- **NEVER get view matrix basis vectors wrong** — `right` and `up` must be derived from `forward` via cross products: `right = cross(forward_horizontal, world_up)`, `up = cross(right, forward)`. Sign errors in right.z or up.z make the view matrix NON-ORTHOGONAL → systematic geometric distortion (Z-axis shortened, objects look like trapezoid).
- **NEVER mismatch push constant byte offsets with GLSL struct layout** — GLSL PushConstants follow std430 alignment. After camera_pitch at byte 112, the layout is: `_pad2(116)` → `has_texture(120)` → `specular_strength(124)` → `ambient_strength(128)` → `shininess(132)` → `end_padding(136-143)`. NOT 3 padding zeros + 4 data values at offset 128.

## Bug Fix History

### 2026-05-26: Vertex stride mismatch — alternating tab text invisible
**现象**: TabWidget奇数标签页(1-based位置1,3)文字不显示，偶数标签页正常。两个TabWidget都有此问题。

**根因**: `DrawCommand::Triangle`、`DrawCommand::Line`、`DrawCommand::RectOutline` 的vertices只有6 floats (x,y,r,g,b,a)，**缺少UV坐标**。但UI pipeline的vertex stride=32 bytes (8 floats)。Rect fill和Text vertices有8 floats，Triangle/Line/RectOutline只有6 floats，导致：

1. 含Triangle的batch在flat vertex buffer中不对齐stride 8 (如132 floats → 132/8=16.5，余4 floats)
2. 后续Text batch的`first_vertex=(offset/8) as u32`截断计算(16.5→16)，pipeline从offset 128读取而非132
3. Text vertices整体偏移4 floats → position/color/UV全部错位 → 文字乱码或不可见
4. 交替模式：含Triangle的batch产生4-float余数→下一个Text错位→再下一个Rect+Triangle累积84 floats(84/8=10.5)→再再下一个Text又错位...直到某个组合凑整对齐

**修复**: 给Triangle、Line、RectOutline每个vertex加`0.0, 0.0` UV坐标，使所有vertex类型统一8 floats/vertex (stride 32 bytes)。Shader中`frag_uv==(0,0)`触发`out_color=frag_color`直通路径(不采样纹理)，与Rect fill vertices行为一致。

**修改文件**: `ui_vulkan_renderer.rs`
- Triangle: `[p1.x,p1.y,r,g,b,a]` → `[p1.x,p1.y,r,g,b,a,0.0,0.0]` (3 vertices)
- Line: `[x0,y0,r,g,b,a]` → `[x0,y0,r,g,b,a,0.0,0.0]` (6 vertices)
- RectOutline: `[x,y,r,g,b,a]` → `[x,y,r,g,b,a,0.0,0.0]` (8 vertices)

### 2026-05-26: View matrix basis vectors sign error — ALL objects Z-axis shortened (trapezoid)
**现象**: 所有物体在任何非零相机角度下Z轴缩短，看上去像梯形。用户确认是真bug。

**根因**: `rotation.vert` viewMatrix()的`right`和`up`向量Z分量符号反了：
- 错误: `right = vec3(cy, 0, -sy)` — right.z应为`+sy`
- 错误: `up = vec3(sy*sp, cp, cy*sp)` — up.z应为`-cy*sp`

正确的推导:
- `forward_horizontal = vec3(sy, 0, -cy)` (yaw旋转)
- `right = cross(forward_horizontal, world_up) = cross((sy,0,-cy), (0,1,0)) = (cy, 0, sy)` ← 注意right.z是+sy
- `up = cross(right, forward) = cross((cy,0,sy), (sy*cp,-sp,-cy*cp)) = (sy*sp, cp, -cy*sp)` ← 注意up.z是-cy*sp

错误符号导致视图矩阵**非正交**: `dot(right, forward) ≈ -0.54` (应为0!), `dot(up, forward) ≈ -0.09` (应为0!)。非正交矩阵把世界坐标倾斜变换到视图空间，投影后产生系统性Z轴压缩/梯形畸变。

yaw=0, pitch=0时恰好正确(sy=0, sp=0), 但任何非零角度都引入畸变。默认相机yaw=-0.3, pitch=-0.3就有显著畸变。

**修复**: `rotation.vert` 第37-38行:
- `vec3 right = vec3(cy, 0, -sy)` → `vec3 right = vec3(cy, 0, sy)`
- `vec3 up = vec3(sy * sp, cp, cy * sp)` → `vec3 up = vec3(sy * sp, cp, -cy * sp)`
- 重新编译shader: `glslc shaders/rotation.vert -o shaders/rotation.vert.spv`

**修改文件**: `shaders/rotation.vert`

### 2026-05-26: Push constant layout misalignment — lighting parameters wrong
**现象**: 实体纹理、高光、环境光、高光指数在GLSL中读到错误值。

**根因**: Rust push_data在camera_pitch(offset 112)之后放了**3个padding零**(bytes 116-127), 然后在offset 128放4个data值。但GLSL struct期望:
- offset 116: _pad2 (1 float)
- offset 120: has_texture
- offset 124: specular_strength
- offset 128: ambient_strength
- offset 132: shininess
- offset 136-143: struct end padding

3个零覆盖了has_texture和specular_strength, 导致GLSL读到has_texture=0.0(永远无纹理)、specular=0.0(无高光)。

**修复**: 5个push_data位置全部改为: `1个pad → 4个data值 → 2个end padding`
```
self.camera_pitch,
0.0f32,  // _pad2 at offset 116
has_texture_f, specular, ambient, shininess,  // offsets 120-132
0.0f32, 0.0f32,  // struct end padding to 144 bytes
```

**修改文件**: `ui_vulkan_renderer.rs` (5个push_data位置: ~2800, ~2867, ~2945, ~2999, ~3046)

### 2026-05-27: Phase 2/2b scissor继承 — 拖动预览窗时闪过密密麻麻emoji
**现象**: 鼠标拖动预览窗时，预览窗内部闪过密密麻麻的emoji，"像是预览窗背部还有一层专门用来渲染emoji了"。

**根因**: Vulkan scissor state是**per-draw-command动态状态**，如果不显式重设，后续draw command会继承前一个draw command的scissor。渲染管线3阶段:

1. **Phase 1** (before_preview batches, layers 0-1): 最后一个batch的scissor可能是restrictive clip_rect(如树形视图0-250px)
2. **Phase 2** (preview texture quad): 继承Phase 1最后的scissor → preview quad被裁剪 → 部分预览窗区域没有preview quad覆盖
3. 未覆盖区域显示Phase 1的UI内容(emoji text) → "密密麻麻铺满各种emoji"

同理Phase 2b(preview border)也可能继承Phase 1 scissor。

另外，`is_preview_window_context` flag在PreviewWindow未选中时不重置为false，影响后续widget的transparent Rect背景跳过和border stroke路由。

**修复** (3处):
1. Phase 2开始前: 设全屏scissor `vk::Rect2D { offset: 0, extent: self.extent }` — 确保preview quad不被Phase 1 restrictive scissor裁剪
2. Phase 2b开始前: 同样设全屏scissor — 确保preview border不被裁剪
3. `generate_render_data_recursive`: 每个widget边界处重置 `is_preview_window_context = false` — PreviewWindow未选中时不影响后续widget

**修改文件**: `ui_vulkan_renderer.rs` (~line3828 Phase 2 scissor, ~line3892 Phase 2b scissor), `widget_tree.rs` (~line3720 per-widget reset)

### 2026-05-27: generate_render_data返回值未使用编译错误
**现象**: `cargo build --release`报 `error[E0308]: mismatched types` — line 3285 `tree_guard.generate_render_data(&*font_atlas_guard)` 返回 `Vec<RenderData>` 但结果未赋值。

**根因**: `generate_render_data`从内部存储改为返回`Vec<RenderData>`后，atlas上传准备阶段的第一次调用(line 3285)不需要返回值(只需side effect: 光栅化glyph)，但缺少分号导致Rust期望unit type。

**修复**: 加分号 `tree_guard.generate_render_data(&*font_atlas_guard);` 丢弃返回值。渲染阶段的第二次调用(line 3391)正确赋值 `let render_data = ...`。

**修改文件**: `ui_vulkan_renderer.rs` line 3285

### 2026-05-29: 截图源图像错误 — ray_tracing模式截图深蓝色(空白)
**现象**: ray_tracing模式下按P截图，结果图片全是深蓝色(背景色)，看不到任何渲染内容。
**根因**: `capture_preview_screenshot` 硬编码从 `self.offscreen_image` 截图，但ray_tracing模式渲染到 `RayTracePipeline.output_image`，offscreen_image是空的(只有背景色)。rasterization模式offscreen_image有内容(经FXAA处理)。
**修复**:
1. 新增 `screenshot_source_image: vk::Image` 和 `screenshot_source_extent: vk::Extent2D` 字段，每帧从 `RenderPassOutput` 更新
2. `registry.rs`: `render_frame` 返回 `Option<(vk::DescriptorSet, RenderPassOutput)>` 而非 `Option<vk::DescriptorSet>`，提供截图源信息
3. `draw_frame`: 存储 `output.color_image` → `screenshot_source_image`, `output.extent` → `screenshot_source_extent`
4. `capture_preview_screenshot`: 用 `screenshot_source_image` 替代 `self.offscreen_image`
**修改文件**: `ui_vulkan_renderer.rs`(screenshot_source字段+draw_frame更新+截图函数), `registry.rs`(render_frame返回类型)

### 2026-05-29: raytrace反射效果增强 — color bleeding不可见
**现象**: 立方体靠墙一面看不到墙的颜色反射(color bleeding)，raytrace间接光照几乎不可见。
**根因**: 3个错误叠加:
1. 间接光线只2条，概率太低，大多数像素0条命中
2. `radiance`计算用`indirect_dir`(间接射线方向)dot光源方向，而非命中点面法线dot光源方向 — 物理模型错误，Lambert反射应基于命中面法线
3. 衰减`1/(d+1)`太强，远处命中(如墙到对面墙)贡献几乎为零
**修复**:
1. 间接光线2→8条，提高命中概率
2. `IndirectHit`新增`normal`字段，`trace_ray`计算命中点面法线
3. `radiance`用命中法线`dot(light_dir)`代替间接射线方向dot光源 — 正确Lambert漫反射
4. 衰减`1/(d+1)`→`1/(sqrt(d)+0.5)` — 减弱距离衰减，远处命中仍有贡献
5. 最终乘数0.3→1.0(除以8取平均，每条贡献1/8)
**修改文件**: `shaders/raytrace.comp`(IndirectHit+normal, 8条间接光线, 命中法线radiance, sqrt衰减), `shaders/raytrace.spv`(重编译)

### 2026-05-29: 管线切换崩溃 — ray_tracing→rasterization STATUS_ACCESS_VIOLATION
**现象**: 切换ray_tracing→rasterization后立即崩溃(STATUS_ACCESS_VIOLATION 0xc0000005)，日志显示"管线切换成功:rasterization"。
**根因**: `switch_pipeline` 中只保存/恢复了 offscreen/depth/fxaa 资源（8项），遗漏了 `vertex_buffer`, `vertex_buffer_memory`, `index_buffer`, `index_buffer_memory`（4项）。`std::mem::take(&mut active_resources)` 把这些buffer带走 → `mark_for_destroy` 销毁 → 新 `allocate_resources` 分配null handle → `RasterPipeline.record()` 用null的vertex_buffer → ACCESS_VIOLATION。
**修复**: `switch_pipeline` 中新增 `saved_vertex_buffer`, `saved_vertex_buffer_memory`, `saved_index_buffer`, `saved_index_buffer_memory` 的 `.take()` 保存和填回恢复。
**修改文件**: `ui_vulkan_renderer.rs` switch_pipeline方法

| File | Lines | Role |
|------|-------|------|
| `ui_vulkan_renderer.rs` | 4098 | Central renderer — init, game pass, UI pass, outline pass, FBO management |
| `ui_renderer.rs` | 559 | Font atlas texture + MSDF rendering |
| `renderer.rs` | 607 | Pipeline creation, render pass setup, swapchain management |
| `rotation_renderer.rs` | 815 | Entity mesh rendering (rotation demo) |
| `mono_rotation_renderer.rs` | 750 | Entity mesh rendering (Mono demo) |