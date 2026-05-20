use crate::ecs::*;
use crate::math::{Vec3, Quaternion};

#[no_mangle]
pub extern "C" fn scene_create() -> *mut Scene {
    let scene = Box::new(Scene::new());
    Box::into_raw(scene)
}

#[no_mangle]
pub extern "C" fn scene_destroy(scene: *mut Scene) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(scene);
    }
}

#[no_mangle]
pub extern "C" fn scene_create_entity(scene: *mut Scene) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let entity = (*scene).create_entity();
        entity.id
    }
}

#[no_mangle]
pub extern "C" fn scene_create_cube(scene: *mut Scene) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let entity = (*scene).create_cube();
        entity.id
    }
}

#[no_mangle]
pub extern "C" fn scene_set_entity_position(scene: *mut Scene, entity_id: u64, x: f32, y: f32, z: f32) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        if let Some(mut transform) = (*scene).world.get_component::<LocalTransform>(entity) {
            transform.position = Vec3::new(x, y, z);
            (*scene).world.add_component(entity, transform);
        }
    }
}

#[no_mangle]
pub extern "C" fn scene_set_entity_rotation(scene: *mut Scene, entity_id: u64, x: f32, y: f32, z: f32, w: f32) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        if let Some(mut transform) = (*scene).world.get_component::<LocalTransform>(entity) {
            transform.rotation = Quaternion::new(x, y, z, w);
            (*scene).world.add_component(entity, transform);
        }
    }
}

#[no_mangle]
pub extern "C" fn scene_set_entity_scale(scene: *mut Scene, entity_id: u64, x: f32, y: f32, z: f32) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        if let Some(mut transform) = (*scene).world.get_component::<LocalTransform>(entity) {
            transform.scale = Vec3::new(x, y, z);
            (*scene).world.add_component(entity, transform);
        }
    }
}

#[no_mangle]
pub extern "C" fn scene_attach_script(scene: *mut Scene, entity_id: u64, script_path: *const i8, class_name: *const i8) {
    if scene.is_null() {
        return;
    }
    let script_path_str = unsafe {
        std::ffi::CStr::from_ptr(script_path).to_string_lossy().into_owned()
    };
    let class_name_str = unsafe {
        std::ffi::CStr::from_ptr(class_name).to_string_lossy().into_owned()
    };
    
    unsafe {
        let entity = Entity::new(entity_id);
        let script = ScriptComponent::from_path(&script_path_str, &class_name_str);
        (*scene).attach_script(entity, script);
        
        let asset = ScriptAsset::new(script_path_str);
        (*scene).register_script_asset(asset);
    }
}

#[no_mangle]
pub extern "C" fn scene_detach_script(scene: *mut Scene, entity_id: u64) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        (*scene).detach_script(entity);
    }
}

#[no_mangle]
pub extern "C" fn scene_set_game_state(scene: *mut Scene, state: i32) {
    if scene.is_null() {
        return;
    }
    let state_enum = match state {
        0 => GameState::Editing,
        1 => GameState::Running,
        2 => GameState::Paused,
        _ => GameState::Editing,
    };
    
    unsafe {
        (*scene).set_state(state_enum);
    }
}

#[no_mangle]
pub extern "C" fn scene_get_game_state(scene: *const Scene) -> i32 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        (*scene).state as i32
    }
}

#[no_mangle]
pub extern "C" fn scene_pick_entity(scene: *const Scene, 
                                    origin_x: f32, origin_y: f32, origin_z: f32,
                                    dir_x: f32, dir_y: f32, dir_z: f32) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let origin = Vec3::new(origin_x, origin_y, origin_z);
        let direction = Vec3::new(dir_x, dir_y, dir_z);
        
        match (*scene).pick_entity(origin, direction) {
            Some(entity) => entity.id,
            None => 0,
        }
    }
}

#[no_mangle]
pub extern "C" fn scene_select_entity(scene: *mut Scene, entity_id: u64) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        (*scene).select_entity(entity);
    }
}

#[no_mangle]
pub extern "C" fn scene_clear_selection(scene: *mut Scene) {
    if scene.is_null() {
        return;
    }
    unsafe {
        (*scene).clear_selection();
    }
}

#[no_mangle]
pub extern "C" fn scene_is_entity_selected(scene: *const Scene, entity_id: u64) -> bool {
    if scene.is_null() {
        return false;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        (*scene).is_entity_selected(entity)
    }
}

#[no_mangle]
pub extern "C" fn scene_get_selected_count(scene: *const Scene) -> u32 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        (*scene).selected_entities.len() as u32
    }
}

#[no_mangle]
pub extern "C" fn scene_update(scene: *mut Scene, delta_time: f32) {
    if scene.is_null() {
        return;
    }
    unsafe {
        (*scene).update(delta_time);
    }
}

#[no_mangle]
pub extern "C" fn scene_entity_count(scene: *const Scene) -> u32 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        (*scene).entity_count() as u32
    }
}

#[no_mangle]
pub extern "C" fn scene_get_entity_position(scene: *const Scene, entity_id: u64, 
                                             out_x: *mut f32, out_y: *mut f32, out_z: *mut f32) {
    if scene.is_null() || out_x.is_null() || out_y.is_null() || out_z.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        if let Some(pos) = (*scene).get_entity_position(entity) {
            *out_x = pos.x;
            *out_y = pos.y;
            *out_z = pos.z;
        }
    }
}

#[no_mangle]
pub extern "C" fn scene_get_entity_rotation(scene: *const Scene, entity_id: u64,
                                             out_x: *mut f32, out_y: *mut f32, out_z: *mut f32, out_w: *mut f32) {
    if scene.is_null() || out_x.is_null() || out_y.is_null() || out_z.is_null() || out_w.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        if let Some(rot) = (*scene).get_entity_rotation(entity) {
            *out_x = rot.x;
            *out_y = rot.y;
            *out_z = rot.z;
            *out_w = rot.w;
        }
    }
}

#[no_mangle]
pub extern "C" fn scene_get_entity_scale(scene: *const Scene, entity_id: u64,
                                          out_x: *mut f32, out_y: *mut f32, out_z: *mut f32) {
    if scene.is_null() || out_x.is_null() || out_y.is_null() || out_z.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        if let Some(scale) = (*scene).get_entity_scale(entity) {
            *out_x = scale.x;
            *out_y = scale.y;
            *out_z = scale.z;
        }
    }
}

#[no_mangle]
pub extern "C" fn scene_rotate_entity(scene: *mut Scene, entity_id: u64, angle_degrees: f32) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        if let Some(mut transform) = (*scene).world.get_component::<LocalTransform>(entity) {
            let rotation = crate::math::Quaternion::from_axis_angle(crate::math::Vec3::up(), angle_degrees.to_radians());
            transform.rotation = rotation * transform.rotation;
            (*scene).world.add_component(entity, transform);
        }
    }
}