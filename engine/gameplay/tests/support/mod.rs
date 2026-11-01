//! 测试用 mock：把 physics / animation 的查询与桥接替换成可控实现。
//!
//! 每个集成测试通过 `mod support;` 引入本模块。

#![allow(dead_code)]

use hezhou_gameplay::{
    ActionState, ActionStateBridge, ActionTransition, AnimationEvent, AnimationEventSource,
    CameraCollisionQuery, CameraSweep, CapsuleMoveResult, CharacterCollisionQuery, GroundHit,
    HitContact, HitDetectionQuery, Hitbox, HitboxPose, MotorOutput, MoveContact, Vec3,
    VisibilityQuery,
};

/// 常用固定帧长（60 FPS）。
pub const DT: f32 = 1.0 / 60.0;

// ---------------------------------------------------------------------------
// 角色碰撞查询
// ---------------------------------------------------------------------------

/// 可配置的角色碰撞查询。
///
/// - 地面高度 = `ground_y + ground_slope * (-z)`（可以模拟坡度）；
/// - `clamp_in_move = false` 时 `move_capsule` 不做地面解算（用于测试地面吸附）；
/// - `wall_z` 在 z 方向放一堵墙（法线 +Z）；
/// - `ceiling_y` 放一个天花板。
pub struct MockCharacterQuery {
    pub ground_y: f32,
    pub ground_slope: f32,
    pub has_ground: bool,
    pub ground_end_x: Option<f32>,
    pub clamp_in_move: bool,
    pub ceiling_y: Option<f32>,
    pub wall_z: Option<f32>,
}

impl Default for MockCharacterQuery {
    fn default() -> Self {
        Self {
            ground_y: 0.0,
            ground_slope: 0.0,
            has_ground: true,
            ground_end_x: None,
            clamp_in_move: true,
            ceiling_y: None,
            wall_z: None,
        }
    }
}

impl MockCharacterQuery {
    pub fn flat() -> Self {
        Self::default()
    }

    pub fn with_slope(ground_slope: f32) -> Self {
        Self {
            ground_slope,
            ..Self::default()
        }
    }

    fn ground_height(&self, position: Vec3) -> f32 {
        self.ground_y + self.ground_slope * (-position.z)
    }

    fn ground_exists(&self, position: Vec3) -> bool {
        self.has_ground
            && self
                .ground_end_x
                .map(|end_x| position.x <= end_x)
                .unwrap_or(true)
    }
}

impl CharacterCollisionQuery for MockCharacterQuery {
    fn ground_probe(&self, feet: Vec3, _radius: f32, max_distance: f32) -> Option<GroundHit> {
        if !self.ground_exists(feet) {
            return None;
        }
        let height = self.ground_height(feet);
        let distance = feet.y - height;
        if distance <= max_distance && distance >= -1e-3 {
            Some(GroundHit::new(
                Vec3::new(feet.x, height, feet.z),
                Vec3::UP,
                distance.max(0.0),
            ))
        } else {
            None
        }
    }

    fn move_capsule(
        &self,
        feet: Vec3,
        delta: Vec3,
        _radius: f32,
        height: f32,
    ) -> CapsuleMoveResult {
        let mut position = feet + delta;
        let mut contacts = Vec::new();
        if self.clamp_in_move && self.ground_exists(position) {
            let ground = self.ground_height(position);
            if position.y <= ground {
                let depth = ground - position.y;
                position.y = ground;
                contacts.push(MoveContact::new(position, Vec3::UP, depth));
            }
        }
        if let Some(ceiling) = self.ceiling_y {
            if position.y + height > ceiling {
                let depth = position.y + height - ceiling;
                position.y = ceiling - height;
                contacts.push(MoveContact::new(position, Vec3::DOWN, depth));
            }
        }
        if let Some(wall) = self.wall_z {
            if position.z < wall {
                let depth = wall - position.z;
                position.z = wall;
                contacts.push(MoveContact::new(position, Vec3::BACK, depth));
            }
        }
        CapsuleMoveResult::with_contacts(position, contacts)
    }
}

// ---------------------------------------------------------------------------
// 相机 / 视线查询
// ---------------------------------------------------------------------------

/// 相机避障 mock：`obstacle_ratio = Some(r)` 表示在 from→to 的 r 处有障碍。
pub struct MockCameraQuery {
    pub obstacle_ratio: Option<f32>,
    pub visible: bool,
}

impl Default for MockCameraQuery {
    fn default() -> Self {
        Self {
            obstacle_ratio: None,
            visible: true,
        }
    }
}

impl CameraCollisionQuery for MockCameraQuery {
    fn sweep_camera(&self, from: Vec3, to: Vec3, _radius: f32) -> CameraSweep {
        match self.obstacle_ratio {
            Some(ratio) => {
                let position = from + (to - from) * ratio.clamp(0.0, 1.0);
                CameraSweep::blocked(position, (from - position).normalized_or(Vec3::FORWARD))
            }
            None => CameraSweep::clear(to),
        }
    }
}

impl VisibilityQuery for MockCameraQuery {
    fn has_line_of_sight(&self, _from: Vec3, _to: Vec3) -> bool {
        self.visible
    }
}

// ---------------------------------------------------------------------------
// 命中查询
// ---------------------------------------------------------------------------

/// 命中盒查询 mock：固定返回给定接触列表。
pub struct MockHitQuery {
    pub contacts: Vec<HitContact>,
}

impl MockHitQuery {
    pub fn new(contacts: Vec<HitContact>) -> Self {
        Self { contacts }
    }

    pub fn hitting(targets: &[u64]) -> Self {
        Self {
            contacts: targets
                .iter()
                .map(|raw| HitContact::new((*raw).into(), Vec3::ZERO, Vec3::FORWARD))
                .collect(),
        }
    }
}

impl HitDetectionQuery for MockHitQuery {
    fn query_hitbox(&self, _hitbox: &Hitbox, _pose: &HitboxPose) -> Vec<HitContact> {
        self.contacts.clone()
    }
}

// ---------------------------------------------------------------------------
// 动画桥接 / 事件
// ---------------------------------------------------------------------------

/// 记录状态切换的动画桥接 mock。
#[derive(Default)]
pub struct MockAnimationBridge {
    pub transitions: Vec<ActionTransition>,
    pub reject: bool,
    pub remaining: Option<f32>,
}

impl MockAnimationBridge {
    /// 最近一次请求的目标状态。
    pub fn last_state(&self) -> Option<ActionState> {
        self.transitions.last().map(|transition| transition.to)
    }
}

impl ActionStateBridge for MockAnimationBridge {
    fn request_transition(&mut self, transition: ActionTransition) -> bool {
        if self.reject {
            return false;
        }
        self.transitions.push(transition);
        true
    }

    fn state_remaining(&self) -> Option<f32> {
        self.remaining
    }
}

/// 动画事件源 mock。
#[derive(Default)]
pub struct MockAnimationEvents {
    pub events: Vec<AnimationEvent>,
}

impl AnimationEventSource for MockAnimationEvents {
    fn drain_events(&mut self, out: &mut Vec<AnimationEvent>) {
        out.append(&mut self.events);
    }
}

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 构造一个"完全在地面站立"的 MotorOutput（用于动画状态测试）。
pub fn grounded_output(speed: f32) -> MotorOutput {
    MotorOutput {
        position: Vec3::ZERO,
        velocity: Vec3::new(0.0, 0.0, -speed),
        grounded: true,
        just_jumped: false,
        just_landed: false,
        speed,
        move_intensity: speed / 7.0,
    }
}
