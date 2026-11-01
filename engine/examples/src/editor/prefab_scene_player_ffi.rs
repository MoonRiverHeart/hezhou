//! 0.2 Prefab / SceneManager / Player 导出 dry-run 编辑器 FFI（task-21）。
//!
//! 设计要点：
//! - **场景槽**：`SceneManager` 保存命名场景快照（Scene 值）；编辑器始终编辑同一个
//!   `Scene` 对象（`SCENE` 指针稳定），切换时把目标槽快照 `Project::restore_from_json`
//!   恢复到该对象，因此渲染器/C# 侧指针不需要更新；
//! - 切走前先把当前编辑状态写回当前场景槽（`Project::snapshot_to_json` + 重新注册）；
//! - Prefab 直接使用 `core::Prefab::{capture_from_scene, save_file, load_file}` 与
//!   `Scene::instantiate_prefab_with`；
//! - Player 导出只做 **dry-run**：从 AssetDatabase 生成资产清单 + 当前脚本/运行时目录，
//!   调 `PlayerBuilder::plan_both_targets()`，把双平台 `manifest.json/plan.json/file-list.txt`
//!   写到输出目录；不复制真实二进制（运行时 host 后续实现）。
//!
//! 所有失败只写日志并返回错误码，不 panic。

use std::cell::UnsafeCell;
use std::ffi::{c_char, c_void, CStr};
use std::path::{Path, PathBuf};

use hezhou_core::{
    Entity, Prefab, PrefabInstantiateOptions, PrefabOverrides, Project, Scene, SceneManager, Vec3,
};
use hezhou_dfx::*;
use hezhou_player::{
    CookManifest as PlayerCookManifest, ManifestAsset, PlayerBuildConfig, PlayerBuildInput,
    PlayerBuilder, RuntimePayload, ScriptAssembly, TargetPlatform,
};
use serde::Deserialize;

/// 单线程编辑器全局：SceneManager（内部 Scene 含非 Send/Sync 组件，用 UnsafeCell 包装）。
struct SceneManagerSlot(UnsafeCell<Option<Box<SceneManager>>>);
// 编辑器为单线程 UI 循环；与现有 static mut 全局模式一致。
unsafe impl Sync for SceneManagerSlot {}

#[allow(clippy::declare_interior_mutable_const)]
const EMPTY_MANAGER_SLOT: SceneManagerSlot = SceneManagerSlot(UnsafeCell::new(None));

static SCENE_MANAGER: SceneManagerSlot = EMPTY_MANAGER_SLOT;
static mut LAST_EXPORT_SUMMARY: Option<String> = None;

/// 编辑器启动时初始化场景管理器（不加载任何场景槽）。
pub(crate) fn startup_init() {
    unsafe {
        let slot = &mut *SCENE_MANAGER.0.get();
        if slot.is_none() {
            *slot = Some(Box::new(SceneManager::new()));
        }
    }
    dfx_info!("SceneManager", "场景管理器已初始化（场景槽 0，当前无）");
}

fn manager_mut() -> Option<&'static mut SceneManager> {
    unsafe { (&mut *SCENE_MANAGER.0.get()).as_deref_mut() }
}

fn manager_ref() -> Option<&'static SceneManager> {
    unsafe { (&*SCENE_MANAGER.0.get()).as_deref() }
}

fn editor_scene_mut() -> Option<&'static mut Scene> {
    unsafe { super::SCENE.map(|ptr| &mut *ptr) }
}

fn read_c_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr).to_str().ok().map(str::to_string) }
}

fn write_c_string(buffer: *mut c_char, buffer_size: usize, text: &str) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let bytes = text.as_bytes();
    let copy_len = std::cmp::min(bytes.len(), buffer_size - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, buffer, copy_len);
        *buffer.add(copy_len) = 0;
    }
    copy_len
}

// === Prefab ===

fn prefab_capture_impl(scene_ptr: *mut c_void, entity_id: u64, name: &str, path: &str) -> i32 {
    let scene: &mut Scene = if scene_ptr.is_null() {
        match editor_scene_mut() {
            Some(scene) => scene,
            None => {
                dfx_error!("Prefab", "捕获失败: 场景未初始化");
                return -1;
            }
        }
    } else {
        unsafe { &mut *(scene_ptr as *mut Scene) }
    };

    let prefab = match Prefab::capture_from_scene(scene, Entity::new(entity_id), name) {
        Ok(prefab) => prefab,
        Err(error) => {
            dfx_error!("Prefab", "捕获失败: entity={} name={}: {}", entity_id, name, error);
            return -1;
        }
    };
    if let Err(error) = prefab.save_file(path) {
        dfx_error!("Prefab", "保存失败（{}）: {}", path, error);
        return -2;
    }
    dfx_info!(
        "Prefab",
        "捕获 Prefab 成功: {}（entity={}）-> {}",
        name,
        entity_id,
        path
    );
    0
}

