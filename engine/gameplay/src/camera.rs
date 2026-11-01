//! 第三人称跟随相机与锁定目标（Lock-On）。
//!
//! - [`ThirdPersonCamera`]：yaw/pitch 轨道相机，跟随锚点、支持缩放与相机避障；
//! - [`LockOnState`]：从候选目标中选取锁定对象，处理保持/自动解除与瞄准点；
//! - 相机避障与视线检查均通过 trait 注入，见 [`CameraCollisionQuery`] /
//!   [`VisibilityQuery`]。
//!
//! 朝向约定与 [`crate::math`] 一致：`yaw = 0` 看向 -Z，pitch 为正值时相机
//! 位于锚点上方并向下看。

use crate::ids::CombatantId;
use crate::math::{rotate_towards_angle, smoothing_alpha, wrap_angle, Vec3};
use crate::query::{CameraCollisionQuery, VisibilityQuery};

/// 第三人称相机参数。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThirdPersonCameraConfig {
    /// 期望距离（米）。
    pub distance: f32,
    /// 最小距离。
    pub min_distance: f32,
    /// 最大距离。
    pub max_distance: f32,
    /// 锚点（角色脚底）上方的观察高度。
    pub height_offset: f32,
    /// 右肩偏移。
    pub shoulder_offset: f32,
    /// 最小 pitch（弧度，负值表示相机低于锚点向上看）。
    pub min_pitch: f32,
    /// 最大 pitch（弧度，正值表示相机高于锚点向下看）。
    pub max_pitch: f32,
    /// 水平视角灵敏度（弧度/单位输入）。
    pub yaw_sensitivity: f32,
    /// 垂直视角灵敏度（弧度/单位输入）。
    pub pitch_sensitivity: f32,
    /// 跟随平滑速度；0 表示立即跟随。
    pub follow_speed: f32,
    /// 相机避障球体半径。
    pub collision_radius: f32,
    /// 避障后与障碍物保持的间距。
    pub collision_padding: f32,
}

impl Default for ThirdPersonCameraConfig {
    fn default() -> Self {
        Self {
            distance: 4.0,
            min_distance: 1.0,
            max_distance: 8.0,
            height_offset: 1.5,
            shoulder_offset: 0.4,
            min_pitch: -0.5,
            max_pitch: 1.2,
            yaw_sensitivity: 0.0025,
            pitch_sensitivity: 0.0025,
            follow_speed: 12.0,
            collision_radius: 0.25,
            collision_padding: 0.05,
        }
    }
}

/// 一帧的视角输入。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraLookInput {
    /// 水平旋转增量（通常来自鼠标/右摇杆）。
    pub yaw_delta: f32,
    /// 垂直旋转增量。
    pub pitch_delta: f32,
    /// 缩放增量（正数拉远）。
    pub zoom_delta: f32,
}

/// 相机解算结果。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraOutput {
    /// 相机世界位置。
    pub position: Vec3,
    /// 相机注视点。
    pub look_at: Vec3,
    /// 相机前向（单位向量）。
    pub forward: Vec3,
    /// 相机右向（水平单位向量）。
    pub right: Vec3,
    /// 当前 yaw（弧度）。
    pub yaw: f32,
    /// 当前 pitch（弧度）。
    pub pitch: f32,
    /// 实际距离（考虑避障后可能小于期望距离）。
    pub distance: f32,
    /// 本帧是否发生相机避障。
    pub collided: bool,
}

/// 第三人称轨道相机。
#[derive(Clone, Debug)]
pub struct ThirdPersonCamera {
    config: ThirdPersonCameraConfig,
    yaw: f32,
    pitch: f32,
    desired_distance: f32,
    follow_anchor: Vec3,
    initialized: bool,
}

impl ThirdPersonCamera {
    pub fn new(config: ThirdPersonCameraConfig) -> Self {
        let desired_distance = config
            .distance
            .clamp(config.min_distance, config.max_distance);
        Self {
            config,
            yaw: 0.0,
            pitch: 0.2,
            desired_distance,
            follow_anchor: Vec3::ZERO,
            initialized: false,
        }
    }

