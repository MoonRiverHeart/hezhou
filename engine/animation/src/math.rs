//! 最小线性代数实现（Vec3 / Quat / Mat4 / Transform）。
//!
//! 设计说明：
//! - 本 crate 刻意不依赖 `hezhou-core`，以便 `cargo test -p hezhou-animation`
//!   离线独立运行；数值语义与 `core::math` 保持一致（列主序 mat4、右手系）。
//! - 与渲染层交互时请重点确认矩阵布局：`Mat4::cols[i]` 是**第 i 列**，
//!   与 Vulkan/GLSL 的 column-major `mat4` 内存布局一致。
//! - 所有角度类 API 使用**弧度**；度制只在便捷构造器 `Quat::from_euler_degrees` 中出现。

use std::ops::{Add, Mul, Neg, Sub};

/// 浮点比较容差（动画/蒙皮场景够用）。
pub const EPSILON: f32 = 1e-6;

/// 三维向量。
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

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn splat(v: f32) -> Self {
        Self::new(v, v, v)
    }

    #[inline]
    pub fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    #[inline]
    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    #[inline]
    pub fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// 归一化；零向量返回 `Vec3::ZERO`（不产生 NaN）。
    #[inline]
    pub fn normalize_or_zero(self) -> Self {
        let len = self.length();
        if len > EPSILON {
            Self::new(self.x / len, self.y / len, self.z / len)
        } else {
            Self::ZERO
        }
    }

    /// 线性插值：`t=0` 返回 `self`，`t=1` 返回 `rhs`。
    #[inline]
    pub fn lerp(self, rhs: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self::new(
            self.x + (rhs.x - self.x) * t,
            self.y + (rhs.y - self.y) * t,
            self.z + (rhs.z - self.z) * t,
        )
    }

    #[inline]
    pub fn approx_eq(self, rhs: Self, eps: f32) -> bool {
        (self.x - rhs.x).abs() <= eps
            && (self.y - rhs.y).abs() <= eps
            && (self.z - rhs.z).abs() <= eps
    }
}

impl Add for Vec3 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Neg for Vec3 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