fn prefab_instantiate_impl(
    scene_ptr: *mut c_void,
    path: &str,
    parent_id: u64,
    x: f32,
    y: f32,
    z: f32,
) -> u64 {
    let scene: &mut Scene = if scene_ptr.is_null() {
        match editor_scene_mut() {
            Some(scene) => scene,
            None => {
                dfx_error!("Prefab", "实例化失败: 场景未初始化");
                return 0;
            }
        }
    } else {
        unsafe { &mut *(scene_ptr as *mut Scene) }
    };

    let prefab = match Prefab::load_file(path) {
        Ok(prefab) => prefab,
        Err(error) => {
            dfx_error!("Prefab", "实例化失败，读取 Prefab 失败（{}）: {}", path, error);
            return 0;
        }
    };

    let overrides = PrefabOverrides::default().with_position(Vec3::new(x, y, z));
    let mut options = PrefabInstantiateOptions::new().with_overrides(&overrides);
    if parent_id != 0 {
        options = options.with_parent(Entity::new(parent_id));
    }

    match scene.instantiate_prefab_with(&prefab, &options) {
        Ok(instance) => {
            dfx_info!(
                "Prefab",
                "实例化 Prefab 成功: {}（root={}, entities={}, parent={}）",
                path,
                instance.root.id,
                instance.len(),
                parent_id
            );
            instance.root.id
        }
        Err(error) => {
            dfx_error!("Prefab", "实例化失败（{}）: {}", path, error);
            0
        }
    }
}

// === SceneManager ===

fn scene_manager_save_current_impl(name: &str) -> i32 {
    let Some(scene) = editor_scene_mut() else {
        dfx_error!("SceneManager", "保存失败: 场景未初始化");
        return -1;
    };
    let json = match Project::snapshot_to_json(scene) {
        Ok(json) => json,
        Err(error) => {
            dfx_error!("SceneManager", "场景快照失败（{}）: {}", name, error);
            return -1;
        }
    };
    let entity_count = scene.get_all_entities().len();

    let Some(mgr) = manager_mut() else {
        dfx_error!("SceneManager", "保存失败: 管理器未初始化");
        return -1;
    };
    if let Err(error) = mgr.load_scene_from_project_json(name, &json) {
        dfx_error!("SceneManager", "保存场景槽失败（{}）: {}", name, error);
        return -1;
    }
    if mgr.current_name().is_none() {
        if let Err(error) = mgr.switch_to(name) {
            dfx_error!("SceneManager", "设置当前场景失败（{}）: {}", name, error);
            return -1;
        }
    }
    dfx_info!(
        "SceneManager",
        "保存场景槽: {}（实体 {}，场景槽 {}，当前 {:?}）",
        name,
        entity_count,
        mgr.scene_count(),
        mgr.current_name()
    );
    0
}

fn scene_manager_load_file_impl(name: &str, path: &str) -> i32 {
    let json = match std::fs::read_to_string(path) {
        Ok(json) => json,
        Err(error) => {
            dfx_error!("SceneManager", "读取场景文件失败（{}）: {}", path, error);
            return -1;
        }
    };
    let Some(mgr) = manager_mut() else {
        dfx_error!("SceneManager", "加载失败: 管理器未初始化");
        return -1;
    };
    match mgr.load_scene_from_project_json(name, &json) {
        Ok(()) => {
            dfx_info!(
                "SceneManager",
                "加载场景文件到槽: {} <- {}（场景槽 {}，当前 {:?}）",
                name,
                path,
                mgr.scene_count(),
                mgr.current_name()
            );
            0
        }
        Err(error) => {
            dfx_error!("SceneManager", "场景文件解析失败（{}）: {}", path, error);
            -1
        }
    }
}

