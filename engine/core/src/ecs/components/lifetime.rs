use crate::ecs::Component;

/// 生命周期组件：倒计时结束后由 LifetimeSystem 销毁实体。
#[repr(C)]
#[derive(Clone, Debug)]
pub struct LifetimeComponent {
    pub remaining: f32,
    pub destroy_on_expire: bool,
}

impl Component for LifetimeComponent {
    fn type_id() -> crate::ecs::ComponentTypeId { 300 }
    fn type_name() -> &'static str { "LifetimeComponent" }
}

impl LifetimeComponent {
    pub fn new(seconds: f32) -> Self {
        Self { remaining: seconds, destroy_on_expire: true }
    }
}
