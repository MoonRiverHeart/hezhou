//! 3D 空间音频：监听者、衰减模型、声像（pan）。
//!
//! 坐标约定与引擎其余模块保持一致：右手系，`+X` 向右、`+Y` 向上、`-Z` 向前
//! （[`Listener::default`] 的朝向）。
//!
//! 距离衰减在每一段渲染回调开始时按当前监听者/声源位置计算一次，
//! 具体公式见 [`Attenuation::gain_at`]。

/// 三维向量（不引入外部数学库，保持 crate 零依赖）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    /// X 分量。
    pub x: f32,
    /// Y 分量。
    pub y: f32,
    /// Z 分量。
    pub z: f32,
}

impl Vec3 {
    /// 构造向量。
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// 零向量。
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    /// 标量乘法。
    pub fn scale(self, scalar: f32) -> Self {
        Self::new(self.x * scalar, self.y * scalar, self.z * scalar)
    }

    /// 点积。
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// 叉积。
    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// 长度。
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    /// 归一化（零向量返回自身，避免 NaN）。
    pub fn normalized(self) -> Self {
        let length = self.length();
        if length <= f32::EPSILON {
            Self::ZERO
        } else {
            self.scale(1.0 / length)
        }
    }

    /// 两点距离。
    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self {
        self.scale(scalar)
    }
}

/// 监听者（通常是玩家相机）：决定 3D 声源的距离衰减与左右声像。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Listener {
    /// 世界坐标位置。
    pub position: Vec3,
    /// 朝向（单位向量，默认 -Z）。
    pub forward: Vec3,
    /// 上方向（单位向量，默认 +Y）。
    pub up: Vec3,
}

impl Default for Listener {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            forward: Vec3::new(0.0, 0.0, -1.0),
            up: Vec3::new(0.0, 1.0, 0.0),
        }
    }
}

impl Listener {
    /// 构造监听者。
    pub fn new(position: Vec3, forward: Vec3, up: Vec3) -> Self {
        Self {
            position,
            forward,
            up,
        }
    }

    /// 右方向 = forward × up（归一化；退化时回退到 +X）。
    pub fn right(&self) -> Vec3 {
        let right = self
            .forward
            .normalized()
            .cross(self.up.normalized())
            .normalized();
        if right == Vec3::ZERO {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            right
        }
    }
}

/// 距离衰减模型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AttenuationModel {
    /// 不衰减（等价于 2D 播放）。
    None,
    /// 线性：从 `min_distance` 到 `max_distance` 线性衰减到 0。
    Linear,
    /// 反比（默认）：经典游戏 3D 音效模型，近距离平缓、远距离持续衰减。
    #[default]
    Inverse,
    /// 指数：按 `rolloff` 次幂衰减，曲线更陡。
    Exponential,
}

/// 距离衰减参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attenuation {
    /// 衰减模型。
    pub model: AttenuationModel,
    /// 近场距离：小于等于该距离时增益恒为 1。
    pub min_distance: f32,
    /// 远场距离：大于等于该距离时增益为 0（硬截止）。
    pub max_distance: f32,
    /// 衰减强度（模型相关，越大衰减越快）。
    pub rolloff: f32,
}

impl Default for Attenuation {
    fn default() -> Self {
        Self {
            model: AttenuationModel::Inverse,
            min_distance: 1.0,
            max_distance: 50.0,
            rolloff: 1.0,
        }
    }
}

impl Attenuation {
    /// 构造衰减参数。
    pub fn new(
        model: AttenuationModel,
        min_distance: f32,
        max_distance: f32,
        rolloff: f32,
    ) -> Self {
        Self {
            model,
            min_distance,
            max_distance,
            rolloff,
        }
    }

    /// 计算给定距离处的增益（`0.0..=1.0`，单调不增）。
    ///
    /// 公式（`d` = 距离，`min` = min_distance，`max` = max_distance）：
    /// - `d <= min` 或模型为 [`AttenuationModel::None`]：1
    /// - `d >= max`：0（硬截止，保证远处音效彻底静音）
    /// - 线性：`1 - t`，其中 `t = (d-min)/(max-min)`
    /// - 反比：`min/(min + rolloff*(d-min))`，再乘线性收尾因子 `(1-t)`
    ///   以保证在 `max` 处连续归零（`min == 0` 时退化为线性）
    /// - 指数：`(1-t)^rolloff`
    pub fn gain_at(&self, distance: f32) -> f32 {
        if self.model == AttenuationModel::None {
            return 1.0;
        }
        let d = distance.max(0.0);
        let min = self.min_distance.max(0.0);
        let max = self.max_distance.max(min + f32::EPSILON);
        if d <= min {
            return 1.0;
        }
        if d >= max {
            return 0.0;
        }
        let t = ((d - min) / (max - min)).clamp(0.0, 1.0);
        let rolloff = self.rolloff.max(0.0001);
        let gain = match self.model {
            AttenuationModel::None => 1.0,
            AttenuationModel::Linear => 1.0 - t,
            AttenuationModel::Inverse => {
                if min <= f32::EPSILON {
                    1.0 - t
                } else {
                    let raw = min / (min + rolloff * (d - min));
                    raw.clamp(0.0, 1.0) * (1.0 - t)
                }
            }
            AttenuationModel::Exponential => (1.0 - t).powf(rolloff),
        };
        gain.clamp(0.0, 1.0)
    }
}

