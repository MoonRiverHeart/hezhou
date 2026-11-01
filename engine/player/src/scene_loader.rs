//! Player 起始场景加载：把 `manifest.start_scene` 对应的包内 Project JSON 恢复成
//! [`hezhou_core::Scene`]，供运行时/窗口 host 渲染。
//!
//! 数据来源与回退规则：
//! - 优先按资产索引（[`PlayerAssetIndex::start_scene`]）解析 cook `output`，得到
//!   `<PlayerRoot>/assets/<output>`；legacy 包没有索引时退回 `manifest.start_scene`
//!   相对 `assets/` 的路径；
//! - 读取 JSON → [`hezhou_core::Project::restore_from_json`]（替换语义，保留实体 ID/层级/
//!   Transform/组件）；
//! - 文件缺失、JSON 非法、Project 格式不兼容、恢复 panic、或场景 0 实体时，
//!   **回退到内置测试场景**（cube + directional light）并记录 WARN，不 panic、不中断启动。
//!
//! 已知限制：当前只加载 Project JSON（编辑器工程/快照格式），cook `imported/*.json`
//! 的资产内容与 C# 脚本初始化留给后续 game host（见 README TODO）。

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use hezhou_core::{GameState, Project, Scene};

use crate::asset_index::PlayerAssetIndex;
use crate::manifest::PlayerManifest;
use crate::paths::to_native_path;

/// 起始场景的来源。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartSceneSource {
    /// 成功从包内 Project JSON 恢复。
    Project {
        /// 实际读取的文件路径。
        path: PathBuf,
        /// 恢复出的实体数。
        entities: usize,
    },
    /// 回退到内置测试场景（cube + directional light）。
    BuiltinFallback {
        /// 回退原因（同时写入 WARN 日志）。
        reason: String,
    },
}

impl StartSceneSource {
    /// 是否使用了内置回退场景。
    pub fn is_fallback(&self) -> bool {
        matches!(self, Self::BuiltinFallback { .. })
    }

    /// 实体数（Project 来源为恢复出的实体数；回退场景为内置 cube+light = 2）。
    pub fn entity_count(&self) -> usize {
        match self {
            Self::Project { entities, .. } => *entities,
            Self::BuiltinFallback { .. } => 2,
        }
    }

    /// 单行摘要（启动日志/CLI 输出）。
    pub fn summary(&self) -> String {
        match self {
            Self::Project { path, entities } => {
                format!("Project JSON {}（{entities} 个实体）", path.display())
            }
            Self::BuiltinFallback { reason } => {
                format!("内置测试场景（cube + directional light）：{reason}")
            }
        }
    }
}

/// 起始场景加载结果（永远成功；失败时使用内置回退场景并给出 warnings）。
pub struct LoadedStartScene {
    /// 已恢复的场景（回退时为内置 cube + light）。
    pub scene: Scene,
    /// 场景来源。
    pub source: StartSceneSource,
    /// 需要以 WARN 级别写入日志的说明。
    pub warnings: Vec<String>,
}

impl LoadedStartScene {
    /// 实体数。
    pub fn entity_count(&self) -> usize {
        self.scene.get_all_entities().len()
    }

    /// 单行摘要。
    pub fn summary(&self) -> String {
        self.source.summary()
    }
}

/// 内置测试场景：cube + directional light（供回退/空场景使用）。
pub fn builtin_test_scene() -> Scene {
    let mut scene = Scene::new();
    scene.create_cube();
    scene.create_directional_light();
    // 内置场景仅用于渲染验证；GameState 由 host 的 renderer.set_game_state(1) 控制。
    scene.state = GameState::Running;
    scene
}

/// 加载 `manifest.start_scene` 对应的真实场景。
///
/// 永远返回 [`LoadedStartScene`]：成功时 `source = Project`；任何失败都回退到
/// [`builtin_test_scene`] 并在 `warnings` 中给出可读原因（调用方负责 WARN 日志）。
pub fn load_start_scene(
    root: &Path,
    manifest: &PlayerManifest,
    index: &PlayerAssetIndex,
) -> LoadedStartScene {
    let assets_dir = manifest.assets_dir(root);
    let path = match index.start_scene() {
        Some(entry) => assets_dir.join(to_native_path(&entry.output, manifest.target)),
        // legacy 包（无 cook manifest/索引）：start_scene 本身就相对 assets/。
        None => manifest.start_scene_path(root),
    };

    match load_project_scene(&path) {
        Ok(scene) => {
            let entities = scene.get_all_entities().len();
            if entities == 0 {
                let reason = format!(
                    "起始场景 {} 是合法 Project JSON 但没有实体",
                    path.display()
                );
                return fallback(reason);
            }
            LoadedStartScene {
                scene,
                source: StartSceneSource::Project { path, entities },
                warnings: Vec::new(),
            }
        }
        Err(error) => fallback(format!("读取起始场景 {} 失败：{error}", path.display())),
    }
}

/// 读取并恢复单个 Project JSON 文件（失败返回可读原因）。
fn load_project_scene(path: &Path) -> Result<Scene, String> {
    let json = std::fs::read_to_string(path).map_err(|error| format!("{error}"))?;
    let mut scene = Scene::new();
    match catch_unwind(AssertUnwindSafe(|| {
        Project::restore_from_json(&json, &mut scene)
    })) {
        Ok(Ok(())) => Ok(scene),
        Ok(Err(error)) => Err(format!("Project JSON 解析/恢复失败：{error}")),
        Err(_) => Err("Project JSON 恢复时发生 panic（已捕获）".to_string()),
    }
}

/// 构造回退结果（内置测试场景 + WARN）。
fn fallback(reason: String) -> LoadedStartScene {
    LoadedStartScene {
        scene: builtin_test_scene(),
        source: StartSceneSource::BuiltinFallback {
            reason: reason.clone(),
        },
        warnings: vec![format!("{reason}；回退内置测试场景（cube + directional light）")],
    }
}

/// 供 `RuntimeReport`/日志使用的场景摘要（可 Clone，不含 Scene 本体）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartSceneSummary {
    /// 场景来源。
    pub source: StartSceneSource,
    /// 实体数。
    pub entities: usize,
    /// WARN 说明。
    pub warnings: Vec<String>,
}

impl StartSceneSummary {
    /// 从加载结果提取摘要。
    pub fn from_loaded(loaded: &LoadedStartScene) -> Self {
        Self {
            source: loaded.source.clone(),
            entities: loaded.entity_count(),
            warnings: loaded.warnings.clone(),
        }
    }

    /// 单行摘要。
    pub fn summary(&self) -> String {
        format!("{}（{} 个实体）", self.source.summary(), self.entities)
    }
}
