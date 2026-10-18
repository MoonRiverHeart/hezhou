use hezhou_core::ecs::{AnimationClip, AnimationKeyframe, AnimationPlayer, AnimationSystem, LocalTransform, World};
use hezhou_core::math::{Quaternion, Vec3};

fn clip() -> AnimationClip {
    AnimationClip::new("walk", 1.0)
        .with_key(AnimationKeyframe::new(0.0, Vec3::new(0.0, 0.0, 0.0), Quaternion::identity(), Vec3::new(1.0, 1.0, 1.0)))
        .with_key(AnimationKeyframe::new(1.0, Vec3::new(2.0, 0.0, 0.0), Quaternion::identity(), Vec3::new(1.0, 1.0, 1.0)))
}

#[test]
fn animation_system_advances_transform() {
    let mut world = World::new();
    world.add_system(Box::new(AnimationSystem));
    let e = world.create_entity();
    world.add_component(e, LocalTransform::new());
    world.add_component(e, AnimationPlayer::new(clip()));
    world.update(0.5);
    let t = world.get_component::<LocalTransform>(e).unwrap();
    assert!((t.position.x - 1.0).abs() < 0.01, "x={}", t.position.x);
}

#[test]
fn animation_loops() {
    let mut world = World::new();
    world.add_system(Box::new(AnimationSystem));
    let e = world.create_entity();
    world.add_component(e, LocalTransform::new());
    let mut player = AnimationPlayer::new(clip());
    player.time = 0.9;
    world.add_component(e, player);
    world.update(0.2);
    let t = world.get_component::<LocalTransform>(e).unwrap();
    assert!((t.position.x - 0.2).abs() < 0.05, "looped x={}", t.position.x);
}
