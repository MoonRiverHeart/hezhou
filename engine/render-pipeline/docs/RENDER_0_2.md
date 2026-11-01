# Hezhou 0.2 渲染能力：PBR / Shadow Map / Tonemap / Bloom / 质量档

> 状态：第一轮交付（引擎级能力，编辑器接入点见 §8）
> 维护：render-owner（task-4）
> 边界：仅涉及 `engine/render-pipeline/**`、`engine/rhi-vulkan/**`、`engine/shaders/**`；
> 未修改 `examples/`、C#、`core`、`asset`。

## 1. 能力总览

| 能力 | 状态 | 说明 |
|---|---|---|
| PBR 材质 | ✅ 最小实现 | albedo / metallic / roughness / normal / AO / emissive；CPU 参考 BRDF + `pbr.vert/frag` |
| 方向光 Shadow Map | ✅ 1~4 级 cascade | 深度图 `D32_SFLOAT`、array 分层、比较采样器、3x3 PCF、bias/纹素吸附可配 |
| Tonemap + Gamma | ✅ | None / Reinhard / ACES / Filmic + exposure + gamma |
| Bloom | ✅ 可选 | 阈值+软膝提取、半分辨率 mip 链计划、Tonemap pass 合成 |
| 质量档 低/中/高 | ✅ | 阴影开关、cascade 数、阴影分辨率、Bloom 开关、FXAA 开关 |
| Blinn-Phong 回退 | ✅ 保留 | `RenderSettings::default()` 即 0.1 legacy 行为，现有 raster/FXAA/UI 路径不变 |
| 编辑器实际启用 | ✅ 已接入（task-9） | `UIVulkanRenderer` 在 PBR 质量档走新 draw path；默认设置仍走 legacy，零回归 |
| PBR 模式完整视觉 | ✅ 已补齐（task-18） | 天空盒（全屏渐变）、选中 outline、3D Gizmo 三轴、合成蒙皮 mesh 均进入实际 draw |
| 蒙皮实际 draw | ✅ 合成 mesh 验证（task-18） | 64B stride + location 3/4 + joint palette 真实 draw；`upload_skin_palette` 供 asset/Player 接入 |
| PBR 模式 resize | ✅ GPU 冒烟（task-18） | 320×180 → 640×360 重建并重新录制通过 |

## 2. 模块结构

### render-pipeline（纯 CPU，可单测）

| 文件 | 内容 |
|---|---|
| `src/pbr.rs` | `PbrMaterial` / `PbrLight` / `PbrPushConstant`(96B) / `PbrFrameUbo`(480B) / CPU 参考 BRDF |
| `src/shadow.rs` | `ShadowSettings` / `ShadowConfig` / `ShadowFrameData` / cascade 分割 / 光源矩阵 / 纹素吸附 / `ShadowPushConstant`(128B) |
| `src/post.rs` | `TonemapOperator` / `TonemapSettings` / `BloomSettings` / `PostTonemapPushConstant`(16B) / `BloomPrefilterPushConstant`(16B) |
| `src/quality.rs` | `RenderQualityPreset` / `QualitySettings` / `RenderSettings`（聚合入口） |
| `src/context.rs` | `RenderContext.render_settings` + `aspect()` / `shadow_frame()` / `pbr_frame_ubo()` |
| `tests/shader_contract.rs` | GLSL 布局/常量与 Rust 结构体一致性、`.spv` 有效性、glslc 真实编译 |
| `tests/render_settings.rs` | RenderContext → 阴影矩阵 → PBR UBO 集成链路 |

### rhi-vulkan（Vulkan 层资源规划与创建）

