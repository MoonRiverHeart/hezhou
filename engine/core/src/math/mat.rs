use crate::math::{Quaternion, Vec3, Vec4};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Mat4 {
    pub data: [[f32; 4]; 4],
}

impl Mat4 {
    pub fn identity() -> Self {
        Self {
            data: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn zero() -> Self {
        Self {
            data: [[0.0; 4]; 4],
        }
    }

    pub fn translate(v: Vec3) -> Self {
        Self {
            data: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [v.x, v.y, v.z, 1.0],
            ],
        }
    }

    pub fn scale(v: Vec3) -> Self {
        Self {
            data: [
                [v.x, 0.0, 0.0, 0.0],
                [0.0, v.y, 0.0, 0.0],
                [0.0, 0.0, v.z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn from_quaternion(q: Quaternion) -> Self {
        let x2 = q.x + q.x;
        let y2 = q.y + q.y;
        let z2 = q.z + q.z;

        let xx = q.x * x2;
        let xy = q.x * y2;
        let xz = q.x * z2;
        let yy = q.y * y2;
        let yz = q.y * z2;
        let zz = q.z * z2;
        let wx = q.w * x2;
        let wy = q.w * y2;
        let wz = q.w * z2;

        Self {
            data: [
                [1.0 - yy - zz, xy + wz, xz - wy, 0.0],
                [xy - wz, 1.0 - xx - zz, yz + wx, 0.0],
                [xz + wy, yz - wx, 1.0 - xx - yy, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let forward = (target - eye).normalized();
        let right = Vec3::cross(&forward, &up).normalized();
        let up = Vec3::cross(&right, &forward);

        Self {
            data: [
                [right.x, up.x, -forward.x, 0.0],
                [right.y, up.y, -forward.y, 0.0],
                [right.z, up.z, -forward.z, 0.0],
                [
                    -Vec3::dot(&right, &eye),
                    -Vec3::dot(&up, &eye),
                    Vec3::dot(&forward, &eye),
                    1.0,
                ],
            ],
        }
    }

    pub fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> Self {
        let tan_half_fov = (fov * 0.5).tan();
        let range = near - far;

        Self {
            data: [
                [1.0 / (aspect * tan_half_fov), 0.0, 0.0, 0.0],
                [0.0, 1.0 / tan_half_fov, 0.0, 0.0],
                [0.0, 0.0, (-near - far) / range, 1.0],
                [0.0, 0.0, 2.0 * far * near / range, 0.0],
            ],
        }
    }

    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        Self {
            data: [
                [2.0 / (right - left), 0.0, 0.0, 0.0],
                [0.0, 2.0 / (top - bottom), 0.0, 0.0],
                [0.0, 0.0, -2.0 / (far - near), 0.0],
                [
                    -(right + left) / (right - left),
                    -(top + bottom) / (top - bottom),
                    -(far + near) / (far - near),
                    1.0,
                ],
            ],
        }
    }

    pub fn multiply(a: &Self, b: &Self) -> Self {
        let mut result = Self::zero();

        for i in 0..4 {
            for j in 0..4 {
                result.data[i][j] = a.data[i][0] * b.data[0][j]
                    + a.data[i][1] * b.data[1][j]
                    + a.data[i][2] * b.data[2][j]
                    + a.data[i][3] * b.data[3][j];
            }
        }

        result
    }

    pub fn transform_point(&self, v: Vec3) -> Vec3 {
        Vec3::new(
            self.data[0][0] * v.x + self.data[1][0] * v.y + self.data[2][0] * v.z + self.data[3][0],
            self.data[0][1] * v.x + self.data[1][1] * v.y + self.data[2][1] * v.z + self.data[3][1],
            self.data[0][2] * v.x + self.data[1][2] * v.y + self.data[2][2] * v.z + self.data[3][2],
        )
    }
}

impl std::ops::Mul for Mat4 {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        Mat4::multiply(&self, &other)
    }
}

impl std::ops::Mul<Vec4> for Mat4 {
    type Output = Vec4;
    fn mul(self, v: Vec4) -> Vec4 {
        Vec4::new(
            self.data[0][0] * v.x
                + self.data[0][1] * v.y
                + self.data[0][2] * v.z
                + self.data[0][3] * v.w,
            self.data[1][0] * v.x
                + self.data[1][1] * v.y
                + self.data[1][2] * v.z
                + self.data[1][3] * v.w,
            self.data[2][0] * v.x
                + self.data[2][1] * v.y
                + self.data[2][2] * v.z
                + self.data[2][3] * v.w,
            self.data[3][0] * v.x
                + self.data[3][1] * v.y
                + self.data[3][2] * v.z
                + self.data[3][3] * v.w,
        )
    }
}
