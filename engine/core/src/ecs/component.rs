pub type ComponentTypeId = u32;

#[repr(C)]
pub struct ComponentTypeInfo {
    pub type_id: ComponentTypeId,
    pub size: usize,
    pub name: *const std::os::raw::c_char,
}

pub trait Component: Sized {
    fn type_id() -> ComponentTypeId;
    fn type_name() -> &'static str;
}

#[macro_export]
macro_rules! define_component {
    ($name:ident, $type_id:expr) => {
        impl Component for $name {
            fn type_id() -> ComponentTypeId {
                $type_id
            }
            fn type_name() -> &'static str {
                stringify!($name)
            }
        }
    };
}

pub struct TransformComponent {
    pub position: crate::math::Vec3,
    pub rotation: crate::math::Quaternion,
    pub scale: crate::math::Vec3,
}

define_component!(TransformComponent, 1);

impl Default for TransformComponent {
    fn default() -> Self {
        Self {
            position: crate::math::Vec3::zero(),
            rotation: crate::math::Quaternion::identity(),
            scale: crate::math::Vec3::one(),
        }
    }
}

pub struct NameComponent {
    pub name: String,
}

define_component!(NameComponent, 2);

pub struct TagComponent {
    pub tag: u64,
}

define_component!(TagComponent, 3);