| 文件 | 内容 |
|---|---|
| `src/render_quality.rs` | 质量档 → `ShadowPassDesc`（image/view/sampler/render pass 创建参数）、`BloomPlan`、pass 顺序、显存估算 |
| `src/shadow_map.rs` | `ShadowMapResources`：真实创建深度图/视图/采样器/render pass/framebuffer；`#[ignore]` GPU 冒烟测试 |
| `src/skinning_plan.rs` | 蒙皮 pipeline 接入计划：`SkinnedVertexInput`（stride 64 + 5 attribute）、joint palette descriptor binding、palette 字节数、已编译 SPIR-V 校验 |
| `src/pbr_forward.rs` | **实际绘制路径**：`PbrForwardPass`（HDR 目标 + PBR/Shadow/Bloom/Tonemap + 天空盒 + 选中 outline + 3D Gizmo pipeline + 录制函数）、`SkinnedPipelineState`（蒙皮 pipeline + 内置合成 skinned mesh + palette 动画/上传）、`camera_view_matrix/projection`、GPU 冒烟测试 |
| `tests/pbr_forward_gpu.rs` | `#[ignore]` GPU 冒烟：High/Low 完整视觉 + 合成蒙皮 draw + resize 真实提交 |
| `src/ui_vulkan_renderer.rs` | 新增 `render_settings` 字段与 `set_render_quality_preset` / `set_render_settings` / `render_plan` 接口 |

### shaders（新增，均带中文注释与契约常量）

| 文件 | 用途 |
|---|---|
| `pbr.vert` / `pbr.frag` | PBR 前向着色（HDR 线性输出，含 CSM 阴影采样、法线贴图、3x3 PCF） |
| `shadow_depth.vert` / `shadow_depth.frag` | 阴影深度 pass（仅深度写入） |
| `post.vert` | 全屏三角形（后处理共用顶点） |
| `post_tonemap.frag` | Bloom 合成 + Tonemap + Gamma → LDR |
| `bloom_prefilter.frag` | 亮度阈值/软膝提取 + 4-tap 降采样 |
| `skinning.vert` / `skinning.frag` | 骨骼蒙皮（animation-owner 提供，本轮已编译 `.spv`；接入计划见 §7） |

## 3. GPU 数据布局契约

所有布局由 `tests/shader_contract.rs` 自动校验（解析 GLSL 块 + 与 Rust `offset_of!` 对比）。

| 数据 | 大小 | 对应 Shader | 字段 |
|---|---|---|---|
| `PbrPushConstant` | 96 B | `pbr.vert` / `pbr.frag` | `mat4 model; vec4 base_color; vec4 material_params`（x=metallic,y=roughness,z=normal_scale,w=ao） |
| `PbrFrameUbo` | 480 B | `pbr.vert` / `pbr.frag`（set 0 binding 0） | direction/intensity、color/shadow_strength、ambient/cascade_count、cascade_splits、shadow_params、view、proj、light_view_proj[4]、camera_pos |
| `ShadowPushConstant` | 128 B | `shadow_depth.vert` | `mat4 light_view_proj; mat4 model` |
| `PostTonemapPushConstant` | 16 B | `post_tonemap.frag` | `vec4 params`（x=exposure,y=gamma,z=tonemap_op,w=bloom_intensity） |
| `BloomPrefilterPushConstant` | 16 B | `bloom_prefilter.frag` | `vec4 params`（x=threshold,y=soft_knee,z=texel_x,w=texel_y） |
| 阴影 sampler | — | set 0 binding 1 | `sampler2DArrayShadow`，`D32_SFLOAT`，`LESS_OR_EQUAL`，`CLAMP_TO_BORDER`，`FLOAT_OPAQUE_WHITE` |
| 材质贴图 | — | set 1 binding 0..3 | albedo / normal / metallic-roughness(G=roughness,B=metallic) / AO |

契约常量（Rust ↔ GLSL 双端校验）：

| 常量 | 值 | Rust 位置 |
|---|---|---|
| `MAX_CASCADES` | 4 | `shadow::MAX_CASCADES` |
| `PBR_PI` | 3.14159265 | `pbr::PBR_PI` |
| `DIELECTRIC_F0` | 0.04 | `pbr::DIELECTRIC_F0` |
| `MIN_ROUGHNESS` | 0.045 | `pbr::MIN_ROUGHNESS` |
| `TONEMAP_NONE/REINHARD/ACES/FILMIC` | 0/1/2/3 | `post::TonemapOperator::as_u32` |

> **约定**：修改上述任何字段/常量，必须同时修改 shader 与 Rust 结构体；`shader_contract` 测试会失败。

## 4. Shader 编译

