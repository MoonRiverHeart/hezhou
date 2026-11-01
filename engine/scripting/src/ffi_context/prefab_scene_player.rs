use std::ffi::{c_char, c_void};

// === 0.2 Prefab FFI（编辑器集成 task-21） ===

/// 捕获场景中某个实体子树为 Prefab 并保存 JSON 文件。
///
/// FFI 契约: `prefab_capture_from_scene(scene, entity_id, name, path)` → i32
/// 返回 0=成功，-1=场景/实体无效，-2=保存失败。
pub type PrefabCaptureFromSceneFn =
    extern "C" fn(*mut c_void, u64, *const c_char, *const c_char) -> i32;

/// 从 Prefab JSON 文件实例化实体子树，返回新根实体 ID（0=失败）。
///
/// FFI 契约: `prefab_instantiate(scene, path, parent_id, x, y, z)` → u64
/// `parent_id == 0` 表示作为根实体；`x/y/z` 为 Prefab 根的位置覆盖。
pub type PrefabInstantiateFn =
    extern "C" fn(*mut c_void, *const c_char, u64, f32, f32, f32) -> u64;

// === 0.2 SceneManager FFI ===

/// 把当前编辑场景快照保存为命名场景槽（不存在则创建，已存在则替换）。
///
/// FFI 契约: `scene_manager_save_current(name)` → i32（0=成功，-1=场景/管理器不可用）
pub type SceneManagerSaveCurrentFn = extern "C" fn(*const c_char) -> i32;

/// 从 Project JSON 文件加载场景到命名场景槽（不切换当前场景）。
///
/// FFI 契约: `scene_manager_load_file(name, path)` → i32（0=成功，-1=读取/解析失败）
pub type SceneManagerLoadFileFn = extern "C" fn(*const c_char, *const c_char) -> i32;

/// 切换到已注册场景：先把当前编辑状态写回当前槽，再把目标槽内容恢复到编辑场景。
///
/// FFI 契约: `scene_manager_switch(name)` → i32
/// 返回 0=成功，-1=场景未注册/恢复失败，-2=当前场景尚未保存为场景槽。
pub type SceneManagerSwitchFn = extern "C" fn(*const c_char) -> i32;

/// 卸载命名场景槽（0=成功，-1=未找到）。
pub type SceneManagerUnloadFn = extern "C" fn(*const c_char) -> i32;

/// 已注册场景槽数量。
pub type SceneManagerCountFn = extern "C" fn() -> i32;

/// 当前场景槽名（无则写入空串），返回写入字节数。
pub type SceneManagerCurrentNameFn = extern "C" fn(*mut c_char, usize) -> usize;

/// 全部场景槽名（NUL 分隔、排序），返回写入字节数。
pub type SceneManagerNamesFn = extern "C" fn(*mut c_char, usize) -> usize;

// === 0.2 Player 导出 dry-run FFI ===

/// 生成 Windows + Linux 双平台 Player dry-run 计划（不复制二进制）。
///
/// FFI 契约: `player_export_dry_run(request_json)` → i32
/// 请求 JSON 字段：`project_root/output_dir/product_name/version/start_scene/scripts_dir/runtime_dir/required_font`。
/// 返回 0=成功；-1=JSON/配置/计划生成失败（失败摘要可通过 `player_export_summary` 读取）。
pub type PlayerExportDryRunFn = extern "C" fn(*const c_char) -> i32;

/// 真实构建当前平台 Player 包（Cooker + PlayerBuilder::build + `--validate`）。
///
/// FFI 契约: `player_export_build(request_json)` → i32
/// 请求 JSON 与 dry-run 相同，额外支持 `executable_path`（空=自动查找编辑器同目录 hezhou-player）。
/// 输出：`<output_dir>/cooked/` + `<output_dir>/<当前平台>/`（exe/assets/scripts/runtime/manifest）。
/// 返回 0=成功（validate 通过）；-1=失败（摘要含原因，半成品已清理）。
pub type PlayerExportBuildFn = extern "C" fn(*const c_char) -> i32;

/// 最近一次 Player dry-run 摘要（多行文本），返回写入字节数。
pub type PlayerExportSummaryFn = extern "C" fn(*mut c_char, usize) -> usize;
