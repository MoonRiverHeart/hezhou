//! Player C# Scene Context ABI（task-32 v1 / task-33 v2 / task-34 v3 / task-35 v4）：
//! 把 `hezhou_core::Scene`、Player 动画运行时、资产目录与音频运行时以
//! `#[repr(C)]` 函数指针表暴露给 C# `Hezhou.PlayerGame`。
//!
//! # ABI 契约（C# `[StructLayout(LayoutKind.Sequential)]` 必须一致）
//!
//! v1 基础字段（顺序/偏移不变）：
//!
//! | offset | 字段 | C# 类型 |
//! |---|---|---|
//! | 0  | version: u32 | uint（当前 [`PLAYER_CSHARP_CONTEXT_VERSION`] = 2） |
//! | 4  | reserved: u32 | uint（始终 0） |
//! | 8  | scene: *mut Scene | IntPtr |
//! | 16 | scene_entity_count | IntPtr → delegate `uint(IntPtr)` |
//! | 24 | scene_create_cube | IntPtr → delegate `ulong(IntPtr)` |
//! | 32 | scene_create_entity | IntPtr → delegate `ulong(IntPtr)` |
//! | 40 | scene_set_entity_position | IntPtr → delegate `int(IntPtr, ulong, float, float, float)` |
//! | 48 | scene_get_entity_position | IntPtr → delegate `int(IntPtr, ulong, out float, out float, out float)` |
//! | 56 | log | IntPtr → delegate `void(IntPtr)`（可选；C 字符串） |
//!
//! v2 tail 追加（task-33；保持 v1 字段顺序，旧 v1 读取方忽略尾部）：
//!
//! | offset | 字段 | C# 类型 |
//! |---|---|---|
//! | 64 | scene_set_entity_rotation | `int(IntPtr, ulong, float x, float y, float z)`（欧拉角，度） |
//! | 72 | scene_get_entity_rotation | `int(IntPtr, ulong, out float, out float, out float)`（欧拉角，度） |
//! | 80 | scene_set_entity_scale | `int(IntPtr, ulong, float, float, float)` |
//! | 88 | scene_get_entity_scale | `int(IntPtr, ulong, out float, out float, out float)` |
//! | 96 | scene_entity_exists | `int(IntPtr, ulong)`（1 存在 / 0 不存在） |
//! | 104 | scene_destroy_entity | `int(IntPtr, ulong)`（0 成功 / -1 失败） |
//!
//! v3 tail 追加（task-34；保持 v1/v2 布局；`animation_state` 指向 host 栈上的
//! [`crate::animation_control::PlayerAnimationState`]）：
//!
//! | offset | 字段 | 类型 / delegate |
//! |---|---|---|
//! | 112 | `animation_state` | `IntPtr`（可为 0 = 无动画） |
//! | 120 | `animation_clip_count` | `uint(IntPtr state)` |
//! | 128 | `animation_clip_name` | `int(IntPtr, uint index, u8* buf, uint len)`（返回所需/写入字节数） |
//! | 136 | `animation_clip_duration` | `float(IntPtr, uint index)`（失败 -1） |
//! | 144 | `animation_current` | `int(IntPtr)`（当前 clip 下标，无 -1） |
//! | 152 | `animation_play` | `int(IntPtr, uint index)`（0/-1） |
//! | 160 | `animation_set_time` | `int(IntPtr, float seconds)`（0/-1） |
//! | 168 | `animation_get_time` | `float(IntPtr)`（失败 NaN） |
//! | 176 | `animation_set_speed` | `int(IntPtr, float speed)`（0/-1） |
//! | 184 | `animation_get_speed` | `float(IntPtr)`（失败 NaN） |
//! | 192 | `animation_set_playing` | `int(IntPtr, int playing)`（0/-1） |
//!
//! v4 tail 追加（task-35；资产目录 + 音频运行时）：
//!
//! | offset | 字段 | 类型 / delegate |
//! |---|---|---|
//! | 200 | `assets` | `IntPtr` → `PlayerAssetCatalog`（可空） |
//! | 208 | `asset_count` | `uint(IntPtr assets)` |
//! | 216 | `asset_id` | `int(IntPtr, uint index, u8* buf, uint len)`（UTF-8 UUID） |
//! | 224 | `asset_kind` | `int(IntPtr, uint index, u8* buf, uint len)`（如 `scene`/`audio`） |
//! | 232 | `asset_path` | `int(IntPtr, uint index, u8* buf, uint len)`（包内 output） |
//! | 240 | `audio` | `IntPtr` → `PlayerAudioRuntime`（可空） |
//! | 248 | `audio_play_first` | `int(IntPtr)`（0/-1） |
//! | 256 | `audio_pause` | `int(IntPtr)`（0/-1） |
//! | 264 | `audio_resume` | `int(IntPtr)`（0/-1） |
//! | 272 | `audio_stop` | `int(IntPtr)`（0/-1） |
//! | 280 | `audio_set_volume` | `int(IntPtr, float volume)`（0/-1） |
//! | 288 | `audio_is_playing` | `int(IntPtr)`（1/0） |
//! | 296 | `audio_backend` | `int(IntPtr, u32 unused, u8* buf, uint len)`（UTF-8，如 `null`） |
//!
//! 所有函数均为 `extern "C"`、内部 `catch_unwind`；`scene == null`、实体不存在、
//! 输出指针为空时返回 0/-1 且绝不 panic（panic 跨 FFI 是 UB）。rotation 使用
//! **欧拉角（度）**，与 core `Quaternion::from_euler_degrees`/`to_euler_degrees` 对应。
//!
//! # 版本兼容策略
//!
//! v2/v3/v4 只在尾部追加字段、不改旧布局；C# 侧读取 `Version`：
//! - `>= 4`：解析全部字段（资产/音频 API 可用）；
//! - `== 3`：解析 v1/v2/v3，资产/音频跳过；
//! - `== 2`：解析 v1+v2，动画/资产/音频跳过；
//! - `== 1`：只解析 v1（兼容旧 Player/DLL）；
//! - 其他：记录日志后回退空转。
//!
//! # 生命周期
//!
//! `PlayerCSharpContext` 由 `WindowPlayerHost` 在栈上创建，覆盖整个帧循环；
//! C# 不得在 `Shutdown()` 之后继续持有/调用 `scene` 指针。context 本身按值传给
//! C# `Initialize(IntPtr context)`，C# 必须复制结构体内容，不能保存指向 context
//! 栈内存的指针。

use std::ffi::CStr;
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

use hezhou_core::ecs::{Entity, Scene};
use hezhou_core::math::{Quaternion, Vec3};

use crate::animation_control::PlayerAnimationState;
use crate::asset_catalog::{PlayerAssetCatalog, PlayerAssetEntry};
use crate::audio_runtime::PlayerAudioRuntime;

/// 当前 Player C# Context ABI 版本。
pub const PLAYER_CSHARP_CONTEXT_VERSION: u32 = 4;