工程仍是**手动编译 SPIR-V**（与现有流程一致），新增文件已编译并入库：

```bash
cd engine
glslc shaders/pbr.vert -o shaders/pbr.vert.spv
glslc shaders/pbr.frag -o shaders/pbr.frag.spv
glslc shaders/shadow_depth.vert -o shaders/shadow_depth.vert.spv
glslc shaders/shadow_depth.frag -o shaders/shadow_depth.frag.spv
glslc shaders/post.vert -o shaders/post.vert.spv
glslc shaders/post_tonemap.frag -o shaders/post_tonemap.frag.spv
glslc shaders/bloom_prefilter.frag -o shaders/bloom_prefilter.frag.spv
```

`shader_contract.rs` 会在检测到 `glslc` 时额外做一次真实编译校验；无 glslc 环境自动跳过。

## 5. 渲染顺序与回退

```
Shadow（可选，每级 cascade 一次）
  → GameOpaque（HDR：R16G16B16A16_SFLOAT）
  → BloomPrefilter（可选）
  → TonemapBloom（可选：Bloom 合成 + Tonemap + Gamma → LDR）
  → FXAA（保留）
  → UI（swapchain）
```

- **UI / FXAA / 现有 raster pipeline 不变**：新能力通过 `RenderSettings` 增量提供，legacy 路径继续走 `rotation.vert/frag`。
- **回退开关**：`RenderSettings::legacy_compatible()` / `ShadingModel::BlinnPhong`；
  `RenderSettings::default()` 就是 legacy，因此未切换质量档时视觉零回归。
- 阴影关闭时 `shadow_pass_desc()` 返回 `None`，不分配深度图。
- Bloom 关闭时 Tonemap pass 直接采样 HDR 场景（无 Bloom 纹理）。

### 5.1 实际接入 UIVulkanRenderer（task-9，已生效）

`UIVulkanRenderer` 内新增 `pbr_forward: Option<PbrForwardPass>` + `pbr_skinned: Option<SkinnedPipelineState>`，
仅在 `RenderSettings::shading == ShadingModel::Pbr` 且未启用 PipelineRegistry 时激活：

```
draw_frame:
  ensure_pbr_forward()               // 设置/尺寸变化时创建或重建
  if pbr_forward:                    // 新路径
      Shadow pass (每 cascade)       // shadow_depth.vert/frag, D32 array
      HDR pass:
        天空盒（sky.vert/frag）      // 全屏渐变，clear color 用天空底色
        Game(PBR)                    // pbr.vert/frag, R16G16B16A16_SFLOAT + 复用 D32 depth
        选中 outline（rotation.vert/frag，144B push，放大 1.05 + alpha blend）
        3D Gizmo 三轴（同一 outline pipeline + gizmo 顶点缓冲）
        合成蒙皮 mesh（skinning.vert/rotation.frag，stride64 + palette）
      Bloom prefilter (可选)         // bloom_prefilter.frag, 半分辨率
      Tonemap                        // post_tonemap.frag → renderer 的 LDR offscreen image
  else:                              // Legacy 路径（默认）
      rotation.vert/frag + 天空盒 + outline + gizmo（完全不变）
  FXAA（既有代码，两个路径共用）        // 采样 LDR offscreen image
  UI（既有代码）
```

关键实现点：

