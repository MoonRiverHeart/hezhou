# Game Preview with FXAA Implementation

## Architecture

```
┌─────────────────────────────────────────────────┐
│  Game Pass (dynamic extent)                      │
│  - 渲染旋转正方体 (36 vertices, 6 colored faces) │
│  - 输出到 offscreen_image                        │
│  - Push constants: rotation, width, height       │
│  - cull_mode: BACK, front_face: CCW              │
└─────────────────────────────────────────────────┘
                       ↓ transition
┌─────────────────────────────────────────────────┐
│  FXAA Pass (same extent as game)                 │
│  - 采样 offscreen_image                          │
│  - 应用FXAA边缘平滑算法                          │
│  - 输出到 offscreen_fxaa_image                   │
└─────────────────────────────────────────────────┘
                       ↓ transition
┌─────────────────────────────────────────────────┐
│  UI Pass (swapchain extent)                      │
│  - 渲染编辑器UI (ui shader)                      │
│  - PreviewWindow显示offscreen纹理                │
│  - cull_mode: NONE (2D UI不剔除)                 │
└─────────────────────────────────────────────────┘
```

## Key Components

### 1. Offscreen Images
- **offscreen_image**: Game pass输出 (dynamic extent, R8G8B8A8_UNORM)
- **offscreen_fxaa_image**: FXAA pass输出 (same extent)
- **Extent**: 动态匹配PreviewWindow尺寸（通过FFI设置）

### 2. Render Passes
- **game_render_pass**: 共用于 Game pass 和 FXAA pass
  - Attachment: `final_layout = SHADER_READ_ONLY_OPTIMAL`
  - Load op: CLEAR, Store op: STORE

### 3. Pipelines
- **game_pipeline**: rotation shader (36顶点正方体，透视投影)
  - cull_mode: BACK, front_face: COUNTER_CLOCKWISE
- **fxaa_pipeline**: fxaa shader (全屏quad，push constant传递分辨率)
- **ui_pipeline**: ui shader (支持MSDF文本和RGB纹理)
  - cull_mode: NONE, front_face: COUNTER_CLOCKWISE

### 4. Descriptor Sets
- **descriptor_set**: 字体图集 (UI文本)
- **preview_descriptor_set**: offscreen_image (预览区域)
- **fxaa_descriptor_set**: offscreen_image (FXAA采样)

### 5. Push Constants
- **Game pass**: `[rotation, width, height]` (3 floats, 12 bytes)
  - rotation: 旋转角度（radians）
  - width/height: 渲染尺寸，用于计算aspect
- **FXAA pass**: `vec2 resolution` (width, height)
- **UI pass**: `vec2 screen_size, vec2 offset, float px_range, bool enable_msdf`

## Shader Details

### rotation.vert (正方体36顶点)
```glsl
layout(push_constant) uniform PushConstants {
    float rotation;
    float width;
    float height;
} pc;

// 8 corners of cube
vec3 positions[8] = vec3[](
    vec3(-0.5, -0.5, -0.5),  // 0: back-bottom-left
    vec3( 0.5, -0.5, -0.5),  // 1: back-bottom-right
    ...
);

// 36 vertices for 6 faces (COUNTER_CLOCKWISE winding)
int vertex_indices[36] = int[](
    // Back face (Z-): 0,1,2, 0,2,3
    // Front face (Z+): 4,6,5, 4,7,6
    // All faces CCW from outside
    ...
);

void main() {
    // Model transform: rotate around Y then tilt
    // View transform: z = -3
    // Perspective projection with dynamic aspect
    float aspect = pc.width / pc.height;
    mat4 proj = perspective(1.0472, aspect, 0.1, 100.0); // 60° FOV
    gl_Position = proj * vec4(view_pos, 1.0);
    
    fragColor = face_colors[gl_VertexIndex / 6];
}
```

### fxaa.frag
```glsl
// 采样5个像素: center + N/S/E/W
vec3 rgb_center = texture(input_texture, in_uv).rgb;
vec3 rgb_n = texture(input_texture, in_uv + vec2(0.0, -texel_size.y)).rgb;
vec3 rgb_s = texture(input_texture, in_uv + vec2(0.0, texel_size.y)).rgb;
vec3 rgb_e = texture(input_texture, in_uv + vec2(texel_size.x, 0.0)).rgb;
vec3 rgb_w = texture(input_texture, in_uv + vec2(-texel_size.x, 0.0)).rgb;

// Luma计算 (灰度权重)
vec3 luma_weights = vec3(0.299, 0.587, 0.114);

// 边缘检测阈值
if (luma_range < 0.0625) {
    out_color = vec4(rgb_center, 1.0);  // 平滑区域，不处理
} else {
    vec3 rgb_avg = (rgb_n + rgb_s + rgb_e + rgb_w + rgb_center) / 5.0;
    out_color = vec4(mix(rgb_center, rgb_avg, blend_factor), 1.0);
}
```

