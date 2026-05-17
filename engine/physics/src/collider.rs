use hezhou_core::math::Vec3;

#[derive(Clone, Debug)]
pub struct Collider {
    shape: ColliderShape,
    offset: Vec3,
    friction: f32,
    restitution: f32,
    is_sensor: bool,
}

impl Collider {
    pub fn new(shape: ColliderShape) -> Self {
        Self {
            shape,
            offset: Vec3::zero(),
            friction: 0.5,
            restitution: 0.0,
            is_sensor: false,
        }
    }

    pub fn shape(&self) -> &ColliderShape {
        &self.shape
    }

    pub fn offset(&self) -> &Vec3 {
        &self.offset
    }

    pub fn set_offset(&mut self, offset: Vec3) {
        self.offset = offset;
    }

    pub fn friction(&self) -> f32 {
        self.friction
    }

    pub fn set_friction(&mut self, friction: f32) {
        self.friction = friction;
    }

    pub fn restitution(&self) -> f32 {
        self.restitution
    }

    pub fn set_restitution(&mut self, restitution: f32) {
        self.restitution = restitution;
    }

    pub fn is_sensor(&self) -> bool {
        self.is_sensor
    }

    pub fn set_sensor(&mut self, is_sensor: bool) {
        self.is_sensor = is_sensor;
    }
}

#[derive(Clone, Debug)]
pub enum ColliderShape {
    Sphere { radius: f32 },
    Box { half_extents: Vec3 },
    Cylinder { half_height: f32, radius: f32 },
    Capsule { half_height: f32, radius: f32 },
    Plane,
}

impl ColliderShape {
    pub fn sphere(radius: f32) -> Self {
        Self::Sphere { radius }
    }

    pub fn box_half_extents(half_extents: Vec3) -> Self {
        Self::Box { half_extents }
    }

    pub fn box_size(size: Vec3) -> Self {
        Self::Box {
            half_extents: size * 0.5,
        }
    }

    pub fn cylinder(half_height: f32, radius: f32) -> Self {
        Self::Cylinder {
            half_height,
            radius,
        }
    }

    pub fn capsule(half_height: f32, radius: f32) -> Self {
        Self::Capsule {
            half_height,
            radius,
        }
    }

    pub fn plane() -> Self {
        Self::Plane
    }
}
