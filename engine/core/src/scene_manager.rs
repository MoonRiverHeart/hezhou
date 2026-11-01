//! SceneManager：场景加载/切换/卸载（0.2 引擎能力，不含编辑器 UI）。
//!
//! 目标：给 Player / 编辑器运行模式提供"内存场景集合 + 当前场景"基础能力，
//! 不涉及文件格式本身（场景文件沿用 [`crate::project::Project`] JSON）。
//!
//! # 语义
//!
//! - `load_scene`：注册（或替换）一个内存 [`Scene`]，不自动切换；
//! - `switch_to`：把当前场景切换到已注册场景，并记录历史；原场景保留在内存中，
//!   切回时实体状态不丢失（适合"主菜单 ↔ 关卡"往返）；
//! - `switch_back`：回到上一个场景；
//! - `unload_scene`：把场景移出管理器并返回给调用方；卸载时切回 `Editing` 状态、
//!   标记脚本资源未加载，调用方负责销毁 C# 实例等外部资源；
//! - `unload_all`：卸载全部场景。
//!
//! # 与 Project / Editor 的接入契约
//!
//! - **Project**：`load_scene_from_project` / `load_scene_from_project_json` 直接复用
//!   `Project::sync_to_scene` / `Project::restore_from_json`，项目文件格式不变。
//! - **Editor**：编辑器"运行/暂停/切换场景"建议调用 `switch_to(name)`；
//!   暂停仍由 `Scene::set_state(GameState::Paused)` 处理。FFI 层后续可按
//!   `scene_manager_load_*` / `scene_manager_switch` 暴露（不在本任务范围内）。
//! - **Player**：启动时 `load_scene_from_project_json(start_scene_json)`，运行期
//!   `switch_to` 切换关卡；`unload_scene` 返回的 `Scene` 由调用方 drop。
//! - **资源清理**：切换不销毁场景；卸载才销毁。脚本资源/实例的销毁属于
//!   scripting 层，SceneManager 只负责把场景状态切回 `Editing` 并标记未加载。

use std::collections::HashMap;

use crate::ecs::Scene;
use crate::project::{Project, ProjectError};

/// 一次场景切换的信息（供 Editor/Player 做转场、日志、资源刷新）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneSwitch {
    /// 切换前场景名；首次切换为 `None`。
    pub from: Option<String>,
    /// 切换后场景名。
    pub to: String,
}

/// SceneManager 错误。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneError {
    /// 场景未注册。
    NotFound(String),
    /// 没有可回退的上一个场景。
    NoPreviousScene,
    /// 从 Project JSON 加载失败。
    LoadFailed(String),
}

impl std::fmt::Display for SceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(name) => write!(f, "scene not found: {name}"),
            Self::NoPreviousScene => write!(f, "no previous scene to switch back to"),
            Self::LoadFailed(err) => write!(f, "scene load failed: {err}"),
        }
    }
}

impl std::error::Error for SceneError {}

impl From<ProjectError> for SceneError {
    fn from(err: ProjectError) -> Self {
        Self::LoadFailed(err.to_string())
    }
}

/// 内存场景管理器。
#[derive(Default)]
pub struct SceneManager {
    scenes: HashMap<String, Scene>,
    current: Option<String>,
    history: Vec<String>,
}