| 项 | 实现 |
|---|---|
| PBR descriptor | set0 = PbrFrameUbo(480B) + 阴影 `sampler2DArrayShadow`；set1 = albedo/normal/MR/AO（默认中性 1x1 贴图） |
| 阴影 | 质量档分辨率/cascade 数；`ShadowMapResources` 创建 D32 array + 逐层 framebuffer；`LESS_OR_EQUAL` + `CLAMP_TO_BORDER` |
| 天空盒 | `PbrForwardPass` 创建 HDR pass 兼容的 sky pipeline（32B push：top/bottom） |
| outline/Gizmo | HDR pass 兼容 pipeline，复用 legacy 144B push + renderer 的 light/texture descriptor sets（layout 兼容）；Gizmo 顶点缓冲按需创建 |
| 合成蒙皮 | `SkinnedPipelineState` 内置 8 顶点/36 索引合成 cube + 2 关节动画 palette，每帧 `record_synthetic` 真实 draw |
| 阴影关闭 | 绑定 1x1 占位 D32 array（shader 在 `cascade_count=0` 时直接返回受光） |
| Bloom 关闭 | Tonemap 的 bloom binding 指向 HDR 自身且 `bloom_intensity=0`，无需占位纹理 |
| SRGB 目标 | LDR 格式为 SRGB 时把 tonemap gamma 置 1.0，由硬件编码，避免双重 gamma |
| 质量档切换 | 下一帧检测 `settings != pass.settings()` → 旧 pass 进 `retired_pbr`（cleanup 统一销毁），临时重建 |
| resize | `apply_pending_offscreen_resize` 先退役旧 PBR 通道，下一帧用新 depth/ldr view 重建（GPU 冒烟覆盖 320→640） |
| Legacy 零回归 | 默认设置 `pbr_forward=None`，draw_frame 仍执行原有 legacy 分支（天空盒/outline/gizmo/Blinn-Phong 全保留） |

Draw path 证据 API（编辑器/测试可直接查询）：

```rust
renderer.set_render_quality_preset(RenderQualityPreset::High);
assert!(renderer.pbr_forward_active());
println!("{}", renderer.pbr_draw_path_summary().unwrap());
// 例如: PBR active: quality=high shading=PBR shadows=ON 3cascade@2048 bloom=true ... hdr=1280x720 skinned_pipeline=true
```

GPU 冒烟测试（`rhi-vulkan/tests/pbr_forward_gpu.rs`，`#[ignore]`）真实创建资源并录制
Shadow→Game(PBR)→Bloom→Tonemap 命令后提交，High/Low 两档均通过。

## 6. 质量档

| 档位 | 着色 | 阴影 | cascade | 阴影分辨率/级 | Bloom | Tonemap | FXAA |
|---|---|---|---|---|---|---|---|
| Low | PBR | 关 | 1 | 512 | 关 | 开 | 开 |
| Medium | PBR | 开 | 2 | 1024 | 关 | 开 | 开 |
| High | PBR | 开 | 3 | 2048 | 开 | 开 | 开 |
| Custom | 可覆盖 | 可覆盖 | 可覆盖 | 可覆盖 | 可覆盖 | 可覆盖 | 可覆盖 |
| Legacy（默认） | Blinn-Phong | 关 | — | — | 关 | 关 | 开 |

```rust
use hezhou_render_pipeline::{RenderQualityPreset, RenderSettings};

// 切换质量档
let mut settings = RenderSettings::for_preset(RenderQualityPreset::High);
settings.tonemap.exposure = 1.2;
settings = settings.validate();

// 每帧交给渲染后端
let frame = settings.shadow_config();
```

阴影显存估算（`render_quality::shadow_map_bytes`）：

| 档位 | 显存 |
|---|---|
| High | 2048² × 3 × 4B = **48 MiB** |
| Medium | 1024² × 2 × 4B = **8 MiB** |
| Low | 0（不分配） |

## 7. 蒙皮（Skinning）pipeline 接入契约（与 animation-owner 协作）

`animation-owner` 已交付 `engine/animation`。**task-18 已完成**：GPU pipeline 创建
（`hezhou_rhi_vulkan::SkinnedPipelineState`）、palette 资源与 `upload_skin_palette` API，
并且 **PBR draw path 已用合成 skinned mesh 真实绘制**（8 顶点 cube，上下两段分属 2 个关节，
每帧动画 palette）；High/Low/resize 三组 GPU 冒烟覆盖。真实角色 mesh 仍需 asset/core 提供
顶点/索引 + 每帧 palette（接口已就绪）。

| 项目 | 契约 |
|---|---|
| 顶点 stride | 32 → **64 bytes**（前 32 字节与现有顶点完全一致） |
| location 3 | `R32G32B32A32_UINT`（joint indices），offset 32 |
| location 4 | `R32G32B32A32_SFLOAT`（joint weights），offset 48 |
| joint palette | set = 0, binding = 1（binding 0 是 LightData），std430 `mat4 joints[]` |
| palette 数据 | 每帧 `machine.sample` → `JointPalette::from_pose` → `to_le_bytes()` 上传 |
| 容量 | ≤256 关节可用 UBO（16 KiB）；SSBO 无此限制 |
| push constant | 复用现有 144 bytes 布局（`skinning.vert` 与 `rotation.vert` 一致） |
| shader | `shaders/skinning.vert` / `skinning.frag` 已编译为 `.spv` |