### ui.frag (RGB texture mode)
```glsl
if (pc.enable_msdf) {
    // MSDF文本渲染 (alpha only)
    float alpha = smoothstep(-1.0, 1.0, (texture_color.r - 0.5) * px_range);
    out_color = vec4(frag_color.rgb, frag_color.a * alpha);
} else {
    // RGB纹理显示 (预览区域)
    out_color = vec4(texture_color.rgb * frag_color.rgb, texture_color.a * frag_color.a);
}
```

## Frame Rendering Flow

### Command Buffer Sequence
```rust
// 1. Game Pass
cmd_pipeline_barrier(offscreen: UNDEFINED → COLOR_ATTACHMENT_OPTIMAL)
cmd_begin_render_pass(game_render_pass, offscreen_framebuffer)
cmd_bind_pipeline(game_pipeline)
cmd_set_viewport/scissor(offscreen_extent)
cmd_push_constants([angle, width, height])
cmd_draw(36 vertices)  // 正方体
cmd_end_render_pass()

// 2. Game → FXAA transition
cmd_pipeline_barrier(offscreen: COLOR_ATTACHMENT → SHADER_READ_ONLY)
cmd_pipeline_barrier(offscreen_fxaa: UNDEFINED → COLOR_ATTACHMENT_OPTIMAL)

// 3. FXAA Pass
cmd_begin_render_pass(game_render_pass, offscreen_fxaa_framebuffer)
cmd_bind_pipeline(fxaa_pipeline)
cmd_bind_descriptor_sets(fxaa_descriptor_set)
cmd_set_viewport/scissor(offscreen_extent)
cmd_push_constants([width, height])
cmd_draw(6 vertices)  // 全屏quad
cmd_end_render_pass()

// 4. FXAA → UI transition
cmd_pipeline_barrier(offscreen_fxaa: COLOR_ATTACHMENT → SHADER_READ_ONLY)

// 5. UI Pass
cmd_begin_render_pass(ui_render_pass, swapchain_framebuffer)
cmd_bind_pipeline(ui_pipeline)
cmd_bind_descriptor_sets(descriptor_set)  // 字体图集
cmd_push_constants(screen_size, px_range, enable_msdf=true)
cmd_draw(UI vertices)  // 文本、按钮等

// 6. Preview Quad (if texture_id=1 in draw commands)
cmd_bind_descriptor_sets(preview_descriptor_set)  // offscreen纹理
cmd_push_constants(enable_msdf=false)  // RGB纹理模式
cmd_draw(6 vertices)  // PreviewWindow quad

cmd_end_render_pass()
```

## Performance

| Pass | Resolution | Vertices | Texture Samples |
|------|-----------|----------|----------------|
| Game | dynamic (e.g. 760x420) | 36 | 0 |
| FXAA | same as game | 6 | 5 per pixel |
| UI | 1280x720 (or resized) | ~5000 | 1 per glyph |

## Issue Resolution History

### Issue 1: Game Pass渲染尺寸固定，PreviewWindow动态尺寸导致拉伸

**Problem**:
- Game Pass渲染到固定512x512 offscreen texture
- PreviewWindow根据屏幕布局计算尺寸（760x420）
- 512x512 → 760x420 拉伸显示，正方体变形

**Root Cause**:
- Offscreen extent硬编码为512x512
- PreviewWindow尺寸动态计算
- 纹理缩放导致变形

**Solution**: 让Game Pass的offscreen渲染尺寸动态匹配PreviewWindow

**Implementation**:
1. **FFI接口**: `UI.SetGamePreviewExtent(width, height)`
2. **Rust方法**: `UIVulkanRenderer::set_game_preview_extent()`
   - 重建offscreen_image, image_view, framebuffer
   - 重建offscreen_fxaa_image, image_view, framebuffer
   - 更新descriptor sets
3. **C#调用**: EditorScript创建PreviewWindow后调用FFI

**Commit**: `3811b7d`

---

### Issue 2: Shader透视投影使用固定aspect=1.0

**Problem**:
- Shader中 `perspective(fov, 1.0, near, far)` 固定aspect=1.0
- 实际渲染760x420，aspect=1.81
- 正方体仍然扁扁的

**Root Cause**:
- Shader不知道实际渲染尺寸
- 无法计算正确的aspect ratio

**Solution**: 通过push constant传递width/height

**Implementation**:
1. **Shader**: PushConstants添加width/height字段
   ```glsl
   layout(push_constant) uniform PushConstants {
       float rotation;
       float width;
       float height;
   } pc;
   
   float aspect = pc.width / pc.height;
   mat4 proj = perspective(1.0472, aspect, 0.1, 100.0);
   ```
2. **Rust**: PushConstantRange size从4改为12
3. **Draw**: 传递 `[angle, width, height]`

**Commit**: `1cea5b6`

---

### Issue 3: UI和Game Pipeline背面剔除混淆

