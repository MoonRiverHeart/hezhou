use std::ffi::{CString, c_char, CStr};
use std::sync::{LazyLock, Mutex};
use crate::ecs::property::*;
use crate::ecs::{Entity, Scene};

/// Static cache for CString pointers returned across FFI.
/// Raw pointers must remain valid for the lifetime of the program,
/// so we leak CStrings into this cache and never free them.
static CSTRING_CACHE: LazyLock<Mutex<Vec<CString>>> = LazyLock::new(|| Mutex::new(Vec::new()));

fn cache_cstring(s: &str) -> *const c_char {
    let cstr = CString::new(s).unwrap_or_else(|_| CString::new("").unwrap());
    let ptr = cstr.as_ptr();
    let mut cache = CSTRING_CACHE.lock().unwrap();
    cache.push(cstr);
    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_entity_get_property_count() -> u32 {
    get_entity_property_descriptors().len() as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_entity_get_property_name(index: u32) -> *const c_char {
    let descriptors = get_entity_property_descriptors();
    if index < descriptors.len() as u32 {
        cache_cstring(&descriptors[index as usize].name)
    } else {
        cache_cstring("")
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_entity_get_property_type(index: u32) -> u32 {
    let descriptors = get_entity_property_descriptors();
    if index < descriptors.len() as u32 {
        descriptors[index as usize].property_type as u32
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_entity_get_property_category(index: u32) -> *const c_char {
    let descriptors = get_entity_property_descriptors();
    if index < descriptors.len() as u32 {
        cache_cstring(&descriptors[index as usize].category)
    } else {
        cache_cstring("")
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_entity_get_property_read_only(index: u32) -> bool {
    let descriptors = get_entity_property_descriptors();
    if index < descriptors.len() as u32 {
        descriptors[index as usize].read_only
    } else {
        true
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_entity_get_property_value_float3(
    scene: *mut Scene,
    entity_id: u64,
    property_name: *const c_char,
    out_x: *mut f32,
    out_y: *mut f32,
    out_z: *mut f32,
) -> bool {
    if scene.is_null() || property_name.is_null() || out_x.is_null() || out_y.is_null() || out_z.is_null() {
        return false;
    }
    let name_str = unsafe { CStr::from_ptr(property_name).to_string_lossy().into_owned() };
    unsafe {
        let entity = Entity::new(entity_id);
        let value = get_entity_property_value(&*scene, entity, &name_str);
        match value {
            Some(PropertyValue::Float3 { x, y, z }) => {
                *out_x = x;
                *out_y = y;
                *out_z = z;
                true
            }
            _ => false,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_entity_get_property_value_string(
    scene: *mut Scene,
    entity_id: u64,
    property_name: *const c_char,
    out_buf: *mut c_char,
    buf_len: u32,
) -> u32 {
    if scene.is_null() || property_name.is_null() || out_buf.is_null() || buf_len == 0 {
        return 0;
    }
    let name_str = unsafe { CStr::from_ptr(property_name).to_string_lossy().into_owned() };
    unsafe {
        let entity = Entity::new(entity_id);
        let value = get_entity_property_value(&*scene, entity, &name_str);
        match value {
            Some(PropertyValue::String(s)) => {
                let bytes = s.as_bytes();
                let copy_len = bytes.len().min(buf_len as usize - 1);
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf as *mut u8, copy_len);
                *out_buf.add(copy_len) = 0;
                s.len() as u32
            }
            Some(PropertyValue::Int(i)) => {
                let s = i.to_string();
                let bytes = s.as_bytes();
                let copy_len = bytes.len().min(buf_len as usize - 1);
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf as *mut u8, copy_len);
                *out_buf.add(copy_len) = 0;
                s.len() as u32
            }
            Some(PropertyValue::Bool(b)) => {
                let s = if b { "true" } else { "false" };
                let bytes = s.as_bytes();
                let copy_len = bytes.len().min(buf_len as usize - 1);
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf as *mut u8, copy_len);
                *out_buf.add(copy_len) = 0;
                s.len() as u32
            }
            _ => {
                *out_buf = 0;
                0
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_entity_set_property_value_float3(
    scene: *mut Scene,
    entity_id: u64,
    property_name: *const c_char,
    x: f32,
    y: f32,
    z: f32,
) -> bool {
    if scene.is_null() || property_name.is_null() {
        return false;
    }
    let name_str = unsafe { CStr::from_ptr(property_name).to_string_lossy().into_owned() };
    unsafe {
        let entity = Entity::new(entity_id);
        let value = PropertyValue::Float3 { x, y, z };
        set_entity_property_value(&mut *scene, entity, &name_str, &value)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_entity_set_property_value_string(
    scene: *mut Scene,
    entity_id: u64,
    property_name: *const c_char,
    value: *const c_char,
) -> bool {
    if scene.is_null() || property_name.is_null() || value.is_null() {
        return false;
    }
    let name_str = unsafe { CStr::from_ptr(property_name).to_string_lossy().into_owned() };
    let value_str = unsafe { CStr::from_ptr(value).to_string_lossy().into_owned() };
    unsafe {
        let entity = Entity::new(entity_id);
        // Handle Bool values passed as string "true"/"false"
        if value_str == "true" || value_str == "false" {
            let b = value_str == "true";
            let pv = PropertyValue::Bool(b);
            set_entity_property_value(&mut *scene, entity, &name_str, &pv)
        } else {
            let pv = PropertyValue::String(value_str);
            set_entity_property_value(&mut *scene, entity, &name_str, &pv)
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_entity_get_property_value_float(
    scene: *mut Scene,
    entity_id: u64,
    property_name: *const c_char,
    out_value: *mut f32,
) -> bool {
    if scene.is_null() || property_name.is_null() || out_value.is_null() {
        return false;
    }
    let name_str = unsafe { CStr::from_ptr(property_name).to_string_lossy().into_owned() };
    unsafe {
        let entity = Entity::new(entity_id);
        let value = get_entity_property_value(&*scene, entity, &name_str);
        match value {
            Some(PropertyValue::Float(f)) => {
                *out_value = f;
                true
            }
            _ => false,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_entity_set_property_value_float(
    scene: *mut Scene,
    entity_id: u64,
    property_name: *const c_char,
    value: f32,
) -> bool {
    if scene.is_null() || property_name.is_null() {
        return false;
    }
    let name_str = unsafe { CStr::from_ptr(property_name).to_string_lossy().into_owned() };
    unsafe {
        let entity = Entity::new(entity_id);
        let pv = PropertyValue::Float(value);
        set_entity_property_value(&mut *scene, entity, &name_str, &pv)
    }
}