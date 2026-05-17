#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0)
    }

    pub fn one() -> Self {
        Self::new(1.0, 1.0)
    }

    pub fn magnitude(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalized(&self) -> Self {
        let mag = self.magnitude();
        if mag > 0.0 {
            Self::new(self.x / mag, self.y / mag)
        } else {
            Self::zero()
        }
    }

    pub fn dot(a: &Self, b: &Self) -> f32 {
        a.x * b.x + a.y * b.y
    }

    pub fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        Self::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self {
        Self::new(self.x * scalar, self.y * scalar)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    pub fn one() -> Self {
        Self::new(1.0, 1.0, 1.0)
    }

    pub fn up() -> Self {
        Self::new(0.0, 1.0, 0.0)
    }

    pub fn down() -> Self {
        Self::new(0.0, -1.0, 0.0)
    }

    pub fn forward() -> Self {
        Self::new(0.0, 0.0, -1.0)
    }

    pub fn back() -> Self {
        Self::new(0.0, 0.0, 1.0)
    }

    pub fn right() -> Self {
        Self::new(1.0, 0.0, 0.0)
    }

    pub fn left() -> Self {
        Self::new(-1.0, 0.0, 0.0)
    }

    pub fn magnitude(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalized(&self) -> Self {
        let mag = self.magnitude();
        if mag > 0.0 {
            Self::new(self.x / mag, self.y / mag, self.z / mag)
        } else {
            Self::zero()
        }
    }

    pub fn dot(a: &Self, b: &Self) -> f32 {
        a.x * b.x + a.y * b.y + a.z * b.z
    }

    pub fn cross(a: &Self, b: &Self) -> Self {
        Self::new(
            a.y * b.z - a.z * b.y,
            a.z * b.x - a.x * b.z,
            a.x * b.y - a.y * b.x,
        )
    }

    pub fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        Self::new(
            a.x + (b.x - a.x) * t,
            a.y + (b.y - a.y) * t,
            a.z + (b.z - a.z) * t,
        )
    }

    pub fn distance(a: &Self, b: &Self) -> f32 {
        (*a - *b).magnitude()
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
        Self::new(self.x * scalar, self.y * scalar, self.z * scalar)
    }
}

impl std::ops::Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn from_vec3(v: Vec3, w: f32) -> Self {
        Self::new(v.x, v.y, v.z, w)
    }

    pub fn to_vec3(&self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Vec2u {
    pub x: u32,
    pub y: u32,
}

impl Vec2u {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self::new(0, 0)
    }

    pub fn width(&self) -> u32 {
        self.x
    }

    pub fn height(&self) -> u32 {
        self.y
    }
}

impl std::ops::Add for Vec2u {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Vec3u {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl Vec3u {
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self::new(0, 0, 0)
    }

    pub fn width(&self) -> u32 {
        self.x
    }

    pub fn height(&self) -> u32 {
        self.y
    }

    pub fn depth(&self) -> u32 {
        self.z
    }
}

impl std::ops::Add for Vec3u {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl From<[f32; 3]> for Vec3 {
    fn from(arr: [f32; 3]) -> Self {
        Self::new(arr[0], arr[1], arr[2])
    }
}

impl From<Vec3> for [f32; 3] {
    fn from(v: Vec3) -> Self {
        [v.x, v.y, v.z]
    }
}

impl From<[f32; 2]> for Vec2 {
    fn from(arr: [f32; 2]) -> Self {
        Self::new(arr[0], arr[1])
    }
}

impl From<Vec2> for [f32; 2] {
    fn from(v: Vec2) -> Self {
        [v.x, v.y]
    }
}

impl From<[f32; 4]> for Vec4 {
    fn from(arr: [f32; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }
}

impl From<Vec4> for [f32; 4] {
    fn from(v: Vec4) -> Self {
        [v.x, v.y, v.z, v.w]
    }
}
