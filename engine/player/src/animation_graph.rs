//! Player 蒙皮动画状态图（task-37）：`hezhou-asset/animation-bridge` 把
//! `ImportedScene` 适配为运行时 `Skeleton`/`Skin`/`AnimationClip`，再绑定到
//! [`hezhou_animation::AnimationStateMachine`] 的 9 个状态
//! （Idle/Run/Jump/Attack/Hit/Death/Land/Dodge/Block）。
//!
//! # clip → 状态映射
//!
//! 按 clip 名称**大小写不敏感**匹配状态关键词（`idle`/`run`/`jump`/`attack`/
//! `hit`/`death`/`land`/`dodge`/`block`）；同一状态取第一个匹配 clip。
//! 没有任何 clip 匹配 `Idle` 时，`Idle` 使用第一个 clip；其余状态无 clip 时
//! `AnimationStateMachine::sample` 返回绑定姿态（空 clip 回退）。
//!
//! # 每帧
//!
//! [`PlayerAnimationGraph::update`] 推进状态机/过渡并消费事件；
//! [`PlayerAnimationGraph::sample_palette`] 采样当前（或过渡混合）Pose →
//! `JointPalette`，窗口 host 上传 GPU。无 skeleton/skin/import 失败时返回可读错误，
//! host 回退到旧的单 clip palette 路径。
//!
//! # 限制（0.2）
//!
//! - 只做状态切换/过渡/采样，不使用动画事件驱动 gameplay；
//! - 无自定义混合树/遮罩/IK/根运动；
//! - 单骨架/skin（取第一个），多角色共享同一状态图（当前 Player 单蒙皮角色）。

use hezhou_animation::gltf::AnimationAssetData;
use hezhou_animation::{
    AnimationState, AnimationStateMachine, JointPalette, Pose, Skin, Skeleton, StateEvent,
    TransitionCurve,
};
use hezhou_asset::animation_bridge::imported_scene_to_gltf_asset;

use crate::skinned_asset::SelectedSkinnedAsset;

/// 状态图错误。
#[derive(Clone, Debug, PartialEq)]
pub enum AnimationGraphError {
    /// 没有选中蒙皮资产。
    NoSkinnedAsset,
    /// ImportedScene → runtime 转换失败（无 skeleton/非法骨骼等）。
    Bridge { message: String },
    /// 运行时数据缺少 skin。
    NoSkin,
    /// 未知状态名（C# 传入）。
    UnknownState { name: String },
    /// 状态机拒绝切换（同状态/终止状态等）。
    TransitionRejected { state: AnimationState },
}

impl std::fmt::Display for AnimationGraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSkinnedAsset => write!(f, "没有可用的蒙皮资产"),
            Self::Bridge { message } => write!(f, "imported 场景转运行时动画失败：{message}"),
            Self::NoSkin => write!(f, "运行时动画数据缺少 skin"),
            Self::UnknownState { name } => write!(f, "未知动画状态 `{name}`"),
            Self::TransitionRejected { state } => {
                write!(f, "状态机拒绝切换到 {}", state.name())
            }
        }
    }
}

impl std::error::Error for AnimationGraphError {}

/// clip 摘要（诊断/日志）。
#[derive(Clone, Debug, PartialEq)]
pub struct AnimationGraphClip {
    /// clip 名。
    pub name: String,
    /// 时长（秒）。
    pub duration: f32,
    /// 映射到的状态（无匹配为 None）。
    pub state: Option<AnimationState>,
}

/// Player 蒙皮动画状态图。
pub struct PlayerAnimationGraph {
    skeleton: Skeleton,
    skin: Skin,
    machine: AnimationStateMachine,
    clips: Vec<AnimationGraphClip>,
    mapping: Vec<(AnimationState, Option<String>)>,
    warnings: Vec<String>,
    transitions: u64,
}

impl PlayerAnimationGraph {
    /// 从选中蒙皮资产的 ImportedScene 构建状态图。
    pub fn from_selected_asset(
        selected: Option<&SelectedSkinnedAsset>,
    ) -> Result<Self, AnimationGraphError> {
        let asset = selected.ok_or(AnimationGraphError::NoSkinnedAsset)?;
        let bridge = imported_scene_to_gltf_asset(&asset.scene);
        let warnings = bridge.warnings.clone();
        let data = bridge
            .into_runtime()
            .map_err(|error| AnimationGraphError::Bridge {
                message: error.to_string(),
            })?;
        Self::from_asset_data(data, warnings)
    }