/// 单个 3D 声源的参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialParams {
    /// 声源世界坐标。
    pub position: Vec3,
    /// 距离衰减参数。
    pub attenuation: Attenuation,
}

impl SpatialParams {
    /// 以默认衰减参数创建 3D 声源。
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            attenuation: Attenuation::default(),
        }
    }

    /// 指定衰减参数。
    pub fn with_attenuation(mut self, attenuation: Attenuation) -> Self {
        self.attenuation = attenuation;
        self
    }
}

/// 单次 3D 求值结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialOutput {
    /// 监听者到声源的距离。
    pub distance: f32,
    /// 距离衰减增益（`0..=1`）。
    pub gain: f32,
    /// 左右声像（`-1` 左，`0` 居中，`+1` 右）。
    pub pan: f32,
}

/// 计算 3D 声源相对于监听者的距离、衰减增益与声像。
///
/// 声像由“监听者 -> 声源”方向在监听者右方向上的投影决定：
/// 投影为 +1 表示正右方，-1 表示正左方，0 表示正前/正后（居中）。
pub fn evaluate(listener: &Listener, spatial: &SpatialParams) -> SpatialOutput {
    let offset = spatial.position - listener.position;
    let distance = offset.length();
    let gain = spatial.attenuation.gain_at(distance);
    let direction = offset.normalized();
    let pan = direction.dot(listener.right()).clamp(-1.0, 1.0);
    SpatialOutput {
        distance,
        gain,
        pan,
    }
}

