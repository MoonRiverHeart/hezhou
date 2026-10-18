use hezhou_core::math::Vec3;
use hezhou_physics::{ColliderShape, PhysicsWorld, RigidBodyType};

#[test]
fn dynamic_box_collides_with_static_box() {
    let mut world = PhysicsWorld::new();
    world.set_gravity(Vec3::new(0.0, 0.0, 0.0));
    let a = world.create_body();
    let b = world.create_body_with_type(RigidBodyType::Static);
    world.create_collider(ColliderShape::box_size(Vec3::new(1.0, 1.0, 1.0)), a);
    world.create_collider(ColliderShape::box_size(Vec3::new(1.0, 1.0, 1.0)), b);
    world.get_body_mut(a).unwrap().set_position(Vec3::new(0.0, 0.0, 0.0));
    world.get_body_mut(b).unwrap().set_position(Vec3::new(0.0, 0.0, 0.8));
    world.step(1.0 / 60.0);
    assert!(!world.collision_events().is_empty(), "expected collision event");
    let event = world.collision_events()[0];
    assert!(event.body_a == a || event.body_a == b);
    let pa = *world.get_body(a).unwrap().position();
    assert!(pa.z < -0.05 || pa.z > 0.05, "dynamic body should be separated, z={}", pa.z);
}

#[test]
fn sphere_collision_event_is_reported_once() {
    let mut world = PhysicsWorld::new();
    world.set_gravity(Vec3::new(0.0, 0.0, 0.0));
    let a = world.create_body();
    let b = world.create_body_with_type(RigidBodyType::Static);
    world.create_collider(ColliderShape::sphere(0.5), a);
    world.create_collider(ColliderShape::sphere(0.5), b);
    world.get_body_mut(a).unwrap().set_position(Vec3::new(0.0, 0.0, 0.0));
    world.get_body_mut(b).unwrap().set_position(Vec3::new(0.0, 0.0, 0.5));
    world.step(1.0 / 60.0);
    assert_eq!(world.collision_events().len(), 1);
    world.step(1.0 / 60.0);
    assert_eq!(world.collision_events().len(), 0, "collision should not re-fire on second frame");
}