    /// 直接从运行时动画数据构建（测试/嵌入资源用）。
    pub fn from_asset_data(
        data: AnimationAssetData,
        mut warnings: Vec<String>,
    ) -> Result<Self, AnimationGraphError> {
        let skin = data
            .primary_skin()
            .cloned()
            .ok_or(AnimationGraphError::NoSkin)?;
        let skeleton = data.skeleton.clone();
        let mut machine =
            AnimationStateMachine::new().with_default_transition(0.2, TransitionCurve::SmoothStep);

        // 按 clip 名称映射 9 个状态。
        let mut mapping: Vec<(AnimationState, Option<String>)> = AnimationState::ALL
            .into_iter()
            .map(|state| (state, None))
            .collect();
        for (state, slot) in &mut mapping {
            let keyword = state.name().to_ascii_lowercase();
            if let Some(clip) = data
                .clips
                .iter()
                .find(|clip| clip.name.to_ascii_lowercase().contains(&keyword))
            {
                machine.set_clip(*state, clip.clone());
                *slot = Some(clip.name.clone());
            }
        }
        // 无匹配 clip 时 Idle 用首个 clip；其余状态空 clip → 绑定姿态。
        if mapping
            .iter()
            .find(|(state, _)| *state == AnimationState::Idle)
            .map(|(_, name)| name.is_none())
            .unwrap_or(false)
        {
            if let Some(first) = data.clips.first() {
                machine.set_clip(AnimationState::Idle, first.clone());
                if let Some((_, slot)) = mapping
                    .iter_mut()
                    .find(|(state, _)| *state == AnimationState::Idle)
                {
                    *slot = Some(first.name.clone());
                }
            } else {
                warnings.push("ImportedScene 没有动画 clip：全部状态使用绑定姿态".to_string());
            }
        }
        machine.transition_to(AnimationState::Idle, 0.0);

        let mut clips: Vec<AnimationGraphClip> = data
            .clips
            .iter()
            .map(|clip| AnimationGraphClip {
                name: clip.name.clone(),
                duration: clip.duration,
                state: mapping
                    .iter()
                    .find(|(_, name)| name.as_deref() == Some(clip.name.as_str()))
                    .map(|(state, _)| *state),
            })
            .collect();
        clips.sort_by(|left, right| left.name.cmp(&right.name));

        Ok(Self {
            skeleton,
            skin,
            machine,
            clips,
            mapping,
            warnings,
            transitions: 0,
        })
    }

    /// 每帧推进状态机与过渡，返回事件。
    pub fn update(&mut self, dt: f32) -> Vec<StateEvent> {
        let events = self.machine.update(dt);
        for event in &events {
            if matches!(event, StateEvent::TransitionStarted { .. }) {
                self.transitions += 1;
            }
        }
        events
    }

    /// 采样当前（含过渡混合）Pose。
    pub fn sample(&self) -> Pose {
        self.machine.sample(&self.skeleton)
    }

    /// 采样并生成 GPU joint palette。
    pub fn sample_palette(&self) -> JointPalette {
        let pose = self.sample();
        JointPalette::from_pose(&self.skeleton, &self.skin, &pose)
    }

    /// 切换状态（C# `animation_set_state`）；同状态返回 Ok（no-op）。
    pub fn set_state(&mut self, name: &str) -> Result<AnimationState, AnimationGraphError> {
        let state = AnimationState::from_name(name.trim())
            .ok_or_else(|| AnimationGraphError::UnknownState { name: name.to_string() })?;
        if self.machine.current_state() == state {
            return Ok(state);
        }
        if self.machine.transition_to(state, 0.2) {
            self.transitions += 1;
            Ok(state)
        } else {
            Err(AnimationGraphError::TransitionRejected { state })
        }
    }

    /// 触发状态（C# `animation_trigger`）；同状态 restart（便于重复攻击）。
    pub fn trigger(&mut self, name: &str) -> Result<AnimationState, AnimationGraphError> {
        let state = AnimationState::from_name(name.trim())
            .ok_or_else(|| AnimationGraphError::UnknownState { name: name.to_string() })?;
        if self.machine.current_state() == state {
            self.machine.restart();
            return Ok(state);
        }
        if self.machine.transition_to(state, 0.2) {
            self.transitions += 1;
            Ok(state)
        } else {
            Err(AnimationGraphError::TransitionRejected { state })
        }
    }

    /// 当前状态。
    pub fn current_state(&self) -> AnimationState {
        self.machine.current_state()
    }

    /// 当前状态名。
    pub fn current_state_name(&self) -> &'static str {
        self.machine.current_state().name()
    }

    /// 是否正在过渡。
    pub fn is_transitioning(&self) -> bool {
        self.machine.is_transitioning()
    }

    /// 过渡 alpha（无过渡 1.0）。
    pub fn transition_alpha(&self) -> f32 {
        self.machine.transition_alpha()
    }

    /// clip 列表摘要。
    pub fn clips(&self) -> &[AnimationGraphClip] {
        &self.clips
    }

    /// 9 状态 → clip 映射（None = 空 clip/绑定姿态）。
    pub fn mapping(&self) -> &[(AnimationState, Option<String>)] {
        &self.mapping
    }

    /// 过渡次数（统计）。
    pub fn transition_count(&self) -> u64 {
        self.transitions
    }

    /// 非致命告警（桥接跳过通道等）。
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// 单行摘要。
    pub fn summary(&self) -> String {
        let state = self.machine.current_state();
        let mapped = self
            .mapping
            .iter()
            .filter(|(_, name)| name.is_some())
            .count();
        format!(
            "动画状态图：{} 个 clip / 9 状态（映射 {}），current={} time={:.3}s transitioning={} alpha={:.2} transitions={}",
            self.clips.len(),
            mapped,
            state.name(),
            self.machine.current_time(),
            self.machine.is_transitioning(),
            self.machine.transition_alpha(),
            self.transitions
        )
    }
}

