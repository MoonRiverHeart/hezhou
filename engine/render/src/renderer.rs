use hezhou_core::math::*;
use crate::color::Color;
use crate::camera::Camera;
use crate::surface::RenderSurface;

pub struct Renderer {
    surface: Option<RenderSurface>,
    camera: Camera,
    clear_color: Color,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            surface: None,
            camera: Camera::new(0),
            clear_color: Color::black(),
        }
    }
    
    pub fn set_surface(&mut self, surface: RenderSurface) {
        self.surface = Some(surface);
    }
    
    pub fn get_surface(&self) -> Option<&RenderSurface> {
        self.surface.as_ref()
    }
    
    pub fn get_surface_mut(&mut self) -> Option<&mut RenderSurface> {
        self.surface.as_mut()
    }
    
    pub fn resize(&mut self, width: i32, height: i32) {
        if let Some(surface) = &mut self.surface {
            surface.resize(width, height);
            self.camera.set_aspect(width as f32 / height as f32);
        }
    }
    
    pub fn destroy_surface(&mut self) {
        self.surface = None;
    }
    
    pub fn begin_frame(&self) {
        if let Some(surface) = &self.surface {
            surface.make_current();
        }
    }
    
    pub fn end_frame(&self) {
        if let Some(surface) = &self.surface {
            surface.present();
        }
    }
    
    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
    }
    
    pub fn get_camera(&self) -> &Camera {
        &self.camera
    }
    
    pub fn get_camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }
    
    pub fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }
    
    pub fn get_clear_color(&self) -> &Color {
        &self.clear_color
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}