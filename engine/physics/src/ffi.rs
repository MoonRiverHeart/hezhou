use crate::PhysicsWorld;
use crate::{ColliderShape, RigidBodyType};
use std::os::raw::c_void;

#[no_mangle]
pub extern "C" fn physics_world_create() -> *mut c_void {
    let world = Box::new(PhysicsWorld::new());
    Box::into_raw(world) as *mut c_void
}

#[no_mangle]
pub extern "C" fn physics_world_destroy(world: *mut c_void) {
    if world.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(world as *mut PhysicsWorld);
    }
}

#[no_mangle]
pub extern "C" fn physics_world_step(world: *mut c_void, dt: f32) {
    if world.is_null() {
        return;
    }
    unsafe {
        let world = &mut *(world as *mut PhysicsWorld);
        world.step(dt);
    }
}

#[no_mangle]
pub extern "C" fn physics_world_set_gravity(world: *mut c_void, x: f32, y: f32, z: f32) {
    if world.is_null() {
        return;
    }
    unsafe {
        let world = &mut *(world as *mut PhysicsWorld);
        world.set_gravity(hezhou_core::math::Vec3::new(x, y, z));
    }
}

#[no_mangle]
pub extern "C" fn physics_create_dynamic_body(world: *mut c_void) -> u64 {
    if world.is_null() {
        return 0;
    }
    unsafe {
        let world = &mut *(world as *mut PhysicsWorld);
        world.create_body()
    }
}

#[no_mangle]
pub extern "C" fn physics_create_static_body(world: *mut c_void) -> u64 {
    if world.is_null() {
        return 0;
    }
    unsafe {
        let world = &mut *(world as *mut PhysicsWorld);
        world.create_body_with_type(RigidBodyType::Static)
    }
}

#[no_mangle]
pub extern "C" fn physics_create_kinematic_body(world: *mut c_void) -> u64 {
    if world.is_null() {
        return 0;
    }
    unsafe {
        let world = &mut *(world as *mut PhysicsWorld);
        world.create_body_with_type(RigidBodyType::Kinematic)
    }
}

#[no_mangle]
pub extern "C" fn physics_destroy_body(world: *mut c_void, body_id: u64) {
    if world.is_null() {
        return;
    }
    unsafe {
        let world = &mut *(world as *mut PhysicsWorld);
        world.destroy_body(body_id);
    }
}

#[no_mangle]
pub extern "C" fn physics_body_set_position(world: *mut c_void, body_id: u64, x: f32, y: f32, z: f32) {
    if world.is_null() {
        return;
    }
    unsafe {
        let world = &mut *(world as *mut PhysicsWorld);
        if let Some(body) = world.get_body_mut(body_id) {
            body.set_position(hezhou_core::math::Vec3::new(x, y, z));
        }
    }
}

#[no_mangle]
pub extern "C" fn physics_body_get_position(world: *mut c_void, body_id: u64, out_x: *mut f32, out_y: *mut f32, out_z: *mut f32) {
    if world.is_null() || out_x.is_null() || out_y.is_null() || out_z.is_null() {
        return;
    }
    unsafe {
        let world = &*(world as *mut PhysicsWorld);
        if let Some(body) = world.get_body(body_id) {
            *out_x = body.position().x;
            *out_y = body.position().y;
            *out_z = body.position().z;
        }
    }
}

#[no_mangle]
pub extern "C" fn physics_body_set_velocity(world: *mut c_void, body_id: u64, x: f32, y: f32, z: f32) {
    if world.is_null() {
        return;
    }
    unsafe {
        let world = &mut *(world as *mut PhysicsWorld);
        if let Some(body) = world.get_body_mut(body_id) {
            body.set_velocity(hezhou_core::math::Vec3::new(x, y, z));
        }
    }
}

#[no_mangle]
pub extern "C" fn physics_body_set_mass(world: *mut c_void, body_id: u64, mass: f32) {
    if world.is_null() {
        return;
    }
    unsafe {
        let world = &mut *(world as *mut PhysicsWorld);
        if let Some(body) = world.get_body_mut(body_id) {
            body.set_mass(mass);
        }
    }
}

#[no_mangle]
pub extern "C" fn physics_create_sphere_collider(world: *mut c_void, radius: f32, body_id: u64) -> u64 {
    if world.is_null() {
        return 0;
    }
    unsafe {
        let world = &mut *(world as *mut PhysicsWorld);
        world.create_collider(ColliderShape::sphere(radius), body_id)
    }
}

#[no_mangle]
pub extern "C" fn physics_create_box_collider(world: *mut c_void, half_x: f32, half_y: f32, half_z: f32, body_id: u64) -> u64 {
    if world.is_null() {
        return 0;
    }
    unsafe {
        let world = &mut *(world as *mut PhysicsWorld);
        world.create_collider(ColliderShape::box_half_extents(hezhou_core::math::Vec3::new(half_x, half_y, half_z)), body_id)
    }
}