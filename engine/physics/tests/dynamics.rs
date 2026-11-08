//! 0.3 动态刚体测试：自由落体/落地、两球碰撞动量、堆叠分离、
//! CCD 防穿透、距离关节、阻尼与睡眠。

use hezhou_core::math::Vec3;
use hezhou_physics::{ColliderShape, PhysicsWorld, RigidBodyType};

fn add_static_box(world: &mut PhysicsWorld, center: Vec3, half: Vec3) -> (u64, u64) {
    let body = world.create_body_with_type(RigidBodyType::Static);
    let collider = world.create_collider(ColliderShape::box_half_extents(half), body);
    world.get_body_mut(body).unwrap().set_position(center);
    (body, collider)
}

fn add_dynamic_box(world: &mut PhysicsWorld, center: Vec3, half: Vec3, mass: f32) -> (u64, u64) {
    let body = world.create_body();
    let collider = world.create_collider(ColliderShape::box_half_extents(half), body);
    let b = world.get_body_mut(body).unwrap();
    b.set_position(center);
    b.set_mass(mass);
    (body, collider)
}

fn add_dynamic_sphere(
    world: &mut PhysicsWorld,
    center: Vec3,
    radius: f32,
    mass: f32,
) -> (u64, u64) {
    let body = world.create_body();
    let collider = world.create_collider(ColliderShape::sphere(radius), body);
    let b = world.get_body_mut(body).unwrap();
    b.set_position(center);
    b.set_mass(mass);
    (body, collider)
}

#[test]
fn free_fall_and_land_on_static_box() {
    let mut world = PhysicsWorld::new();
    // 地面：中心 y=-0.5，半高 0.5 → 顶面 y=0。
    add_static_box(
        &mut world,
        Vec3::new(0.0, -0.5, 0.0),
        Vec3::new(5.0, 0.5, 5.0),
    );
    let (body, _) = add_dynamic_box(
        &mut world,
        Vec3::new(0.0, 3.0, 0.0),
        Vec3::new(0.5, 0.5, 0.5),
        1.0,
    );

    // 2 秒自由落体 + 落地稳定。
    for _ in 0..180 {
        world.step(1.0 / 60.0);
    }

    let b = world.get_body(body).unwrap();
    assert!(
        (b.position().y - 0.5).abs() < 0.05,
        "落地后中心应停在 0.5，实际 {}",
        b.position().y
    );
    assert!(
        b.velocity().y.abs() < 0.2,
        "落地后垂直速度应接近 0，实际 {}",
        b.velocity().y
    );
}

#[test]
fn free_fall_and_land_on_plane() {
    let mut world = PhysicsWorld::new();
    let plane = world.create_body_with_type(RigidBodyType::Static);
    world.create_collider(ColliderShape::plane(), plane);
    let (body, _) = add_dynamic_sphere(&mut world, Vec3::new(0.0, 3.0, 0.0), 0.5, 1.0);

    for _ in 0..180 {
        world.step(1.0 / 60.0);
    }

    let b = world.get_body(body).unwrap();
    assert!(
        (b.position().y - 0.5).abs() < 0.05,
        "球应停在 y=0.5，实际 {}",
        b.position().y
    );
    assert!(b.velocity().y.abs() < 0.2);
}

#[test]
fn two_spheres_elastic_collision_conserves_momentum() {
    let mut world = PhysicsWorld::new();
    world.set_gravity(Vec3::zero());

    let (body_a, collider_a) = add_dynamic_sphere(&mut world, Vec3::new(-1.0, 0.0, 0.0), 0.5, 1.0);
    let (body_b, collider_b) = add_dynamic_sphere(&mut world, Vec3::new(1.0, 0.0, 0.0), 0.5, 1.0);
    for collider in [collider_a, collider_b] {
        world
            .get_collider_mut(collider)
            .unwrap()
            .set_restitution(1.0);
    }
    world
        .get_body_mut(body_a)
        .unwrap()
        .set_velocity(Vec3::new(2.0, 0.0, 0.0));
    world
        .get_body_mut(body_b)
        .unwrap()
        .set_velocity(Vec3::new(-2.0, 0.0, 0.0));

    let initial_momentum = 2.0 + (-2.0);
    for _ in 0..120 {
        world.step(1.0 / 60.0);
    }

    let a = world.get_body(body_a).unwrap();
    let b = world.get_body(body_b).unwrap();
    // 动量守恒。
    let momentum = a.velocity().x + b.velocity().x;
    assert!(
        (momentum - initial_momentum).abs() < 0.1,
        "动量应守恒，实际 {}",
        momentum
    );
    // 等质量完全弹性碰撞：速度交换。
    assert!(
        (a.velocity().x + 2.0).abs() < 0.15,
        "A 速度应变为 -2，实际 {}",
        a.velocity().x
    );
    assert!(
        (b.velocity().x - 2.0).abs() < 0.15,
        "B 速度应变为 2，实际 {}",
        b.velocity().x
    );
    // 碰撞后应已分离。
    let separation = (b.position().x - a.position().x).abs();
    assert!(separation > 0.95, "碰撞后应分离，间距 {}", separation);
}