/// C# 侧函数指针：`scene_entity_count(scene) -> u32`。
pub type SceneEntityCountFn = unsafe extern "C" fn(*mut Scene) -> u32;
/// C# 侧函数指针：`scene_create_cube(scene) -> entity_id`。
pub type SceneCreateCubeFn = unsafe extern "C" fn(*mut Scene) -> u64;
/// C# 侧函数指针：`scene_create_entity(scene) -> entity_id`。
pub type SceneCreateEntityFn = unsafe extern "C" fn(*mut Scene) -> u64;
/// C# 侧函数指针：`scene_set_entity_position(scene, entity, x, y, z) -> 0/-1`。
pub type SceneSetEntityPositionFn =
    unsafe extern "C" fn(*mut Scene, u64, f32, f32, f32) -> i32;
/// C# 侧函数指针：`scene_get_entity_position(scene, entity, out x, out y, out z) -> 0/-1`。
pub type SceneGetEntityPositionFn =
    unsafe extern "C" fn(*mut Scene, u64, *mut f32, *mut f32, *mut f32) -> i32;
/// C# 侧函数指针：`log(ptr)`（C 字符串，可选）。
pub type PlayerContextLogFn = unsafe extern "C" fn(*const c_char);
/// C# 侧函数指针：`scene_set_entity_rotation(scene, entity, x, y, z) -> 0/-1`
/// （欧拉角，度）。
pub type SceneSetEntityRotationFn =
    unsafe extern "C" fn(*mut Scene, u64, f32, f32, f32) -> i32;
/// C# 侧函数指针：`scene_get_entity_rotation(scene, entity, out x, out y, out z) -> 0/-1`
/// （欧拉角，度）。
pub type SceneGetEntityRotationFn =
    unsafe extern "C" fn(*mut Scene, u64, *mut f32, *mut f32, *mut f32) -> i32;
/// C# 侧函数指针：`scene_set_entity_scale(scene, entity, x, y, z) -> 0/-1`。
pub type SceneSetEntityScaleFn = unsafe extern "C" fn(*mut Scene, u64, f32, f32, f32) -> i32;
/// C# 侧函数指针：`scene_get_entity_scale(scene, entity, out x, out y, out z) -> 0/-1`。
pub type SceneGetEntityScaleFn =
    unsafe extern "C" fn(*mut Scene, u64, *mut f32, *mut f32, *mut f32) -> i32;
/// C# 侧函数指针：`scene_entity_exists(scene, entity) -> 1/0`。
pub type SceneEntityExistsFn = unsafe extern "C" fn(*mut Scene, u64) -> i32;
/// C# 侧函数指针：`scene_destroy_entity(scene, entity) -> 0/-1`。
pub type SceneDestroyEntityFn = unsafe extern "C" fn(*mut Scene, u64) -> i32;

// ==== v3：蒙皮动画运行时（task-34）====
/// C# 侧函数指针：`animation_clip_count(state) -> u32`。
pub type AnimationClipCountFn = unsafe extern "C" fn(*mut PlayerAnimationState) -> u32;
/// C# 侧函数指针：`animation_clip_name(state, index, buf, len) -> i32`。
pub type AnimationClipNameFn =
    unsafe extern "C" fn(*mut PlayerAnimationState, u32, *mut u8, u32) -> i32;
/// C# 侧函数指针：`animation_clip_duration(state, index) -> f32`。
pub type AnimationClipDurationFn = unsafe extern "C" fn(*mut PlayerAnimationState, u32) -> f32;
/// C# 侧函数指针：`animation_current(state) -> i32`。
pub type AnimationCurrentFn = unsafe extern "C" fn(*mut PlayerAnimationState) -> i32;
/// C# 侧函数指针：`animation_play(state, index) -> 0/-1`。
pub type AnimationPlayFn = unsafe extern "C" fn(*mut PlayerAnimationState, u32) -> i32;
/// C# 侧函数指针：`animation_set_time(state, seconds) -> 0/-1`。
pub type AnimationSetTimeFn = unsafe extern "C" fn(*mut PlayerAnimationState, f32) -> i32;
/// C# 侧函数指针：`animation_get_time(state) -> f32`。
pub type AnimationGetTimeFn = unsafe extern "C" fn(*mut PlayerAnimationState) -> f32;
/// C# 侧函数指针：`animation_set_speed(state, speed) -> 0/-1`。
pub type AnimationSetSpeedFn = unsafe extern "C" fn(*mut PlayerAnimationState, f32) -> i32;
/// C# 侧函数指针：`animation_get_speed(state) -> f32`。
pub type AnimationGetSpeedFn = unsafe extern "C" fn(*mut PlayerAnimationState) -> f32;
/// C# 侧函数指针：`animation_set_playing(state, playing) -> 0/-1`。
pub type AnimationSetPlayingFn = unsafe extern "C" fn(*mut PlayerAnimationState, i32) -> i32;

// ==== v4：资产目录 + 音频运行时（task-35）====
/// C# 侧函数指针：`asset_count(assets) -> u32`。
pub type AssetCountFn = unsafe extern "C" fn(*mut PlayerAssetCatalog) -> u32;
/// C# 侧函数指针：`asset_id/asset_kind/asset_path(assets, index, buf, len) -> i32`。
pub type AssetStringFn =
    unsafe extern "C" fn(*mut PlayerAssetCatalog, u32, *mut u8, u32) -> i32;
/// C# 侧函数指针：`audio_play_first(audio) -> 0/-1`。
pub type AudioPlayFirstFn = unsafe extern "C" fn(*mut PlayerAudioRuntime) -> i32;
/// C# 侧函数指针：`audio_pause(audio) -> 0/-1`。
pub type AudioPauseFn = unsafe extern "C" fn(*mut PlayerAudioRuntime) -> i32;
/// C# 侧函数指针：`audio_resume(audio) -> 0/-1`。
pub type AudioResumeFn = unsafe extern "C" fn(*mut PlayerAudioRuntime) -> i32;
/// C# 侧函数指针：`audio_stop(audio) -> 0/-1`。
pub type AudioStopFn = unsafe extern "C" fn(*mut PlayerAudioRuntime) -> i32;
/// C# 侧函数指针：`audio_set_volume(audio, volume) -> 0/-1`。
pub type AudioSetVolumeFn = unsafe extern "C" fn(*mut PlayerAudioRuntime, f32) -> i32;
/// C# 侧函数指针：`audio_is_playing(audio) -> 1/0`。
pub type AudioIsPlayingFn = unsafe extern "C" fn(*mut PlayerAudioRuntime) -> i32;
/// C# 侧函数指针：`audio_backend(audio, unused, buf, len) -> i32`（UTF-8）。
pub type AudioBackendFn = unsafe extern "C" fn(*mut PlayerAudioRuntime, u32, *mut u8, u32) -> i32;