/// 四元数 `(x, y, z, w)`，表示旋转；默认约定为单位四元数。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Default for Quat {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Quat {
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// 由 `(x, y, z, w)` 构造并归一化；零四元数退化为单位四元数。
    pub fn from_xyzw_normalized(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self::new(x, y, z, w).normalize_or_identity()
    }

    /// 绕任意轴旋转 `radians` 弧度。
    pub fn from_axis_angle(axis: Vec3, radians: f32) -> Self {
        let axis = axis.normalize_or_zero();
        if axis.length_squared() <= EPSILON {
            return Self::IDENTITY;
        }
        let half = radians * 0.5;
        let s = half.sin();
        Self::new(axis.x * s, axis.y * s, axis.z * s, half.cos()).normalize_or_identity()
    }

    /// 欧拉角（度）构造：先绕 Y、再绕 X、再绕 Z（YXZ 顺序，符合编辑器习惯）。
    pub fn from_euler_degrees(x_deg: f32, y_deg: f32, z_deg: f32) -> Self {
        let to_rad = std::f32::consts::PI / 180.0;
        let qx = Self::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), x_deg * to_rad);
        let qy = Self::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), y_deg * to_rad);
        let qz = Self::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), z_deg * to_rad);
        qy * qx * qz
    }

    #[inline]
    pub fn to_array(self) -> [f32; 4] {
        [self.x, self.y, self.z, self.w]
    }

    #[inline]
    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z + self.w * rhs.w
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    /// 归一化；长度过小返回单位四元数。
    pub fn normalize_or_identity(self) -> Self {
        let len = self.length();
        if len > EPSILON {
            let inv = 1.0 / len;
            Self::new(self.x * inv, self.y * inv, self.z * inv, self.w * inv)
        } else {
            Self::IDENTITY
        }
    }

    #[inline]
    pub fn conjugate(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, self.w)
    }

    #[inline]
    pub fn inverse(self) -> Self {
        // 单位四元数的逆等于共轭；非单位时按长度平方缩放。
        let len_sq = self.dot(self);
        if len_sq <= EPSILON {
            return Self::IDENTITY;
        }
        let inv = 1.0 / len_sq;
        Self::new(-self.x * inv, -self.y * inv, -self.z * inv, self.w * inv)
    }

    /// 四元数乘法（Hamilton 积）：`self * rhs` 表示先应用 `rhs` 再应用 `self`。
    pub fn mul_quat(self, rhs: Self) -> Self {
        Self::new(
            self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
            self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
        )
    }

    /// 球面插值（最短路径）。`t=0` 返回 `a`，`t=1` 返回 `b`。
    pub fn slerp(a: Self, b: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let a = a.normalize_or_identity();
        let mut b = b.normalize_or_identity();
        let mut dot = a.dot(b);
        if dot < 0.0 {
            b = Self::new(-b.x, -b.y, -b.z, -b.w);
            dot = -dot;
        }
        // 夹角非常小时退化为归一化线性插值，避免 sin(0) 除零。
        if dot > 0.9995 {
            return Self::new(
                a.x + (b.x - a.x) * t,
                a.y + (b.y - a.y) * t,
                a.z + (b.z - a.z) * t,
                a.w + (b.w - a.w) * t,
            )
            .normalize_or_identity();
        }
        let theta_0 = dot.clamp(-1.0, 1.0).acos();
        let sin_theta_0 = theta_0.sin();
        let theta = theta_0 * t;
        let s0 = (theta_0 - theta).sin() / sin_theta_0;
        let s1 = theta.sin() / sin_theta_0;
        Self::new(
            a.x * s0 + b.x * s1,
            a.y * s0 + b.y * s1,
            a.z * s0 + b.z * s1,
            a.w * s0 + b.w * s1,
        )
        .normalize_or_identity()
    }

    /// 把向量按该旋转变换：`v' = v + 2 * cross(qv, cross(qv, v) + w * v)`。
    pub fn rotate_vec3(self, v: Vec3) -> Vec3 {
        let q = self.normalize_or_identity();
        let qv = Vec3::new(q.x, q.y, q.z);
        let t = qv.cross(v) * 2.0;
        v + t * q.w + qv.cross(t)
    }

    /// 转换为列主序旋转矩阵（不含平移/缩放）。
    pub fn to_mat4(self) -> Mat4 {
        let q = self.normalize_or_identity();
        let (x, y, z, w) = (q.x, q.y, q.z, q.w);
        let xx = x * x;
        let yy = y * y;
        let zz = z * z;
        let xy = x * y;
        let xz = x * z;
        let yz = y * z;
        let wx = w * x;
        let wy = w * y;
        let wz = w * z;
        let mut m = Mat4::IDENTITY;
        m.cols[0] = [1.0 - 2.0 * (yy + zz), 2.0 * (xy + wz), 2.0 * (xz - wy), 0.0];
        m.cols[1] = [2.0 * (xy - wz), 1.0 - 2.0 * (xx + zz), 2.0 * (yz + wx), 0.0];
        m.cols[2] = [2.0 * (xz + wy), 2.0 * (yz - wx), 1.0 - 2.0 * (xx + yy), 0.0];
        m.cols[3] = [0.0, 0.0, 0.0, 1.0];
        m
    }

    pub fn approx_eq(self, rhs: Self, eps: f32) -> bool {
        // 允许 q 与 -q 表示同一旋转。
        let direct = (self.x - rhs.x).abs() <= eps
            && (self.y - rhs.y).abs() <= eps
            && (self.z - rhs.z).abs() <= eps
            && (self.w - rhs.w).abs() <= eps;
        let negated = (self.x + rhs.x).abs() <= eps
            && (self.y + rhs.y).abs() <= eps
            && (self.z + rhs.z).abs() <= eps
            && (self.w + rhs.w).abs() <= eps;
        direct || negated
    }
}

impl Mul for Quat {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        self.mul_quat(rhs)
    }
}