    /// 当前配置。
    pub fn config(&self) -> &ThirdPersonCameraConfig {
        &self.config
    }

    /// 修改配置。
    pub fn set_config(&mut self, config: ThirdPersonCameraConfig) {
        self.config = config;
        self.desired_distance = self
            .desired_distance
            .clamp(config.min_distance, config.max_distance);
    }

    /// 当前 yaw。
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// 当前 pitch。
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// 直接设置 yaw（锁定/传送后复位用）。
    pub fn set_yaw(&mut self, yaw: f32) {
        self.yaw = wrap_angle(yaw);
    }

    /// 直接设置 pitch（会被限制在配置范围内）。
    pub fn set_pitch(&mut self, pitch: f32) {
        self.pitch = pitch.clamp(self.config.min_pitch, self.config.max_pitch);
    }

    /// 当前期望距离。
    pub fn distance(&self) -> f32 {
        self.desired_distance
    }

    /// 立即把跟随锚点对齐到目标（相机瞬移/场景切换用）。
    pub fn snap_to(&mut self, anchor: Vec3) {
        self.follow_anchor = anchor;
        self.initialized = true;
    }

    /// 处理视角输入（不移动相机）。
    pub fn apply_look(&mut self, look: &CameraLookInput) {
        self.yaw = wrap_angle(self.yaw + look.yaw_delta * self.config.yaw_sensitivity);
        self.pitch = (self.pitch + look.pitch_delta * self.config.pitch_sensitivity)
            .clamp(self.config.min_pitch, self.config.max_pitch);
        self.desired_distance = (self.desired_distance + look.zoom_delta)
            .clamp(self.config.min_distance, self.config.max_distance);
    }

    /// 计算前向（含 pitch）。
    pub fn forward(&self) -> Vec3 {
        let horizontal = Vec3::from_yaw(self.yaw);
        Vec3::new(
            horizontal.x * self.pitch.cos(),
            -self.pitch.sin(),
            horizontal.z * self.pitch.cos(),
        )
        .normalized()
    }

    /// 水平右向。
    pub fn right(&self) -> Vec3 {
        Vec3::right_from_yaw(self.yaw)
    }

    /// 把"输入空间"的水平移动方向转为世界空间方向（相机相对移动）。
    ///
    /// `input.x` 为右，`input.z` 为前；`input.y` 被忽略。
    pub fn camera_relative_move(&self, input: Vec3) -> Vec3 {
        (self.right() * input.x + Vec3::from_yaw(self.yaw) * input.z)
            .horizontal()
            .normalized()
    }

    /// 根据锚点计算相机位置/注视点。
    ///
    /// `anchor` 为角色脚底位置；内部会加上 `height_offset` 并做平滑。
    pub fn update(
        &mut self,
        dt: f32,
        anchor: Vec3,
        collision: &dyn CameraCollisionQuery,
    ) -> CameraOutput {
        let dt = dt.clamp(0.0, 0.1);
        let target_anchor = anchor + Vec3::UP * self.config.height_offset;
        if !self.initialized {
            self.follow_anchor = target_anchor;
            self.initialized = true;
        } else {
            let alpha = smoothing_alpha(self.config.follow_speed, dt);
            self.follow_anchor = Vec3::lerp(self.follow_anchor, target_anchor, alpha);
        }

        let forward = self.forward();
        let right = self.right();
        let shoulder = right * self.config.shoulder_offset;
        let ray_start = self.follow_anchor + shoulder;
        let desired_position = ray_start - forward * self.desired_distance;

        let sweep =
            collision.sweep_camera(ray_start, desired_position, self.config.collision_radius);
        let collided = sweep.hit;
        let safe_position = if collided {
            let to_safe = sweep.position - ray_start;
            let safe_distance = to_safe.magnitude() - self.config.collision_padding;
            let direction = to_safe.normalized_or(-forward);
            ray_start + direction * safe_distance.max(0.0)
        } else {
            desired_position
        };
        let actual_distance = (safe_position - ray_start).magnitude();

        CameraOutput {
            position: safe_position,
            look_at: self.follow_anchor + shoulder,
            forward,
            right,
            yaw: self.yaw,
            pitch: self.pitch,
            distance: actual_distance,
            collided,
        }
    }