impl SceneManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册（或替换）一个内存场景；不改变当前场景。
    pub fn load_scene(&mut self, name: impl Into<String>, scene: Scene) {
        let name = name.into();
        // 替换同名场景时同步修正 current/history，避免指向旧对象。
        if self.current.as_deref() == Some(name.as_str()) {
            self.current = Some(name.clone());
        }
        self.scenes.insert(name, scene);
    }

    /// 从已加载的 [`Project`] 构建场景并注册。
    pub fn load_scene_from_project(&mut self, name: impl Into<String>, project: &Project) {
        let mut scene = Scene::new();
        project.sync_to_scene(&mut scene);
        self.load_scene(name, scene);
    }

    /// 从 Project JSON（工程文件内容/场景快照）构建场景并注册。
    pub fn load_scene_from_project_json(
        &mut self,
        name: impl Into<String>,
        json: &str,
    ) -> Result<(), SceneError> {
        let mut scene = Scene::new();
        Project::restore_from_json(json, &mut scene)?;
        self.load_scene(name, scene);
        Ok(())
    }

    /// 场景是否已注册。
    pub fn is_loaded(&self, name: &str) -> bool {
        self.scenes.contains_key(name)
    }

    /// 读取场景（不改变当前）。
    pub fn scene(&self, name: &str) -> Option<&Scene> {
        self.scenes.get(name)
    }

    /// 可变读取场景（Editor 编辑非当前场景时使用）。
    pub fn scene_mut(&mut self, name: &str) -> Option<&mut Scene> {
        self.scenes.get_mut(name)
    }

    /// 已注册场景数量。
    pub fn scene_count(&self) -> usize {
        self.scenes.len()
    }

    /// 已注册场景名（排序，输出稳定）。
    pub fn scene_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.scenes.keys().map(String::as_str).collect();
        names.sort_unstable();
        names
    }

    /// 当前场景名。
    pub fn current_name(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// 当前场景。
    pub fn current_scene(&self) -> Option<&Scene> {
        self.current
            .as_deref()
            .and_then(|name| self.scenes.get(name))
    }

    /// 可变的当前场景（运行时驱动 `Scene::update`）。
    pub fn current_scene_mut(&mut self) -> Option<&mut Scene> {
        self.current
            .as_deref()
            .and_then(|name| self.scenes.get_mut(name))
    }

    /// 上一个场景名（历史栈顶）。
    pub fn previous_scene_name(&self) -> Option<&str> {
        self.history.last().map(String::as_str)
    }

    /// 切换到已注册场景；原场景保留在内存中。
    ///
    /// 切换到当前场景是幂等操作（不写历史、不改状态）。
    pub fn switch_to(&mut self, name: &str) -> Result<SceneSwitch, SceneError> {
        if !self.scenes.contains_key(name) {
            return Err(SceneError::NotFound(name.to_string()));
        }
        let from = self.current.clone();
        if from.as_deref() == Some(name) {
            return Ok(SceneSwitch {
                from,
                to: name.to_string(),
            });
        }
        if let Some(previous) = from.clone() {
            self.history.push(previous);
        }
        self.current = Some(name.to_string());
        Ok(SceneSwitch {
            from,
            to: name.to_string(),
        })
    }

    /// 回到上一个场景。
    pub fn switch_back(&mut self) -> Result<SceneSwitch, SceneError> {
        let Some(previous) = self.history.pop() else {
            return Err(SceneError::NoPreviousScene);
        };
        if !self.scenes.contains_key(&previous) {
            return Err(SceneError::NotFound(previous));
        }
        let from = self.current.replace(previous.clone());
        Ok(SceneSwitch { from, to: previous })
    }

    /// 卸载场景并返回其所有权（供调用方销毁脚本实例等外部资源）。
    ///
    /// 卸载时会：把场景切回 `Editing`、标记脚本资源未加载、从历史中移除。
    /// 若卸载的是当前场景，`current` 变为 `None`（调用方需要再 `switch_to`）。
    pub fn unload_scene(&mut self, name: &str) -> Result<Scene, SceneError> {
        let mut scene = self
            .scenes
            .remove(name)
            .ok_or_else(|| SceneError::NotFound(name.to_string()))?;
        // 卸载语义：不保留运行态。`set_state(Editing)` 会调用 unload_scripts。
        scene.set_state(crate::ecs::GameState::Editing);
        if self.current.as_deref() == Some(name) {
            self.current = None;
        }
        self.history.retain(|item| item != name);
        Ok(scene)
    }

    /// 卸载全部场景，返回 (场景名, Scene) 列表（按场景名排序，输出稳定）。
    pub fn unload_all(&mut self) -> Vec<(String, Scene)> {
        let mut removed: Vec<(String, Scene)> = Vec::new();
        let mut names = self
            .scene_names()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        names.sort();
        for name in names {
            if let Some(mut scene) = self.scenes.remove(&name) {
                scene.set_state(crate::ecs::GameState::Editing);
                removed.push((name, scene));
            }
        }
        self.current = None;
        self.history.clear();
        removed
    }
}

impl std::fmt::Debug for SceneManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SceneManager")
            .field("scenes", &self.scene_names())
            .field("current", &self.current)
            .field("history", &self.history)
            .finish()
    }
}
