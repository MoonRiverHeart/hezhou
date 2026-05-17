use crate::*;
use hezhou_core::math::*;
use hezhou_harmony::OH_NativeWindow;
use std::ffi::{c_char, c_float, c_int};

#[unsafe(no_mangle)]
pub extern "C" fn render_engine_create() -> *mut RenderEngine {
    let engine = Box::new(RenderEngine::new());
    Box::into_raw(engine)
}

#[unsafe(no_mangle)]
pub extern "C" fn render_engine_destroy(engine: *mut RenderEngine) {
    if engine.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(engine);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn render_init_surface(
    engine: *mut RenderEngine,
    window: *mut OH_NativeWindow,
    width: c_int,
    height: c_int,
) {
    if engine.is_null() {
        return;
    }
    unsafe {
        (*engine).init_surface(window, width, height);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn render_resize(engine: *mut RenderEngine, width: c_int, height: c_int) {
    if engine.is_null() {
        return;
    }
    unsafe {
        (*engine).resize(width, height);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn render_begin_frame(engine: *mut RenderEngine) {
    if engine.is_null() {
        return;
    }
    unsafe {
        (*engine).begin_frame();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn render_end_frame(engine: *mut RenderEngine) {
    if engine.is_null() {
        return;
    }
    unsafe {
        (*engine).end_frame();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn render_set_clear_color(
    engine: *mut RenderEngine,
    r: c_float,
    g: c_float,
    b: c_float,
    a: c_float,
) {
    if engine.is_null() {
        return;
    }
    unsafe {
        (*engine).set_clear_color(Color { r, g, b, a });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn render_create_camera(engine: *mut RenderEngine) -> CameraId {
    if engine.is_null() {
        return 0;
    }
    unsafe {
        let camera = (*engine).create_camera();
        camera.id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn render_camera_set_position(
    camera_id: CameraId,
    x: c_float,
    y: c_float,
    z: c_float,
) {
}

#[unsafe(no_mangle)]
pub extern "C" fn render_camera_set_rotation(
    camera_id: CameraId,
    qx: c_float,
    qy: c_float,
    qz: c_float,
    qw: c_float,
) {
}

#[unsafe(no_mangle)]
pub extern "C" fn render_camera_set_fov(camera_id: CameraId, fov: c_float) {}

#[unsafe(no_mangle)]
pub extern "C" fn render_camera_set_clip_planes(camera_id: CameraId, near: c_float, far: c_float) {}

#[unsafe(no_mangle)]
pub extern "C" fn render_camera_set_aspect(camera_id: CameraId, aspect: c_float) {}

#[unsafe(no_mangle)]
pub extern "C" fn render_camera_look_at(
    camera_id: CameraId,
    tx: c_float,
    ty: c_float,
    tz: c_float,
) {
}

#[unsafe(no_mangle)]
pub extern "C" fn renderer_create() -> *mut Renderer {
    let renderer = Box::new(Renderer::new());
    Box::into_raw(renderer)
}

#[unsafe(no_mangle)]
pub extern "C" fn renderer_destroy(renderer: *mut Renderer) {
    if renderer.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(renderer);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn renderer_set_clear_color(
    renderer: *mut Renderer,
    r: c_float,
    g: c_float,
    b: c_float,
    a: c_float,
) {
    if renderer.is_null() {
        return;
    }
    unsafe {
        (*renderer).set_clear_color(Color { r, g, b, a });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn renderer_resize(renderer: *mut Renderer, width: c_int, height: c_int) {
    if renderer.is_null() {
        return;
    }
    unsafe {
        (*renderer).resize(width, height);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn renderer_begin_frame(renderer: *mut Renderer) {
    if renderer.is_null() {
        return;
    }
    unsafe {
        (*renderer).begin_frame();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn renderer_end_frame(renderer: *mut Renderer) {
    if renderer.is_null() {
        return;
    }
    unsafe {
        (*renderer).end_frame();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mesh_create_triangle() -> MeshId {
    static mut NEXT_MESH_ID: MeshId = 1;
    unsafe {
        let id = NEXT_MESH_ID;
        NEXT_MESH_ID += 1;
        id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mesh_create_quad() -> MeshId {
    static mut NEXT_MESH_ID: MeshId = 1000;
    unsafe {
        let id = NEXT_MESH_ID;
        NEXT_MESH_ID += 1;
        id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mesh_transform(
    mesh_id: MeshId,
    tx: c_float,
    ty: c_float,
    tz: c_float,
    rx: c_float,
    ry: c_float,
    rz: c_float,
    rw: c_float,
    sx: c_float,
    sy: c_float,
    sz: c_float,
) {
}

#[unsafe(no_mangle)]
pub extern "C" fn texture_create(
    width: u32,
    height: u32,
    format: u32,
    data: *const u8,
    data_size: usize,
) -> TextureId {
    static mut NEXT_TEXTURE_ID: TextureId = 1;
    unsafe {
        let id = NEXT_TEXTURE_ID;
        NEXT_TEXTURE_ID += 1;
        id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn material_create(shader_name: *const c_char) -> MaterialId {
    static mut NEXT_MATERIAL_ID: MaterialId = 1;
    unsafe {
        let id = NEXT_MATERIAL_ID;
        NEXT_MATERIAL_ID += 1;
        id
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn material_set_base_color(
    material_id: MaterialId,
    r: c_float,
    g: c_float,
    b: c_float,
    a: c_float,
) {
}

#[unsafe(no_mangle)]
pub extern "C" fn material_set_texture(material_id: MaterialId, slot: u32, texture_id: TextureId) {}
