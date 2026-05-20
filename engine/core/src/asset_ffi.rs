use std::ffi::c_char;
use crate::asset_library::{get_asset_library, MeshType};
use crate::project::{get_project, ProjectSettings, ProjectEntity, ProjectEntityTransform};
use crate::ecs::Scene;

#[unsafe(no_mangle)]
pub extern "C" fn asset_library_get_category_count() -> usize {
    get_asset_library().get_category_count()
}

#[unsafe(no_mangle)]
pub extern "C" fn asset_library_get_category_name(index: usize, buffer: *mut c_char, buffer_size: usize) -> bool {
    if buffer.is_null() || buffer_size == 0 {
        return false;
    }
    
    let lib = get_asset_library();
    if let Some(name) = lib.get_category_name(index) {
        let bytes = name.as_bytes();
        let copy_len = bytes.len().min(buffer_size - 1);
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer as *mut u8, copy_len);
            *buffer.add(copy_len) = 0;
        }
        true
    } else {
        unsafe { *buffer = 0; }
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn asset_library_get_asset_count(category_index: usize) -> usize {
    get_asset_library().get_asset_count_in_category(category_index)
}

#[unsafe(no_mangle)]
pub extern "C" fn asset_library_get_asset_info(
    category_index: usize,
    asset_index: usize,
    id_buffer: *mut u64,
    name_buffer: *mut c_char,
    name_size: usize,
    type_buffer: *mut u32,
    desc_buffer: *mut c_char,
    desc_size: usize
) -> bool {
    if id_buffer.is_null() || name_buffer.is_null() || type_buffer.is_null() {
        return false;
    }
    
    let lib = get_asset_library();
    if let Some(asset) = lib.get_asset_by_category_index(category_index, asset_index) {
        unsafe {
            *id_buffer = asset.id;
            *type_buffer = asset.asset_type as u32;
            
            let name_bytes = asset.name.as_bytes();
            let name_copy_len = name_bytes.len().min(name_size - 1);
            std::ptr::copy_nonoverlapping(name_bytes.as_ptr(), name_buffer as *mut u8, name_copy_len);
            *name_buffer.add(name_copy_len) = 0;
            
            if !desc_buffer.is_null() && desc_size > 0 {
                if let Some(desc) = &asset.description {
                    let desc_bytes = desc.as_bytes();
                    let desc_copy_len = desc_bytes.len().min(desc_size - 1);
                    std::ptr::copy_nonoverlapping(desc_bytes.as_ptr(), desc_buffer as *mut u8, desc_copy_len);
                    *desc_buffer.add(desc_copy_len) = 0;
                } else {
                    *desc_buffer = 0;
                }
            }
        }
        true
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn asset_library_create_entity_from_template(scene: *mut Scene, template_id: u64) -> u64 {
    if scene.is_null() {
        return 0;
    }
    
    let lib = get_asset_library();
    unsafe {
        match lib.create_entity_from_template(template_id, &mut *scene) {
            Some(entity) => entity.id,
            None => 0,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn asset_library_create_mesh_entity(scene: *mut Scene, mesh_type: u32) -> u64 {
    if scene.is_null() {
        return 0;
    }
    
    let mesh = MeshType::from_index(mesh_type);
    let lib = get_asset_library();
    unsafe {
        let entity = lib.create_entity_with_mesh(mesh, &mut *scene);
        entity.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn project_create_new(name: *const c_char, path: *const c_char) -> bool {
    if name.is_null() || path.is_null() {
        return false;
    }
    
    let name_str = unsafe { std::ffi::CStr::from_ptr(name).to_string_lossy().into_owned() };
    let path_str = unsafe { std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned() };
    let path_buf = std::path::PathBuf::from(&path_str);
    
    match crate::project::Project::create_new(name_str, path_buf) {
        Ok(new_project) => {
            let mut project = get_project().lock().unwrap();
            *project = new_project;
            true
        }
        Err(_) => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn project_load(path: *const c_char) -> bool {
    if path.is_null() {
        return false;
    }
    
    let path_str = unsafe { std::ffi::CStr::from_ptr(path).to_string_lossy().into_owned() };
    let path_buf = std::path::PathBuf::from(&path_str);
    
    match crate::project::Project::load(path_buf) {
        Ok(loaded_project) => {
            let mut project = get_project().lock().unwrap();
            *project = loaded_project;
            true
        }
        Err(_) => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn project_save() -> bool {
    let mut project = get_project().lock().unwrap();
    project.save().is_ok()
}

#[unsafe(no_mangle)]
pub extern "C" fn project_get_name(buffer: *mut c_char, size: usize) -> bool {
    if buffer.is_null() || size == 0 {
        return false;
    }
    
    let project = get_project().lock().unwrap();
    let name = project.get_name();
    let bytes = name.as_bytes();
    let copy_len = bytes.len().min(size - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer as *mut u8, copy_len);
        *buffer.add(copy_len) = 0;
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn project_get_path(buffer: *mut c_char, size: usize) -> bool {
    if buffer.is_null() || size == 0 {
        return false;
    }
    
    let project = get_project().lock().unwrap();
    let path_str = project.get_path().to_string_lossy().to_string();
    let bytes = path_str.as_bytes();
    let copy_len = bytes.len().min(size - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer as *mut u8, copy_len);
        *buffer.add(copy_len) = 0;
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn project_get_entity_count() -> usize {
    let project = get_project().lock().unwrap();
    project.get_entity_count()
}

#[unsafe(no_mangle)]
pub extern "C" fn project_is_loaded() -> bool {
    let project = get_project().lock().unwrap();
    project.is_loaded()
}

#[unsafe(no_mangle)]
pub extern "C" fn project_sync_to_scene(scene: *mut Scene) {
    if scene.is_null() {
        return;
    }
    
    let project = get_project().lock().unwrap();
    unsafe {
        project.sync_to_scene(&mut *scene);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn project_sync_from_scene(scene: *const Scene) {
    if scene.is_null() {
        return;
    }
    
    let mut project = get_project().lock().unwrap();
    unsafe {
        project.sync_from_scene(&*scene);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn project_get_settings(out_width: *mut u32, out_height: *mut u32, out_fps: *mut u32) -> bool {
    if out_width.is_null() || out_height.is_null() || out_fps.is_null() {
        return false;
    }
    
    let project = get_project().lock().unwrap();
    let settings = project.get_settings();
    
    unsafe {
        *out_width = settings.game_width;
        *out_height = settings.game_height;
        *out_fps = settings.fps;
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn project_set_settings(width: u32, height: u32, fps: u32) -> bool {
    let mut project = get_project().lock().unwrap();
    project.set_settings(ProjectSettings {
        game_width: width,
        game_height: height,
        fps,
    });
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn project_get_entity_info(
    entity_id: u64,
    name_buffer: *mut c_char,
    name_size: usize,
    out_pos: *mut f32,
    out_rot: *mut f32,
    out_scale: *mut f32,
    mesh_type_buffer: *mut c_char,
    mesh_type_size: usize
) -> bool {
    let project = get_project().lock().unwrap();
    
    if let Some(entity) = project.get_entity(entity_id) {
        unsafe {
            if !name_buffer.is_null() && name_size > 0 {
                let bytes = entity.name.as_bytes();
                let copy_len = bytes.len().min(name_size - 1);
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), name_buffer as *mut u8, copy_len);
                *name_buffer.add(copy_len) = 0;
            }
            
            if !out_pos.is_null() {
                *out_pos = entity.transform.position[0];
                *out_pos.add(1) = entity.transform.position[1];
                *out_pos.add(2) = entity.transform.position[2];
            }
            
            if !out_rot.is_null() {
                *out_rot = entity.transform.rotation[0];
                *out_rot.add(1) = entity.transform.rotation[1];
                *out_rot.add(2) = entity.transform.rotation[2];
            }
            
            if !out_scale.is_null() {
                *out_scale = entity.transform.scale[0];
                *out_scale.add(1) = entity.transform.scale[1];
                *out_scale.add(2) = entity.transform.scale[2];
            }
            
            if !mesh_type_buffer.is_null() && mesh_type_size > 0 {
                let bytes = entity.mesh_type.as_bytes();
                let copy_len = bytes.len().min(mesh_type_size - 1);
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), mesh_type_buffer as *mut u8, copy_len);
                *mesh_type_buffer.add(copy_len) = 0;
            }
        }
        true
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn project_add_entity(
    entity_id: u64,
    name: *const c_char,
    pos_x: f32, pos_y: f32, pos_z: f32,
    rot_x: f32, rot_y: f32, rot_z: f32,
    scale_x: f32, scale_y: f32, scale_z: f32,
    mesh_type: *const c_char
) -> bool {
    if name.is_null() || mesh_type.is_null() {
        return false;
    }
    
    let name_str = unsafe { std::ffi::CStr::from_ptr(name).to_string_lossy().into_owned() };
    let mesh_type_str = unsafe { std::ffi::CStr::from_ptr(mesh_type).to_string_lossy().into_owned() };
    
    let mut project = get_project().lock().unwrap();
    
    project.add_entity(ProjectEntity {
        id: entity_id,
        name: name_str,
        transform: ProjectEntityTransform {
            position: [pos_x, pos_y, pos_z],
            rotation: [rot_x, rot_y, rot_z],
            scale: [scale_x, scale_y, scale_z],
        },
        mesh_type: mesh_type_str,
        scripts: Vec::new(),
        components: std::collections::HashMap::new(),
    });
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn project_remove_entity(entity_id: u64) -> bool {
    let mut project = get_project().lock().unwrap();
    project.remove_entity(entity_id);
    true
}