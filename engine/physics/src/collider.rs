use hezhou_core::math::Vec3;

/// 碰撞层常量与位掩码辅助。
///
/// 语义：`layer` 表示“我在哪一层”，`mask` 表示“我愿意与哪些层交互”。
/// 两个碰撞体交互的条件是双向的：
/// `(a.layer & b.mask) != 0 && (b.layer & a.mask) != 0`。
pub mod layers {
    /// 默认层（未显式设置时使用）。
    pub const DEFAULT: u32 = 1 << 0;
    /// 玩家角色。
    pub const PLAYER: u32 = 1 << 1;
    /// 敌人。
    pub const ENEMY: u32 = 1 << 2;
    /// 静态场景（地面/墙体）。
    pub const WORLD: u32 = 1 << 3;
    /// 触发器（sensor）。
    pub const TRIGGER: u32 = 1 << 4;
    /// 相机碰撞体。
    pub const CAMERA: u32 = 1 << 5;

    /// 所有层。
    pub const ALL: u32 = u32::MAX;
    /// 空层（不与任何层交互）。
    pub const NONE: u32 = 0;
}

/// 判断两个碰撞体（层 + 掩码）是否允许交互。
pub fn layers_interact(a_layer: u32, a_mask: u32, b_layer: u32, b_mask: u32) -> bool {
    (a_layer & b_mask) != 0 && (b_layer & a_mask) != 0
}

/// 碰撞体：形状 + 局部偏移 + 物理材质 + 层/掩码 + 触发器标记。
#[derive(Clone, Debug)]
pub struct Collider {
    shape: ColliderShape,
    offset: Vec3,
    friction: f32,
    restitution: f32,
    is_sensor: bool,
    layer: u32,
    mask: u32,
}

impl Collider {
    pub fn new(shape: ColliderShape) -> Self {
        Self {
            shape,
            offset: Vec3::zero(),
            friction: 0.5,
            restitution: 0.0,
            is_sensor: false,
            layer: layers::DEFAULT,
            mask: layers::ALL,
        }
    }

    pub fn shape(&self) -> &ColliderShape {
        &self.shape
    }

    /// 相对刚体位置的局部偏移（0.2 不随刚体旋转，见 `world` 模块说明）。
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

    /// 是否为触发器（sensor）：参与 overlap/事件检测，但不参与位置分离。
    pub fn is_sensor(&self) -> bool {
        self.is_sensor
    }

    pub fn set_sensor(&mut self, is_sensor: bool) {
        self.is_sensor = is_sensor;
    }

    /// 碰撞体所在层（单个 bit）。
    pub fn layer(&self) -> u32 {
        self.layer
    }

    pub fn set_layer(&mut self, layer: u32) {
        self.layer = layer;
    }

    /// 碰撞体愿意交互的层掩码（可多 bit）。
    pub fn mask(&self) -> u32 {
        self.mask
    }

    pub fn set_mask(&mut self, mask: u32) {
        self.mask = mask;
    }

    /// 与另一个碰撞体是否允许交互（层/掩码双向检查）。
    pub fn interacts_with(&self, other: &Collider) -> bool {
        layers_interact(self.layer, self.mask, other.layer, other.mask)
    }
}

/// 碰撞体形状。
///
/// 0.2 约定（与现有 AABB 能力保持一致）：
/// - `Box` 为轴对齐盒，忽略刚体旋转；
/// - `Capsule`/`Cylinder` 的轴固定为世界 Y 轴（角色始终直立）；
/// - `Plane` 为无限平面，法线由刚体旋转作用于 +Y 得到。
///
/// TODO(0.3)：支持任意朝向的盒/胶囊与圆柱求交。
#[derive(Clone, Debug)]
pub enum ColliderShape {
    Sphere {
        radius: f32,
    },
    Box {
        half_extents: Vec3,
    },
    Cylinder {
        half_height: f32,
        radius: f32,
    },
    /// 胶囊：`half_height` 为圆柱段半高，总高 = `2 * (half_height + radius)`。
    Capsule {
        half_height: f32,
        radius: f32,
    },
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

    /// 形状轴对齐包围盒的半尺寸；`Plane` 无限大，返回 `None`。
    pub fn aabb_half_extents(&self) -> Option<Vec3> {
        match self {
            Self::Sphere { radius } => Some(Vec3::new(*radius, *radius, *radius)),
            Self::Box { half_extents } => Some(*half_extents),
            Self::Cylinder {
                half_height,
                radius,
            } => Some(Vec3::new(*radius, *half_height, *radius)),
            Self::Capsule {
                half_height,
                radius,
            } => Some(Vec3::new(*radius, half_height + radius, *radius)),
            Self::Plane => None,
        }
    }

    /// 胶囊/圆柱的总高度（供角色控制器换算脚底位置）。
    pub fn total_height(&self) -> Option<f32> {
        match self {
            Self::Capsule {
                half_height,
                radius,
            }
            | Self::Cylinder {
                half_height,
                radius,
            } => Some((half_height + radius) * 2.0),
            Self::Sphere { radius } => Some(radius * 2.0),
            Self::Box { half_extents } => Some(half_extents.y * 2.0),
            Self::Plane => None,
        }
    }
}