#[test]
fn two_spheres_inelastic_collision_shares_momentum() {
    let mut world = PhysicsWorld::new();
    world.set_gravity(Vec3::zero());

    let (body_a, _) = add_dynamic_sphere(&mut world, Vec3::new(-1.0, 0.0, 0.0), 0.5, 1.0);
    let (body_b, _) = add_dynamic_sphere(&mut world, Vec3::new(1.0, 0.0, 0.0), 0.5, 2.0);
    world
        .get_body_mut(body_a)
        .unwrap()
        .set_velocity(Vec3::new(3.0, 0.0, 0.0));

    let initial_momentum = 1.0 * 3.0 + 2.0 * 0.0;
    for _ in 0..120 {
        world.step(1.0 / 60.0);
    }

    let a = world.get_body(body_a).unwrap();
    let b = world.get_body(body_b).unwrap();
    let momentum = a.velocity().x + 2.0 * b.velocity().x;
    assert!(
        (momentum - initial_momentum).abs() < 0.1,
        "动量应守恒，实际 {}",
        momentum
    );
    // 完全非弹性：两者共同速度应为 3 / (1+2) = 1。
    assert!(
        (a.velocity().x - 1.0).abs() < 0.1 && (b.velocity().x - 1.0).abs() < 0.1,
        "共同速度应约为 1，实际 A={} B={}",
        a.velocity().x,
        b.velocity().x
    );
}

#[test]
fn dynamic_boxes_stack_without_sinking() {
    let mut world = PhysicsWorld::new();
    add_static_box(
        &mut world,
        Vec3::new(0.0, -0.5, 0.0),
        Vec3::new(5.0, 0.5, 5.0),
    );

    let half = Vec3::new(0.5, 0.5, 0.5);
    let bodies: Vec<u64> = [1.0f32, 2.1, 3.2]
        .iter()
        .map(|y| add_dynamic_box(&mut world, Vec3::new(0.0, *y, 0.0), half, 1.0).0)
        .collect();

    for _ in 0..240 {
        world.step(1.0 / 60.0);
    }

    let expected = [0.5f32, 1.5, 2.5];
    for (index, body) in bodies.iter().enumerate() {
        let y = world.get_body(*body).unwrap().position().y;
        assert!(
            (y - expected[index]).abs() < 0.1,
            "第 {} 个箱子的 y 应在 {} 附近，实际 {}",
            index,
            expected[index],
            y
        );
    }
}

#[test]
fn ccd_prevents_tunneling_through_thin_wall() {
    fn simulate(ccd: bool) -> f32 {
        let mut world = PhysicsWorld::new();
        world.set_gravity(Vec3::zero());
        world.set_ccd_enabled(ccd);
        // 薄墙：z 厚度 0.1（半尺寸 0.05）。
        add_static_box(&mut world, Vec3::zero(), Vec3::new(5.0, 5.0, 0.05));
        let (body, _) = add_dynamic_sphere(&mut world, Vec3::new(0.0, 0.0, -5.0), 0.5, 1.0);
        {
            let b = world.get_body_mut(body).unwrap();
            b.set_velocity(Vec3::new(0.0, 0.0, 200.0));
            b.set_ccd_enabled(ccd);
        }
        for _ in 0..60 {
            world.step(1.0 / 60.0);
        }
        world.get_body(body).unwrap().position().z
    }

    let without_ccd = simulate(false);
    assert!(
        without_ccd > 0.5,
        "未启用 CCD 的高速球应穿透薄墙，实际 z={}",
        without_ccd
    );

    let with_ccd = simulate(true);
    assert!(
        with_ccd < -0.4 && with_ccd > -0.8,
        "启用 CCD 后应停在墙前（约 z=-0.55），实际 z={}",
        with_ccd
    );
}