fn scene_manager_switch_impl(name: &str) -> i32 {
    let Some(mgr) = manager_mut() else {
        dfx_error!("SceneManager", "切换失败: 管理器未初始化");
        return -1;
    };
    let Some(current) = mgr.current_name().map(str::to_string) else {
        dfx_warn!(
            "SceneManager",
            "切换失败: 当前编辑场景尚未保存为场景槽（先执行「保存当前场景」）"
        );
        return -2;
    };

    // 1) 当前编辑状态写回当前场景槽。
    if let Some(scene) = editor_scene_mut() {
        match Project::snapshot_to_json(scene) {
            Ok(json) => {
                if let Err(error) = mgr.load_scene_from_project_json(&current, &json) {
                    dfx_warn!("SceneManager", "写回当前场景槽失败（{}）: {}", current, error);
                }
            }
            Err(error) => {
                dfx_warn!("SceneManager", "当前场景快照失败（{}）: {}", current, error);
            }
        }
    }

    if !mgr.is_loaded(name) {
        dfx_error!("SceneManager", "切换失败: 场景槽不存在: {}", name);
        return -1;
    }

    // 2) 目标槽快照 → 恢复到编辑场景（SCENE 指针保持不变）。
    let target_json = match mgr.scene(name) {
        Some(scene) => match Project::snapshot_to_json(scene) {
            Ok(json) => json,
            Err(error) => {
                dfx_error!("SceneManager", "目标场景快照失败（{}）: {}", name, error);
                return -1;
            }
        },
        None => {
            dfx_error!("SceneManager", "切换失败: 场景槽不存在: {}", name);
            return -1;
        }
    };

    let Some(scene) = editor_scene_mut() else {
        dfx_error!("SceneManager", "切换失败: 场景未初始化");
        return -1;
    };
    if let Err(error) = Project::restore_from_json(&target_json, scene) {
        dfx_error!("SceneManager", "恢复场景失败（{}）: {}", name, error);
        return -1;
    }
    if let Err(error) = mgr.switch_to(name) {
        dfx_error!("SceneManager", "切换当前场景失败（{}）: {}", name, error);
        return -1;
    }

    dfx_info!(
        "SceneManager",
        "切换场景: {} -> {}（实体数 {}）",
        current,
        name,
        scene.get_all_entities().len()
    );
    0
}

fn scene_manager_unload_impl(name: &str) -> i32 {
    let Some(mgr) = manager_mut() else {
        return -1;
    };
    let was_current = mgr.current_name() == Some(name);
    match mgr.unload_scene(name) {
        Ok(scene) => {
            if was_current {
                dfx_warn!(
                    "SceneManager",
                    "已卸载当前场景槽 {}（编辑场景保留最后一次内容；请保存到新场景槽或重新加载）",
                    name
                );
            } else {
                dfx_info!(
                    "SceneManager",
                    "已卸载场景槽: {}（实体数 {}，剩余 {}）",
                    name,
                    scene.get_all_entities().len(),
                    mgr.scene_count()
                );
            }
            0
        }
        Err(error) => {
            dfx_error!("SceneManager", "卸载失败（{}）: {}", name, error);
            -1
        }
    }
}

// === Player 导出 dry-run ===

#[derive(Debug, Deserialize)]
struct PlayerExportRequest {
    #[serde(default)]
    project_root: String,
    #[serde(default)]
    output_dir: String,
    #[serde(default)]
    product_name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    start_scene: String,
    #[serde(default)]
    scripts_dir: String,
    #[serde(default)]
    runtime_dir: String,
    #[serde(default)]
    required_font: Option<String>,
}

