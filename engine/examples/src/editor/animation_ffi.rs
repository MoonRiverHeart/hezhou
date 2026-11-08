//! 编辑器动画时间轴第一批（0.3 task-43）。
//!
//! - glTF/GLB 资产 → `hezhou_asset::import::gltf::import_gltf_file` → `ImportedScene`，
//!   提取 animations（名称/时长/通道数）与 skeletons（名称/joint 数）；
//! - 查询缓存按「路径 + mtime」失效；
//! - 预览时间状态机（当前 clip/时间/时长/playing），由 C# 每帧 tick 驱动；
//! - FFI 失败安全：坏资产/缺文件返回可读错误 JSON，不 panic。
//!
//! 本轮限制：无角色渲染、无混合树、无关键帧编辑（仅浏览/拖动/播放时间）。

use std::collections::HashMap;
use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use hezhou_animation::{AnimationClip, AnimationState, AnimationStateMachine};
use hezhou_asset::import::gltf::import_gltf_file;
use hezhou_asset::import::ImportedScene;
use hezhou_asset::meta::GltfImportSettings;
use hezhou_dfx::*;
use serde_json::json;

/// 单个 clip 的只读元数据（用于预览状态机与 C# 列表）。
#[derive(Clone, Debug, PartialEq)]
struct ClipInfo {
    name: String,
    duration: f32,
    channels: usize,
}

/// 查询缓存条目（路径 + mtime）。
struct CacheEntry {
    modified: Option<SystemTime>,
    json: String,
    clips: Vec<ClipInfo>,
}

static mut CACHE: Option<HashMap<PathBuf, CacheEntry>> = None;

/// 预览状态（单实例，编辑器单线程）。
struct PreviewState {
    playing: bool,
    time: f64,
    duration: f64,
    clips: Vec<ClipInfo>,
    clip_index: usize,
}

impl PreviewState {
    fn empty() -> Self {
        Self {
            playing: false,
            time: 0.0,
            duration: 0.0,
            clips: Vec::new(),
            clip_index: 0,
        }
    }
}

static mut PREVIEW: Option<PreviewState> = None;

/// 9 状态动画状态机（与 clip 时间轴共享推进；不渲染角色）。
static mut STATE_MACHINE: Option<AnimationStateMachine> = None;
/// 初始化状态图时的资产路径（快照/错误提示用）。
static mut GRAPH_PATH: Option<String> = None;
/// 9 状态绑定信息：状态名、剪断索引、时长（秒）。
static mut GRAPH_STATES: Vec<(String, Option<usize>, f32)> = Vec::new();
/// 状态机是否检测到 skeleton（无骨骼也可预览状态/过渡，仅提示）。
static mut GRAPH_HAS_SKELETON: bool = false;
/// 累计状态机事件数（含过渡/进入/退出/clip 事件）。
static mut GRAPH_EVENT_COUNT: u64 = 0;

/// 9 个状态名（与 `hezhou_animation::AnimationState` 对应，大小写不敏感）。
const STATE_NAMES: [&str; 9] = [
    "Idle", "Run", "Jump", "Attack", "Hit", "Death", "Land", "Dodge", "Block",
];

#[allow(static_mut_refs)]
fn state_machine_mut() -> Option<&'static mut AnimationStateMachine> {
    unsafe { (*std::ptr::addr_of_mut!(STATE_MACHINE)).as_mut() }
}

#[allow(static_mut_refs)]
fn graph_states() -> &'static mut Vec<(String, Option<usize>, f32)> {
    unsafe { &mut *std::ptr::addr_of_mut!(GRAPH_STATES) }
}

#[allow(static_mut_refs)]
fn preview() -> &'static mut PreviewState {
    unsafe {
        PREVIEW.get_or_insert_with(PreviewState::empty);
        PREVIEW.as_mut().expect("preview initialized")
    }
}

#[allow(static_mut_refs)]
fn cache() -> &'static mut HashMap<PathBuf, CacheEntry> {
    unsafe {
        CACHE.get_or_insert_with(HashMap::new);
        CACHE.as_mut().expect("cache initialized")
    }
}