Rust 侧已提供（`hezhou_rhi_vulkan::skinning_plan` 契约 + `SkinnedPipelineState` 实际 pipeline）：

```rust
use hezhou_rhi_vulkan::{SkinnedPipelineState, skinning_plan};

// 顶点输入计划（stride 64 + 5 个 attribute）
let vertex_input = skinning_plan::SkinnedVertexInput::new();

// GPU pipeline 由 UIVulkanRenderer::ensure_pbr_forward 自动创建；
// 每帧上传 palette：
renderer.upload_skin_palette(&palette.to_le_bytes())?;
```

**当前状态**：pipeline/palette/合成 skinned mesh 已进入 PBR HDR pass 的真实 draw，
`upload_skin_palette` 供 asset/Player 上传真实角色 palette；接入点见 §9 与 §12。

## 8. 已知限制（0.2）

1. **只有 1 盏方向光**：多光源/点光/聚光不在本轮范围。
2. **无 IBL/SSAO/GI**：环境项为常量 ambient × albedo × AO。
3. **阴影只支持不透明几何**：无 alpha test、无透明阴影、无 PCSS。
4. **无逐级视锥剔除/GPU 遮挡剔除**：每级 cascade 全量绘制该级可见实体（第二轮可加）。
5. **法线贴图使用导数 TBN**：无顶点切线数据；对强各向异性表面质量有限。
6. **HDR 目标格式固定 `R16G16B16A16_SFLOAT`**：未做设备格式能力回退。
7. **PBR 模式下的简化**（legacy 模式不受影响）:
   - 材质固定 `RenderSettings.default_material` + 中性默认贴图；per-entity 材质/纹理未接入
   - `PipelineRegistry` 启用时 PBR 通道不激活（走 registry 管线）
8. **GPU 测试 ignored**：`create_shadow_map_on_gpu` / `pbr_forward_gpu_*` 需真实 Vulkan 设备。
9. **阴影分辨率需 2 的幂**：`normalize_shadow_resolution` 向下取整收敛。
10. **SPIR-V 手动编译**：修改 shader 后必须重跑 glslc，否则运行时使用旧 `.spv`。
11. **蒙皮真实角色数据未接入**：draw path 用合成 skinned mesh 验证（pipeline 正确性），
    真实角色需 asset/Player 提供 64B skinned 顶点/索引 + 每帧 palette（`upload_skin_palette` 已就绪）。
12. **质量档/resize 时旧 PBR 通道延迟到 cleanup 销毁**：避免运行期销毁 Vulkan 资源触发
    驱动 HEAP_CORRUPTION；频繁切换质量档会暂时多占显存（换档不频繁，可接受）。
13. **PBR 模式 gizmo/outline 复用 legacy 着色器**：视觉效果与 legacy 一致，但需要 renderer
    每帧更新 legacy light UBO（已实现）。

## 9. 编辑器接入点（给 Lead 的集成清单）

### Rust 侧（已完成，可直接调用）

```rust
// 1. 选择质量档（低/中/高）→ 下一帧自动重建 PBR 通道
renderer.set_render_quality_preset(RenderQualityPreset::High);
// 或整数入口（FFI 便利）
renderer.set_render_quality_preset_u32(2);

// 2. 查询实际 draw path 状态（日志/状态栏/自检证据）
if renderer.pbr_forward_active() {
    println!("{}", renderer.pbr_draw_path_summary().unwrap());
}

// 3. 每帧渲染计划（pass 顺序 + 阴影/Bloom 资源描述）
let plan = renderer.render_plan();

// 4. 蒙皮 palette 上传（需要 PBR 质量档）
let uploaded = renderer.upload_skin_palette(&palette.to_le_bytes())?;
```

### 需要 Lead 决策/接入的部分（C# / FFI，本轮未越界）