/// `#[repr(C)]` Player C# Context：版本 + Scene 指针 + 6 个函数指针。
#[repr(C)]
pub struct PlayerCSharpContext {
    /// ABI 版本（[`PLAYER_CSHARP_CONTEXT_VERSION`]）。
    pub version: u32,
    /// 保留（0），保证 scene 指针 8 字节对齐。
    pub reserved: u32,
    /// `hezhou_core::ecs::Scene` 裸指针（host 栈上场景，生命周期覆盖帧循环）。
    pub scene: *mut Scene,
    /// 实体数量。
    pub scene_entity_count: Option<SceneEntityCountFn>,
    /// 创建内置 cube，返回实体 id（失败 0）。
    pub scene_create_cube: Option<SceneCreateCubeFn>,
    /// 创建空实体，返回实体 id（失败 0）。
    pub scene_create_entity: Option<SceneCreateEntityFn>,
    /// 设置实体位置，返回 0 成功 / -1 失败。
    pub scene_set_entity_position: Option<SceneSetEntityPositionFn>,
    /// 读取实体位置，返回 0 成功 / -1 失败。
    pub scene_get_entity_position: Option<SceneGetEntityPositionFn>,
    /// 可选日志函数（C 字符串）。
    pub log: Option<PlayerContextLogFn>,
    // ==== v2 tail 追加（task-33）：保持 v1 布局不变 ====
    /// 设置实体 rotation（欧拉角，度），返回 0 成功 / -1 失败。
    pub scene_set_entity_rotation: Option<SceneSetEntityRotationFn>,
    /// 读取实体 rotation（欧拉角，度），返回 0 成功 / -1 失败。
    pub scene_get_entity_rotation: Option<SceneGetEntityRotationFn>,
    /// 设置实体 scale，返回 0 成功 / -1 失败。
    pub scene_set_entity_scale: Option<SceneSetEntityScaleFn>,
    /// 读取实体 scale，返回 0 成功 / -1 失败。
    pub scene_get_entity_scale: Option<SceneGetEntityScaleFn>,
    /// 实体是否存在：1 存在 / 0 不存在。
    pub scene_entity_exists: Option<SceneEntityExistsFn>,
    /// 销毁实体：0 成功 / -1 失败。
    pub scene_destroy_entity: Option<SceneDestroyEntityFn>,
    // ==== v3 tail 追加（task-34）：蒙皮动画运行时 ====
    /// 动画运行时状态指针（host 栈上；0 = 无动画）。
    pub animation_state: *mut PlayerAnimationState,
    /// clip 数量。
    pub animation_clip_count: Option<AnimationClipCountFn>,
    /// clip 名（UTF-8 写入调用方 buffer）。
    pub animation_clip_name: Option<AnimationClipNameFn>,
    /// clip 时长；失败 -1。
    pub animation_clip_duration: Option<AnimationClipDurationFn>,
    /// 当前 clip 下标；无 -1。
    pub animation_current: Option<AnimationCurrentFn>,
    /// 播放指定 clip。
    pub animation_play: Option<AnimationPlayFn>,
    /// 设置当前时间。
    pub animation_set_time: Option<AnimationSetTimeFn>,
    /// 读取当前时间；失败 NaN。
    pub animation_get_time: Option<AnimationGetTimeFn>,
    /// 设置播放速度。
    pub animation_set_speed: Option<AnimationSetSpeedFn>,
    /// 读取播放速度；失败 NaN。
    pub animation_get_speed: Option<AnimationGetSpeedFn>,
    /// 设置播放/暂停。
    pub animation_set_playing: Option<AnimationSetPlayingFn>,
    // ==== v4 tail 追加（task-35）：资产目录 + 音频运行时 ====
    /// 资产目录指针（host 栈上；可空）。
    pub assets: *mut PlayerAssetCatalog,
    /// 资产数量。
    pub asset_count: Option<AssetCountFn>,
    /// 资产 AssetId（UTF-8 UUID）。
    pub asset_id: Option<AssetStringFn>,
    /// 资产 kind（如 `scene`/`audio`）。
    pub asset_kind: Option<AssetStringFn>,
    /// 资产包内 output 路径。
    pub asset_path: Option<AssetStringFn>,
    /// 音频运行时指针（host 栈上；可空）。
    pub audio: *mut PlayerAudioRuntime,
    /// 播放第一个音频。
    pub audio_play_first: Option<AudioPlayFirstFn>,
    /// 暂停音频。
    pub audio_pause: Option<AudioPauseFn>,
    /// 恢复音频。
    pub audio_resume: Option<AudioResumeFn>,
    /// 停止音频。
    pub audio_stop: Option<AudioStopFn>,
    /// 设置音频主音量。
    pub audio_set_volume: Option<AudioSetVolumeFn>,
    /// 音频是否在播放。
    pub audio_is_playing: Option<AudioIsPlayingFn>,
    /// 音频后端名称（UTF-8，如 `null`/`cpal`）。
    pub audio_backend: Option<AudioBackendFn>,
}

impl PlayerCSharpContext {
    /// 为场景创建 context；`scene` 必须在本 context 使用期间保持有效。
    /// 动画 API 默认不可用（`animation_state = null`）。
    pub fn new(scene: *mut Scene) -> Self {
        Self::new_with_animation(scene, std::ptr::null_mut())
    }

    /// 为场景 + 动画运行时状态创建 context（v3）。
    ///
    /// `animation_state` 必须在本 context 使用期间保持有效（host 栈上状态）。
    pub fn new_with_animation(
        scene: *mut Scene,
        animation_state: *mut PlayerAnimationState,
    ) -> Self {
        Self::new_with_runtime(
            scene,
            animation_state,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    }

    /// 为场景 + 动画 + 资产目录 + 音频运行时创建 context（v4）。
    ///
    /// 指针必须在本 context 使用期间保持有效（host 栈上状态）；可为 null（功能关闭）。
    pub fn new_with_runtime(
        scene: *mut Scene,
        animation_state: *mut PlayerAnimationState,
        assets: *mut PlayerAssetCatalog,
        audio: *mut PlayerAudioRuntime,
    ) -> Self {
        Self {
            version: PLAYER_CSHARP_CONTEXT_VERSION,
            reserved: 0,
            scene,
            scene_entity_count: Some(scene_entity_count),
            scene_create_cube: Some(scene_create_cube),
            scene_create_entity: Some(scene_create_entity),
            scene_set_entity_position: Some(scene_set_entity_position),
            scene_get_entity_position: Some(scene_get_entity_position),
            log: Some(player_context_log),
            scene_set_entity_rotation: Some(scene_set_entity_rotation),
            scene_get_entity_rotation: Some(scene_get_entity_rotation),
            scene_set_entity_scale: Some(scene_set_entity_scale),
            scene_get_entity_scale: Some(scene_get_entity_scale),
            scene_entity_exists: Some(scene_entity_exists),
            scene_destroy_entity: Some(scene_destroy_entity),
            animation_state,
            animation_clip_count: Some(animation_clip_count),
            animation_clip_name: Some(animation_clip_name),
            animation_clip_duration: Some(animation_clip_duration),
            animation_current: Some(animation_current),
            animation_play: Some(animation_play),
            animation_set_time: Some(animation_set_time),
            animation_get_time: Some(animation_get_time),
            animation_set_speed: Some(animation_set_speed),
            animation_get_speed: Some(animation_get_speed),
            animation_set_playing: Some(animation_set_playing),
            assets,
            asset_count: Some(asset_count),
            asset_id: Some(asset_id),
            asset_kind: Some(asset_kind),
            asset_path: Some(asset_path),
            audio,
            audio_play_first: Some(audio_play_first),
            audio_pause: Some(audio_pause),
            audio_resume: Some(audio_resume),
            audio_stop: Some(audio_stop),
            audio_set_volume: Some(audio_set_volume),
            audio_is_playing: Some(audio_is_playing),
            audio_backend: Some(audio_backend),
        }
    }

    /// `context` 的裸指针（传给 C# `Initialize(IntPtr)`）。
    pub fn as_ptr(&self) -> *const Self {
        self as *const Self
    }

    /// 版本校验（C# 侧同样校验）。
    pub fn is_supported(&self) -> bool {
        self.version == PLAYER_CSHARP_CONTEXT_VERSION && !self.scene.is_null()
    }
}

impl std::fmt::Debug for PlayerCSharpContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerCSharpContext")
            .field("version", &self.version)
            .field("scene", &self.scene)
            .finish()
    }
}