/// 解析 glTF 资产路径：绝对路径 / 相对 cwd / 相对 cwd/assets。
fn resolve_asset_path(input: &str) -> Result<PathBuf, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("资产路径为空".to_string());
    }
    let direct = PathBuf::from(trimmed);
    if direct.is_absolute() {
        return Ok(direct);
    }
    let cwd = std::env::current_dir().map_err(|error| format!("获取当前目录失败: {error}"))?;
    let candidates = [
        cwd.join(trimmed),
        cwd.join("assets").join(trimmed),
        cwd.join("assets").join(trimmed.trim_start_matches("assets/")),
    ];
    for candidate in candidates {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "资产不存在: {}（尝试 cwd / cwd/assets）",
        trimmed
    ))
}

/// 从 `ImportedScene` 构建 summary JSON + clip 列表（纯函数，便于单测）。
fn summary_from_scene(path: &Path, scene: &ImportedScene) -> (String, Vec<ClipInfo>) {
    let clips: Vec<ClipInfo> = scene
        .animations
        .iter()
        .enumerate()
        .map(|(index, animation)| ClipInfo {
            name: animation
                .name
                .clone()
                .unwrap_or_else(|| format!("clip_{index}")),
            duration: animation.duration,
            channels: animation.channels.len(),
        })
        .collect();
    let skeleton_name = scene
        .skeletons
        .first()
        .and_then(|skeleton| skeleton.name.clone());
    let joint_count = scene
        .skeletons
        .first()
        .map(|skeleton| skeleton.joints.len())
        .unwrap_or(0);
    let skeletons: Vec<serde_json::Value> = scene
        .skeletons
        .iter()
        .enumerate()
        .map(|(index, skeleton)| {
            json!({
                "index": index,
                "name": skeleton.name,
                "joint_count": skeleton.joints.len(),
                "root_joint": skeleton.root_joint,
            })
        })
        .collect();
    let clips_json: Vec<serde_json::Value> = clips
        .iter()
        .enumerate()
        .map(|(index, clip)| {
            json!({
                "index": index,
                "name": clip.name,
                "duration": (clip.duration as f64 * 1000.0).round() / 1000.0,
                "channels": clip.channels,
            })
        })
        .collect();
    let value = json!({
        "path": path.to_string_lossy().replace('\\', "/"),
        "skeleton": {
            "name": skeleton_name,
            "joint_count": joint_count,
        },
        "skeletons": skeletons,
        "clips": clips_json,
        "clip_count": clips.len(),
        "warnings": scene.warnings,
    });
    (value.to_string(), clips)
}

/// 加载资产并返回 summary JSON + clip 列表（带 mtime 缓存）。
fn load_asset_summary(input: &str) -> Result<(String, Vec<ClipInfo>), String> {
    let path = resolve_asset_path(input)?;
    let modified = std::fs::metadata(&path).and_then(|meta| meta.modified()).ok();

    if let Some(entry) = cache().get(&path) {
        if entry.modified == modified {
            return Ok((entry.json.clone(), entry.clips.clone()));
        }
    }

    let scene = import_gltf_file(&path, &GltfImportSettings::default()).map_err(|error| {
        format!(
            "导入 glTF 失败（{}）: {}",
            path.display(),
            error
        )
    })?;
    let (json_text, clips) = summary_from_scene(&path, &scene);
    cache().insert(
        path,
        CacheEntry {
            modified,
            json: json_text.clone(),
            clips: clips.clone(),
        },
    );
    Ok((json_text, clips))
}

fn reset_preview_from_clips(clips: Vec<ClipInfo>) {
    let mut state = preview();
    state.playing = false;
    state.time = 0.0;
    state.clip_index = 0;
    state.duration = clips.first().map(|clip| clip.duration as f64).unwrap_or(0.0);
    state.clips = clips;
}

fn write_c_string(buffer: *mut c_char, buffer_size: usize, text: &str) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let bytes = text.as_bytes();
    let copy_len = bytes.len().min(buffer_size - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer as *mut u8, copy_len);
        *buffer.add(copy_len) = 0;
    }
    copy_len
}

fn read_c_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { std::ffi::CStr::from_ptr(ptr).to_str().ok().map(str::to_string) }
}

// === 9 状态状态机（task-47） ===