fn player_export_impl(request_json: &str) -> Result<String, String> {
    let request: PlayerExportRequest =
        serde_json::from_str(request_json).map_err(|error| format!("请求 JSON 解析失败: {error}"))?;

    let cwd = std::env::current_dir().map_err(|error| format!("获取当前目录失败: {error}"))?;
    let project_root = if request.project_root.trim().is_empty() {
        cwd.clone()
    } else {
        PathBuf::from(&request.project_root)
    };
    let output_dir = if request.output_dir.trim().is_empty() {
        project_root.join("build").join("player_dry_run")
    } else {
        PathBuf::from(&request.output_dir)
    };
    let product_name = if request.product_name.trim().is_empty() {
        "HezhouGame".to_string()
    } else {
        request.product_name.clone()
    };
    let version = if request.version.trim().is_empty() {
        env!("CARGO_PKG_VERSION").to_string()
    } else {
        request.version.clone()
    };
    let start_scene = if request.start_scene.trim().is_empty() {
        "scenes/main.scene.json".to_string()
    } else {
        request.start_scene.clone()
    };
    let scripts_dir = if request.scripts_dir.trim().is_empty() {
        project_root.join("scripts")
    } else {
        PathBuf::from(&request.scripts_dir)
    };
    let runtime_dir = if request.runtime_dir.trim().is_empty() {
        project_root.join("runtime").join("mono")
    } else {
        PathBuf::from(&request.runtime_dir)
    };

    // 1) 资产清单：AssetDatabase 文件资产（相对 assets/ 路径）+ 启动场景/字体兜底条目。
    let rows = super::audio_asset_ffi::export_asset_rows();
    let mut assets: Vec<ManifestAsset> = rows
        .iter()
        .map(|row| {
            let mut asset = ManifestAsset::new(&row.path, &row.kind);
            asset.size_bytes = row.size_bytes;
            asset.checksum = row.checksum.clone();
            asset.asset_id = Some(row.asset_id.clone());
            asset
        })
        .collect();
    let has_asset = |assets: &[ManifestAsset], path: &str| {
        assets.iter().any(|asset| asset.path == path)
    };
    let mut synthetic_count = 0usize;
    if !has_asset(&assets, &start_scene) {
        let mut asset = ManifestAsset::new(start_scene.clone(), "scene");
        asset.asset_id = None;
        assets.push(asset);
        synthetic_count += 1;
    }
    if let Some(font) = request.required_font.as_deref() {
        if !font.trim().is_empty() && !has_asset(&assets, font) {
            assets.push(ManifestAsset::new(font.to_string(), "font"));
            synthetic_count += 1;
        }
    }
    assets.sort_by(|left, right| left.path.cmp(&right.path));

    // 2) 构建配置 + 脚本/运行时输入。
    let mut config = PlayerBuildConfig::new(
        product_name.clone(),
        version.clone(),
        start_scene.clone(),
        TargetPlatform::Windows,
    );
    config = config.with_required_font(request.required_font.clone());

    let scripts = match ScriptAssembly::from_dir(&scripts_dir) {
        Ok(scripts) => scripts,
        Err(error) => {
            dfx_warn!("Player", "脚本目录不可用（{}）: {}", scripts_dir.display(), error);
            Vec::new()
        }
    };
    let runtime = if runtime_dir.is_dir() {
        match RuntimePayload::from_dir(&runtime_dir) {
            Ok(payload) => payload,
            Err(error) => {
                dfx_warn!("Player", "运行时目录读取失败（{}）: {}", runtime_dir.display(), error);
                RuntimePayload::empty()
            }
        }
    } else {
        RuntimePayload::empty()
    };

    let mut input = PlayerBuildInput::new(config, PlayerCookManifest::new(assets));
    input.scripts = scripts;
    input.runtime = runtime;
    let script_count = input.scripts.len();
    let runtime_empty = input.runtime.is_empty();

    // 3) 双平台 dry-run 计划。
    let plans = PlayerBuilder::new(input)
        .plan_both_targets()
        .map_err(|error| format!("生成双平台计划失败: {error}"))?;

    std::fs::create_dir_all(&output_dir)
        .map_err(|error| format!("创建输出目录失败（{}）: {error}", output_dir.display()))?;

    let mut summary = String::new();
    summary.push_str("Player 导出 dry-run（不复制二进制，运行时 host 后续实现）\n");
    summary.push_str(&format!("  输出目录: {}\n", output_dir.display()));
    summary.push_str(&format!("  产品: {}  版本: {}\n", product_name, version));
    summary.push_str(&format!("  启动场景: assets/{}\n", start_scene));
    summary.push_str(&format!(
        "  资产: {}（合成条目 {}）  脚本: {}  运行时: {}\n",
        rows.len() + synthetic_count,
        synthetic_count,
        script_count,
        if runtime_empty {
            "空（warnings 会提示）".to_string()
        } else {
            "已收集".to_string()
        }
    ));

    let mut total_known = 0u64;
    for plan in &plans {
        let target_dir = output_dir.join(plan.target.as_str());
        plan.write_plan(&target_dir)
            .map_err(|error| format!("写计划失败（{}）: {error}", target_dir.display()))?;
        let warning_codes: Vec<&str> = plan.warnings.iter().map(|warning| warning.code()).collect();
        total_known += plan.total_known_size();
        summary.push_str(&format!(
            "  [{}] 文件 {}  已知大小 {} B  警告 [{}]  计划: {}\n",
            plan.target.as_str(),
            plan.files.len(),
            plan.total_known_size(),
            if warning_codes.is_empty() {
                "-".to_string()
            } else {
                warning_codes.join(",")
            },
            target_dir.join("plan.json").display()
        ));
    }
    summary.push_str(&format!("  已知总大小: {} B\n", total_known));

    let summary_path = output_dir.join("export_summary.txt");
    std::fs::write(&summary_path, &summary)
        .map_err(|error| format!("写摘要失败（{}）: {error}", summary_path.display()))?;
    summary.push_str(&format!("  摘要文件: {}\n", summary_path.display()));

    dfx_info!("Player", "Player 导出 dry-run 完成:\n{}", summary);
    Ok(summary)
}