/// 在 `catch_unwind` + 空指针保护下访问 Scene。
fn with_scene<R>(scene: *mut Scene, default: R, f: impl FnOnce(&mut Scene) -> R) -> R {
    if scene.is_null() {
        return default;
    }
    match catch_unwind(AssertUnwindSafe(|| f(unsafe { &mut *scene }))) {
        Ok(value) => value,
        Err(_) => default,
    }
}

fn find_entity(scene: &Scene, entity_id: u64) -> Option<Entity> {
    scene
        .get_all_entities()
        .into_iter()
        .find(|entity| entity.id == entity_id)
}

/// `scene_entity_count(scene)`：实体数量（空指针/panic → 0）。
pub unsafe extern "C" fn scene_entity_count(scene: *mut Scene) -> u32 {
    with_scene(scene, 0, |scene| {
        scene.entity_count().min(u32::MAX as usize) as u32
    })
}

/// `scene_create_cube(scene)`：创建内置 cube，返回实体 id（失败 0）。
pub unsafe extern "C" fn scene_create_cube(scene: *mut Scene) -> u64 {
    with_scene(scene, 0, |scene| scene.create_cube().id)
}

/// `scene_create_entity(scene)`：创建空实体，返回实体 id（失败 0）。
pub unsafe extern "C" fn scene_create_entity(scene: *mut Scene) -> u64 {
    with_scene(scene, 0, |scene| scene.create_entity().id)
}

/// `scene_set_entity_position(scene, entity, x, y, z)`：0 成功 / -1 失败。
pub unsafe extern "C" fn scene_set_entity_position(
    scene: *mut Scene,
    entity_id: u64,
    x: f32,
    y: f32,
    z: f32,
) -> i32 {
    with_scene(scene, -1, |scene| {
        let Some(entity) = find_entity(scene, entity_id) else {
            return -1;
        };
        scene.set_entity_position(entity, Vec3::new(x, y, z));
        0
    })
}

/// `scene_get_entity_position(scene, entity, out x, out y, out z)`：0 成功 / -1 失败。
///
/// # Safety
/// `out_x/out_y/out_z` 必须是可写的 `f32` 指针（C# `out float` 满足）；空指针返回 -1。
pub unsafe extern "C" fn scene_get_entity_position(
    scene: *mut Scene,
    entity_id: u64,
    out_x: *mut f32,
    out_y: *mut f32,
    out_z: *mut f32,
) -> i32 {
    if out_x.is_null() || out_y.is_null() || out_z.is_null() {
        return -1;
    }
    with_scene(scene, -1, |scene| {
        let Some(entity) = find_entity(scene, entity_id) else {
            return -1;
        };
        let Some(position) = scene.get_entity_position(entity) else {
            return -1;
        };
        unsafe {
            *out_x = position.x;
            *out_y = position.y;
            *out_z = position.z;
        }
        0
    })
}

/// `scene_set_entity_rotation(scene, entity, x, y, z)`：设置欧拉角（度），0 成功 / -1 失败。
pub unsafe extern "C" fn scene_set_entity_rotation(
    scene: *mut Scene,
    entity_id: u64,
    x: f32,
    y: f32,
    z: f32,
) -> i32 {
    with_scene(scene, -1, |scene| {
        let Some(entity) = find_entity(scene, entity_id) else {
            return -1;
        };
        scene.set_entity_rotation(entity, Quaternion::from_euler_degrees(x, y, z));
        0
    })
}

/// `scene_get_entity_rotation(scene, entity, out x, out y, out z)`：读取欧拉角（度）。
///
/// # Safety
/// `out_x/out_y/out_z` 必须是可写的 `f32` 指针（C# `out float` 满足）；空指针返回 -1。
pub unsafe extern "C" fn scene_get_entity_rotation(
    scene: *mut Scene,
    entity_id: u64,
    out_x: *mut f32,
    out_y: *mut f32,
    out_z: *mut f32,
) -> i32 {
    if out_x.is_null() || out_y.is_null() || out_z.is_null() {
        return -1;
    }
    with_scene(scene, -1, |scene| {
        let Some(entity) = find_entity(scene, entity_id) else {
            return -1;
        };
        let Some(rotation) = scene.get_entity_rotation(entity) else {
            return -1;
        };
        let euler = rotation.to_euler_degrees();
        unsafe {
            *out_x = euler[0];
            *out_y = euler[1];
            *out_z = euler[2];
        }
        0
    })
}

/// `scene_set_entity_scale(scene, entity, x, y, z)`：0 成功 / -1 失败。
pub unsafe extern "C" fn scene_set_entity_scale(
    scene: *mut Scene,
    entity_id: u64,
    x: f32,
    y: f32,
    z: f32,
) -> i32 {
    with_scene(scene, -1, |scene| {
        let Some(entity) = find_entity(scene, entity_id) else {
            return -1;
        };
        scene.set_entity_scale(entity, Vec3::new(x, y, z));
        0
    })
}

