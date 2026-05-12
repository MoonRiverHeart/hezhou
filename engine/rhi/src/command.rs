use crate::{BufferHandle, TextureHandle, PipelineHandle, RenderPassHandle, FramebufferHandle, CommandBufferHandle};
use hezhou_core::math::Vec3u;

pub trait CommandBuffer {
    fn handle(&self) -> CommandBufferHandle;
    
    fn begin(&mut self);
    fn end(&mut self);
    
    fn begin_render_pass(&mut self, pass: RenderPassHandle, framebuffer: FramebufferHandle, clear_values: &[ClearValue]);
    fn end_render_pass(&mut self);
    
    fn bind_pipeline(&mut self, pipeline: PipelineHandle);
    fn bind_vertex_buffer(&mut self, slot: u32, buffer: BufferHandle, offset: usize);
    fn bind_index_buffer(&mut self, buffer: BufferHandle, offset: usize, index_type: IndexType);
    fn bind_uniform_buffer(&mut self, set: u32, binding: u32, buffer: BufferHandle, offset: usize, size: usize);
    fn bind_texture(&mut self, set: u32, binding: u32, texture: TextureHandle);
    
    fn set_viewport(&mut self, x: f32, y: f32, width: f32, height: f32, min_depth: f32, max_depth: f32);
    fn set_scissor(&mut self, x: u32, y: u32, width: u32, height: u32);
    fn set_push_constants(&mut self, offset: usize, data: &[u8]);
    
    fn draw(&mut self, vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32);
    fn draw_indexed(&mut self, index_count: u32, instance_count: u32, first_index: u32, vertex_offset: i32, first_instance: u32);
    
    fn copy_buffer(&mut self, src: BufferHandle, dst: BufferHandle, src_offset: usize, dst_offset: usize, size: usize);
    fn copy_buffer_to_texture(&mut self, src: BufferHandle, dst: TextureHandle, dst_offset: Vec3u, extent: Vec3u);
}

#[derive(Clone, Copy, Debug)]
pub enum IndexType {
    U16,
    U32,
}

#[derive(Clone, Copy, Debug)]
pub enum ClearValue {
    Color([f32; 4]),
    Depth(f32),
    Stencil(u32),
    DepthStencil(f32, u32),
}

impl ClearValue {
    pub fn color(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self::Color([r, g, b, a])
    }
    
    pub fn depth(depth: f32) -> Self {
        Self::Depth(depth)
    }
    
    pub fn depth_stencil(depth: f32, stencil: u32) -> Self {
        Self::DepthStencil(depth, stencil)
    }
}