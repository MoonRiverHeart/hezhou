use std::ffi::{CStr, CString};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use hezhou_core::asset_ffi::*;
use hezhou_core::scene_ffi::*;

fn unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("hezhou_ffi_{}_{}", std::process::id(), nanos))
}

#[test]
fn project_ffi_create_sync_reload_and_null_safety() {
    // 全局 Project 单例只在该测试中使用，避免其他测试并行污染。
    unsafe {
        assert!(!project_create_new(std::ptr::null(), std::ptr::null()));
        assert!(!project_load(std::ptr::null()));
        assert!(!project_get_name(std::ptr::null_mut(), 0));
        project_sync_to_scene(std::ptr::null_mut());
        project_sync_from_scene(std::ptr::null());
    }

    let temp = unique_temp_dir();
    std::fs::create_dir_all(&temp).unwrap();

    let name = CString::new("FfiContract").unwrap();
    let root = CString::new(temp.to_string_lossy().to_string()).unwrap();
    unsafe {
        assert!(project_create_new(name.as_ptr(), root.as_ptr()));
    }
    assert!(project_is_loaded());
    assert_eq!(project_get_entity_count(), 0);

    let mut name_buf = [0i8; 64];
    assert!(project_get_name(name_buf.as_mut_ptr(), name_buf.len()));
    let read_name = unsafe { CStr::from_ptr(name_buf.as_ptr()) }
        .to_string_lossy()
        .to_string();
    assert_eq!(read_name, "FfiContract");

    let scene = scene_create();
    let cube = scene_create_cube(scene);
    unsafe {
        project_sync_from_scene(scene);
    }
    assert_eq!(project_get_entity_count(), 1);
    assert!(project_save());

    // 再次 sync_to_scene 必须是替换语义：场景仍只有 1 个实体。
    unsafe {
        project_sync_to_scene(scene);
    }
    assert_eq!(scene_entity_count(scene), 1);

    let mut pos = [0.0f32; 3];
    let mut rot = [0.0f32; 3];
    let mut scale = [0.0f32; 3];
    let mut mesh_buf = [0i8; 64];
    let mut entity_name = [0i8; 64];
    let ok = project_get_entity_info(
        cube,
        entity_name.as_mut_ptr(),
        entity_name.len(),
        pos.as_mut_ptr(),
        rot.as_mut_ptr(),
        scale.as_mut_ptr(),
        mesh_buf.as_mut_ptr(),
        mesh_buf.len(),
    );
    assert!(ok);
    let mesh = unsafe { CStr::from_ptr(mesh_buf.as_ptr()) }
        .to_string_lossy()
        .to_string();
    assert_eq!(mesh, "Cube");

    // 支持直接加载工程目录内的 project.json。
    let project_json = temp.join("FfiContract").join("project.json");
    let json = CString::new(project_json.to_string_lossy().to_string()).unwrap();
    unsafe {
        assert!(project_load(json.as_ptr()));
    }

    unsafe {
        scene_destroy(scene);
    }
    let _ = std::fs::remove_dir_all(&temp);
}
