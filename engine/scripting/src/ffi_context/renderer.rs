pub type SetGamePreviewExtentFn = extern "C" fn(u32, u32);
pub type SetCameraParamsFn = extern "C" fn(f32, f32, f32, f32, f32);
pub type SetRendererGameStateFn = extern "C" fn(i32);
pub type GetRendererGameStateFn = extern "C" fn() -> i32;
pub type SetEntityTransformFn = extern "C" fn(f32, f32, f32, f32, f32, f32, f32, f32, f32, f32);
pub type SetEntityAngleFn = extern "C" fn(f32);
pub type GetEntityAngleFn = extern "C" fn() -> f32;

// === 0.2 渲染质量 / PBR（FFI ↔ 编辑器画质切换） ===

/// 切换渲染质量档
///
/// FFI 契约: `renderer_set_quality_preset(preset)` → i32
/// - `preset`: 0=Low, 1=Medium, 2=High, 3=Legacy（默认，Blinn-Phong 回退）
/// - 返回: 0=成功, -1=renderer 未初始化, -2=非法档位
pub type RendererSetQualityPresetFn = extern "C" fn(i32) -> i32;

/// 查询 PBR draw path 是否激活
///
/// FFI 契约: `renderer_pbr_active()` → i32（1=激活, 0=未激活/legacy）
pub type RendererPbrActiveFn = extern "C" fn() -> i32;

/// 读取 PBR/质量档摘要（状态栏展示）
///
/// FFI 契约: `renderer_get_pbr_summary(buffer, buffer_size)` → usize
/// 返回实际写入的字节数（不含末尾 `\0`），0 表示失败/缓冲区不可用。
pub type RendererGetPbrSummaryFn = extern "C" fn(*mut std::ffi::c_char, usize) -> usize;
