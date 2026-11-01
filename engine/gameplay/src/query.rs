//! 物理查询抽象。
//!
//! gameplay 只声明"能力需要什么查询"，不绑定具体物理实现；
//! `engine/physics` 完成自研动作物理后，由集成层提供实现（可放在适配层，
//! 也可以在 physics crate 中实现这些 trait）。
//!
//! 需要底层提供的查询能力：
//! 1. 角色胶囊扫掠 + 地面探测（`CharacterCollisionQuery`）；
//! 2. 相机球体扫掠（`CameraCollisionQuery`）；
//! 3. 锁定目标的视线检查（`VisibilityQuery`）；
//! 4. 攻击命中盒重叠查询（见 `combat::hitbox::HitDetectionQuery`）。

use crate::math::Vec3;

/// 地面探测结果。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GroundHit {
    /// 接触点（脚底应吸附到的位置）。
    pub point: Vec3,
    /// 接触面法线（单位向量，通常朝上）。
    pub normal: Vec3,
    /// 脚底到接触点的距离（>= 0）。
    pub distance: f32,
}

impl GroundHit {
    pub fn new(point: Vec3, normal: Vec3, distance: f32) -> Self {
        Self {
            point,
            normal: normal.normalized(),
            distance,
        }
    }
}

/// 胶囊移动过程中产生的一个接触面。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MoveContact {
    /// 接触点（世界空间）。
    pub point: Vec3,
    /// 接触面法线（单位向量，指向角色外侧）。
    pub normal: Vec3,
    /// 沿扫描方向的接触距离。
    pub distance: f32,
}

impl MoveContact {
    pub fn new(point: Vec3, normal: Vec3, distance: f32) -> Self {
        Self {
            point,
            normal: normal.normalized(),
            distance,
        }
    }
}

/// 胶囊扫掠的解算结果。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CapsuleMoveResult {
    /// 解算后的脚底位置（已消除穿透/滑动后的位置）。
    pub position: Vec3,
    /// 本次移动发生的接触面。
    pub contacts: Vec<MoveContact>,
}

impl CapsuleMoveResult {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            contacts: Vec::new(),
        }
    }

    pub fn with_contacts(position: Vec3, contacts: Vec<MoveContact>) -> Self {
        Self { position, contacts }
    }

    /// 是否发生过任何接触。
    pub fn is_blocked(&self) -> bool {
        !self.contacts.is_empty()
    }

    /// 取第一个可站立的地面接触（法线 Y 分量 >= `min_slope_cos`）。
    pub fn ground_contact(&self, min_slope_cos: f32) -> Option<&MoveContact> {
        self.contacts
            .iter()
            .find(|contact| contact.normal.y >= min_slope_cos)
    }

    /// 取天花板接触（法线朝下）。
    pub fn ceiling_contact(&self) -> Option<&MoveContact> {
        self.contacts
            .iter()
            .find(|contact| contact.normal.y <= -0.5)
    }
}

/// 角色移动所需的碰撞查询。
///
/// 实现方需保证：`move_capsule` 返回的 `position` 是脚底位置，
/// `contacts` 中的法线为世界空间单位向量。
pub trait CharacterCollisionQuery {
    /// 从脚底 `feet` 向下探测地面。
    ///
    /// - `radius`：角色胶囊半径；
    /// - `max_distance`：最大探测距离，超出返回 `None`；
    /// - 返回值中的 `point` 是脚底应吸附到的位置。
    fn ground_probe(&self, feet: Vec3, radius: f32, max_distance: f32) -> Option<GroundHit>;

    /// 胶囊从 `feet` 扫掠 `delta` 后解算位置与接触面。
    ///
    /// - `feet`：扫掠开始的脚底位置；
    /// - `delta`：期望位移（世界空间）；
    /// - `radius` / `height`：角色胶囊尺寸。
    fn move_capsule(&self, feet: Vec3, delta: Vec3, radius: f32, height: f32) -> CapsuleMoveResult;
}

/// 相机球体扫掠结果。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraSweep {
    /// 允许使用的最远位置（无碰撞时为入参 `to`）。
    pub position: Vec3,
    /// 是否发生遮挡。
    pub hit: bool,
    /// 遮挡面法线（无碰撞时为零向量）。
    pub normal: Vec3,
}

impl CameraSweep {
    /// 无碰撞。
    pub fn clear(position: Vec3) -> Self {
        Self {
            position,
            hit: false,
            normal: Vec3::ZERO,
        }
    }

    /// 有遮挡。
    pub fn blocked(position: Vec3, normal: Vec3) -> Self {
        Self {
            position,
            hit: true,
            normal: normal.normalized(),
        }
    }
}

/// 相机避障所需的碰撞查询。
pub trait CameraCollisionQuery {
    /// 从 `from`（角色头部附近）向 `to`（期望相机位置）扫掠一个半径为 `radius` 的球体。
    ///
    /// 返回的 `position` 为相机可安全放置的位置（通常比 `to` 更靠近 `from`）。
    fn sweep_camera(&self, from: Vec3, to: Vec3, radius: f32) -> CameraSweep;
}

/// 视线查询（用于锁定目标时判断是否被遮挡）。
pub trait VisibilityQuery {
    /// `from` 到 `to` 之间是否没有遮挡。
    fn has_line_of_sight(&self, from: Vec3, to: Vec3) -> bool;
}

/// 一个"平地"参考实现：只处理水平无限地面，不做相机遮挡与墙体。
///
/// 用于示例、调试与单元测试；正式运行时应替换为 physics crate 的实现。
#[derive(Clone, Copy, Debug)]
pub struct FlatGroundQuery {
    /// 地面高度（世界空间 Y）。
    pub ground_y: f32,
}

impl FlatGroundQuery {
    pub fn new(ground_y: f32) -> Self {
        Self { ground_y }
    }
}

impl Default for FlatGroundQuery {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl CharacterCollisionQuery for FlatGroundQuery {
    fn ground_probe(&self, feet: Vec3, _radius: f32, max_distance: f32) -> Option<GroundHit> {
        let distance = feet.y - self.ground_y;
        if (-1e-3..=max_distance).contains(&distance) {
            Some(GroundHit::new(
                Vec3::new(feet.x, self.ground_y, feet.z),
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
        _height: f32,
    ) -> CapsuleMoveResult {
        let mut position = feet + delta;
        let mut contacts = Vec::new();
        if position.y <= self.ground_y {
            let depth = self.ground_y - position.y;
            position.y = self.ground_y;
            contacts.push(MoveContact::new(position, Vec3::UP, depth));
        }
        CapsuleMoveResult::with_contacts(position, contacts)
    }
}

impl CameraCollisionQuery for FlatGroundQuery {
    fn sweep_camera(&self, _from: Vec3, to: Vec3, _radius: f32) -> CameraSweep {
        CameraSweep::clear(to)
    }
}

impl VisibilityQuery for FlatGroundQuery {
    fn has_line_of_sight(&self, _from: Vec3, _to: Vec3) -> bool {
        true
    }
}
