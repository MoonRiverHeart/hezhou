use hezhou_core::ecs::{LifetimeComponent, LifetimeSystem, World};

#[test]
fn lifetime_system_destroys_expired_entity() {
    let mut world = World::new();
    world.add_system(Box::new(LifetimeSystem));
    let e = world.create_entity();
    world.add_component(e, LifetimeComponent::new(0.05));
    world.update(0.1);
    assert_eq!(world.entity_count(), 0);
}

#[test]
fn lifetime_system_keeps_entity_before_expiry() {
    let mut world = World::new();
    world.add_system(Box::new(LifetimeSystem));
    let e = world.create_entity();
    world.add_component(e, LifetimeComponent::new(0.5));
    world.update(0.1);
    assert_eq!(world.entity_count(), 1);
    assert!(world.get_component::<LifetimeComponent>(e).is_some());
}
