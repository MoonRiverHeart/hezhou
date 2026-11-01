//! 与动画模块的状态机桥接。
//!
//! gameplay 只决定"逻辑上角色应该处于什么动作状态"，不依赖具体动画实现：
//!
//! - [`ActionStateBridge`]：gameplay → 动画。逻辑侧请求切换状态，动画侧决定
//!   播放哪个 clip、如何混合；返回 `false` 表示动画资产拒绝（例如缺少该动作）。
//! - [`AnimationEventSource`]：动画 → gameplay。动画侧的命中帧/脚步/取消窗口
//!   等事件可以被战斗逻辑消费（0.2 的连招窗口默认由时间驱动，动画事件是可选增强）。
//! - [`ActionAnimator`]：逻辑侧的小状态机，负责去重与一次性动作重启。
//!
//! 集成方式（不引入对 animation crate 的编译期依赖）：
//! 1. animation crate 或集成层实现 `ActionStateBridge` / `AnimationEventSource`；
//! 2. 每帧把 [`crate::motor::MotorOutput`] 交给 [`locomotion_state`] 得到基础移动状态；
//! 3. 战斗/闪避等一次性动作用 [`ActionAnimator::play_one_shot`] 覆盖基础状态；
//! 4. 通过 [`ActionStateBridge::state_remaining`] 让逻辑侧知道动画剩余时间（可选）。

use crate::motor::MotorOutput;

/// 动画动作标识（对应动画资产中的 clip/action）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionId(pub u32);

impl ActionId {
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// 逻辑动作状态。
///
/// 这是"能力层"的通用状态集合，不包含具体游戏的招式名称/数值；
/// 具体攻击通过 `Attack(ActionId)` 引用动画资产。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionState {
    /// 待机。
    Idle,
    /// 步行。
    Move,
    /// 奔跑。
    Run,
    /// 上升中（跳跃）。
    Jump,
    /// 下落。
    Fall,
    /// 落地瞬间（一次性）。
    Land,
    /// 轻/普通攻击。
    Attack(ActionId),
    /// 重攻击。
    HeavyAttack(ActionId),
    /// 闪避。
    Dodge,
    /// 格挡。
    Block,
    /// 受击。
    Hit,
    /// 死亡。
    Death,
}

impl ActionState {
    /// 是否属于基础移动状态（可循环）。
    pub fn is_locomotion(self) -> bool {
        matches!(self, Self::Idle | Self::Move | Self::Run)
    }

    /// 是否处于空中状态。
    pub fn is_airborne(self) -> bool {
        matches!(self, Self::Jump | Self::Fall)
    }

    /// 是否需要重启式播放（一次性动作）。
    pub fn is_one_shot(self) -> bool {
        matches!(
            self,
            Self::Land
                | Self::Attack(_)
                | Self::HeavyAttack(_)
                | Self::Dodge
                | Self::Hit
                | Self::Death
        )
    }
}

/// 状态切换参数。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransitionSpec {
    /// 交叉淡入淡出时长（秒）。
    pub fade_seconds: f32,
    /// 相同状态是否强制重启（用于连续攻击/受击）。
    pub restart: bool,
}

impl Default for TransitionSpec {
    fn default() -> Self {
        Self {
            fade_seconds: 0.15,
            restart: false,
        }
    }
}

impl TransitionSpec {
    /// 一次性动作的默认过渡（短淡化 + 重启）。
    pub fn one_shot() -> Self {
        Self {
            fade_seconds: 0.08,
            restart: true,
        }
    }
}

/// 一次状态切换请求。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionTransition {
    /// 切换前状态；首次同步为 `None`。
    pub from: Option<ActionState>,
    /// 目标状态。
    pub to: ActionState,
    /// 过渡参数。
    pub spec: TransitionSpec,
}

/// 动画侧需要实现的状态桥接 trait。
pub trait ActionStateBridge {
    /// 请求切换到 `transition.to`。
    ///
    /// 返回 `true` 表示动画层接受（随后逻辑侧会认为状态已切换）；
    /// 返回 `false` 表示拒绝（例如缺少该动作资产），逻辑侧可重试或降级。
    fn request_transition(&mut self, transition: ActionTransition) -> bool;

    /// 当前动作的剩余时间（秒）。默认 `None` 表示由 gameplay 时间驱动。
    fn state_remaining(&self) -> Option<f32> {
        None
    }
}

/// 动画事件（帧事件 / 状态事件）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimationEvent {
    /// 动作开始。
    ActionStarted(ActionId),
    /// 动作结束。
    ActionEnded(ActionId),
    /// 命中帧（攻击生效帧）；`hit_index` 用于多段攻击。
    HitFrame { action: ActionId, hit_index: u8 },
    /// 取消/连招窗口打开。
    CancelWindowOpen(ActionId),
    /// 取消/连招窗口关闭。
    CancelWindowClose(ActionId),
    /// 脚步。
    Footstep {
        /// 左右脚标记。
        foot: u8,
    },
    /// 受击动作开始。
    HitReactionStart(ActionId),
}

/// 动画侧事件源（gameplay 每帧拉取）。
pub trait AnimationEventSource {
    /// 取出本帧产生的事件；实现方可自行清空内部缓冲。
    fn drain_events(&mut self, out: &mut Vec<AnimationEvent>);
}

/// 逻辑侧动作动画同步器。
///
/// 负责"不要重复请求同一状态"以及一次性动作的重启，避免每帧都向动画层发命令。
#[derive(Clone, Debug, Default)]
pub struct ActionAnimator {
    current: Option<ActionState>,
}

impl ActionAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前已同步到动画层的逻辑状态。
    pub fn current(&self) -> Option<ActionState> {
        self.current
    }

    /// 清空记录（动画层重建/传送后调用）。
    pub fn reset(&mut self) {
        self.current = None;
    }

    /// 同步到基础移动状态（相同状态不重复请求）。
    pub fn sync(&mut self, desired: ActionState, bridge: &mut dyn ActionStateBridge) -> bool {
        self.sync_with(desired, bridge, TransitionSpec::default())
    }

    /// 同步到指定状态并可自定义过渡参数。
    pub fn sync_with(
        &mut self,
        desired: ActionState,
        bridge: &mut dyn ActionStateBridge,
        spec: TransitionSpec,
    ) -> bool {
        if !spec.restart && self.current == Some(desired) {
            return false;
        }
        let accepted = bridge.request_transition(ActionTransition {
            from: self.current,
            to: desired,
            spec,
        });
        if accepted {
            self.current = Some(desired);
        }
        accepted
    }

    /// 播放一次性动作（攻击/闪避/受击/死亡）；相同状态也会重启。
    pub fn play_one_shot(
        &mut self,
        state: ActionState,
        bridge: &mut dyn ActionStateBridge,
    ) -> bool {
        self.sync_with(state, bridge, TransitionSpec::one_shot())
    }
}

/// 根据角色移动输出推导基础移动动画状态。
///
/// `run_threshold` 是 Move → Run 的水平速度阈值（米/秒）。
/// 落地瞬间返回 [`ActionState::Land`]（一次性状态，由调用方决定何时恢复基础状态）。
pub fn locomotion_state(output: &MotorOutput, run_threshold: f32) -> ActionState {
    if output.just_landed {
        return ActionState::Land;
    }
    if !output.grounded {
        return if output.velocity.y > 0.0 {
            ActionState::Jump
        } else {
            ActionState::Fall
        };
    }
    if output.speed <= 0.05 {
        ActionState::Idle
    } else if output.speed < run_threshold.max(0.0) {
        ActionState::Move
    } else {
        ActionState::Run
    }
}
