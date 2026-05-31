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
pub extern "C" fn scene_create_plane(scene: *mut Scene) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let entity = (*scene).create_plane();
        entity.id
    }
}

#[no_mangle]
pub extern "C" fn scene_create_cornell_box(scene: *mut Scene) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let entity = (*scene).create_cornell_box();
        entity.id
    }
}

#[no_mangle]
pub extern "C" fn scene_create_directional_light(scene: *mut Scene) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let entity = (*scene).create_directional_light();
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
pub extern "C" fn scene_root_entity_count(scene: *const Scene) -> u32 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        (*scene).root_entities.len() as u32
    }
}

#[no_mangle]
pub extern "C" fn scene_get_root_entity_id_at(scene: *const Scene, index: u32) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let root = &(*scene).root_entities;
        if index as usize >= root.len() {
            return 0;
        }
        root[index as usize].id
    }
}

#[no_mangle]
pub extern "C" fn scene_remove_entity(scene: *mut Scene, entity_id: u64) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        (*scene).remove_entity(entity);
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

#[no_mangle]
pub extern "C" fn scene_set_entity_name(scene: *mut Scene, entity_id: u64, name_ptr: *const i8) {
    if scene.is_null() || name_ptr.is_null() {
        return;
    }
    let name = unsafe {
        std::ffi::CStr::from_ptr(name_ptr).to_string_lossy().into_owned()
    };
    unsafe {
        let entity = Entity::new(entity_id);
        (*scene).set_entity_name(entity, name);
    }
}

#[no_mangle]
pub extern "C" fn scene_get_entity_name(scene: *const Scene, entity_id: u64, buffer_ptr: *mut i8, buffer_size: usize) -> usize {
    if scene.is_null() || buffer_ptr.is_null() || buffer_size == 0 {
        return 0;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        if let Some(name) = (*scene).get_entity_name(entity) {
            let bytes = name.as_bytes();
            let copy_len = bytes.len().min(buffer_size - 1);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer_ptr as *mut u8, copy_len);
            *buffer_ptr.add(copy_len) = 0;
            copy_len
        } else {
            *buffer_ptr = 0;
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn scene_attach_script_binding(scene: *mut Scene, entity_id: u64, script_path: *const i8, class_name: *const i8) {
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
        (*scene).attach_script_binding(entity, script_path_str, class_name_str);
    }
}

#[no_mangle]
pub extern "C" fn scene_remove_script_binding(scene: *mut Scene, entity_id: u64, script_index: usize) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        (*scene).remove_script_binding(entity, script_index);
    }
}

#[no_mangle]
pub extern "C" fn scene_get_script_binding_count(scene: *const Scene, entity_id: u64) -> usize {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        (*scene).get_script_binding_count(entity)
    }
}

#[no_mangle]
pub extern "C" fn scene_get_script_binding_info(scene: *const Scene, entity_id: u64, index: usize,
                                                  path_buffer: *mut i8, path_buffer_size: usize,
                                                  class_buffer: *mut i8, class_buffer_size: usize,
                                                  out_enabled: *mut bool) -> bool {
    if scene.is_null() || path_buffer.is_null() || class_buffer.is_null() || out_enabled.is_null() {
        return false;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        if let Some(binding) = (*scene).get_script_binding(entity, index) {
            let path_bytes = binding.script_path.as_bytes();
            let path_copy_len = path_bytes.len().min(path_buffer_size - 1);
            std::ptr::copy_nonoverlapping(path_bytes.as_ptr(), path_buffer as *mut u8, path_copy_len);
            *path_buffer.add(path_copy_len) = 0;
            
            let class_bytes = binding.class_name.as_bytes();
            let class_copy_len = class_bytes.len().min(class_buffer_size - 1);
            std::ptr::copy_nonoverlapping(class_bytes.as_ptr(), class_buffer as *mut u8, class_copy_len);
            *class_buffer.add(class_copy_len) = 0;
            
            *out_enabled = binding.enabled;
            true
        } else {
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn scene_set_script_binding_enabled(scene: *mut Scene, entity_id: u64, index: usize, enabled: bool) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        (*scene).set_script_binding_enabled(entity, index, enabled);
    }
}

#[no_mangle]
pub extern "C" fn scene_set_script_binding_instance_id(scene: *mut Scene, entity_id: u64, index: usize, instance_id: u64) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        (*scene).set_script_binding_instance_id(entity, index, instance_id);
    }
}

#[no_mangle]
pub extern "C" fn scene_get_script_binding_instance_id(scene: *const Scene, entity_id: u64, index: usize) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        (*scene).get_script_binding(entity, index).map(|b| b.instance_id).unwrap_or(0)
    }
}

#[no_mangle]
pub extern "C" fn scene_set_parent(scene: *mut Scene, entity_id: u64, parent_id: u64) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        let parent = if parent_id != 0 {
            Some(Entity::new(parent_id))
        } else {
            None
        };
        (*scene).set_parent(entity, parent);
    }
}

#[no_mangle]
pub extern "C" fn scene_get_parent(scene: *const Scene, entity_id: u64) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let entity = Entity::new(entity_id);
        (*scene).get_parent(entity).map(|p| p.id).unwrap_or(0)
    }
}

#[no_mangle]
pub extern "C" fn scene_get_child_count(scene: *const Scene, parent_id: u64) -> usize {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let parent = Entity::new(parent_id);
        (*scene).get_children(parent).len()
    }
}

#[no_mangle]
pub extern "C" fn scene_get_child_id(scene: *const Scene, parent_id: u64, index: usize) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let parent = Entity::new(parent_id);
        let children = (*scene).get_children(parent);
        children.get(index).map(|c| c.id).unwrap_or(0)
    }
}