/// `scene_get_entity_scale(scene, entity, out x, out y, out z)`：0 成功 / -1 失败。
///
/// # Safety
/// `out_x/out_y/out_z` 必须是可写的 `f32` 指针（C# `out float` 满足）；空指针返回 -1。
pub unsafe extern "C" fn scene_get_entity_scale(
    scene: *mut Scene,
    entity_id: u64,
    out_x: *mut f32,
    out_y: *mut f32,
    out_z: *mut f32,
) -> i32 {
    if out_x.is_null() || out_y.is_null() || out_z.is_null() {
        return -1;
    }
    with_scene(scene, -1, |scene| {
        let Some(entity) = find_entity(scene, entity_id) else {
            return -1;
        };
        let Some(scale) = scene.get_entity_scale(entity) else {
            return -1;
        };
        unsafe {
            *out_x = scale.x;
            *out_y = scale.y;
            *out_z = scale.z;
        }
        0
    })
}

/// `scene_entity_exists(scene, entity)`：1 存在 / 0 不存在（空指针/panic → 0）。
pub unsafe extern "C" fn scene_entity_exists(scene: *mut Scene, entity_id: u64) -> i32 {
    with_scene(scene, 0, |scene| {
        i32::from(find_entity(scene, entity_id).is_some())
    })
}

/// `scene_destroy_entity(scene, entity)`：销毁实体，0 成功 / -1 失败（不存在/空指针）。
pub unsafe extern "C" fn scene_destroy_entity(scene: *mut Scene, entity_id: u64) -> i32 {
    with_scene(scene, -1, |scene| {
        let Some(entity) = find_entity(scene, entity_id) else {
            return -1;
        };
        scene.remove_entity(entity);
        0
    })
}

/// 在 `catch_unwind` + 空指针保护下访问动画运行时状态。
fn with_animation<R>(
    state: *mut PlayerAnimationState,
    default: R,
    f: impl FnOnce(&mut PlayerAnimationState) -> R,
) -> R {
    if state.is_null() {
        return default;
    }
    match catch_unwind(AssertUnwindSafe(|| f(unsafe { &mut *state }))) {
        Ok(value) => value,
        Err(_) => default,
    }
}

/// `animation_clip_count(state)`：clip 数量（空状态/panic → 0）。
pub unsafe extern "C" fn animation_clip_count(state: *mut PlayerAnimationState) -> u32 {
    with_animation(state, 0, |state| state.clip_count().min(u32::MAX as usize) as u32)
}

/// `animation_clip_name(state, index, buf, len) -> i32`：UTF-8 clip 名。
///
/// - `buf == null || len == 0`：返回所需字节数（不写）；
/// - 否则写入最多 `len` 字节，返回写入字节数；调用方可用返回值判断截断；
/// - 无 clip/越界 → -1。
pub unsafe extern "C" fn animation_clip_name(
    state: *mut PlayerAnimationState,
    index: u32,
    buf: *mut u8,
    len: u32,
) -> i32 {
    with_animation(state, -1, |state| {
        let Some(name) = state.clip_name(index as usize) else {
            return -1;
        };
        let bytes = name.unwrap_or("").as_bytes();
        if buf.is_null() || len == 0 {
            return bytes.len().min(i32::MAX as usize) as i32;
        }
        let written = bytes.len().min(len as usize);
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, written);
        }
        written as i32
    })
}

/// `animation_clip_duration(state, index)`：clip 时长（秒）；无效/越界 → -1。
pub unsafe extern "C" fn animation_clip_duration(
    state: *mut PlayerAnimationState,
    index: u32,
) -> f32 {
    with_animation(state, -1.0, |state| {
        state.clip_duration(index as usize).unwrap_or(-1.0)
    })
}

/// `animation_current(state)`：当前 clip 下标；无 → -1。
pub unsafe extern "C" fn animation_current(state: *mut PlayerAnimationState) -> i32 {
    with_animation(state, -1, |state| state.current())
}

/// `animation_play(state, index)`：播放指定 clip；0 成功 / -1 失败。
pub unsafe extern "C" fn animation_play(
    state: *mut PlayerAnimationState,
    index: u32,
) -> i32 {
    with_animation(state, -1, |state| {
        if state.play(index as usize).is_ok() {
            0
        } else {
            -1
        }
    })
}

/// `animation_set_time(state, seconds)`：设置当前时间；0 成功 / -1 失败。
pub unsafe extern "C" fn animation_set_time(
    state: *mut PlayerAnimationState,
    seconds: f32,
) -> i32 {
    with_animation(state, -1, |state| {
        if state.set_time(seconds).is_ok() {
            0
        } else {
            -1
        }
    })
}

/// `animation_get_time(state)`：当前时间（秒）；无效/panic → NaN。
pub unsafe extern "C" fn animation_get_time(state: *mut PlayerAnimationState) -> f32 {
    with_animation(state, f32::NAN, |state| state.time())
}

/// `animation_set_speed(state, speed)`：设置播放速度；0 成功 / -1 失败。
pub unsafe extern "C" fn animation_set_speed(
    state: *mut PlayerAnimationState,
    speed: f32,
) -> i32 {
    with_animation(state, -1, |state| {
        if state.set_speed(speed).is_ok() {
            0
        } else {
            -1
        }
    })
}

/// `animation_get_speed(state)`：播放速度；无效/panic → NaN。
pub unsafe extern "C" fn animation_get_speed(state: *mut PlayerAnimationState) -> f32 {
    with_animation(state, f32::NAN, |state| state.speed())
}

/// `animation_set_playing(state, playing)`：播放/暂停；0 成功 / -1 失败。
pub unsafe extern "C" fn animation_set_playing(
    state: *mut PlayerAnimationState,
    playing: i32,
) -> i32 {
    with_animation(state, -1, |state| {
        state.set_playing(playing != 0);
        0
    })
}

/// 在 `catch_unwind` + 空指针保护下访问资产目录。
fn with_catalog<R>(
    catalog: *mut PlayerAssetCatalog,
    default: R,
    f: impl FnOnce(&mut PlayerAssetCatalog) -> R,
) -> R {
    if catalog.is_null() {
        return default;
    }
    match catch_unwind(AssertUnwindSafe(|| f(unsafe { &mut *catalog }))) {
        Ok(value) => value,
        Err(_) => default,
    }
}

/// 在 `catch_unwind` + 空指针保护下访问音频运行时。
fn with_audio<R>(
    audio: *mut PlayerAudioRuntime,
    default: R,
    f: impl FnOnce(&mut PlayerAudioRuntime) -> R,
) -> R {
    if audio.is_null() {
        return default;
    }
    match catch_unwind(AssertUnwindSafe(|| f(unsafe { &mut *audio }))) {
        Ok(value) => value,
        Err(_) => default,
    }
}

/// 把 UTF-8 文本写入调用方 buffer；`buf=null/len=0` 时返回所需字节数。
fn write_utf8(text: &str, buf: *mut u8, len: u32) -> i32 {
    let bytes = text.as_bytes();
    if buf.is_null() || len == 0 {
        return bytes.len().min(i32::MAX as usize) as i32;
    }
    let written = bytes.len().min(len as usize);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, written);
    }
    written as i32
}