    /// 把视角朝空间中的一点旋转（锁定目标用）。
    ///
    /// `turn_speed` 为弧度/秒；返回是否已基本对准。
    pub fn aim_at_point(&mut self, anchor: Vec3, point: Vec3, dt: f32, turn_speed: f32) -> bool {
        let to_point = point - (anchor + Vec3::UP * self.config.height_offset);
        if to_point.magnitude_squared() <= f32::EPSILON {
            return true;
        }
        let horizontal = to_point.horizontal();
        let desired_yaw = if horizontal.magnitude_squared() > f32::EPSILON {
            horizontal.yaw()
        } else {
            self.yaw
        };
        let desired_pitch = (-to_point.y).atan2(horizontal.magnitude());
        let max_step = turn_speed.max(0.0) * dt.clamp(0.0, 0.1);
        let before = self.yaw;
        self.yaw = wrap_angle(rotate_towards_angle(self.yaw, desired_yaw, max_step));
        self.pitch = rotate_towards_angle(
            self.pitch,
            desired_pitch.clamp(self.config.min_pitch, self.config.max_pitch),
            max_step,
        )
        .clamp(self.config.min_pitch, self.config.max_pitch);
        wrap_angle(self.yaw - before).abs() <= 1e-3
            && wrap_angle(self.yaw - desired_yaw).abs() <= 0.01
    }
}

impl Default for ThirdPersonCamera {
    fn default() -> Self {
        Self::new(ThirdPersonCameraConfig::default())
    }
}

/// 锁定目标候选。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LockTargetCandidate {
    /// 候选目标 ID。
    pub id: CombatantId,
    /// 目标脚底/中心位置。
    pub position: Vec3,
    /// 目标包围球半径（用于瞄准点与屏幕占比估计）。
    pub radius: f32,
}

impl LockTargetCandidate {
    pub fn new(id: CombatantId, position: Vec3) -> Self {
        Self {
            id,
            position,
            radius: 0.5,
        }
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
}

/// 锁定策略参数。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LockOnConfig {
    /// 从相机位置到目标的最大可选距离。
    pub max_distance: f32,
    /// 超过该距离自动解除锁定（应大于 `max_distance` 形成迟滞）。
    pub break_distance: f32,
    /// 候选目标相对相机前方的最大夹角（度）。
    pub max_angle_deg: f32,
    /// 瞄准点在目标位置之上的高度。
    pub aim_height: f32,
    /// 切换到更优目标所需的评分优势（防止来回抖动）。
    pub switch_margin: f32,
}

impl Default for LockOnConfig {
    fn default() -> Self {
        Self {
            max_distance: 20.0,
            break_distance: 25.0,
            max_angle_deg: 70.0,
            aim_height: 1.2,
            switch_margin: 0.15,
        }
    }
}

/// 锁定状态。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LockOnState {
    current: Option<CombatantId>,
    target_position: Vec3,
    aim_height: f32,
}

/// 锁定目标组件（与 [`LockOnState`] 同义，命名与 0.2 能力清单保持一致）。
pub type LockOnTarget = LockOnState;

/// 锁定更新结果。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LockOnUpdate {
    /// 当前锁定的目标。
    pub target: Option<CombatantId>,
    /// 瞄准点（目标位置 + `aim_height`）。
    pub aim_point: Option<Vec3>,
    /// 目标是否因超距/消失被自动解除。
    pub broken: bool,
}

impl LockOnState {
    pub fn new() -> Self {
        Self::default()
    }
    /// 当前锁定目标。
    pub fn current(&self) -> Option<CombatantId> {
        self.current
    }

    /// 是否已锁定。
    pub fn is_locked(&self) -> bool {
        self.current.is_some()
    }

    /// 当前瞄准点。
    pub fn aim_point(&self) -> Option<Vec3> {
        self.current
            .map(|_| self.target_position + Vec3::UP * self.aim_height)
    }

