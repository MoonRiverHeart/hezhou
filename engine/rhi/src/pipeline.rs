use crate::ShaderHandle;
use hezhou_geometry::{VertexLayout, PrimitiveTopology};

#[derive(Clone, Debug)]
pub struct PipelineDesc {
    pub vertex_shader: ShaderHandle,
    pub fragment_shader: Option<ShaderHandle>,
    pub geometry_shader: Option<ShaderHandle>,
    pub vertex_layout: VertexLayout,
    pub primitive_topology: PrimitiveTopology,
    pub rasterization: RasterizationState,
    pub depth_stencil: DepthStencilState,
    pub blend: BlendState,
    pub layout: PipelineLayout,
}

impl PipelineDesc {
    pub fn new(vertex_shader: ShaderHandle) -> Self {
        Self {
            vertex_shader,
            fragment_shader: None,
            geometry_shader: None,
            vertex_layout: VertexLayout::default(),
            primitive_topology: PrimitiveTopology::TriangleList,
            rasterization: RasterizationState::default(),
            depth_stencil: DepthStencilState::default(),
            blend: BlendState::default(),
            layout: PipelineLayout::default(),
        }
    }
    
    pub fn fragment(mut self, shader: ShaderHandle) -> Self {
        self.fragment_shader = Some(shader);
        self
    }
    
    pub fn topology(mut self, topology: PrimitiveTopology) -> Self {
        self.primitive_topology = topology;
        self
    }
    
    pub fn vertex_layout(mut self, layout: VertexLayout) -> Self {
        self.vertex_layout = layout;
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Geometry,
    Compute,
}

#[derive(Clone, Debug, Default)]
pub struct RasterizationState {
    pub fill_mode: FillMode,
    pub cull_mode: CullMode,
    pub front_face: FrontFace,
    pub depth_bias_enable: bool,
    pub depth_bias_constant_factor: f32,
    pub depth_bias_clamp: f32,
    pub depth_bias_slope_factor: f32,
    pub depth_clamp_enable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillMode {
    Fill,
    Line,
    Point,
}

impl Default for FillMode {
    fn default() -> Self { Self::Fill }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CullMode {
    None,
    Front,
    Back,
    FrontAndBack,
}

impl Default for CullMode {
    fn default() -> Self { Self::Back }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontFace {
    CounterClockwise,
    Clockwise,
}

impl Default for FrontFace {
    fn default() -> Self { Self::CounterClockwise }
}

#[derive(Clone, Debug, Default)]
pub struct DepthStencilState {
    pub depth_test_enable: bool,
    pub depth_write_enable: bool,
    pub depth_compare_op: CompareOp,
    pub stencil_test_enable: bool,
    pub front: StencilOpState,
    pub back: StencilOpState,
}

impl DepthStencilState {
    pub fn depth_none() -> Self {
        Self { depth_test_enable: false, depth_write_enable: false, ..Default::default() }
    }
    
    pub fn depth_read() -> Self {
        Self { depth_test_enable: true, depth_write_enable: false, ..Default::default() }
    }
    
    pub fn depth_write() -> Self {
        Self { depth_test_enable: true, depth_write_enable: true, ..Default::default() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompareOp {
    Never, Less, Equal, LessOrEqual, Greater, NotEqual, GreaterOrEqual, Always,
}

impl Default for CompareOp {
    fn default() -> Self { Self::Less }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StencilOpState {
    pub fail_op: StencilOp,
    pub pass_op: StencilOp,
    pub depth_fail_op: StencilOp,
    pub compare_op: CompareOp,
    pub compare_mask: u32,
    pub write_mask: u32,
    pub reference: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StencilOp {
    Keep, Zero, Replace, IncrementAndClamp, DecrementAndClamp, Invert, IncrementAndWrap, DecrementAndWrap,
}

impl Default for StencilOp {
    fn default() -> Self { Self::Keep }
}

#[derive(Clone, Debug, Default)]
pub struct BlendState {
    pub enable: bool,
    pub color_blend_op: BlendOp,
    pub src_color_blend_factor: BlendFactor,
    pub dst_color_blend_factor: BlendFactor,
    pub alpha_blend_op: BlendOp,
    pub src_alpha_blend_factor: BlendFactor,
    pub dst_alpha_blend_factor: BlendFactor,
    pub color_write_mask: ColorComponentFlags,
}

impl BlendState {
    pub fn opaque() -> Self { Self { enable: false, ..Default::default() } }
    pub fn alpha_blend() -> Self {
        Self {
            enable: true,
            src_color_blend_factor: BlendFactor::SrcAlpha,
            dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
            ..Default::default()
        }
    }
    pub fn additive() -> Self {
        Self {
            enable: true,
            src_color_blend_factor: BlendFactor::SrcAlpha,
            dst_color_blend_factor: BlendFactor::One,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::One,
            ..Default::default()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendOp { Add, Subtract, ReverseSubtract, Min, Max }

impl Default for BlendOp {
    fn default() -> Self { Self::Add }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendFactor {
    Zero, One, SrcColor, OneMinusSrcColor, SrcAlpha, OneMinusSrcAlpha,
    DstColor, OneMinusDstColor, DstAlpha, OneMinusDstAlpha, SrcAlphaSaturate,
    ConstantColor, OneMinusConstantColor, ConstantAlpha, OneMinusConstantAlpha,
}

impl Default for BlendFactor {
    fn default() -> Self { Self::Zero }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ColorComponentFlags {
    pub r: bool, pub g: bool, pub b: bool, pub a: bool,
}

impl ColorComponentFlags {
    pub const ALL: Self = Self { r: true, g: true, b: true, a: true };
    pub const NONE: Self = Self { r: false, g: false, b: false, a: false };
    pub const RGB: Self = Self { r: true, g: true, b: true, a: false };
}

#[derive(Clone, Debug, Default)]
pub struct PipelineLayout {
    pub set_layouts: Vec<DescriptorSetLayout>,
    pub push_constant_ranges: Vec<PushConstantRange>,
}

#[derive(Clone, Debug)]
pub struct DescriptorSetLayout {
    pub bindings: Vec<DescriptorBinding>,
}

#[derive(Clone, Debug)]
pub struct DescriptorBinding {
    pub binding: u32,
    pub descriptor_type: DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: ShaderStageFlags,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptorType {
    UniformBuffer, StorageBuffer, SampledTexture, Sampler, StorageTexture,
}

#[derive(Clone, Copy, Debug)]
pub struct ShaderStageFlags {
    pub vertex: bool, pub fragment: bool, pub geometry: bool, pub compute: bool,
}

impl Default for ShaderStageFlags {
    fn default() -> Self { Self { vertex: true, fragment: true, geometry: false, compute: false } }
}

#[derive(Clone, Debug)]
pub struct PushConstantRange {
    pub offset: usize,
    pub size: usize,
    pub stage_flags: ShaderStageFlags,
}