impl std::fmt::Debug for PlayerAnimationGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerAnimationGraph")
            .field("clips", &self.clips.len())
            .field("current", &self.machine.current_state())
            .field("transitioning", &self.machine.is_transitioning())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hezhou_animation::{AnimationClip, Skeleton, Skin, Transform, Vec3};

    fn asset_data(clip_names: &[&str]) -> AnimationAssetData {
        let mut skeleton = Skeleton::new("hero");
        let root = skeleton.add_root_joint("root", Transform::IDENTITY);
        skeleton
            .add_joint(
                "hand",
                root,
                Transform::from_trs(Vec3::new(0.0, 1.0, 0.0), hezhou_animation::Quat::IDENTITY),
            )
            .unwrap();
        skeleton.rebuild_inverse_bind_matrices().unwrap();
        let skin = Skin::from_skeleton(&skeleton);
        let clips = clip_names
            .iter()
            .map(|name| AnimationClip::new(*name, 1.0))
            .collect();
        AnimationAssetData {
            skeleton,
            skins: vec![skin],
            clips,
        }
    }

    #[test]
    fn maps_nine_states_by_clip_names() {
        let data = asset_data(&["Idle", "Run", "Attack", "JumpLoop", "HitReact", "Death", "Land", "Dodge", "Block"]);
        let graph = PlayerAnimationGraph::from_asset_data(data, Vec::new()).expect("graph");
        assert_eq!(graph.clips().len(), 9);
        assert_eq!(graph.mapping().len(), 9);
        for state in AnimationState::ALL {
            let (_, name) = graph
                .mapping()
                .iter()
                .find(|(candidate, _)| *candidate == state)
                .unwrap();
            assert!(name.is_some(), "{state:?} 应匹配 clip");
        }
        assert_eq!(graph.current_state(), AnimationState::Idle);
        assert!(!graph.is_transitioning());
        assert_eq!(graph.transition_alpha(), 1.0);
        assert!(graph.summary().contains("9 状态"));
        assert_eq!(graph.sample_palette().len(), 2);
    }

    #[test]
    fn unknown_clip_names_fall_back_to_first_clip_for_idle() {
        let data = asset_data(&["Spin", "Wave"]);
        let graph = PlayerAnimationGraph::from_asset_data(data, Vec::new()).expect("graph");
        let idle = graph
            .mapping()
            .iter()
            .find(|(state, _)| *state == AnimationState::Idle)
            .unwrap();
        assert_eq!(idle.1.as_deref(), Some("Spin"));
        assert!(graph
            .mapping()
            .iter()
            .find(|(state, _)| *state == AnimationState::Run)
            .unwrap()
            .1
            .is_none());
    }

    #[test]
    fn set_state_and_trigger_transition() {
        let data = asset_data(&["Idle", "Run", "Attack"]);
        let mut graph = PlayerAnimationGraph::from_asset_data(data, Vec::new()).expect("graph");
        assert_eq!(graph.set_state("run").unwrap(), AnimationState::Run);
        assert_eq!(graph.current_state(), AnimationState::Run);
        assert!(graph.is_transitioning());
        graph.update(0.1);
        assert!(graph.transition_alpha() < 1.0);
        graph.update(0.2);
        assert!(!graph.is_transitioning());
        assert_eq!(graph.trigger("Attack").unwrap(), AnimationState::Attack);
        // 同状态 trigger restart；未知状态报错。
        assert_eq!(graph.trigger("Attack").unwrap(), AnimationState::Attack);
        assert!(matches!(
            graph.set_state("Nope"),
            Err(AnimationGraphError::UnknownState { .. })
        ));
        assert!(graph.transition_count() >= 2);
    }

    #[test]
    fn empty_clip_data_uses_bind_pose_and_reports_error() {
        let data = asset_data(&[]);
        let graph = PlayerAnimationGraph::from_asset_data(data, Vec::new()).expect("graph");
        assert_eq!(graph.current_state(), AnimationState::Idle);
        assert_eq!(graph.sample_palette().len(), 2);
        assert!(graph.summary().contains("0 个 clip"));
        // 无 skin 数据报错。
        let error = PlayerAnimationGraph::from_asset_data(
            AnimationAssetData {
                skeleton: Skeleton::new("empty"),
                skins: Vec::new(),
                clips: Vec::new(),
            },
            Vec::new(),
        )
        .expect_err("无 skin");
        assert!(matches!(error, AnimationGraphError::NoSkin));
    }
}