    /// 锁定目标位置。
    pub fn target_position(&self) -> Option<Vec3> {
        self.current.map(|_| self.target_position)
    }

    /// 解除锁定。
    pub fn clear(&mut self) {
        self.current = None;
        self.target_position = Vec3::ZERO;
        self.aim_height = 0.0;
    }

    /// 选择锁定目标。
    ///
    /// - 若当前目标仍有效，则保持（带 `switch_margin` 的迟滞），不会每帧乱跳；
    /// - 否则从候选里选"前方 + 距离"综合评分最低者；
    /// - `visibility` 用于过滤被墙遮挡的目标（可传 `None`）。
    pub fn select(
        &mut self,
        camera_position: Vec3,
        camera_forward: Vec3,
        candidates: &[LockTargetCandidate],
        config: &LockOnConfig,
        visibility: Option<&dyn VisibilityQuery>,
    ) -> Option<CombatantId> {
        if let Some(current_id) = self.current {
            if let Some(candidate) = candidates.iter().find(|c| c.id == current_id) {
                let distance = camera_position.distance(&candidate.position);
                // 迟滞：已锁定目标只要没超过 break_distance 就保持，避免每帧抖动。
                if distance <= config.break_distance {
                    self.target_position = candidate.position;
                    self.aim_height = config.aim_height;
                    return Some(current_id);
                }
            }
            self.clear();
        }

        let mut best: Option<(f32, &LockTargetCandidate)> = None;
        for candidate in candidates {
            let Some(score) = self.evaluate(
                candidate,
                camera_position,
                camera_forward,
                config,
                visibility,
            ) else {
                continue;
            };
            let better = match best {
                Some((best_score, _)) => score < best_score,
                None => true,
            };
            if better {
                best = Some((score, candidate));
            }
        }

        match best {
            Some((_, candidate)) => {
                self.current = Some(candidate.id);
                self.target_position = candidate.position;
                self.aim_height = config.aim_height;
                Some(candidate.id)
            }
            None => None,
        }
    }

    fn evaluate(
        &self,
        candidate: &LockTargetCandidate,
        camera_position: Vec3,
        camera_forward: Vec3,
        config: &LockOnConfig,
        visibility: Option<&dyn VisibilityQuery>,
    ) -> Option<f32> {
        let to_target = candidate.position - camera_position;
        let distance = to_target.magnitude();
        if distance > config.max_distance || distance <= f32::EPSILON {
            return None;
        }
        let direction = to_target / distance;
        let angle = crate::math::angle_between(camera_forward, direction);
        if angle > config.max_angle_deg.to_radians() {
            return None;
        }
        if let Some(visibility) = visibility {
            let eye = camera_position;
            let aim = candidate.position + Vec3::UP * (config.aim_height * 0.5);
            if !visibility.has_line_of_sight(eye, aim) {
                return None;
            }
        }
        let angle_score = angle / config.max_angle_deg.to_radians();
        let distance_score = distance / config.max_distance;
        Some(angle_score * 0.7 + distance_score * 0.3)
    }

    /// 每帧更新锁定目标位置并处理自动解除。
    ///
    /// `camera_position` 用于判断是否超出 `break_distance`。
    pub fn update(
        &mut self,
        camera_position: Vec3,
        candidates: &[LockTargetCandidate],
        config: &LockOnConfig,
    ) -> LockOnUpdate {
        let Some(current_id) = self.current else {
            return LockOnUpdate {
                target: None,
                aim_point: None,
                broken: false,
            };
        };
        let Some(candidate) = candidates.iter().find(|c| c.id == current_id) else {
            self.clear();
            return LockOnUpdate {
                target: None,
                aim_point: None,
                broken: true,
            };
        };
        if camera_position.distance(&candidate.position) > config.break_distance {
            self.clear();
            return LockOnUpdate {
                target: None,
                aim_point: None,
                broken: true,
            };
        }
        self.target_position = candidate.position;
        self.aim_height = config.aim_height;
        LockOnUpdate {
            target: self.current,
            aim_point: self.aim_point(),
            broken: false,
        }
    }
}