/// 状态名（大小写不敏感）→ `AnimationState`。
fn animation_state_from_name(name: &str) -> Option<AnimationState> {
    let needle = name.trim().to_ascii_lowercase();
    STATE_NAMES
        .iter()
        .find(|state| state.to_ascii_lowercase() == needle)
        .and_then(|state| AnimationState::from_name(&state.to_ascii_lowercase()))
}

/// 场景中按 clip 名称（大小写不敏感）查找动画索引。
fn find_animation_index(scene: &ImportedScene, name: &str) -> Option<usize> {
    scene.animations.iter().position(|animation| {
        animation
            .name
            .as_deref()
            .map(|clip_name| clip_name.eq_ignore_ascii_case(name))
            .unwrap_or(false)
    })
}

/// 从 ImportedScene 构建 9 状态图：按 clip 名称映射，Idle 无匹配时用首个 clip/clip_index。
fn build_state_machine(scene: &ImportedScene, clip_index_override: i32) -> AnimationStateMachine {
    let mut machine = AnimationStateMachine::new();
    let mut states: Vec<(String, Option<usize>, f32)> = Vec::new();
    let override_index = if clip_index_override >= 0 {
        Some(clip_index_override as usize)
    } else {
        None
    };
    let first_clip = if scene.animations.is_empty() {
        None
    } else {
        Some(0)
    };

    for name in STATE_NAMES {
        let Some(state) = animation_state_from_name(name) else {
            continue;
        };
        let clip_index = if name == "Idle" {
            find_animation_index(scene, name)
                .or(override_index)
                .or(first_clip)
        } else {
            find_animation_index(scene, name)
        };
        if let Some(index) = clip_index {
            if let Some(animation) = scene.animations.get(index) {
                let clip_name = animation
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("clip_{index}"));
                let duration = animation.duration.max(0.0);
                machine.set_clip(state, AnimationClip::new(clip_name, duration));
                states.push((name.to_string(), Some(index), duration));
                continue;
            }
        }
        states.push((name.to_string(), None, 0.0));
    }

    unsafe {
        *std::ptr::addr_of_mut!(GRAPH_STATES) = states;
        GRAPH_HAS_SKELETON = !scene.skeletons.is_empty();
    }
    machine
}

// === FFI ===

/// FFI：查询 glTF 资产的动画/骨骼摘要 JSON。
///
/// 失败时写入 `{"error":"...","clips":[],"skeleton":null}`（仍返回字节数），不 panic。
#[unsafe(no_mangle)]
pub extern "C" fn animation_asset_summary(
    path: *const c_char,
    buffer: *mut c_char,
    buffer_size: usize,
) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let Some(path_text) = read_c_string(path) else {
        let error_json = json!({"error": "资产路径为空", "clips": [], "skeleton": null}).to_string();
        reset_preview_from_clips(Vec::new());
        return write_c_string(buffer, buffer_size, &error_json);
    };
    match load_asset_summary(&path_text) {
        Ok((json_text, clips)) => {
            let clip_count = clips.len();
            reset_preview_from_clips(clips);
            dfx_info!(
                "Animation",
                "资产摘要: {}（clips={}）",
                path_text,
                clip_count
            );
            write_c_string(buffer, buffer_size, &json_text)
        }
        Err(error) => {
            dfx_warn!("Animation", "查询失败: {}", error);
            reset_preview_from_clips(Vec::new());
            let error_json = json!({
                "error": error,
                "path": path_text,
                "clips": [],
                "skeleton": null,
            })
            .to_string();
            write_c_string(buffer, buffer_size, &error_json)
        }
    }
}

/// FFI：每帧驱动预览时间；`playing` >= 0 时同时设置播放状态。返回当前时间（秒）。
#[unsafe(no_mangle)]
pub extern "C" fn animation_preview_tick(dt: f32, playing: i32) -> f64 {
    let mut state = preview();
    if playing >= 0 {
        state.playing = playing != 0;
    }
    if state.playing {
        let delta = if dt.is_finite() && dt > 0.0 {
            dt as f64
        } else {
            0.0
        };
        state.time += delta;
        if state.duration > 0.0 {
            // 循环播放，避免浮点无限增长。
            state.time = state.time % state.duration;
            if state.time < 0.0 {
                state.time += state.duration;
            }
        } else {
            state.time = 0.0;
        }
    }
    // 9 状态机与时间轴共享推进：仅在播放时 update(dt)，事件计数用于快照。
    if state.playing {
        let machine_dt = if dt.is_finite() && dt > 0.0 {
            dt
        } else {
            0.0
        };
        if machine_dt > 0.0 {
            if let Some(machine) = state_machine_mut() {
                let events = machine.update(machine_dt);
                unsafe {
                    GRAPH_EVENT_COUNT = GRAPH_EVENT_COUNT.saturating_add(events.len() as u64);
                }
            }
        }
    }
    state.time
}

