use crate::ecs::*;
use crate::event::*;
use crate::math::*;
use crate::Engine;
use std::ffi::{c_char, c_float, c_int, CStr};
use std::sync::OnceLock;

pub static mut ENGINE_INSTANCE: OnceLock<*mut Engine> = OnceLock::new();

#[unsafe(no_mangle)]
pub extern "C" fn engine_create() -> *mut Engine {
    let engine = Box::new(Engine::new());
    Box::into_raw(engine)
}

#[unsafe(no_mangle)]
pub extern "C" fn engine_destroy(engine: *mut Engine) {
    if engine.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(engine);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn engine_start(engine: *mut Engine) {
    if engine.is_null() {
        return;
    }
    unsafe {
        (*engine).start();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn engine_stop(engine: *mut Engine) {
    if engine.is_null() {
        return;
    }
    unsafe {
        (*engine).stop();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn engine_is_running(engine: *const Engine) -> bool {
    if engine.is_null() {
        return false;
    }
    unsafe { (*engine).is_running }
}

#[unsafe(no_mangle)]
pub extern "C" fn engine_run_frame(engine: *mut Engine, delta_time: c_float) {
    if engine.is_null() {
        return;
    }
    unsafe {
        (*engine).run_frame(delta_time);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn engine_get_time(engine: *const Engine) -> f64 {
    if engine.is_null() {
        return 0.0;
    }
    unsafe { (*engine).time.elapsed }
}

#[unsafe(no_mangle)]
pub extern "C" fn engine_get_delta_time(engine: *const Engine) -> c_float {
    if engine.is_null() {
        return 0.0;
    }
    unsafe { (*engine).time.delta }
}

#[unsafe(no_mangle)]
pub extern "C" fn engine_get_frame_count(engine: *const Engine) -> u64 {
    if engine.is_null() {
        return 0;
    }
    unsafe { (*engine).time.frame_count }
}

#[unsafe(no_mangle)]
pub extern "C" fn engine_get_world(engine: *mut Engine) -> *mut World {
    if engine.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { &mut (*engine).world }
}

#[unsafe(no_mangle)]
pub extern "C" fn engine_get_event_bus(engine: *mut Engine) -> *mut EventBus {
    if engine.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { &mut (*engine).event_bus }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_create_entity(world: *mut World) -> u64 {
    if world.is_null() {
        return 0;
    }
    unsafe {
        let entity = (*world).create_entity();
        entity.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_destroy_entity(world: *mut World, entity_id: u64) {
    if world.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        (*world).destroy_entity(entity);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_entity_exists(world: *const World, entity_id: u64) -> bool {
    if world.is_null() {
        return false;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        (*world).entity_exists(entity)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_set_entity_parent(world: *mut World, entity_id: u64, parent_id: u64) {
    if world.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        let parent = if parent_id == 0 {
            None
        } else {
            Some(Entity::new(parent_id))
        };
        (*world).set_parent(entity, parent);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_get_entity_parent(world: *const World, entity_id: u64) -> u64 {
    if world.is_null() {
        return 0;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        (*world)
            .get_parent(entity)
            .map(|p| p.id)
            .unwrap_or(0)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_get_entity_children(world: *const World, entity_id: u64, out_children: *mut u64, max_count: u32) -> u32 {
    if world.is_null() || out_children.is_null() {
        return 0;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        let children = (*world).get_children(entity);
        let count = children.len().min(max_count as usize) as u32;
        for i in 0..count as usize {
            *out_children.add(i) = children[i].id;
        }
        count
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_add_transform_component(
    world: *mut World,
    entity_id: u64,
    px: f32, py: f32, pz: f32,
    rx: f32, ry: f32, rz: f32, rw: f32,
    sx: f32, sy: f32, sz: f32,
) {
    if world.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        let component = TransformComponent {
            position: Vec3::new(px, py, pz),
            rotation: Quaternion { x: rx, y: ry, z: rz, w: rw },
            scale: Vec3::new(sx, sy, sz),
        };
        (*world).add_component(entity, component);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_get_transform_component(
    world: *const World,
    entity_id: u64,
    out_position: *mut f32,
    out_rotation: *mut f32,
    out_scale: *mut f32,
) -> bool {
    if world.is_null() {
        return false;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        if let Some(component) = (*world).get_component::<TransformComponent>(entity) {
            if !out_position.is_null() {
                *out_position = component.position.x;
                *out_position.add(1) = component.position.y;
                *out_position.add(2) = component.position.z;
            }
            if !out_rotation.is_null() {
                *out_rotation = component.rotation.x;
                *out_rotation.add(1) = component.rotation.y;
                *out_rotation.add(2) = component.rotation.z;
                *out_rotation.add(3) = component.rotation.w;
            }
            if !out_scale.is_null() {
                *out_scale = component.scale.x;
                *out_scale.add(1) = component.scale.y;
                *out_scale.add(2) = component.scale.z;
            }
            return true;
        }
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_set_transform_position(world: *mut World, entity_id: u64, x: f32, y: f32, z: f32) {
    if world.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        if let Some(mut component) = (*world).get_component::<TransformComponent>(entity) {
            component.position = Vec3::new(x, y, z);
            (*world).add_component(entity, component);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_set_transform_rotation(world: *mut World, entity_id: u64, x: f32, y: f32, z: f32, w: f32) {
    if world.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        if let Some(mut component) = (*world).get_component::<TransformComponent>(entity) {
            component.rotation = Quaternion { x, y, z, w };
            (*world).add_component(entity, component);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_set_transform_scale(world: *mut World, entity_id: u64, x: f32, y: f32, z: f32) {
    if world.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        if let Some(mut component) = (*world).get_component::<TransformComponent>(entity) {
            component.scale = Vec3::new(x, y, z);
            (*world).add_component(entity, component);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_add_name_component(world: *mut World, entity_id: u64, name: *const c_char) {
    if world.is_null() || name.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        let name_str = CStr::from_ptr(name).to_string_lossy().into_owned();
        let component = NameComponent { name: name_str };
        (*world).add_component(entity, component);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_get_name_component(world: *const World, entity_id: u64) -> *const c_char {
    if world.is_null() {
        return std::ptr::null();
    }
    static EMPTY_NAME: &[u8] = b"\0";
    unsafe {
        let entity = Entity::new(entity_id);
        if let Some(component) = (*world).get_component::<NameComponent>(entity) {
            component.name.as_ptr() as *const c_char
        } else {
            EMPTY_NAME.as_ptr() as *const c_char
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_add_tag_component(world: *mut World, entity_id: u64, tag: u64) {
    if world.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        let component = TagComponent { tag };
        (*world).add_component(entity, component);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_get_tag_component(world: *const World, entity_id: u64) -> u64 {
    if world.is_null() {
        return 0;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        (*world)
            .get_component::<TagComponent>(entity)
            .map(|c| c.tag)
            .unwrap_or(0)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_has_transform_component(world: *const World, entity_id: u64) -> bool {
    if world.is_null() {
        return false;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        (*world).has_component::<TransformComponent>(entity)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_has_name_component(world: *const World, entity_id: u64) -> bool {
    if world.is_null() {
        return false;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        (*world).has_component::<NameComponent>(entity)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_has_tag_component(world: *const World, entity_id: u64) -> bool {
    if world.is_null() {
        return false;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        (*world).has_component::<TagComponent>(entity)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ecs_entity_count(world: *const World) -> u32 {
    if world.is_null() {
        return 0;
    }
    unsafe { (*world).entity_count() as u32 }
}

#[unsafe(no_mangle)]
pub extern "C" fn event_bus_subscribe(engine: *mut Engine, event_type: i32, callback: EventCallback, priority: c_int) {
    if engine.is_null() {
        return;
    }
    unsafe {
        let event_type = match event_type {
            0 => EventType::PreUpdate,
            1 => EventType::PostUpdate,
            2 => EventType::EntityCreated,
            3 => EventType::EntityDestroyed,
            4 => EventType::Custom,
            _ => EventType::Custom,
        };
        (*engine).event_bus.subscribe(event_type, callback, priority);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn event_bus_publish(engine: *mut Engine, event_type: i32, data: usize) {
    if engine.is_null() {
        return;
    }
    unsafe {
        let event_type = match event_type {
            0 => EventType::PreUpdate,
            1 => EventType::PostUpdate,
            2 => EventType::EntityCreated,
            3 => EventType::EntityDestroyed,
            4 => EventType::Custom,
            _ => EventType::Custom,
        };
        let event = Event::new(event_type, data);
        (*engine).event_bus.publish(event);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn event_bus_dispatch(engine: *mut Engine) {
    if engine.is_null() {
        return;
    }
    unsafe {
        (*engine).event_bus.dispatch();
    }
}