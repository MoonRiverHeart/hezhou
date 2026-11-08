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

// === 0.2 每实体 PBR 材质（core ↔ renderer） ===

/// 设置/清除实体 PBR 材质（JSON 字符串，null/空串=清除）。
///
/// FFI 契约: `scene_set_entity_pbr_material_json(scene, entity_id, json)` → i32
/// 返回 0=成功，-1=实体无渲染组件/场景为空，-2=JSON 解析失败。
pub type SceneSetEntityPbrMaterialJsonFn =
    extern "C" fn(*mut std::ffi::c_void, u64, *const std::ffi::c_char) -> i32;

/// 读取实体 PBR 材质 JSON（无材质时写空串），返回写入字节数。
pub type SceneGetEntityPbrMaterialJsonFn =
    extern "C" fn(*const std::ffi::c_void, u64, *mut std::ffi::c_char, usize) -> usize;

/// 立即把场景中所有 PBR 材质同步到 renderer，返回 renderer 覆盖数。
///
/// FFI 契约: `editor_sync_pbr_materials_now()` → i32
pub type EditorSyncPbrMaterialsNowFn = extern "C" fn() -> i32;

// === 0.3 编辑器 Profiler（task-39） ===

/// 写入 Profiler 快照 JSON（帧时间 ring buffer/FPS/启动耗时/场景与资产统计）。
///
/// FFI 契约: `profiler_snapshot_json(buffer, buffer_size)` → usize
/// 返回写入字节数（0=失败/空缓冲区）。
pub type ProfilerSnapshotJsonFn =
    extern "C" fn(*mut std::ffi::c_char, usize) -> usize;

/// 重置帧统计（保留启动耗时）。
///
/// FFI 契约: `profiler_reset()` → i32（0=成功）
pub type ProfilerResetFn = extern "C" fn() -> i32;

/// 累计记录帧数。
///
/// FFI 契约: `profiler_frame_count()` → i32
pub type ProfilerFrameCountFn = extern "C" fn() -> i32;
