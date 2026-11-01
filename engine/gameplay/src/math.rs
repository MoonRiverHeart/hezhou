//! 基础向量数学，供 gameplay 各能力共用。
//!
//! 本 crate 不直接依赖 core / physics 的具体类型，避免能力组件与底层实现耦合；
//! 集成层需要时可通过 `[f32; 3]` 与引擎 `Vec3` 互转。
//!
//! 朝向约定（全 crate 统一）：
//! - `yaw = 0` 时前向为 [`Vec3::FORWARD`]（-Z）；
//! - `yaw` 增大表示绕 +Y 轴向左转（从上方看逆时针）；
//! - 前向 = `(-sin(yaw), 0, -cos(yaw))`，右向 = `(cos(yaw), 0, -sin(yaw))`。

use std::f32::consts::{PI, TAU};
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// 三维向量（f32）。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    pub const ONE: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };
    pub const UP: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    pub const DOWN: Self = Self {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    };
    pub const FORWARD: Self = Self {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };
    pub const BACK: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };
    pub const RIGHT: Self = Self {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    pub const LEFT: Self = Self {
        x: -1.0,
        y: 0.0,
        z: 0.0,
    };

    /// 构造向量。
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// 三分量取同一值。
    pub const fn splat(value: f32) -> Self {
        Self::new(value, value, value)
    }

    /// 零向量。
    pub const fn zero() -> Self {
        Self::ZERO
    }

    /// 单位向量 (1,1,1)。
    pub const fn one() -> Self {
        Self::ONE
    }

    /// +Y 方向。
    pub const fn up() -> Self {
        Self::UP
    }

    /// +X 方向。
    pub const fn right() -> Self {
        Self::RIGHT
    }

    /// -Z 方向（引擎默认前向）。
    pub const fn forward() -> Self {
        Self::FORWARD
    }

    /// 长度。
    pub fn magnitude(&self) -> f32 {
        self.magnitude_squared().sqrt()
    }

    /// 长度平方。
    pub fn magnitude_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// 归一化；零向量返回零向量。
    pub fn normalized(&self) -> Self {
        let len = self.magnitude();
        if len > f32::EPSILON {
            *self * (1.0 / len)
        } else {
            Self::ZERO
        }
    }

    /// 归一化，零向量时返回给定兜底值。
    pub fn normalized_or(&self, fallback: Self) -> Self {
        if self.magnitude_squared() > f32::EPSILON * f32::EPSILON {
            self.normalized()
        } else {
            fallback
        }
    }

    /// 点积。
    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// 叉积。
    pub fn cross(&self, other: &Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// 两点距离。
    pub fn distance(&self, other: &Self) -> f32 {
        (*self - *other).magnitude()
    }

    /// 两点距离平方。
    pub fn distance_squared(&self, other: &Self) -> f32 {
        (*self - *other).magnitude_squared()
    }

    /// 线性插值。
    pub fn lerp(a: Self, b: Self, t: f32) -> Self {
        a + (b - a) * t
    }

    /// 限制长度不超过 `max_length`。
    pub fn clamp_length(&self, max_length: f32) -> Self {
        let len = self.magnitude();
        if len > max_length && len > f32::EPSILON {
            *self * (max_length / len)
        } else {
            *self
        }
    }

    /// 去掉 Y 分量后的水平向量。
    pub fn horizontal(&self) -> Self {
        Self::new(self.x, 0.0, self.z)
    }

    /// 水平长度。
    pub fn horizontal_length(&self) -> f32 {
        (self.x * self.x + self.z * self.z).sqrt()
    }

    /// 替换 Y 分量。
    pub fn with_y(&self, y: f32) -> Self {
        Self::new(self.x, y, self.z)
    }

    /// 由 yaw 得到水平前向（见模块文档中的朝向约定）。
    pub fn from_yaw(yaw: f32) -> Self {
        Self::new(-yaw.sin(), 0.0, -yaw.cos())
    }

    /// 由水平方向反解 yaw；零向量返回 0。
    pub fn yaw(&self) -> f32 {
        if self.x.abs() + self.z.abs() <= f32::EPSILON {
            0.0
        } else {
            (-self.x).atan2(-self.z)
        }
    }

    /// 水平右向（见模块文档中的朝向约定）。
    pub fn right_from_yaw(yaw: f32) -> Self {
        Self::new(yaw.cos(), 0.0, -yaw.sin())
    }

    /// 是否包含 NaN。
    pub fn is_nan(&self) -> bool {
        self.x.is_nan() || self.y.is_nan() || self.z.is_nan()
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl SubAssign for Vec3 {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self {
        Self::new(self.x * scalar, self.y * scalar, self.z * scalar)
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;
    fn div(self, scalar: f32) -> Self {
        Self::new(self.x / scalar, self.y / scalar, self.z / scalar)
    }
}

impl Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl From<[f32; 3]> for Vec3 {
    fn from(value: [f32; 3]) -> Self {
        Self::new(value[0], value[1], value[2])
    }
}

impl From<Vec3> for [f32; 3] {
    fn from(value: Vec3) -> Self {
        [value.x, value.y, value.z]
    }
}

/// 两个向量之间的夹角（弧度，0..=PI）。
pub fn angle_between(a: Vec3, b: Vec3) -> f32 {
    let denom = a.magnitude() * b.magnitude();
    if denom <= f32::EPSILON {
        return 0.0;
    }
    (a.dot(&b) / denom).clamp(-1.0, 1.0).acos()
}

/// 把角度规范到 (-PI, PI]。
pub fn wrap_angle(angle: f32) -> f32 {
    let mut wrapped = (angle + PI) % TAU;
    if wrapped <= 0.0 {
        wrapped += TAU;
    }
    wrapped - PI
}

/// 把角度按最短方向朝目标旋转，单次最多 `max_delta` 弧度。
pub fn rotate_towards_angle(current: f32, target: f32, max_delta: f32) -> f32 {
    let diff = wrap_angle(target - current);
    current + diff.clamp(-max_delta.max(0.0), max_delta.max(0.0))
}

/// 把 `current` 朝 `target` 移动最多 `max_delta` 长度。
pub fn move_towards(current: Vec3, target: Vec3, max_delta: f32) -> Vec3 {
    let diff = target - current;
    let distance = diff.magnitude();
    if distance <= max_delta || distance <= f32::EPSILON {
        target
    } else {
        current + diff * (max_delta.max(0.0) / distance)
    }
}

/// 指数平滑系数：`speed <= 0` 时立即到达。
pub fn smoothing_alpha(speed: f32, dt: f32) -> f32 {
    if speed <= 0.0 {
        1.0
    } else {
        1.0 - (-speed * dt).exp()
    }
}
