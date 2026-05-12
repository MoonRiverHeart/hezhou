pub mod surface;
pub mod camera;
pub mod renderer;
pub mod color;
pub mod mesh;
pub mod material;
pub mod texture;
pub mod ffi;

pub use surface::*;
pub use camera::*;
pub use renderer::*;
pub use color::*;
pub use mesh::*;
pub use material::*;
pub use texture::*;

use hezhou_core::math::*;
use hezhou_harmony::OH_NativeWindow;
use crate::color::Color;
use crate::camera::Camera;
use crate::texture::TextureId;

pub struct RenderEngine {
    surface: Option<RenderSurface>,
    camera: Option<Camera>,
    clear_color: Color,
    mesh_count: u64,
}

impl RenderEngine {
    pub fn new() -> Self {
        Self {
            surface: None,
            camera: None,
            clear_color: Color::black(),
            mesh_count: 0,
        }
    }
    
    pub fn init_surface(&mut self, window: *mut OH_NativeWindow, width: i32, height: i32) {
        let mut surface = RenderSurface::create(window, width, height);
        surface.init();
        self.surface = Some(surface);
    }
    
    pub fn resize(&mut self, width: i32, height: i32) {
        if let Some(surface) = &mut self.surface {
            surface.resize(width, height);
        }
    }
    
    pub fn destroy_surface(&mut self) {
        self.surface = None;
    }
    
    pub fn begin_frame(&mut self) {
        if let Some(surface) = &self.surface {
            surface.make_current();
        }
    }
    
    pub fn end_frame(&mut self) {
        if let Some(surface) = &self.surface {
            surface.present();
        }
    }
    
    pub fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }
    
    pub fn clear(&self) {
    }
    
    pub fn create_camera(&mut self) -> &mut Camera {
        let camera = Camera::new(1);
        self.camera = Some(camera);
        self.camera.as_mut().unwrap()
    }
    
    pub fn get_camera(&self) -> Option<&Camera> {
        self.camera.as_ref()
    }
    
    pub fn get_camera_mut(&mut self) -> Option<&mut Camera> {
        self.camera.as_mut()
    }
}

impl Default for RenderEngine {
    fn default() -> Self {
        Self::new()
    }
}