/// 列主序 4x4 矩阵：`cols[i]` 为第 i 列（与 GLSL `mat4` 内存布局一致）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4 {
    pub cols: [[f32; 4]; 4],
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mat4 {
    pub const IDENTITY: Self = Self {
        cols: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };

    #[inline]
    pub const fn from_cols(cols: [[f32; 4]; 4]) -> Self {
        Self { cols }
    }

    pub fn from_translation(t: Vec3) -> Self {
        let mut m = Self::IDENTITY;
        m.cols[3] = [t.x, t.y, t.z, 1.0];
        m
    }

    pub fn from_scale(s: Vec3) -> Self {
        let mut m = Self::IDENTITY;
        m.cols[0][0] = s.x;
        m.cols[1][1] = s.y;
        m.cols[2][2] = s.z;
        m
    }

    pub fn from_quat(q: Quat) -> Self {
        q.to_mat4()
    }

    /// 由 T/R/S 构造 `T * R * S` 矩阵。
    pub fn from_trs(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        let r = rotation.to_mat4();
        let mut m = Mat4::IDENTITY;
        for c in 0..3 {
            m.cols[c][0] = r.cols[c][0] * scale.x;
            m.cols[c][1] = r.cols[c][1] * scale.y;
            m.cols[c][2] = r.cols[c][2] * scale.z;
            m.cols[c][3] = 0.0;
        }
        m.cols[3] = [translation.x, translation.y, translation.z, 1.0];
        m
    }

    /// 矩阵乘法：`self * rhs`（右乘，符合列向量约定）。
    pub fn mul_mat4(&self, rhs: &Mat4) -> Mat4 {
        let mut out = Mat4::IDENTITY;
        for c in 0..4 {
            let r = rhs.cols[c];
            for row in 0..4 {
                out.cols[c][row] = self.cols[0][row] * r[0]
                    + self.cols[1][row] * r[1]
                    + self.cols[2][row] * r[2]
                    + self.cols[3][row] * r[3];
            }
        }
        out
    }

    /// 变换点（`w=1`，含平移）。
    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        let x =
            self.cols[0][0] * p.x + self.cols[1][0] * p.y + self.cols[2][0] * p.z + self.cols[3][0];
        let y =
            self.cols[0][1] * p.x + self.cols[1][1] * p.y + self.cols[2][1] * p.z + self.cols[3][1];
        let z =
            self.cols[0][2] * p.x + self.cols[1][2] * p.y + self.cols[2][2] * p.z + self.cols[3][2];
        let w =
            self.cols[0][3] * p.x + self.cols[1][3] * p.y + self.cols[2][3] * p.z + self.cols[3][3];
        if w.abs() > EPSILON && (w - 1.0).abs() > EPSILON {
            Vec3::new(x / w, y / w, z / w)
        } else {
            Vec3::new(x, y, z)
        }
    }

    /// 变换方向（`w=0`，不含平移）。
    pub fn transform_vector(&self, v: Vec3) -> Vec3 {
        Vec3::new(
            self.cols[0][0] * v.x + self.cols[1][0] * v.y + self.cols[2][0] * v.z,
            self.cols[0][1] * v.x + self.cols[1][1] * v.y + self.cols[2][1] * v.z,
            self.cols[0][2] * v.x + self.cols[1][2] * v.y + self.cols[2][2] * v.z,
        )
    }

    /// 3x3 线性部分（用于法线/切线变换）。
    pub fn linear3(&self) -> [[f32; 3]; 3] {
        [
            [self.cols[0][0], self.cols[0][1], self.cols[0][2]],
            [self.cols[1][0], self.cols[1][1], self.cols[1][2]],
            [self.cols[2][0], self.cols[2][1], self.cols[2][2]],
        ]
    }

    pub fn transpose(&self) -> Mat4 {
        let mut out = Mat4::IDENTITY;
        for (c, column) in out.cols.iter_mut().enumerate() {
            for (r, value) in column.iter_mut().enumerate() {
                *value = self.cols[r][c];
            }
        }
        out
    }

    /// 通用 4x4 求逆（Gauss-Jordan，部分主元）；奇异矩阵返回 `None`。
    // 说明：Gauss-Jordan 消元天然需要行列双下标，这里保留索引循环以便对照算法。
    #[allow(clippy::needless_range_loop)]
    pub fn inverse(&self) -> Option<Mat4> {
        // 行主序增广矩阵 [M | I]
        let mut m = [[0.0f32; 8]; 4];
        for r in 0..4 {
            for c in 0..4 {
                m[r][c] = self.cols[c][r];
                m[r][c + 4] = if r == c { 1.0 } else { 0.0 };
            }
        }
        for col in 0..4 {
            let mut pivot = col;
            for r in (col + 1)..4 {
                if m[r][col].abs() > m[pivot][col].abs() {
                    pivot = r;
                }
            }
            if m[pivot][col].abs() < 1e-8 {
                return None;
            }
            m.swap(col, pivot);
            let inv_pivot = 1.0 / m[col][col];
            for c in 0..8 {
                m[col][c] *= inv_pivot;
            }
            for r in 0..4 {
                if r == col {
                    continue;
                }
                let factor = m[r][col];
                if factor.abs() <= f32::EPSILON {
                    continue;
                }
                for c in 0..8 {
                    m[r][c] -= factor * m[col][c];
                }
            }
        }
        let mut out = Mat4::IDENTITY;
        for r in 0..4 {
            for c in 0..4 {
                out.cols[c][r] = m[r][c + 4];
            }
        }
        Some(out)
    }

    /// 平铺为 16 个 f32（列主序），可直接上传 GPU。
    pub fn to_cols_array(&self) -> [f32; 16] {
        let mut out = [0.0f32; 16];
        for (c, column) in self.cols.iter().enumerate() {
            out[c * 4..c * 4 + 4].copy_from_slice(column);
        }
        out
    }

    pub fn approx_eq(&self, rhs: &Mat4, eps: f32) -> bool {
        self.cols
            .iter()
            .zip(rhs.cols.iter())
            .flat_map(|(a, b)| a.iter().zip(b.iter()))
            .all(|(a, b)| (a - b).abs() <= eps)
    }
}