/// FFI：选择 clip（索引越界返回 -1）；切换后时间归零。
#[unsafe(no_mangle)]
pub extern "C" fn animation_preview_set_clip(index: i32) -> i32 {
    let mut state = preview();
    if index < 0 || index as usize >= state.clips.len() {
        return -1;
    }
    state.clip_index = index as usize;
    state.time = 0.0;
    state.duration = state.clips[index as usize].duration as f64;
    0
}

/// FFI：设置当前时间（clamp 到 [0, duration]），返回实际时间。
#[unsafe(no_mangle)]
pub extern "C" fn animation_preview_set_time(time: f32) -> f64 {
    let mut state = preview();
    let mut value = if time.is_finite() { time as f64 } else { 0.0 };
    value = value.clamp(0.0, state.duration.max(0.0));
    state.time = value;
    value
}

/// FFI：开始播放（无 clip 时也安全）。
#[unsafe(no_mangle)]
pub extern "C" fn animation_preview_play() -> i32 {
    preview().playing = true;
    0
}

/// FFI：暂停播放。
#[unsafe(no_mangle)]
pub extern "C" fn animation_preview_pause() -> i32 {
    preview().playing = false;
    0
}

/// FFI：预览状态快照 JSON（playing/time/duration/clip_index/clip_count/clip_name）。
#[unsafe(no_mangle)]
pub extern "C" fn animation_preview_snapshot(buffer: *mut c_char, buffer_size: usize) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let state = preview();
    let clip_name = state.clips.get(state.clip_index).map(|clip| clip.name.clone());
    let value = json!({
        "playing": state.playing,
        "time": (state.time * 1000.0).round() / 1000.0,
        "duration": (state.duration * 1000.0).round() / 1000.0,
        "clip_index": state.clip_index,
        "clip_count": state.clips.len(),
        "clip_name": clip_name,
    });
    write_c_string(buffer, buffer_size, &value.to_string())
}

// === 9 状态图 FFI（task-47） ===

/// FFI：从 glTF 资产初始化 9 状态图（clip 名称映射；Idle 无匹配用首个 clip）。
///
/// `clip_index` >= 0 时作为 Idle 的兜底 clip 索引。返回 0=成功，-1=路径/导入失败。
#[unsafe(no_mangle)]
pub extern "C" fn animation_preview_init_graph(
    path: *const c_char,
    clip_index: i32,
) -> i32 {
    let Some(path_text) = read_c_string(path) else {
        dfx_warn!("Animation", "状态图初始化失败: 资产路径为空");
        return -1;
    };
    if path_text.trim().is_empty() {
        return -1;
    }
    let resolved = match resolve_asset_path(&path_text) {
        Ok(path) => path,
        Err(error) => {
            dfx_warn!("Animation", "状态图初始化失败: {}", error);
            return -1;
        }
    };
    let scene = match import_gltf_file(&resolved, &GltfImportSettings::default()) {
        Ok(scene) => scene,
        Err(error) => {
            dfx_warn!(
                "Animation",
                "状态图初始化失败（{}）: {}",
                resolved.display(),
                error
            );
            return -1;
        }
    };

    let machine = build_state_machine(&scene, clip_index);
    // 与时间轴共享 clip 列表（reset 预览状态，播放时间从 0 开始）。
    let (_, clips) = summary_from_scene(&resolved, &scene);
    reset_preview_from_clips(clips);
    let bound = graph_states().iter().filter(|(_, index, _)| index.is_some()).count();
    unsafe {
        STATE_MACHINE = Some(machine);
        GRAPH_PATH = Some(resolved.to_string_lossy().to_string());
        GRAPH_EVENT_COUNT = 0;
    }
    dfx_info!(
        "Animation",
        "9 状态图初始化: {}（绑定 clip {}/{}，skeleton={}）",
        resolved.display(),
        bound,
        STATE_NAMES.len(),
        unsafe { GRAPH_HAS_SKELETON }
    );
    0
}