/// `asset_count(assets)`：资产数量（空/panic → 0）。
pub unsafe extern "C" fn asset_count(assets: *mut PlayerAssetCatalog) -> u32 {
    with_catalog(assets, 0, |assets| assets.len().min(u32::MAX as usize) as u32)
}

fn asset_text(
    assets: *mut PlayerAssetCatalog,
    index: u32,
    buf: *mut u8,
    len: u32,
    select: fn(&PlayerAssetEntry) -> String,
) -> i32 {
    with_catalog(assets, -1, |assets| {
        let Some(entry) = assets.entry(index as usize) else {
            return -1;
        };
        write_utf8(&select(entry), buf, len)
    })
}

/// `asset_id(assets, index, buf, len)`：AssetId UUID（UTF-8）；越界 -1。
pub unsafe extern "C" fn asset_id(
    assets: *mut PlayerAssetCatalog,
    index: u32,
    buf: *mut u8,
    len: u32,
) -> i32 {
    asset_text(assets, index, buf, len, |entry| entry.id.to_string())
}

/// `asset_kind(assets, index, buf, len)`：资产 kind（UTF-8，如 `scene`/`audio`）。
pub unsafe extern "C" fn asset_kind(
    assets: *mut PlayerAssetCatalog,
    index: u32,
    buf: *mut u8,
    len: u32,
) -> i32 {
    asset_text(assets, index, buf, len, |entry| {
        entry.kind.as_str().to_string()
    })
}

/// `asset_path(assets, index, buf, len)`：包内相对 `assets/` 的 output 路径。
pub unsafe extern "C" fn asset_path(
    assets: *mut PlayerAssetCatalog,
    index: u32,
    buf: *mut u8,
    len: u32,
) -> i32 {
    asset_text(assets, index, buf, len, |entry| entry.output.clone())
}

/// `audio_play_first(audio)`：播放第一个音频；0 成功 / -1 失败。
pub unsafe extern "C" fn audio_play_first(audio: *mut PlayerAudioRuntime) -> i32 {
    with_audio(audio, -1, |audio| if audio.play_first() { 0 } else { -1 })
}

/// `audio_pause(audio)`：暂停；0 成功 / -1 失败。
pub unsafe extern "C" fn audio_pause(audio: *mut PlayerAudioRuntime) -> i32 {
    with_audio(audio, -1, |audio| if audio.pause() { 0 } else { -1 })
}

/// `audio_resume(audio)`：恢复；0 成功 / -1 失败。
pub unsafe extern "C" fn audio_resume(audio: *mut PlayerAudioRuntime) -> i32 {
    with_audio(audio, -1, |audio| if audio.resume() { 0 } else { -1 })
}

/// `audio_stop(audio)`：停止；0 成功 / -1 失败。
pub unsafe extern "C" fn audio_stop(audio: *mut PlayerAudioRuntime) -> i32 {
    with_audio(audio, -1, |audio| if audio.stop() { 0 } else { -1 })
}

/// `audio_set_volume(audio, volume)`：设置主音量；0 成功 / -1 失败。
pub unsafe extern "C" fn audio_set_volume(
    audio: *mut PlayerAudioRuntime,
    volume: f32,
) -> i32 {
    with_audio(audio, -1, |audio| {
        audio.set_volume(volume);
        0
    })
}

/// `audio_is_playing(audio)`：1 播放中 / 0 其他。
pub unsafe extern "C" fn audio_is_playing(audio: *mut PlayerAudioRuntime) -> i32 {
    with_audio(audio, 0, |audio| i32::from(audio.is_playing()))
}

/// `audio_backend(audio, unused, buf, len)`：后端名称（UTF-8，如 `null`/`cpal`）。
pub unsafe extern "C" fn audio_backend(
    audio: *mut PlayerAudioRuntime,
    _unused: u32,
    buf: *mut u8,
    len: u32,
) -> i32 {
    with_audio(audio, -1, |audio| {
        write_utf8(&audio.backend(), buf, len)
    })
}

