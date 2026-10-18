use std::ffi::{CStr, CString};
use std::ptr;

use hezhou_core::property_ffi::*;
use hezhou_core::scene_ffi::*;

#[test]
fn scene_ffi_null_inputs_are_rejected_without_crashing() {
    unsafe {
        scene_destroy(ptr::null_mut());
        assert_eq!(scene_create_entity(ptr::null_mut()), 0);
        assert_eq!(scene_create_cube(ptr::null_mut()), 0);
        assert_eq!(scene_entity_count(ptr::null()), 0);
        assert_eq!(scene_get_game_state(ptr::null()), 0);
        assert_eq!(scene_pick_entity(ptr::null(), 0.0, 0.0, 0.0, 0.0, 0.0, -1.0), 0);
        scene_set_entity_name(ptr::null_mut(), 0, ptr::null());
        scene_attach_script(ptr::null_mut(), 0, ptr::null(), ptr::null());
        scene_attach_script_binding(ptr::null_mut(), 0, ptr::null(), ptr::null());
        scene_set_parent(ptr::null_mut(), 0, 0);
        scene_remove_entity(ptr::null_mut(), 0);
        scene_update(ptr::null_mut(), 0.016);
    }

    // 属性 FFI 同样必须对 null 保持安全。
    let mut value = 0.0f32;
    let name = CString::new("position").unwrap();
    unsafe {
        assert!(!ui_entity_get_property_value_float(
            ptr::null_mut(),
            0,
            name.as_ptr(),
            &mut value
        ));
        assert!(!ui_entity_set_property_value_float(
            ptr::null_mut(),
            0,
            name.as_ptr(),
            1.0
        ));
        assert!(!ui_entity_get_property_value_float3(
            ptr::null_mut(),
            0,
            name.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut()
        ));
    }
}

#[test]
fn scene_ffi_roundtrip_transforms_names_parent_and_properties() {
    let scene = scene_create();
    assert!(!scene.is_null(), "scene_create returned null");

    let parent = scene_create_cube(scene);
    let child = scene_create_entity(scene);
    assert!(parent > 0);
    assert!(child > 0);

    let name = CString::new("Cube_FFI").unwrap();
    unsafe {
        scene_set_entity_name(scene, parent, name.as_ptr());
    }

    let mut name_buf = [0i8; 64];
    let written = scene_get_entity_name(scene, parent, name_buf.as_mut_ptr(), name_buf.len());
    assert!(written > 0);
    let read_name = unsafe { CStr::from_ptr(name_buf.as_ptr()) }
        .to_string_lossy()
        .to_string();
    assert_eq!(read_name, "Cube_FFI");

    scene_set_entity_position(scene, parent, 1.0, 2.0, 3.0);
    let (mut x, mut y, mut z) = (0.0f32, 0.0f32, 0.0f32);
    scene_get_entity_position(scene, parent, &mut x, &mut y, &mut z);
    assert!((x - 1.0).abs() < 0.001);
    assert!((y - 2.0).abs() < 0.001);
    assert!((z - 3.0).abs() < 0.001);

    scene_set_parent(scene, child, parent);
    assert_eq!(scene_get_parent(scene, child), parent);
    assert_eq!(scene_get_child_count(scene, parent), 1);
    assert_eq!(scene_get_child_id(scene, parent, 0), child);

    // 属性反射 FFI：名称/类型/读写与值接口。
    assert_eq!(ui_entity_get_property_count(), 10);
    let prop_name = unsafe { CStr::from_ptr(ui_entity_get_property_name(0)) }
        .to_string_lossy()
        .to_string();
    assert_eq!(prop_name, "name");
    assert_eq!(ui_entity_get_property_type(1), 1); // position = Float3

    let visible_name = CString::new("visible").unwrap();
    let false_value = CString::new("false").unwrap();
    assert!(unsafe {
        ui_entity_set_property_value_string(
            scene,
            parent,
            visible_name.as_ptr(),
            false_value.as_ptr(),
        )
    });

    let mut visible_buf = [0i8; 16];
    assert!(unsafe {
        ui_entity_get_property_value_string(
            scene,
            parent,
            visible_name.as_ptr(),
            visible_buf.as_mut_ptr(),
            visible_buf.len() as u32,
        ) > 0
    });
    let visible = unsafe { CStr::from_ptr(visible_buf.as_ptr()) }
        .to_string_lossy()
        .to_string();
    assert_eq!(visible, "false");

    unsafe {
        scene_destroy(scene);
    }
}
