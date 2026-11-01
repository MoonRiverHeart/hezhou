//! 引擎级资产管线使用的轻量数学类型。
//!
//! 资产 crate 不依赖 `nalgebra`/`glam` 等具体数学库，避免把渲染实现绑进资产层。
//! 这里只定义导入数据契约所需的最小类型：
//!
//! - [`Vec2`] / [`Vec3`] / [`Vec4`]：分量结构体，serde 序列化为 `{x,y,z}`。
//! - [`Quat`]：四元数，分量顺序 `x, y, z, w`，与 glTF 一致。
//! - [`Mat4`]：列主序 4x4 矩阵，与 glTF / 列向量约定一致。
//! - [`Trs`]：平移 + 旋转 + 缩放，可转成 [`Mat4`]。
//!
//! 其它模块（animation / render / player）可以直接使用这些类型，无需引入 glTF
//! crate 或自行定义一份等价结构。

use serde::{Deserialize, Serialize};

/// 二维向量（UV 坐标等）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    /// X 分量。
    pub x: f32,
    /// Y 分量。
    pub y: f32,
}

impl Vec2 {
    /// 构造二维向量。
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// 转成 `[x, y]` 数组。
    pub const fn to_array(self) -> [f32; 2] {
        [self.x, self.y]
    }
}

/// 三维向量（位置、法线、缩放等）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    /// X 分量。
    pub x: f32,
    /// Y 分量。
    pub y: f32,
    /// Z 分量。
    pub z: f32,
}

impl Vec3 {
    /// 三维零向量。
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    /// 三维单位向量。
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);
    /// 上方向。
    pub const UP: Self = Self::new(0.0, 1.0, 0.0);

    /// 构造三维向量。
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// 从 `[f32; 3]` 构造。
    pub const fn from_array(v: [f32; 3]) -> Self {
        Self::new(v[0], v[1], v[2])
    }

    /// 转成 `[f32; 3]` 数组。
    pub const fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    /// 按标量缩放。
    pub fn scaled(self, factor: f32) -> Self {
        Self::new(self.x * factor, self.y * factor, self.z * factor)
    }
}

/// 四维向量（法线切线、颜色、权重等）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec4 {
    /// X 分量。
    pub x: f32,
    /// Y 分量。
    pub y: f32,
    /// Z 分量。
    pub z: f32,
    /// W 分量。
    pub w: f32,
}

impl Vec4 {
    /// 构造四维向量。
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// 从 `[f32; 4]` 构造。
    pub const fn from_array(v: [f32; 4]) -> Self {
        Self::new(v[0], v[1], v[2], v[3])
    }

    /// 转成 `[f32; 4]` 数组。
    pub const fn to_array(self) -> [f32; 4] {
        [self.x, self.y, self.z, self.w]
    }
}

/// 四元数旋转，分量顺序与 glTF 一致：`x, y, z, w`。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quat {
    /// X 分量。
    pub x: f32,
    /// Y 分量。
    pub y: f32,
    /// Z 分量。
    pub z: f32,
    /// W 分量。
    pub w: f32,
}

impl Default for Quat {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Quat {
    /// 单位四元数。
    pub const IDENTITY: Self = Self::new(0.0, 0.0, 0.0, 1.0);

    /// 构造四元数。
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// 从 `[x, y, z, w]` 构造。
    pub const fn from_array(v: [f32; 4]) -> Self {
        Self::new(v[0], v[1], v[2], v[3])
    }

    /// 转成 `[x, y, z, w]` 数组。
    pub const fn to_array(self) -> [f32; 4] {
        [self.x, self.y, self.z, self.w]
    }

    /// 归一化（长度为 0 时返回单位四元数）。
    pub fn normalized(self) -> Self {
        let len_sq = self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w;
        if len_sq <= f32::EPSILON {
            return Self::IDENTITY;
        }
        let inv = 1.0 / len_sq.sqrt();
        Self::new(self.x * inv, self.y * inv, self.z * inv, self.w * inv)
    }

