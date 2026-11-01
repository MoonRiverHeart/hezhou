use std::ffi::c_char;

// === 0.2 音频 FFI（编辑器集成 task-16） ===

/// 创建/初始化音频引擎。
///
/// FFI 契约: `audio_init()` → i32（0=成功，-1=失败；重复调用幂等）
pub type AudioInitFn = extern "C" fn() -> i32;

/// 关闭并销毁音频引擎（0=成功，-1=未初始化/失败）。
pub type AudioShutdownFn = extern "C" fn() -> i32;

/// 播放工程内第一个音频资产（WAV/OGG；无资产时播内置测试音）。
///
/// FFI 契约: `audio_play_first_asset(looping)` → i32
/// `looping`: 0=单次播放，非 0=循环；返回 0=成功，-1=音频引擎未就绪/加载失败。
pub type AudioPlayFirstAssetFn = extern "C" fn(i32) -> i32;

/// 暂停当前测试音（0=成功，-1=无活动句柄）。
pub type AudioPauseFn = extern "C" fn() -> i32;

/// 恢复当前测试音（0=成功，-1=无活动句柄）。
pub type AudioResumeFn = extern "C" fn() -> i32;

/// 停止当前测试音（0=成功，-1=无活动句柄）。
pub type AudioStopFn = extern "C" fn() -> i32;

/// 设置主音量（自动 clamp 到 [0,1]）。
///
/// FFI 契约: `audio_set_master_volume(volume)` → i32（0=成功，-1=引擎未初始化）
pub type AudioSetMasterVolumeFn = extern "C" fn(f32) -> i32;

/// 设置 3D 监听者位置（相机位置；0=成功，-1=引擎未初始化）。
pub type AudioSetListenerFn = extern "C" fn(f32, f32, f32) -> i32;

/// 读取音频状态摘要（backend/mode/silent/volume/voices 等）。
///
/// FFI 契约: `audio_get_summary(buffer, buffer_size)` → usize（写入字节数，0=不可用）
pub type AudioGetSummaryFn = extern "C" fn(*mut c_char, usize) -> usize;

// === 0.2 资产数据库 FFI（AssetDatabase） ===

/// 打开工程资产数据库并扫描（`path` 为工程根目录，空则用当前工作目录）。
///
/// FFI 契约: `asset_open_project(path_ptr)` → i32（0=成功，-1=打开/扫描失败）
pub type AssetOpenProjectFn = extern "C" fn(*const c_char) -> i32;

/// 重新扫描当前工程资产（0=成功，-1=未打开工程）
pub type AssetScanFn = extern "C" fn() -> i32;

/// 导入一个 glTF/GLB 资产（`path_ptr` 支持工程相对逻辑路径或绝对路径）。
///
/// FFI 契约: `asset_import_gltf(path_ptr)` → i32
/// - 0=成功；-1=未打开工程；-2=路径不在数据库（且无法自动登记）；-3=glTF 导入失败；-4=路径非法
/// - 未扫描的新文件若位于工程 `assets/` 内，会自动创建 `.meta` 登记后导入。
pub type AssetImportGltfFn = extern "C" fn(*const c_char) -> i32;

/// 已注册资产总数（文件资产 + 子资产；未打开工程返回 0）。
pub type AssetCountFn = extern "C" fn() -> i32;

/// 读取资产数据库摘要（project/assets/sub_assets/gltf/warnings）。
///
/// FFI 契约: `asset_get_summary(buffer, buffer_size)` → usize（写入字节数，0=不可用）
pub type AssetGetSummaryFn = extern "C" fn(*mut c_char, usize) -> usize;

/// 查询第 `index` 个资产的 AssetId 与定位符。
///
/// FFI 契约: `asset_get_asset(index, id_buf, id_len, path_buf, path_len)` → i32
/// 返回 0=成功，-1=未打开工程或索引越界；两个 buffer 均写 NUL 结尾字符串。
pub type AssetGetAssetFn =
    extern "C" fn(i32, *mut c_char, usize, *mut c_char, usize) -> i32;