| 事项 | 建议实现位置 | 说明 |
|---|---|---|
| C# 设置质量档 | `engine/scripts/` + `FfiContext` 新增 `renderer_set_quality_preset(int)` | 参数 `0=Low,1=Medium,2=High,3=Custom`；Rust 入口已就绪 |
| C# 查询 PBR 状态 | `FfiContext` 新增 `renderer_get_pbr_summary()` / `renderer_pbr_active()` | 返回字符串/布尔，供状态栏/自检 |
| 编辑器 UI（画质下拉） | `EditorScript.*.cs`（质量档 3 选项 + “Legacy” 回退） | 调用上述 FFI；状态栏显示 `pbr_draw_path_summary` |
| PBR 材质字段编辑 | `core` 材质组件 + `FfiContext` + Inspector | 当前 PBR 模式固定默认材质，需要 core/asset 扩展 |
| 蒙皮 draw 接入 | `core`/`asset` 提供 skinned mesh 顶点/索引 + palette，`rhi-vulkan` 复用 `SkinnedPipelineState` | draw path 已用合成 mesh 验证；真实数据接口 `upload_skin_palette` 已就绪 |
| 阴影/Bloom 调参 | “高级”面板覆盖 `set_render_settings` | `RenderSettings::validate` 会收敛非法值 |
| SPV 打包 | 安装包脚本已包含 `engine/shaders`？ | 需确认打包/CI 是否随包分发新增 `.spv` |

### 每帧数据流（已实现）

```
Scene(DirectionalLightComponent) ──┐
CameraParams + ViewportExtent ─────┤
                                   ├─ RenderContext { render_settings, ... }
RenderSettings(quality) ───────────┘
        │
        ├─ shadow_frame(light_dir) → ShadowFrameData（cascade 矩阵）
        └─ pbr_frame_ubo(light, view, proj, shadow) → PbrFrameUbo（上传 UBO）
```

## 10. 测试与验证

```bash
cd engine

# 单元测试 + shader 契约测试（GPU 测试默认 ignored）
cargo test -p hezhou-render-pipeline -p hezhou-rhi-vulkan

# 阴影贴图 GPU 冒烟（需要 Vulkan 驱动）
cargo test -p hezhou-rhi-vulkan -- --ignored create_shadow_map_on_gpu

# PBR 实际 draw path GPU 冒烟（High/Low 两档真实录制提交）
cargo test -p hezhou-rhi-vulkan --test pbr_forward_gpu -- --ignored

# 编辑器可编译（不改变现有行为）
cargo build -j 1 -p hezhou-examples --features mono
```

覆盖点：

- 材质参数：默认值/越界/NaN 收敛、金属无漫反射、背光为 0、能量非负
- BRDF：GGX 粗糙度特性、Fresnel 端点、AO/自发光
- 阴影：cascade 分割单调性/边界、光源矩阵覆盖视锥、纹素吸附稳定性、非法相机/光源报错
- 质量档：低/中/高开关与单调性、分辨率 2 的幂收敛、effective 双开关
- 后处理：tonemap 有界/单调、gamma 已知值、Bloom 阈值/合成
- Shader 契约：GLSL 字段名/offset 与 Rust `offset_of!` 一致、常量一致、`.spv` 有效、glslc 真实编译
- 蒙皮接入：`SkinnedVertexInput` 与 `hezhou_animation::SKINNED_VERTEX_ATTRIBUTES` 交叉校验、
  palette 字节数、SPIR-V 有效性

### 验证记录（2026-11-01，本机）

第一轮（task-4 能力模块）：

| 项目 | 结果 |
|---|---|
| `cargo test -p hezhou-render-pipeline` | lib 62 passed；`render_settings` 10 passed；`shader_contract` 9 passed |
| `cargo test -p hezhou-rhi-vulkan`（第一轮） | 32 passed，1 ignored（GPU）；`-- --ignored create_shadow_map_on_gpu` 实跑 **passed**（真实 Vulkan 设备，D32 3 层 2048² 创建+销毁） |

task-9（实际接入）：