    /// 转成 4x4 旋转矩阵（列主序）。
    pub fn to_mat4(self) -> Mat4 {
        let q = self.normalized();
        let (x, y, z, w) = (q.x, q.y, q.z, q.w);
        let (x2, y2, z2) = (x + x, y + y, z + z);
        let (xx, xy, xz) = (x * x2, x * y2, x * z2);
        let (yy, yz, zz) = (y * y2, y * z2, z * z2);
        let (wx, wy, wz) = (w * x2, w * y2, w * z2);

        let mut m = Mat4::IDENTITY;
        m.cols[0][0] = 1.0 - (yy + zz);
        m.cols[0][1] = xy + wz;
        m.cols[0][2] = xz - wy;
        m.cols[1][0] = xy - wz;
        m.cols[1][1] = 1.0 - (xx + zz);
        m.cols[1][2] = yz + wx;
        m.cols[2][0] = xz + wy;
        m.cols[2][1] = yz - wx;
        m.cols[2][2] = 1.0 - (xx + yy);
        m
    }
}

/// 列主序 4x4 矩阵。
///
/// `cols[i]` 是第 i 列，`cols[i][j]` 是第 i 列第 j 行；`m * v` 使用列向量约定，
/// 与 glTF 的 `matrix` 字段布局一致。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mat4 {
    /// 列数组，每列 4 个分量。
    pub cols: [[f32; 4]; 4],
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mat4 {
    /// 单位矩阵。
    pub const IDENTITY: Self = Self {
        cols: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };

    /// 从列主序的 16 个 f32 构造（与 glTF `matrix` 顺序一致）。
    pub const fn from_cols_array(v: [f32; 16]) -> Self {
        Self {
            cols: [
                [v[0], v[1], v[2], v[3]],
                [v[4], v[5], v[6], v[7]],
                [v[8], v[9], v[10], v[11]],
                [v[12], v[13], v[14], v[15]],
            ],
        }
    }

    /// 展开为列主序的 16 个 f32。
    pub const fn to_cols_array(&self) -> [f32; 16] {
        [
            self.cols[0][0],
            self.cols[0][1],
            self.cols[0][2],
            self.cols[0][3],
            self.cols[1][0],
            self.cols[1][1],
            self.cols[1][2],
            self.cols[1][3],
            self.cols[2][0],
            self.cols[2][1],
            self.cols[2][2],
            self.cols[2][3],
            self.cols[3][0],
            self.cols[3][1],
            self.cols[3][2],
            self.cols[3][3],
        ]
    }

    /// 平移矩阵。
    pub fn from_translation(t: Vec3) -> Self {
        let mut m = Self::IDENTITY;
        m.cols[3][0] = t.x;
        m.cols[3][1] = t.y;
        m.cols[3][2] = t.z;
        m
    }

    /// 缩放矩阵。
    pub fn from_scale(s: Vec3) -> Self {
        let mut m = Self::IDENTITY;
        m.cols[0][0] = s.x;
        m.cols[1][1] = s.y;
        m.cols[2][2] = s.z;
        m
    }

    /// 矩阵乘法 `self * rhs`。
    pub fn mul_mat4(&self, rhs: &Mat4) -> Mat4 {
        let mut out = Mat4::IDENTITY;
        for (c, out_col) in out.cols.iter_mut().enumerate() {
            for (r, out_val) in out_col.iter_mut().enumerate() {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += self.cols[k][r] * rhs.cols[c][k];
                }
                *out_val = sum;
            }
        }
        out
    }

    /// 变换点（w = 1，包含平移）。
    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        let mut out = Vec3::ZERO;
        let v = [p.x, p.y, p.z, 1.0];
        let mut res = [0.0f32; 4];
        for (r, res_val) in res.iter_mut().enumerate() {
            let mut sum = 0.0;
            for (k, vk) in v.iter().enumerate() {
                sum += self.cols[k][r] * vk;
            }
            *res_val = sum;
        }
        out.x = res[0];
        out.y = res[1];
        out.z = res[2];
        out
    }
}

/// 平移 + 旋转 + 缩放（TRS）局部变换。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Trs {
    /// 平移分量。
    pub translation: Vec3,
    /// 旋转分量。
    pub rotation: Quat,
    /// 缩放分量。
    pub scale: Vec3,
}

impl Default for Trs {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Trs {
    /// 单位变换。
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    /// 组合成 `T * R * S` 矩阵。
    pub fn to_mat4(&self) -> Mat4 {
        let t = Mat4::from_translation(self.translation);
        let r = self.rotation.to_mat4();
        let s = Mat4::from_scale(self.scale);
        t.mul_mat4(&r).mul_mat4(&s)
    }
}

/// 轴对齐包围盒（导入期计算，供剔除/cook 使用）。
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Aabb {
    /// 最小角点。
    pub min: Vec3,
    /// 最大角点。
    pub max: Vec3,
}

impl Aabb {
    /// 空包围盒（min = +inf, max = -inf，便于逐点 expand）。
    pub const EMPTY: Self = Self {
        min: Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
        max: Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
    };

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z
    }

    /// 扩展包围盒以包含一个点。
    pub fn expand_point(&mut self, p: Vec3) {
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.min.z = self.min.z.min(p.z);
        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
        self.max.z = self.max.z.max(p.z);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quaternion_identity_is_identity_matrix() {
        let m = Quat::IDENTITY.to_mat4();
        assert_eq!(m, Mat4::IDENTITY);
    }

    #[test]
    fn trs_translation_applies_to_point() {
        let trs = Trs {
            translation: Vec3::new(1.0, 2.0, 3.0),
            ..Trs::IDENTITY
        };
        let p = trs.to_mat4().transform_point(Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(p, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn mat4_cols_array_roundtrip() {
        let arr: [f32; 16] = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];
        assert_eq!(Mat4::from_cols_array(arr).to_cols_array(), arr);
    }

    #[test]
    fn aabb_expand_and_empty() {
        let mut aabb = Aabb::EMPTY;
        assert!(aabb.is_empty());
        aabb.expand_point(Vec3::new(1.0, -2.0, 0.5));
        aabb.expand_point(Vec3::new(-1.0, 2.0, 0.0));
        assert!(!aabb.is_empty());
        assert_eq!(aabb.min, Vec3::new(-1.0, -2.0, 0.0));
        assert_eq!(aabb.max, Vec3::new(1.0, 2.0, 0.5));
    }

    #[test]
    fn quat_normalized_handles_zero() {
        assert_eq!(Quat::new(0.0, 0.0, 0.0, 0.0).normalized(), Quat::IDENTITY);
    }
}