/// FFI：请求切换状态（名称大小写不敏感）。返回 0=成功，-1=未知状态，-2=未初始化，-3=不允许切换。
#[unsafe(no_mangle)]
pub extern "C" fn animation_preview_set_state(name: *const c_char) -> i32 {
    let Some(name_text) = read_c_string(name) else {
        return -1;
    };
    let Some(state) = animation_state_from_name(&name_text) else {
        dfx_warn!("Animation", "未知状态: {}", name_text);
        return -1;
    };
    let Some(machine) = state_machine_mut() else {
        dfx_warn!("Animation", "状态机未初始化，无法切换到 {}", name_text);
        return -2;
    };
    let from = machine.current_state();
    if machine.request_state(state) {
        dfx_info!(
            "Animation",
            "状态切换: {} -> {}（过渡 {:?}）",
            from.name(),
            state.name(),
            machine.is_transitioning()
        );
        0
    } else {
        dfx_warn!("Animation", "不允许切换到 {}（当前 {}）", state.name(), from.name());
        -3
    }
}

/// FFI：状态机快照 JSON（current/transitioning/alpha/states[]/event_count）。
#[unsafe(no_mangle)]
pub extern "C" fn animation_preview_state_snapshot(
    buffer: *mut c_char,
    buffer_size: usize,
) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let (initialized, path, has_skeleton, event_count) = unsafe {
        (
            STATE_MACHINE.is_some(),
            GRAPH_PATH.clone(),
            GRAPH_HAS_SKELETON,
            GRAPH_EVENT_COUNT,
        )
    };

    let Some(machine) = state_machine_mut() else {
        let value = json!({
            "initialized": false,
            "path": path,
            "has_skeleton": has_skeleton,
            "current_state": null,
            "transitioning": false,
            "transition_alpha": 0.0,
            "event_count": event_count,
            "states": [],
        });
        return write_c_string(buffer, buffer_size, &value.to_string());
    };

    let current = machine.current_state();
    let transition = machine.transition();
    let (transitioning, alpha, from_state, to_state) = match transition {
        Some(transition) => (
            true,
            (transition.alpha() as f64 * 1000.0).round() / 1000.0,
            Some(transition.from.name()),
            Some(transition.to.name()),
        ),
        None => (false, 1.0, None, None),
    };

    let states_json: Vec<serde_json::Value> = STATE_NAMES
        .iter()
        .filter_map(|name| animation_state_from_name(name).map(|state| (name, state)))
        .map(|(name, state)| {
            let entry = machine.entry(state);
            json!({
                "name": name,
                "has_clip": entry.map(|entry| entry.clip.is_some()).unwrap_or(false),
                "duration": entry
                    .map(|entry| (entry.duration() as f64 * 1000.0).round() / 1000.0)
                    .unwrap_or(0.0),
            })
        })
        .collect();

    let value = json!({
        "initialized": initialized,
        "path": path,
        "has_skeleton": has_skeleton,
        "current_state": current.name(),
        "current_index": STATE_NAMES
            .iter()
            .position(|name| animation_state_from_name(name) == Some(current)),
        "current_time": (machine.current_time() as f64 * 1000.0).round() / 1000.0,
        "normalized_time": (machine.normalized_time() as f64 * 1000.0).round() / 1000.0,
        "transitioning": transitioning,
        "transition_alpha": alpha,
        "from_state": from_state,
        "to_state": to_state,
        "event_count": event_count,
        "states": states_json,
    });
    write_c_string(buffer, buffer_size, &value.to_string())
}