| 项目 | 结果 |
|---|---|
| `cargo test -p hezhou-render-pipeline -p hezhou-rhi-vulkan` | render-pipeline 62+10+9 passed；rhi-vulkan **38 passed**，1 ignored（shadow GPU）、2 ignored（PBR GPU） |
| `cargo test -p hezhou-rhi-vulkan --test pbr_forward_gpu -- --ignored` | **2 passed**（High：Shadow→Game(PBR/HDR)→Bloom→Tonemap + skinned pipeline + 128B palette 上传；Low：无阴影/无 Bloom）——真实 Vulkan 设备提交成功 |
| `cargo build -j 1 -p hezhou-examples --features mono` | 通过（exit 0，41.37s） |
| Legacy 零回归 | 默认 `RenderSettings` 为 legacy，draw_frame 仍走原 rotation/Blinn-Phong 分支；PBR 资源完全未创建 |

task-18（完整视觉 + 蒙皮 draw + resize）：

| 项目 | 结果 |
|---|---|
| `cargo test -p hezhou-render-pipeline -p hezhou-rhi-vulkan` | render-pipeline 62+10+9 passed；rhi-vulkan **44 passed**，1 ignored（shadow GPU）、3 ignored（PBR GPU） |
| `cargo test -p hezhou-rhi-vulkan --test pbr_forward_gpu -- --ignored` | **3 passed**（真实 Vulkan 设备）：High = Shadow→Game(PBR/HDR)+天空盒+选中 outline+3D Gizmo+合成蒙皮→Bloom→Tonemap；Low = 同一套视觉元素无阴影/无 Bloom；Resize = 320×180→640×360 重建并重新录制 |
| `cargo build -j 1 -p hezhou-examples --features mono` | 通过（exit 0） |
| Legacy 零回归 | legacy 分支未改动；PBR 新增 pipeline/资源仅在 PBR 质量档创建 |

## 11. 性能风险

| 风险 | 影响 | 缓解 |
|---|---|---|
| High 档阴影 48 MiB 显存 | 集显/低端卡吃紧 | 默认 Medium（8 MiB）；质量档可关阴影 |
| 3 级 cascade 全量绘制 | draw call ×3 | 第二轮加逐级视锥剔除 |
| 3x3 PCF + 法线贴图 | 片元开销上升 | 阴影分辨率/PCF 半径可配；Low 档关阴影 |
| HDR 目标 `R16G16B16A16_SFLOAT` | 带宽/显存 ×2（1080p 约 16 MB） | 仅 PBR 模式创建；legacy 不分配 |
| Bloom 半分辨率目标 | 额外 ~4 MB（1080p） | 仅 High/Custom 开启；单级降采样 |
| Tonemap/Bloom 全屏 pass | 2~3 个全屏 pass 带宽 | 无 Bloom 时少 1 个 pass；Low 档只有 Tonemap |
| 质量档切换时旧通道延迟销毁 | 切换期间暂时多占显存 | 换档不频繁；cleanup 统一回收（规避运行期销毁驱动崩溃） |
| 天空盒/outline/Gizmo/合成蒙皮额外 draw | 每帧多 5~10 个 draw call | 数量固定且很小，相对场景实体可忽略 |
| 合成蒙皮 mesh palette 每帧上传 | 128B HOST_COHERENT 写入 | 可忽略；真实角色 palette ≤16KB/角色 |

## 12. 第二轮 TODO

1. 逐级 cascade 视锥剔除、按实体 AABB 构建光源可见集。
2. 设备格式能力查询 + HDR 格式回退（`R11G11B10F` 等）。
3. Bloom 上采样链（当前单级 prefilter + 合成，未实现多级模糊上采样）。
4. 透明/alpha test 阴影支持。
5. 阴影 golden image 回归（需要自托管 GPU runner）。
6. 真实角色蒙皮接入：asset/Player 提供 64B skinned mesh + 每帧 palette（复用
   `SkinnedPipelineState` / `upload_skin_palette`，合成 mesh 可退化为自检/关闭）。
7. per-entity PBR 材质/纹理：core 材质组件 + FfiContext + Inspector 接入。
8. PBR 模式也支持 `PipelineRegistry` 仲裁（当前 registry 启用时走 registry 管线）。

