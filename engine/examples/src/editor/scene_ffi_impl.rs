

#[unsafe(no_mangle)]
pub extern "C" fn scene_create_editor() -> *mut std::ffi::c_void {
    unsafe {
        let scene = Box::new(hezhou_core::Scene::new());
        let ptr = Box::into_raw(scene);
        super::SCENE = Some(ptr);
        ptr as *mut std::ffi::c_void
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_destroy_editor(scene: *mut std::ffi::c_void) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let _ = Box::from_raw(scene_ptr);
        super::SCENE = None;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_create_cube_editor(scene: *mut std::ffi::c_void) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = (*scene_ptr).create_cube();
        entity.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_attach_script_editor(scene: *mut std::ffi::c_void, entity_id: u64,
                                             script_path: *const i8, class_name: *const i8) {
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
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = hezhou_core::Entity::new(entity_id);
        let script = hezhou_core::ScriptComponent::from_path(&script_path_str, &class_name_str);
        (*scene_ptr).attach_script(entity, script);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_set_game_state_editor(scene: *mut std::ffi::c_void, state: i32) {
    if scene.is_null() {
        return;
    }
    let state_enum = match state {
        0 => hezhou_core::GameState::Editing,
        1 => hezhou_core::GameState::Running,
        2 => hezhou_core::GameState::Paused,
        _ => hezhou_core::GameState::Editing,
    };
    
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        (*scene_ptr).set_state(state_enum);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_game_state_editor(scene: *mut std::ffi::c_void) -> i32 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        (*scene_ptr).state as i32
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_pick_entity_editor(scene: *mut std::ffi::c_void,
                                           ox: f32, oy: f32, oz: f32,
                                           dx: f32, dy: f32, dz: f32) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let origin = hezhou_core::Vec3::new(ox, oy, oz);
        let dir = hezhou_core::Vec3::new(dx, dy, dz);
        match (*scene_ptr).pick_entity(origin, dir) {
            Some(e) => e.id,
            None => 0,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_select_entity_editor(scene: *mut std::ffi::c_void, entity_id: u64) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = hezhou_core::Entity::new(entity_id);
        (*scene_ptr).select_entity(entity);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_update_editor(scene: *mut std::ffi::c_void, dt: f32) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        (*scene_ptr).update(dt);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_entity_position_editor(scene: *mut std::ffi::c_void, entity_id: u64,
                                                    out_x: *mut f32, out_y: *mut f32, out_z: *mut f32) {
    if scene.is_null() || out_x.is_null() || out_y.is_null() || out_z.is_null() {
        return;
    }
    unsafe {
        hezhou_core::scene_get_entity_position(scene as *mut hezhou_core::Scene, entity_id, out_x, out_y, out_z);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_entity_rotation_editor(scene: *mut std::ffi::c_void, entity_id: u64,
                                                    out_x: *mut f32, out_y: *mut f32, out_z: *mut f32, out_w: *mut f32) {
    if scene.is_null() || out_x.is_null() || out_y.is_null() || out_z.is_null() || out_w.is_null() {
        return;
    }
    unsafe {
        hezhou_core::scene_get_entity_rotation(scene as *mut hezhou_core::Scene, entity_id, out_x, out_y, out_z, out_w);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_entity_scale_editor(scene: *mut std::ffi::c_void, entity_id: u64,
                                                 out_x: *mut f32, out_y: *mut f32, out_z: *mut f32) {
    if scene.is_null() || out_x.is_null() || out_y.is_null() || out_z.is_null() {
        return;
    }
    unsafe {
        hezhou_core::scene_get_entity_scale(scene as *mut hezhou_core::Scene, entity_id, out_x, out_y, out_z);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_set_entity_position_editor(scene: *mut std::ffi::c_void, entity_id: u64, x: f32, y: f32, z: f32) {
    if scene.is_null() {
        return;
    }
    unsafe {
        hezhou_core::scene_set_entity_position(scene as *mut hezhou_core::Scene, entity_id, x, y, z);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_set_entity_scale_editor(scene: *mut std::ffi::c_void, entity_id: u64, x: f32, y: f32, z: f32) {
    if scene.is_null() {
        return;
    }
    unsafe {
        hezhou_core::scene_set_entity_scale(scene as *mut hezhou_core::Scene, entity_id, x, y, z);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_rotate_entity_editor(scene: *mut std::ffi::c_void, entity_id: u64, angle_degrees: f32) {
    if scene.is_null() {
        return;
    }
    unsafe {
        hezhou_core::scene_rotate_entity(scene as *mut hezhou_core::Scene, entity_id, angle_degrees);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_set_entity_name_editor(scene: *mut std::ffi::c_void, entity_id: u64, name_ptr: *const i8) {
    if scene.is_null() || name_ptr.is_null() {
        return;
    }
    unsafe {
        hezhou_core::scene_set_entity_name(scene as *mut hezhou_core::Scene, entity_id, name_ptr);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_entity_name_editor(scene: *mut std::ffi::c_void, entity_id: u64, buffer_ptr: *mut i8, buffer_size: usize) -> usize {
    if scene.is_null() || buffer_ptr.is_null() || buffer_size == 0 {
        return 0;
    }
    unsafe {
        hezhou_core::scene_get_entity_name(scene as *mut hezhou_core::Scene, entity_id, buffer_ptr, buffer_size)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_attach_script_binding_editor(scene: *mut std::ffi::c_void, entity_id: u64,
                                                      script_path: *const i8, class_name: *const i8) {
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
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = hezhou_core::Entity::new(entity_id);
        (*scene_ptr).attach_script_binding(entity, script_path_str, class_name_str);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_remove_script_binding_editor(scene: *mut std::ffi::c_void, entity_id: u64, script_index: usize) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = hezhou_core::Entity::new(entity_id);
        (*scene_ptr).remove_script_binding(entity, script_index);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_script_binding_count_editor(scene: *mut std::ffi::c_void, entity_id: u64) -> usize {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = hezhou_core::Entity::new(entity_id);
        (*scene_ptr).get_script_binding_count(entity)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_script_binding_info_editor(scene: *mut std::ffi::c_void, entity_id: u64, index: usize,
                                                         path_buffer: *mut i8, path_buffer_size: usize,
                                                         class_buffer: *mut i8, class_buffer_size: usize,
                                                         out_enabled: *mut bool) -> bool {
    if scene.is_null() || path_buffer.is_null() || class_buffer.is_null() || out_enabled.is_null() {
        return false;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = hezhou_core::Entity::new(entity_id);
        if let Some(binding) = (*scene_ptr).get_script_binding(entity, index) {
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

#[unsafe(no_mangle)]
pub extern "C" fn scene_set_script_binding_enabled_editor(scene: *mut std::ffi::c_void, entity_id: u64, index: usize, enabled: bool) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = hezhou_core::Entity::new(entity_id);
        (*scene_ptr).set_script_binding_enabled(entity, index, enabled);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_create_entity_editor(scene: *mut std::ffi::c_void) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = (*scene_ptr).create_entity();
        entity.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_entity_count_editor(scene: *mut std::ffi::c_void) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        (*scene_ptr).entity_count() as u64
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_entity_id_editor(scene: *mut std::ffi::c_void, index: u64) -> u64 {
    if scene.is_null() {
        return 0;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entities = &(*scene_ptr).root_entities;
        if index < entities.len() as u64 {
            entities[index as usize].id
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn scene_remove_entity_editor(scene: *mut std::ffi::c_void, entity_id: u64) {
    if scene.is_null() {
        return;
    }
    unsafe {
        let scene_ptr = scene as *mut hezhou_core::Scene;
        let entity = hezhou_core::Entity::new(entity_id);
        (*scene_ptr).remove_entity(entity);
    }
}