impl Mul for Mat4 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        self.mul_mat4(&rhs)
    }
}

/// 局部变换（TRS）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    pub fn new(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// 由平移/旋转构造（缩放为 1）。
    pub fn from_trs(translation: Vec3, rotation: Quat) -> Self {
        Self {
            translation,
            rotation,
            scale: Vec3::ONE,
        }
    }

    /// 转换为局部矩阵 `T * R * S`。
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_trs(self.translation, self.rotation, self.scale)
    }

    /// TRS 线性插值：平移/缩放用 lerp，旋转用 slerp。
    pub fn lerp(a: &Transform, b: &Transform, t: f32) -> Transform {
        let t = t.clamp(0.0, 1.0);
        Transform {
            translation: a.translation.lerp(b.translation, t),
            rotation: Quat::slerp(a.rotation, b.rotation, t),
            scale: a.scale.lerp(b.scale, t),
        }
    }

    pub fn approx_eq(&self, rhs: &Transform, eps: f32) -> bool {
        self.translation.approx_eq(rhs.translation, eps)
            && self.rotation.approx_eq(rhs.rotation, eps)
            && self.scale.approx_eq(rhs.scale, eps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mat4_inverse_roundtrip() {
        let m = Mat4::from_trs(
            Vec3::new(1.0, 2.0, 3.0),
            Quat::from_axis_angle(Vec3::UP, 0.7),
            Vec3::new(2.0, 0.5, 1.5),
        );
        let inv = m.inverse().expect("TRS 矩阵可逆");
        let identity = m * inv;
        assert!(identity.approx_eq(&Mat4::IDENTITY, 1e-4));
    }

    #[test]
    fn quat_slerp_endpoints_and_midpoint() {
        let a = Quat::IDENTITY;
        let b = Quat::from_axis_angle(Vec3::UP, std::f32::consts::FRAC_PI_2);
        assert!(Quat::slerp(a, b, 0.0).approx_eq(a, 1e-5));
        assert!(Quat::slerp(a, b, 1.0).approx_eq(b, 1e-5));
        let mid = Quat::slerp(a, b, 0.5);
        let expected = Quat::from_axis_angle(Vec3::UP, std::f32::consts::FRAC_PI_4);
        assert!(mid.approx_eq(expected, 1e-4));
    }
}