// === extern "C" FFI（注册到 FfiContext；见 scripting/src/ffi_context/prefab_scene_player.rs） ===

#[unsafe(no_mangle)]
pub extern "C" fn prefab_capture_from_scene(
    scene: *mut c_void,
    entity_id: u64,
    name: *const c_char,
    path: *const c_char,
) -> i32 {
    let (Some(name), Some(path)) = (read_c_string(name), read_c_string(path)) else {
        dfx_error!("Prefab", "捕获失败: name/path 为空");
        return -1;
    };
    prefab_capture_impl(scene, entity_id, &name, &path)
}

#[unsafe(no_mangle)]
pub extern "C" fn prefab_instantiate(
    scene: *mut c_void,
    path: *const c_char,
    parent_id: u64,
    x: f32,
    y: f32,
    z: f32,
) -> u64 {
    let Some(path) = read_c_string(path) else {
        dfx_error!("Prefab", "实例化失败: path 为空");
        return 0;
    };
    prefab_instantiate_impl(scene, &path, parent_id, x, y, z)
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_manager_save_current(name: *const c_char) -> i32 {
    let Some(name) = read_c_string(name) else {
        return -1;
    };
    if name.trim().is_empty() {
        return -1;
    }
    scene_manager_save_current_impl(&name)
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_manager_load_file(name: *const c_char, path: *const c_char) -> i32 {
    let (Some(name), Some(path)) = (read_c_string(name), read_c_string(path)) else {
        return -1;
    };
    scene_manager_load_file_impl(&name, &path)
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_manager_switch(name: *const c_char) -> i32 {
    let Some(name) = read_c_string(name) else {
        return -1;
    };
    scene_manager_switch_impl(&name)
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_manager_unload(name: *const c_char) -> i32 {
    let Some(name) = read_c_string(name) else {
        return -1;
    };
    scene_manager_unload_impl(&name)
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_manager_count() -> i32 {
    manager_ref().map(|mgr| mgr.scene_count() as i32).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_manager_current_name(buffer: *mut c_char, buffer_size: usize) -> usize {
    let name = manager_ref()
        .and_then(|mgr| mgr.current_name().map(str::to_string))
        .unwrap_or_default();
    write_c_string(buffer, buffer_size, &name)
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_manager_names(buffer: *mut c_char, buffer_size: usize) -> usize {
    let names = manager_ref()
        .map(|mgr| mgr.scene_names().join("\u{0}"))
        .unwrap_or_default();
    write_c_string(buffer, buffer_size, &names)
}

#[unsafe(no_mangle)]
pub extern "C" fn player_export_dry_run(request_json: *const c_char) -> i32 {
    let Some(request_json) = read_c_string(request_json) else {
        let text = "Player 导出失败: 请求 JSON 为空".to_string();
        dfx_error!("Player", "{}", text);
        unsafe { LAST_EXPORT_SUMMARY = Some(text) };
        return -1;
    };
    match player_export_impl(&request_json) {
        Ok(summary) => {
            unsafe { LAST_EXPORT_SUMMARY = Some(summary) };
            0
        }
        Err(error) => {
            let text = format!("Player 导出失败: {error}");
            dfx_error!("Player", "{}", text);
            unsafe { LAST_EXPORT_SUMMARY = Some(text) };
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn player_export_summary(buffer: *mut c_char, buffer_size: usize) -> usize {
    let text = unsafe { LAST_EXPORT_SUMMARY.clone() }
        .unwrap_or_else(|| "尚未执行 Player 导出 dry-run".to_string());
    write_c_string(buffer, buffer_size, &text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_export_request_defaults_parse() {
        let request: PlayerExportRequest = serde_json::from_str("{}").unwrap();
        assert!(request.project_root.is_empty());
        assert!(request.required_font.is_none());
    }

    #[test]
    fn write_c_string_truncates_and_terminates() {
        let mut buffer = [9i8; 8];
        let text = "hello world";
        let written = write_c_string(buffer.as_mut_ptr(), buffer.len(), text);
        assert_eq!(written, 7);
        assert_eq!(buffer[7], 0);
    }
}