/// 把声像值转换为左右声道增益。
///
/// 采用“等功率（常功率）”法则并归一化到居中增益为 1：
/// - `pan = 0` → `(1.0, 1.0)`
/// - `pan = -1` → `(SQRT_2, 0.0)`
/// - `pan = +1` → `(0.0, SQRT_2)`
///
/// 这样 2D 音效（pan=0）不会被额外衰减，同时移动的 3D 声源总功率基本恒定。
pub fn pan_to_gains(pan: f32) -> (f32, f32) {
    let pan = if pan.is_nan() {
        0.0
    } else {
        pan.clamp(-1.0, 1.0)
    };
    let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
    let left = angle.cos() * std::f32::consts::SQRT_2;
    let right = angle.sin() * std::f32::consts::SQRT_2;
    (left, right)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inverse() -> Attenuation {
        Attenuation::new(AttenuationModel::Inverse, 1.0, 50.0, 1.0)
    }

    #[test]
    fn vec3_basic_math() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert_eq!(a + b, Vec3::new(5.0, 7.0, 9.0));
        assert_eq!(a - b, Vec3::new(-3.0, -3.0, -3.0));
        assert_eq!(a * 2.0, Vec3::new(2.0, 4.0, 6.0));
        assert_eq!(a.scale(2.0), Vec3::new(2.0, 4.0, 6.0));
        assert_eq!(a.dot(b), 32.0);
        assert!((Vec3::new(3.0, 4.0, 0.0).length() - 5.0).abs() < 1e-6);
        assert_eq!(Vec3::ZERO.normalized(), Vec3::ZERO);
        assert!((a.normalized().length() - 1.0).abs() < 1e-6);
        assert!((a.distance(b) - 27.0f32.sqrt()).abs() < 1e-6);
        let cross = Vec3::new(1.0, 0.0, 0.0).cross(Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(cross, Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn listener_default_orientation() {
        let listener = Listener::default();
        assert_eq!(listener.position, Vec3::ZERO);
        assert_eq!(listener.forward, Vec3::new(0.0, 0.0, -1.0));
        // 右手系：-Z 前方、+Y 上方 -> 右方为 +X
        assert_eq!(listener.right(), Vec3::new(1.0, 0.0, 0.0));

        let degenerate = Listener::new(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO);
        assert_eq!(degenerate.right(), Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn attenuation_boundaries_and_monotonicity() {
        let attenuation = inverse();
        assert_eq!(attenuation.gain_at(0.0), 1.0);
        assert_eq!(attenuation.gain_at(1.0), 1.0);
        assert_eq!(attenuation.gain_at(50.0), 0.0);
        assert_eq!(attenuation.gain_at(1_000.0), 0.0);

        let mut previous = 1.0;
        for step in 0..60 {
            let gain = attenuation.gain_at(step as f32);
            assert!(
                gain <= previous + 1e-6,
                "增益必须单调不增: d={step} gain={gain} previous={previous}"
            );
            assert!((0.0..=1.0).contains(&gain));
            previous = gain;
        }
        // 中间距离衰减到 (0, 1) 之间
        let mid = attenuation.gain_at(25.0);
        assert!(mid > 0.0 && mid < 1.0);
    }

    #[test]
    fn attenuation_models_differ_and_rolloff_matters() {
        let linear = Attenuation::new(AttenuationModel::Linear, 1.0, 50.0, 1.0);
        assert_eq!(linear.gain_at(1.0), 1.0);
        assert!((linear.gain_at(25.5) - 0.5).abs() < 1e-5);
        assert_eq!(linear.gain_at(50.0), 0.0);

        let none = Attenuation::new(AttenuationModel::None, 1.0, 50.0, 1.0);
        assert_eq!(none.gain_at(49.0), 1.0);

        let exponential = Attenuation::new(AttenuationModel::Exponential, 1.0, 50.0, 2.0);
        let exponential2 = Attenuation::new(AttenuationModel::Exponential, 1.0, 50.0, 0.5);
        assert!(exponential.gain_at(25.0) < exponential2.gain_at(25.0));

        let hard = Attenuation::new(AttenuationModel::Inverse, 1.0, 50.0, 4.0);
        assert!(hard.gain_at(10.0) < inverse().gain_at(10.0));
    }

    #[test]
    fn inverse_with_zero_min_distance_degrades_to_linear() {
        let attenuation = Attenuation::new(AttenuationModel::Inverse, 0.0, 10.0, 1.0);
        assert_eq!(attenuation.gain_at(0.0), 1.0);
        assert!((attenuation.gain_at(5.0) - 0.5).abs() < 1e-5);
        assert_eq!(attenuation.gain_at(10.0), 0.0);
    }

    #[test]
    fn evaluate_distance_gain_and_pan() {
        let listener = Listener::default();

        // 正右侧：pan = +1
        let right = evaluate(&listener, &SpatialParams::new(Vec3::new(10.0, 0.0, 0.0)));
        assert!((right.distance - 10.0).abs() < 1e-6);
        assert!((right.pan - 1.0).abs() < 1e-6);
        assert!(right.gain > 0.0 && right.gain < 1.0);

        // 正左侧：pan = -1
        let left = evaluate(&listener, &SpatialParams::new(Vec3::new(-10.0, 0.0, 0.0)));
        assert!((left.pan + 1.0).abs() < 1e-6);

        // 正前方（-Z）：pan = 0，距离 5
        let front = evaluate(&listener, &SpatialParams::new(Vec3::new(0.0, 0.0, -5.0)));
        assert!(front.pan.abs() < 1e-6);
        assert!((front.distance - 5.0).abs() < 1e-6);

        // 监听者位置上的声源：距离 0、增益 1、pan 0
        let at_origin = evaluate(&listener, &SpatialParams::new(Vec3::ZERO));
        assert_eq!(at_origin.distance, 0.0);
        assert_eq!(at_origin.gain, 1.0);
        assert_eq!(at_origin.pan, 0.0);

        // 超过 max_distance：完全静音
        let far = evaluate(&listener, &SpatialParams::new(Vec3::new(0.0, 0.0, -100.0)));
        assert_eq!(far.gain, 0.0);
    }

    #[test]
    fn listener_rotation_changes_pan() {
        // 监听者朝 +X 看，此时声源在 -Z（原前方）应为右方（+1）？
        // 朝 +X、上 +Y：forward×up = (1,0,0)×(0,1,0) = (0,0,1)，右方为 +Z。
        let listener = Listener::new(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        assert_eq!(listener.right(), Vec3::new(0.0, 0.0, 1.0));
        let output = evaluate(&listener, &SpatialParams::new(Vec3::new(0.0, 0.0, 10.0)));
        assert!((output.pan - 1.0).abs() < 1e-6);
    }

    #[test]
    fn pan_to_gains_is_equal_power_and_center_unity() {
        let (left, right) = pan_to_gains(0.0);
        assert!((left - 1.0).abs() < 1e-6);
        assert!((right - 1.0).abs() < 1e-6);
        assert!((left * left + right * right - 2.0).abs() < 1e-6);

        let (left, right) = pan_to_gains(-1.0);
        assert!((left - std::f32::consts::SQRT_2).abs() < 1e-6);
        assert!(right.abs() < 1e-6);

        let (left, right) = pan_to_gains(1.0);
        assert!(left.abs() < 1e-6);
        assert!((right - std::f32::consts::SQRT_2).abs() < 1e-6);

        let (left, right) = pan_to_gains(f32::NAN);
        assert!((left - 1.0).abs() < 1e-6 && (right - 1.0).abs() < 1e-6);
    }
}
