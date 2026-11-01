//! hezhou-integration — animation ↔ gameplay 集成适配层。
//!
//! `hezhou-animation` 与 `hezhou-gameplay` 两个能力 crate 刻意互不依赖：
//! animation 只知道"状态机/片段/事件"，gameplay 只知道"逻辑动作状态 + 桥接 trait"。
//! 本 crate 是**唯一同时依赖两者**的地方，负责三件事：
//!
//! 1. **状态映射**：[`map_action_state`] 把 gameplay 的 `ActionState`
//!    （Idle/Move/Run/Jump/Fall/Land/Attack/HeavyAttack/Dodge/Block/Hit/Death）
//!    映射为 animation 的 `AnimationState`；
//! 2. **逐招式 clip**：[`AnimationActionAdapter`] / [`AnimationActionBridge`] 在切换
//!    状态前按 `ActionId` 重绑 `Attack` 状态的 clip，实现"同一动画状态播放不同招式"；
//! 3. **事件透传**：animation 的 `StateEvent`（Enter/Exit/Transition/ClipEvent）
//!    经 [`AnimationEventMapper`] 映射为 gameplay 的 `AnimationEvent`（命中帧/连招窗口/
//!    脚步/受击），由 `update` / `drain_events` 交给战斗逻辑。
//!
//! ## 快速上手
//!
//! ```no_run
//! use hezhou_animation::{AnimationClip, Transform};
//! use hezhou_gameplay::{locomotion_state, ActionAnimator, ActionId, ActionState, MotorOutput};
//! use hezhou_integration::AnimationActionAdapter;
//!
//! // 资源加载阶段：注册逐招式 clip
//! let mut adapter = AnimationActionAdapter::with_default_transition(0.12);
//! adapter.set_action_clip(ActionId::new(1), AnimationClip::new("Attack_1", 0.8));
//! adapter.set_action_clip(ActionId::new(2), AnimationClip::new("Attack_2", 0.7));
//!
//! // 每帧：逻辑 -> 桥接 -> 推进 -> 采样
//! // let desired = locomotion_state(&motor_output, 4.0);
//! // let mut animator = ActionAnimator::new();
//! // animator.play_one_shot(ActionState::Attack(ActionId::new(2)), adapter.bridge_mut());
//! // let events = adapter.update(1.0 / 60.0);
//! // let pose = adapter.sample(&skeleton);
//! ```
//!
//! ## 能力边界（0.2）
//!
//! - 只做"能力适配"：不包含具体游戏规则、招式数值、关卡或 AI；
//! - 不创建 GPU 资源：`Pose` 由调用方交给 `JointPalette::from_pose` 后上传（render 侧）；
//! - 不做根运动/位移驱动：位移仍由 gameplay 的 `CharacterMotor` 负责；
//! - animation 扩展状态（Land/Dodge/Block，task-11）已合入，默认一一映射；
//!   `--no-default-features` 保留旧 6 状态兼容映射（Land→Jump、Dodge/Block→Attack），
//!   见 [`mapping`] 与 `extended-states` feature。
//!
//! ## 模块
//!
//! - [`mapping`]：`ActionState` → `AnimationState` 映射与保留 `ActionId`；
//! - [`clip_registry`]：逐招式 / 状态槽位 / 虚拟槽位 clip 注册与解析；
//! - [`event_map`]：`StateEvent` → gameplay `AnimationEvent` 映射；
//! - [`bridge`]：`ActionStateBridge` / `AnimationEventSource` 实现；
//! - [`adapter`]：Editor/Player 每帧使用的便捷封装。

#![forbid(unsafe_code)]

pub mod adapter;
pub mod bridge;
pub mod clip_registry;
pub mod event_map;
pub mod mapping;

pub use adapter::AnimationActionAdapter;
pub use bridge::AnimationActionBridge;
pub use clip_registry::{ActionClipRegistry, ClipBinding, VirtualSlot};
pub use event_map::{AnimationEventMapper, ClipEventTarget};
pub use mapping::{
    action_id, action_id_for_animation_state, map_action_state, reserved_action_ids,
    EXTENDED_ANIMATION_STATES,
};

/// 常用类型的一次性导入。
pub mod prelude {
    pub use crate::adapter::AnimationActionAdapter;
    pub use crate::bridge::AnimationActionBridge;
    pub use crate::clip_registry::{ActionClipRegistry, ClipBinding, VirtualSlot};
    pub use crate::event_map::{AnimationEventMapper, ClipEventTarget};
    pub use crate::mapping::{
        action_id, map_action_state, reserved_action_ids, EXTENDED_ANIMATION_STATES,
    };
}

/// 集成层版本（与 workspace 版本保持一致，便于 Player 日志排查）。
pub const INTEGRATION_VERSION: &str = env!("CARGO_PKG_VERSION");
