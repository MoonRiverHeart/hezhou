# Game Preview with FXAA Implementation

## Architecture

```
┌─────────────────────────────────────────────────┐
│  Game Pass (512x512 offscreen)                   │
│  - 渲染旋转三角形 (rotation shader)              │
│  - 输出到 offscreen_image                        │
│  - Clear color: (0.05, 0.05, 0.1, 1.0)          │
└─────────────────────────────────────────────────┘
                      ↓ transition
┌─────────────────────────────────────────────────┐
│  FXAA Pass (512x512 offscreen_fxaa)              │
│  - 采样 offscreen_image                          │
│  - 应用FXAA边缘平滑算法                          │
│  - 输出到 offscreen_fxaa_image                   │
└─────────────────────────────────────────────────┘
                      ↓ transition
┌─────────────────────────────────────────────────┐
│  UI Pass (1280x720 swapchain)                    │
│  - 渲染编辑器UI (ui shader)                      │
│  - 预览区域显示 FXAA 处理后的纹理                │
│  - 绑定 preview_descriptor_set                   │
└─────────────────────────────────────────────────┘
```

## Key Components

### 1. Offscreen Images
- **offscreen_image**: Game pass输出 (512x512, R8G8B8A8_UNORM)
- **offscreen_fxaa_image**: FXAA pass输出 (512x512, R8G8B8A8_UNORM)

### 2. Render Passes
- **game_render_pass**: 共用于 Game pass 和 FXAA pass
  - Attachment: `final_layout = SHADER_READ_ONLY_OPTIMAL`
  - Load op: CLEAR, Store op: STORE

### 3. Pipelines
- **game_pipeline**: rotation shader (内置顶点，push constant控制旋转角度)
- **fxaa_pipeline**: fxaa shader (全屏quad，push constant传递分辨率)
- **ui_pipeline**: ui shader (支持MSDF文本和RGB纹理)

### 4. Descriptor Sets
- **descriptor_set**: 字体图集 (UI文本)
- **preview_descriptor_set**: offscreen_fxaa_image (预览区域)
- **fxaa_descriptor_set**: offscreen_image (FXAA采样)

### 5. Push Constants
- **Game pass**: `float rotation_angle` (radians)
- **FXAA pass**: `vec2 resolution` (width, height)
- **UI pass**: `vec2 screen_size, vec2 offset, float px_range, bool enable_msdf`

## Shader Details

### rotation.vert
```glsl
// 内置顶点位置
vec2 positions[3] = vec2[](
    vec2(0.0, -0.5),  // 顶部
    vec2(-0.5, 0.5),  // 左下
    vec2(0.5, 0.5)    // 右下
);

// Push constant: rotation angle
float angle = pc.rotation;
mat2 rotation = mat2(cos(angle), -sin(angle), sin(angle), cos(angle));
gl_Position = vec4(rotation * positions[gl_VertexIndex], 0.0, 1.0);
```

### rotation.frag
```glsl
// 内置颜色
vec3 colors[3] = vec3[](
    vec3(1.0, 0.0, 0.0),  // 红色
    vec3(0.0, 1.0, 0.0),  // 绿色
    vec3(0.0, 0.0, 1.0)   // 蓝色
);
outColor = vec4(colors[gl_VertexIndex], 1.0);
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
cmd_set_viewport/scissor(512x512)
cmd_push_constants(rotation_angle)
cmd_draw(3 vertices)  // 旋转三角形
cmd_end_render_pass()

// 2. Game → FXAA transition
cmd_pipeline_barrier(offscreen: COLOR_ATTACHMENT → SHADER_READ_ONLY)
cmd_pipeline_barrier(offscreen_fxaa: UNDEFINED → COLOR_ATTACHMENT_OPTIMAL)

// 3. FXAA Pass
cmd_begin_render_pass(game_render_pass, offscreen_fxaa_framebuffer)
cmd_bind_pipeline(fxaa_pipeline)
cmd_bind_descriptor_sets(fxaa_descriptor_set)
cmd_set_viewport/scissor(512x512)
cmd_push_constants(resolution: 512, 512)
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

// 6. Preview Quad
cmd_bind_descriptor_sets(preview_descriptor_set)  // FXAA输出纹理
cmd_push_constants(enable_msdf=false)  // RGB纹理模式
cmd_draw(6 vertices)  // 预览区域quad

cmd_end_render_pass()
```

## Performance

| Pass | Resolution | Vertices | Texture Samples |
|------|-----------|----------|----------------|
| Game | 512x512 | 3 | 0 |
| FXAA | 512x512 | 6 | 5 per pixel |
| UI | 1280x720 | ~5000 | 1 per glyph |

FXAA只处理512x512预览区域，不影响UI文本渲染。

## Known Issues & Solutions

### Issue 1: Screen Tearing (三角形运动有撕裂)
**原因**: 未启用垂直同步
**解决**: 设置 `swapchainCreateInfo.minImageCount = 3` 或使用 MAILBOX present mode

### Issue 2: Rotation Center Off (三角形旋转中心偏移)
**原因**: rotation shader的顶点位置以(0,0)为中心，不是几何中心
**解决**: 调整顶点位置使几何中心在(0,0)

### Issue 3: Preview Overlap (预览区域覆盖脚本编辑器)
**原因**: 预览quad在UI pass最后渲染，层级在最上
**解决**: 创建PreviewWindow widget作为PreviewPanel的子控件
**架构**:
```
PreviewPanel (Panel widget)
├── Label "游戏预览"
└── PreviewWindow (texture_id=1) ← 游戏预览纹理
```
**实现**:
- PreviewWindow widget: `ui/src/widgets/preview_window.rs`
- FFI接口: `ui_create_preview_window`, `ui_set_preview_texture`
- EditorScript.cs: `_previewWindowId = UI.CreatePreviewWindow(_previewPanel.Id, ...)`
- UIVulkanRenderer: 根据texture_id切换descriptor_set

### Issue 4: Preview Area Size (渲染区域侵占状态栏)
**原因**: 预览区域尺寸计算错误
**解决**: 调整预览区域边界计算

## Git Commits

| Commit | Description |
|--------|-------------|
| `a44e756` | Add FXAA shaders |
| `31df277` | Game preview rendering with offscreen pass |
| `8fe5b54` | Display game preview texture in UI |
| `2af7418` | Complete FXAA post-processing pipeline |
| `a0b4f78` | Fix game pipeline vertex input + RGB texture mode |

## Files Changed

```
engine/rhi-vulkan/src/ui_vulkan_renderer.rs  (+339 lines)
engine/shaders/fxaa.vert                       (新建)
engine/shaders/fxaa.frag                       (新建)
engine/shaders/ui/ui.frag                      (修改: RGB纹理支持)
```