/// FFI：释放状态图（恢复单 clip 时间轴预览）。返回 0。
#[unsafe(no_mangle)]
pub extern "C" fn animation_preview_graph_free() -> i32 {
    unsafe {
        STATE_MACHINE = None;
        GRAPH_PATH = None;
        GRAPH_EVENT_COUNT = 0;
        *std::ptr::addr_of_mut!(GRAPH_STATES) = Vec::new();
    }
    dfx_info!("Animation", "状态图已释放");
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use hezhou_asset::import::{
        AnimationInterpolation, AnimationProperty, ImportSourceFormat, ImportedAnimation,
        ImportedAnimationChannel, ImportedJoint, ImportedSkeleton,
    };
    use hezhou_asset::math::Mat4;

    fn build_scene() -> ImportedScene {
        let joint = ImportedJoint {
            node: 0,
            name: Some("root".to_string()),
            parent: None,
            inverse_bind_matrix: Mat4::IDENTITY,
        };
        let skeleton = ImportedSkeleton {
            name: Some("HeroRig".to_string()),
            joints: vec![joint.clone(), ImportedJoint { node: 1, name: Some("spine".to_string()), parent: Some(0), inverse_bind_matrix: Mat4::IDENTITY }],
            root_joint: Some(0),
        };
        let channel = ImportedAnimationChannel {
            target_node: 0,
            target_joint: Some(0),
            property: AnimationProperty::Translation,
            interpolation: AnimationInterpolation::Linear,
            times: vec![0.0, 1.0],
            values: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        };
        ImportedScene {
            format: ImportSourceFormat::Gltf,
            generator: None,
            nodes: Vec::new(),
            meshes: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
            skeletons: vec![skeleton],
            animations: vec![
                ImportedAnimation {
                    name: Some("Idle".to_string()),
                    channels: vec![channel.clone()],
                    duration: 2.0,
                },
                ImportedAnimation {
                    name: Some("Walk".to_string()),
                    channels: vec![channel],
                    duration: 1.5,
                },
            ],
            external_files: Vec::new(),
            warnings: vec!["test warning".to_string()],
        }
    }

    #[test]
    fn summary_contains_skeleton_and_clips() {
        let scene = build_scene();
        let (json_text, clips) = summary_from_scene(Path::new("assets/models/hero.gltf"), &scene);
        assert_eq!(clips.len(), 2);
        assert_eq!(clips[0].name, "Idle");
        assert_eq!(clips[0].duration, 2.0);
        assert_eq!(clips[0].channels, 1);
        assert!(json_text.contains("\"joint_count\":2"));
        assert!(json_text.contains("HeroRig"));
        assert!(json_text.contains("\"Idle\""));
        assert!(json_text.contains("\"Walk\""));
        assert!(json_text.contains("\"clip_count\":2"));
    }

    #[test]
    fn preview_state_machine_controls_time() {
        let scene = build_scene();
        let (_, clips) = summary_from_scene(Path::new("fixture.gltf"), &scene);
        reset_preview_from_clips(clips);

        assert_eq!(animation_preview_set_clip(1), 0);
        assert_eq!(preview().duration, 1.5);
        animation_preview_play();
        assert!(preview().playing);
        let time = animation_preview_tick(0.5, 1);
        assert!((time - 0.5).abs() < 1e-6);
        animation_preview_pause();
        assert!(!preview().playing);

        let clamped = animation_preview_set_time(10.0);
        assert!((clamped - 1.5).abs() < 1e-6);
        assert_eq!(animation_preview_set_clip(9), -1);
    }

    #[test]
    fn empty_asset_path_is_reported_safely() {
        let error = resolve_asset_path("   ").unwrap_err();
        assert!(error.contains("为空"));
        reset_preview_from_clips(Vec::new());
        assert_eq!(animation_preview_set_clip(0), -1);
        assert_eq!(animation_preview_tick(0.1, -1), 0.0);
    }

    #[test]
    fn imports_real_gltf_fixture_with_animation_and_skeleton() {
        use hezhou_asset::import::gltf::import_gltf_file;
        use hezhou_asset::meta::GltfImportSettings;

        const FIXTURE_GLTF: &str = r#"{"asset": {"version": "2.0"}, "buffers": [{"byteLength": 96, "uri": "data:application/octet-stream;base64,AAAAAAAAgD8AAAAAAAAAAAAAAAAAAIA/AAAAAAAAAAAAAIA/AAAAAAAAAAAAAAAAAAAAAAAAgD8AAAAAAAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAAAAAAIA/"}], "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 8}, {"buffer": 0, "byteOffset": 8, "byteLength": 24}, {"buffer": 0, "byteOffset": 32, "byteLength": 64}], "accessors": [{"bufferView": 0, "componentType": 5126, "count": 2, "type": "SCALAR", "min": [0.0], "max": [1.0]}, {"bufferView": 1, "componentType": 5126, "count": 2, "type": "VEC3"}, {"bufferView": 2, "componentType": 5126, "count": 1, "type": "MAT4"}], "nodes": [{"name": "Joint"}], "skins": [{"joints": [0], "inverseBindMatrices": 2}], "animations": [{"name": "Idle", "channels": [{"sampler": 0, "target": {"node": 0, "path": "translation"}}], "samplers": [{"input": 0, "output": 1, "interpolation": "LINEAR"}]}]}
"#;
        let root = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("target")
            .join("animation_fixture_tmp");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("anim.gltf");
        std::fs::write(&path, FIXTURE_GLTF).unwrap();

        let scene = import_gltf_file(&path, &GltfImportSettings::default())
            .expect("最小 glTF fixture 应导入成功");
        assert_eq!(scene.animations.len(), 1);
        assert_eq!(scene.animations[0].name.as_deref(), Some("Idle"));
        assert!((scene.animations[0].duration - 1.0).abs() < 1e-6);
        assert_eq!(scene.skeletons.len(), 1);
        assert_eq!(scene.skeletons[0].joints.len(), 1);

        let (json_text, clips) = summary_from_scene(&path, &scene);
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].name, "Idle");
        assert!(json_text.contains("\"joint_count\":1"));
    }

    #[test]
    fn state_graph_mapping_transition_and_snapshot() {
        // 9 状态映射：名称大小写不敏感；Idle 无匹配回退首个 clip；其他缺失为空。
        let mut scene = build_scene();
        scene.animations[1].name = Some("run".to_string());
        let mut machine = build_state_machine(&scene, -1);
        assert_eq!(machine.current_state(), AnimationState::Idle);
        assert!(machine.clip(AnimationState::Idle).is_some());
        assert!(machine.clip(AnimationState::Run).is_some(), "Run 应大小写不敏感匹配");
        assert!(machine.clip(AnimationState::Attack).is_none());
        assert_eq!(graph_states().len(), 9);
        assert_eq!(
            graph_states().iter().filter(|(_, index, _)| index.is_some()).count(),
            2
        );

        let mut no_idle = build_scene();
        no_idle.animations.truncate(1);
        no_idle.animations[0].name = Some("WalkOnly".to_string());
        let fallback = build_state_machine(&no_idle, -1);
        assert!(fallback.clip(AnimationState::Idle).is_some(), "Idle 应回退首个 clip");

        let mut empty = build_scene();
        empty.animations.clear();
        let empty_machine = build_state_machine(&empty, -1);
        assert!(empty_machine.clip(AnimationState::Idle).is_none());

        // 过渡 alpha：0 → (0,1) → 1，Death 终止状态不能切出。
        assert!(machine.request_state(AnimationState::Run));
        assert!(machine.is_transitioning());
        assert!(machine.transition_alpha() < 0.5);
        machine.update(0.1);
        let mid = machine.transition_alpha();
        assert!(mid > 0.0 && mid < 1.0, "mid alpha={mid}");
        machine.update(1.0);
        assert!(!machine.is_transitioning());
        assert_eq!(machine.current_state(), AnimationState::Run);
        assert!(machine.request_state(AnimationState::Death));
        machine.update(1.0);
        assert_eq!(machine.current_state(), AnimationState::Death);
        assert!(!machine.request_state(AnimationState::Idle), "Death 终止状态不能切出");

        // FFI 快照在未初始化时安全返回 initialized=false。
        unsafe {
            STATE_MACHINE = None;
            GRAPH_PATH = None;
            GRAPH_EVENT_COUNT = 0;
        }
        let mut buffer = [0i8; 512];
        let written = animation_preview_state_snapshot(buffer.as_mut_ptr(), buffer.len());
        assert!(written > 0);
        let text = unsafe { std::ffi::CStr::from_ptr(buffer.as_ptr()) }
            .to_string_lossy()
            .to_string();
        assert!(text.contains("\"initialized\":false"), "{text}");
        let name = std::ffi::CString::new("Idle").unwrap();
        assert_eq!(animation_preview_set_state(name.as_ptr()), -2);
        assert_eq!(animation_preview_graph_free(), 0);
    }
}