#[test]
fn distance_joint_keeps_rest_length() {
    let mut world = PhysicsWorld::new();
    world.set_gravity(Vec3::zero());

    let (body_a, _) = add_dynamic_box(
        &mut world,
        Vec3::new(-1.5, 0.0, 0.0),
        Vec3::new(0.5, 0.5, 0.5),
        1.0,
    );
    let (body_b, _) = add_dynamic_box(
        &mut world,
        Vec3::new(1.5, 0.0, 0.0),
        Vec3::new(0.5, 0.5, 0.5),
        1.0,
    );
    world.add_distance_joint(body_a, body_b, 2.0);

    for _ in 0..120 {
        world.step(1.0 / 60.0);
    }
    let joint = &world.distance_joints()[0];
    let length = joint.current_length(&world).expect("关节两端应存在");
    assert!(
        (length - 2.0).abs() < 0.1,
        "关节应收敛到 rest_length=2，实际 {}",
        length
    );

    // 给 A 一个垂直速度，距离应仍保持在 2 附近。
    world
        .get_body_mut(body_a)
        .unwrap()
        .set_velocity(Vec3::new(0.0, 4.0, 0.0));
    for _ in 0..90 {
        world.step(1.0 / 60.0);
    }
    let length = world.distance_joints()[0]
        .current_length(&world)
        .expect("关节两端应存在");
    assert!(
        (length - 2.0).abs() < 0.2,
        "运动过程中关节长度应保持，实际 {}",
        length
    );
    let a_y = world.get_body(body_a).unwrap().position().y;
    assert!(a_y > 0.3, "A 应沿切线方向摆动，实际 y={}", a_y);
}

#[test]
fn linear_damping_decays_velocity() {
    let mut world = PhysicsWorld::new();
    world.set_gravity(Vec3::zero());
    let body = world.create_body();
    {
        let b = world.get_body_mut(body).unwrap();
        b.set_velocity(Vec3::new(10.0, 0.0, 0.0));
        b.set_linear_damping(2.0);
    }
    for _ in 0..120 {
        world.step(1.0 / 60.0);
    }
    let speed = world.get_body(body).unwrap().velocity().magnitude();
    assert!(speed < 1.0, "阻尼后速度应明显衰减，实际 {}", speed);
    assert!(speed > 0.01, "阻尼不应瞬间归零，实际 {}", speed);
}

#[test]
fn auto_sleep_puts_resting_body_to_sleep() {
    let mut world = PhysicsWorld::new();
    world.set_auto_sleep(true);
    let mut config = *world.solver_config();
    config.sleep_delay = 0.2;
    world.set_solver_config(config);
    add_static_box(
        &mut world,
        Vec3::new(0.0, -0.5, 0.0),
        Vec3::new(5.0, 0.5, 5.0),
    );
    let (body, _) = add_dynamic_box(
        &mut world,
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.5, 0.5, 0.5),
        1.0,
    );

    for _ in 0..120 {
        world.step(1.0 / 60.0);
    }
    assert!(
        world.get_body(body).unwrap().is_sleeping(),
        "静止的箱子应进入睡眠"
    );

    // 被快速物体撞击后应被唤醒。
    let (striker, _) = add_dynamic_sphere(&mut world, Vec3::new(0.0, 2.5, 0.0), 0.5, 1.0);
    world
        .get_body_mut(striker)
        .unwrap()
        .set_velocity(Vec3::new(0.0, -6.0, 0.0));
    let mut woke = false;
    for _ in 0..60 {
        world.step(1.0 / 60.0);
        if !world.get_body(body).unwrap().is_sleeping() {
            woke = true;
            break;
        }
    }
    assert!(woke, "睡眠刚体被撞击后应唤醒");
}