/// `log(ptr)`：把 C 字符串写到 stderr（Player 日志/控制台）；空指针/panic 安全。
///
/// 该函数仅用于 C# 侧需要把消息回传 Rust 日志时；C# 也可以直接 Console.Error。
pub unsafe extern "C" fn player_context_log(message: *const c_char) {
    if message.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let text = unsafe { CStr::from_ptr(message) }.to_string_lossy();
        eprintln!("[PlayerCSharp] {text}");
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use hezhou_asset::{AssetId, AssetKind};

    #[test]
    fn context_layout_matches_csharp_contract() {
        assert_eq!(PLAYER_CSHARP_CONTEXT_VERSION, 4);
        // 8 字节头 + scene 指针 + v1 6 + v2 6 + v3 11 + v4 13 = 37 个 8 字节槽。
        assert_eq!(
            std::mem::size_of::<PlayerCSharpContext>(),
            8 + std::mem::size_of::<usize>() * 37
        );
        assert_eq!(std::mem::align_of::<PlayerCSharpContext>(), 8);
        let scene = Scene::new();
        let context = PlayerCSharpContext::new(&scene as *const Scene as *mut Scene);
        assert!(context.is_supported());
        assert!(context.scene_entity_count.is_some());
        assert!(context.scene_create_cube.is_some());
        assert!(context.scene_create_entity.is_some());
        assert!(context.scene_set_entity_position.is_some());
        assert!(context.scene_get_entity_position.is_some());
        assert!(context.log.is_some());
        // v2 tail。
        assert!(context.scene_set_entity_rotation.is_some());
        assert!(context.scene_get_entity_rotation.is_some());
        assert!(context.scene_set_entity_scale.is_some());
        assert!(context.scene_get_entity_scale.is_some());
        assert!(context.scene_entity_exists.is_some());
        assert!(context.scene_destroy_entity.is_some());
        // v3 tail：默认无动画状态（指针 0），函数仍可用并安全返回。
        assert!(context.animation_state.is_null());
        assert!(context.animation_clip_count.is_some());
        assert!(context.animation_clip_name.is_some());
        assert!(context.animation_clip_duration.is_some());
        assert!(context.animation_current.is_some());
        assert!(context.animation_play.is_some());
        assert!(context.animation_set_time.is_some());
        assert!(context.animation_get_time.is_some());
        assert!(context.animation_set_speed.is_some());
        assert!(context.animation_get_speed.is_some());
        assert!(context.animation_set_playing.is_some());
        // v4 tail：默认无资产/音频指针。
        assert!(context.assets.is_null());
        assert!(context.asset_count.is_some());
        assert!(context.asset_id.is_some());
        assert!(context.asset_kind.is_some());
        assert!(context.asset_path.is_some());
        assert!(context.audio.is_null());
        assert!(context.audio_play_first.is_some());
        assert!(context.audio_pause.is_some());
        assert!(context.audio_resume.is_some());
        assert!(context.audio_stop.is_some());
        assert!(context.audio_set_volume.is_some());
        assert!(context.audio_is_playing.is_some());
        assert!(context.audio_backend.is_some());
        // 通过 context 里的函数指针调用，验证 ABI 字段可执行。
        let scene_ptr = context.scene;
        let count = unsafe { context.scene_entity_count.unwrap()(scene_ptr) };
        assert_eq!(count, 0);
    }

    #[test]
    fn v3_animation_api_controls_runtime_state() {
        use crate::animation_control::AnimationClipEntry;

        let mut scene = Scene::new();
        let mut animation = PlayerAnimationState::new(vec![
            AnimationClipEntry {
                name: Some("Idle".to_string()),
                duration: 1.0,
            },
            AnimationClipEntry {
                name: Some("Run".to_string()),
                duration: 0.5,
            },
        ]);
        let context = PlayerCSharpContext::new_with_animation(
            &mut scene as *mut Scene,
            &mut animation as *mut PlayerAnimationState,
        );
        assert!(!context.animation_state.is_null());
        let state = context.animation_state;

        assert_eq!(unsafe { animation_clip_count(state) }, 2);
        // 两段式读取 clip 名：先问长度，再写入。
        let needed = unsafe { animation_clip_name(state, 0, std::ptr::null_mut(), 0) };
        assert_eq!(needed, 4);
        let mut buffer = vec![0u8; needed as usize];
        let written =
            unsafe { animation_clip_name(state, 0, buffer.as_mut_ptr(), buffer.len() as u32) };
        assert_eq!(written, 4);
        assert_eq!(std::str::from_utf8(&buffer).unwrap(), "Idle");
        assert!((unsafe { animation_clip_duration(state, 1) } - 0.5).abs() < 1e-6);
        assert_eq!(unsafe { animation_current(state) }, 0);

        assert_eq!(unsafe { animation_play(state, 1) }, 0);
        assert_eq!(unsafe { animation_current(state) }, 1);
        assert_eq!(unsafe { animation_set_time(state, 0.25) }, 0);
        assert!((unsafe { animation_get_time(state) } - 0.25).abs() < 1e-6);
        assert_eq!(unsafe { animation_set_speed(state, 2.5) }, 0);
        assert!((unsafe { animation_get_speed(state) } - 2.5).abs() < 1e-6);
        animation.advance(0.1);
        // Run 时长 0.5s：0.25 + 0.1*2.5 = 0.5 → 循环回 0。
        assert!((unsafe { animation_get_time(state) } - 0.0).abs() < 1e-6);
        assert_eq!(unsafe { animation_set_playing(state, 0) }, 0);
        assert!(!animation.is_playing());
        // 越界/非法值安全失败。
        assert_eq!(unsafe { animation_play(state, 9) }, -1);
        assert_eq!(unsafe { animation_set_speed(state, f32::NAN) }, -1);
    }

    #[test]
    fn v4_asset_catalog_and_audio_api() {
        let scene_id = AssetId::parse("11111111-1111-4111-8111-111111111111").unwrap();
        let audio_id = AssetId::parse("22222222-2222-4222-8222-222222222222").unwrap();
        let catalog = PlayerAssetCatalog::from_entries(vec![
            PlayerAssetEntry {
                id: scene_id,
                kind: AssetKind::Scene,
                output: "assets/main.scene".to_string(),
            },
            PlayerAssetEntry {
                id: audio_id,
                kind: AssetKind::Audio,
                output: "assets/audio/test.wav".to_string(),
            },
        ]);
        let mut audio = PlayerAudioRuntime::null();
        let mut scene = Scene::new();
        let mut animation = PlayerAnimationState::empty();
        let context = PlayerCSharpContext::new_with_runtime(
            &mut scene as *mut Scene,
            &mut animation as *mut PlayerAnimationState,
            &catalog as *const PlayerAssetCatalog as *mut PlayerAssetCatalog,
            &mut audio as *mut PlayerAudioRuntime,
        );
        assert!(!context.assets.is_null());
        assert!(!context.audio.is_null());
        let assets = context.assets;

        assert_eq!(unsafe { asset_count(assets) }, 2);
        let mut buffer = vec![0u8; 64];
        let len = unsafe { asset_id(assets, 0, buffer.as_mut_ptr(), buffer.len() as u32) };
        assert!(len > 0);
        assert_eq!(
            std::str::from_utf8(&buffer[..len as usize]).unwrap(),
            scene_id.to_string()
        );
        let len = unsafe { asset_kind(assets, 1, buffer.as_mut_ptr(), buffer.len() as u32) };
        assert_eq!(std::str::from_utf8(&buffer[..len as usize]).unwrap(), "audio");
        let len = unsafe { asset_path(assets, 1, buffer.as_mut_ptr(), buffer.len() as u32) };
        assert_eq!(
            std::str::from_utf8(&buffer[..len as usize]).unwrap(),
            "assets/audio/test.wav"
        );
        // 越界安全失败。
        assert_eq!(
            unsafe { asset_kind(assets, 9, buffer.as_mut_ptr(), buffer.len() as u32) },
            -1
        );

        let backend_len = unsafe { audio_backend(context.audio, 0, buffer.as_mut_ptr(), 64) };
        assert_eq!(
            std::str::from_utf8(&buffer[..backend_len as usize]).unwrap(),
            "null"
        );
        assert_eq!(unsafe { audio_play_first(context.audio) }, -1);
        assert_eq!(unsafe { audio_is_playing(context.audio) }, 0);
        assert_eq!(unsafe { audio_set_volume(context.audio, 0.5) }, 0);
        assert!((audio.volume() - 0.5).abs() < 1e-6);
        assert_eq!(unsafe { audio_pause(context.audio) }, -1);
        assert_eq!(unsafe { audio_resume(context.audio) }, -1);
        assert_eq!(unsafe { audio_stop(context.audio) }, -1);
    }

    #[test]
    fn null_scene_functions_are_safe() {
        unsafe {
            assert_eq!(scene_entity_count(std::ptr::null_mut()), 0);
            assert_eq!(scene_create_cube(std::ptr::null_mut()), 0);
            assert_eq!(scene_create_entity(std::ptr::null_mut()), 0);
            assert_eq!(
                scene_set_entity_position(std::ptr::null_mut(), 1, 1.0, 2.0, 3.0),
                -1
            );
            let mut x = 9.0f32;
            let mut y = 9.0f32;
            let mut z = 9.0f32;
            assert_eq!(
                scene_get_entity_position(
                    std::ptr::null_mut(),
                    1,
                    &mut x,
                    &mut y,
                    &mut z
                ),
                -1
            );
            assert_eq!((x, y, z), (9.0, 9.0, 9.0), "失败时不写输出");
            // v2：空 scene / 不存在实体均安全。
            assert_eq!(
                scene_set_entity_rotation(std::ptr::null_mut(), 1, 0.0, 90.0, 0.0),
                -1
            );
            assert_eq!(
                scene_get_entity_rotation(
                    std::ptr::null_mut(),
                    1,
                    &mut x,
                    &mut y,
                    &mut z
                ),
                -1
            );
            assert_eq!(
                scene_set_entity_scale(std::ptr::null_mut(), 1, 1.0, 1.0, 1.0),
                -1
            );
            assert_eq!(
                scene_get_entity_scale(
                    std::ptr::null_mut(),
                    1,
                    &mut x,
                    &mut y,
                    &mut z
                ),
                -1
            );
            assert_eq!(scene_entity_exists(std::ptr::null_mut(), 1), 0);
            assert_eq!(scene_destroy_entity(std::ptr::null_mut(), 1), -1);
            // v3：空动画状态安全返回。
            assert_eq!(animation_clip_count(std::ptr::null_mut()), 0);
            assert_eq!(
                animation_clip_name(
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    0
                ),
                -1
            );
            assert_eq!(animation_clip_duration(std::ptr::null_mut(), 0), -1.0);
            assert_eq!(animation_current(std::ptr::null_mut()), -1);
            assert_eq!(animation_play(std::ptr::null_mut(), 0), -1);
            assert_eq!(animation_set_time(std::ptr::null_mut(), 1.0), -1);
            assert!(animation_get_time(std::ptr::null_mut()).is_nan());
            assert_eq!(animation_set_speed(std::ptr::null_mut(), 2.0), -1);
            assert!(animation_get_speed(std::ptr::null_mut()).is_nan());
            assert_eq!(animation_set_playing(std::ptr::null_mut(), 1), -1);
            // v4：空资产/音频安全返回。
            assert_eq!(asset_count(std::ptr::null_mut()), 0);
            assert_eq!(
                asset_id(std::ptr::null_mut(), 0, std::ptr::null_mut(), 0),
                -1
            );
            assert_eq!(
                asset_kind(std::ptr::null_mut(), 0, std::ptr::null_mut(), 0),
                -1
            );
            assert_eq!(
                asset_path(std::ptr::null_mut(), 0, std::ptr::null_mut(), 0),
                -1
            );
            assert_eq!(audio_play_first(std::ptr::null_mut()), -1);
            assert_eq!(audio_pause(std::ptr::null_mut()), -1);
            assert_eq!(audio_resume(std::ptr::null_mut()), -1);
            assert_eq!(audio_stop(std::ptr::null_mut()), -1);
            assert_eq!(audio_set_volume(std::ptr::null_mut(), 0.5), -1);
            assert_eq!(audio_is_playing(std::ptr::null_mut()), 0);
            assert_eq!(
                audio_backend(
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    0
                ),
                -1
            );
            player_context_log(std::ptr::null());
        }
    }

    #[test]
    fn functions_create_and_position_entities() {
        let mut scene = Scene::new();
        let scene_ptr = &mut scene as *mut Scene;
        let before = unsafe { scene_entity_count(scene_ptr) };
        assert_eq!(before, 0);

        let cube = unsafe { scene_create_cube(scene_ptr) };
        let empty = unsafe { scene_create_entity(scene_ptr) };
        assert_ne!(cube, 0);
        assert_ne!(empty, 0);
        assert_eq!(unsafe { scene_entity_count(scene_ptr) }, 2);

        assert_eq!(
            unsafe { scene_set_entity_position(scene_ptr, cube, 2.5, 0.5, -1.5) },
            0
        );
        let mut x = 0.0f32;
        let mut y = 0.0f32;
        let mut z = 0.0f32;
        assert_eq!(
            unsafe { scene_get_entity_position(scene_ptr, cube, &mut x, &mut y, &mut z) },
            0
        );
        assert_eq!((x, y, z), (2.5, 0.5, -1.5));

        // 不存在的实体：失败且不 panic。
        assert_eq!(
            unsafe { scene_set_entity_position(scene_ptr, 0xDEAD_BEEF, 1.0, 1.0, 1.0) },
            -1
        );
        assert_eq!(
            unsafe {
                scene_get_entity_position(scene_ptr, 0xDEAD_BEEF, &mut x, &mut y, &mut z)
            },
            -1
        );
        // 空输出指针：失败安全。
        assert_eq!(
            unsafe { scene_get_entity_position(scene_ptr, cube, std::ptr::null_mut(), &mut y, &mut z) },
            -1
        );
    }

    #[test]
    fn v2_sets_rotation_scale_and_destroys_entity() {
        let mut scene = Scene::new();
        let scene_ptr = &mut scene as *mut Scene;
        let cube = unsafe { scene_create_cube(scene_ptr) };
        assert_eq!(unsafe { scene_entity_exists(scene_ptr, cube) }, 1);

        // 欧拉角（度）：只绕 Y 轴 90°。
        assert_eq!(
            unsafe { scene_set_entity_rotation(scene_ptr, cube, 0.0, 90.0, 0.0) },
            0
        );
        let mut rx = 0.0f32;
        let mut ry = 0.0f32;
        let mut rz = 0.0f32;
        assert_eq!(
            unsafe { scene_get_entity_rotation(scene_ptr, cube, &mut rx, &mut ry, &mut rz) },
            0
        );
        assert!((rx - 0.0).abs() < 0.1, "rx={rx}");
        assert!((ry - 90.0).abs() < 0.1, "ry={ry}");
        assert!((rz - 0.0).abs() < 0.1, "rz={rz}");
        // core 侧真实生效（不是只改出参）。
        let entity = scene.get_all_entities()[0];
        let euler = scene.get_entity_rotation(entity).unwrap().to_euler_degrees();
        assert!((euler[1] - 90.0).abs() < 0.1, "core euler={euler:?}");

        assert_eq!(
            unsafe { scene_set_entity_scale(scene_ptr, cube, 1.5, 2.0, 0.25) },
            0
        );
        let mut sx = 0.0f32;
        let mut sy = 0.0f32;
        let mut sz = 0.0f32;
        assert_eq!(
            unsafe { scene_get_entity_scale(scene_ptr, cube, &mut sx, &mut sy, &mut sz) },
            0
        );
        assert_eq!((sx, sy, sz), (1.5, 2.0, 0.25));

        // 销毁后 exists=false，重复销毁/读取/设置均失败但不 panic。
        assert_eq!(unsafe { scene_destroy_entity(scene_ptr, cube) }, 0);
        assert_eq!(unsafe { scene_entity_exists(scene_ptr, cube) }, 0);
        assert_eq!(unsafe { scene_destroy_entity(scene_ptr, cube) }, -1);
        assert_eq!(
            unsafe { scene_set_entity_scale(scene_ptr, cube, 1.0, 1.0, 1.0) },
            -1
        );
        assert_eq!(
            unsafe { scene_get_entity_rotation(scene_ptr, cube, &mut rx, &mut ry, &mut rz) },
            -1
        );
        assert_eq!(unsafe { scene_entity_count(scene_ptr) }, 0);
    }
}