**Problem**:
- 误将UI pipeline的cull_mode改成BACK
- UI quad顶点顺序CLOCKWISE，被背面剔除
- 结果：所有UI元素消失

**Root Cause**:
- UI和Game pipeline在同一个文件
- 修改时未确认是哪个pipeline
- UI是2D quad，不应该剔除

**Solution**: 区分UI和Game pipeline的剔除设置

| Pipeline | cull_mode | front_face | 原因 |
|---|---|---|---|
| UI pipeline | NONE | CCW | 2D quad不剔除 |
| Game pipeline | BACK | CCW | 3D cube剔除背面 |

**Lesson**: 修改pipeline时确认是UI还是Game

**Commit**: `76ca001`

---

### Issue 4: 正方体顶点顺序混乱

**Problem**:
- 6个面顶点顺序不一致：部分CW，部分CCW
- 法线方向混乱：有的朝外，有的朝里
- 显示效果不一致

**Root Cause**:
- 顶点索引数组编写错误
- 未统一为逆时针顺序

**Solution**: 统一所有面为逆时针（从外部看）

**Vertex Order Fixes**:
```
Back  (Z-): 0,2,1 → 0,1,2 (CW→CCW)
Front (Z+): 4,5,6 → 4,6,5 (CW→CCW)
Left  (X-): 0,4,7 → 0,7,4 (CW→CCW)
Right (X+): 1,6,5 → 不变
Bottom(Y-): 0,1,5 → 不变
Top   (Y+): 3,7,6 → 3,6,7 (CW→CCW)
```

**Commit**: `1286127`

---

### Issue 5: 窗口Resize时预览窗未动态调整

**Problem**:
- 窗口resize更新_screenWidth/_screenHeight
- PreviewWindow尺寸未重新计算
- Game Pass extent未更新
- 正方体aspect ratio错误

**Root Cause**:
- OnResize只更新屏幕尺寸
- 未重新计算PreviewWindow尺寸
- 未调用SetGamePreviewExtent

**Solution**: UpdateLayout中更新PreviewWindow

**Implementation**:
```csharp
// EditorScript.cs UpdateLayout()
float previewWindowWidth = previewWidth - 20f;
float previewWindowHeight = mainHeight - 20f;
UI.SetWidgetLayout(_previewWindowId, 10f, 40f, previewWindowWidth, previewWindowHeight);
UI.SetGamePreviewExtent((uint)previewWindowWidth, (uint)previewWindowHeight);
```

**Test Results**:
- 1280x720: 760x420 (initial)
- 3072x1814: 2552x1514 (maximize)
- 1280x720: 760x420 (restore)

**Commit**: `822448c`

---

## Known Issues & Solutions (Legacy)

### Issue 1: Screen Tearing (三角形运动有撕裂)
**原因**: 未启用垂直同步
**解决**: 设置 `swapchainCreateInfo.minImageCount = 3` 或使用 MAILBOX present mode

### Issue 2: Rotation Center Off (三角形旋转中心偏移)
**原因**: rotation shader的顶点位置以(0,0)为中心，不是几何中心
**解决**: 调整顶点位置使几何中心在(0,0)

### Issue 3: Preview Overlap (预览区域覆盖脚本编辑器)
**原因**: 预览quad在UI pass最后渲染，层级在最上
**解决**: 创建PreviewWindow widget作为PreviewPanel的子控件

### Issue 4: Preview Area Size (渲染区域侵占状态栏)
**原因**: 预览区域尺寸计算错误
**解决**: 调整预览区域边界计算

## Git Commits

| Commit | Description |
|--------|-------------|
| `3811b7d` | Dynamic game preview extent matching PreviewWindow size |
| `1cea5b6` | Fix cube aspect ratio: pass dynamic width/height to shader |
| `76ca001` | Fix UI pipeline cull_mode - don't cull 2D UI |
| `1286127` | Fix cube vertex winding order for backface culling |
| `822448c` | Add PreviewWindow dynamic resize on window resize |

## Files Changed

```
engine/rhi-vulkan/src/ui_vulkan_renderer.rs
  - set_game_preview_extent() method
  - push_constant_data: [angle, width, height]
  - game_pipeline: cull_mode=BACK, front_face=CCW
  - ui_pipeline: cull_mode=NONE

engine/shaders/rotation.vert
  - PushConstants: rotation, width, height
  - aspect = pc.width / pc.height
  - 36 vertices, COUNTER_CLOCKWISE winding

engine/scripting/src/ffi_context.rs
  - SetGamePreviewExtentFn delegate
  - ui_set_game_preview_extent field

engine/scripts/UI.cs
  - SetGamePreviewExtentDelegate
  - UI.SetGamePreviewExtent() method

engine/scripts/EditorScript.cs
  - CreateEditorLayout: SetGamePreviewExtent after CreatePreviewWindow
  - UpdateLayout: recalculate and update extent on resize
```