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
}

