use crate::TextureFormat;

#[derive(Clone, Debug)]
pub struct RenderPassDesc {
    pub attachments: Vec<AttachmentDesc>,
    pub subpasses: Vec<SubpassDesc>,
    pub dependencies: Vec<SubpassDependency>,
}

impl RenderPassDesc {
    pub fn single_color(format: TextureFormat) -> Self {
        Self {
            attachments: vec![AttachmentDesc::color(format)],
            subpasses: vec![SubpassDesc {
                color_attachments: vec![AttachmentRef { attachment: 0, layout: ImageLayout::ColorAttachmentOptimal }],
                depth_stencil_attachment: None,
                input_attachments: vec![],
                resolve_attachments: vec![],
            }],
            dependencies: vec![],
        }
    }
    
    pub fn color_depth(color_format: TextureFormat, depth_format: TextureFormat) -> Self {
        Self {
            attachments: vec![AttachmentDesc::color(color_format), AttachmentDesc::depth(depth_format)],
            subpasses: vec![SubpassDesc {
                color_attachments: vec![AttachmentRef { attachment: 0, layout: ImageLayout::ColorAttachmentOptimal }],
                depth_stencil_attachment: Some(AttachmentRef { attachment: 1, layout: ImageLayout::DepthStencilAttachmentOptimal }),
                input_attachments: vec![],
                resolve_attachments: vec![],
            }],
            dependencies: vec![],
        }
    }
}

#[derive(Clone, Debug)]
pub struct AttachmentDesc {
    pub format: TextureFormat,
    pub samples: u32,
    pub load_op: AttachmentLoadOp,
    pub store_op: AttachmentStoreOp,
    pub stencil_load_op: AttachmentLoadOp,
    pub stencil_store_op: AttachmentStoreOp,
    pub initial_layout: ImageLayout,
    pub final_layout: ImageLayout,
}

impl AttachmentDesc {
    pub fn color(format: TextureFormat) -> Self {
        Self {
            format, samples: 1,
            load_op: AttachmentLoadOp::Clear, store_op: AttachmentStoreOp::Store,
            stencil_load_op: AttachmentLoadOp::DontCare, stencil_store_op: AttachmentStoreOp::DontCare,
            initial_layout: ImageLayout::Undefined, final_layout: ImageLayout::PresentSrc,
        }
    }
    
    pub fn depth(format: TextureFormat) -> Self {
        Self {
            format, samples: 1,
            load_op: AttachmentLoadOp::Clear, store_op: AttachmentStoreOp::Store,
            stencil_load_op: AttachmentLoadOp::Clear, stencil_store_op: AttachmentStoreOp::Store,
            initial_layout: ImageLayout::Undefined, final_layout: ImageLayout::DepthStencilAttachmentOptimal,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachmentLoadOp { Load, Clear, DontCare }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachmentStoreOp { Store, DontCare }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageLayout {
    Undefined, General, ColorAttachmentOptimal, DepthStencilAttachmentOptimal,
    DepthStencilReadOnlyOptimal, ShaderReadOnlyOptimal, TransferSrcOptimal, TransferDstOptimal, PresentSrc,
}

#[derive(Clone, Debug)]
pub struct SubpassDesc {
    pub color_attachments: Vec<AttachmentRef>,
    pub depth_stencil_attachment: Option<AttachmentRef>,
    pub input_attachments: Vec<AttachmentRef>,
    pub resolve_attachments: Vec<AttachmentRef>,
}

#[derive(Clone, Copy, Debug)]
pub struct AttachmentRef {
    pub attachment: usize,
    pub layout: ImageLayout,
}

#[derive(Clone, Debug)]
pub struct SubpassDependency {
    pub src_subpass: u32,
    pub dst_subpass: u32,
    pub src_stage: PipelineStageFlags,
    pub dst_stage: PipelineStageFlags,
    pub src_access: AccessFlags,
    pub dst_access: AccessFlags,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PipelineStageFlags {
    pub top_of_pipe: bool, pub draw_indirect: bool, pub vertex_input: bool, pub vertex_shader: bool,
    pub fragment_shader: bool, pub early_fragment_tests: bool, pub late_fragment_tests: bool,
    pub color_attachment_output: bool, pub compute_shader: bool, pub transfer: bool, pub bottom_of_pipe: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AccessFlags {
    pub indirect_command_read: bool, pub index_read: bool, pub vertex_attribute_read: bool,
    pub uniform_read: bool, pub shader_read: bool, pub shader_write: bool,
    pub color_attachment_read: bool, pub color_attachment_write: bool,
    pub depth_stencil_attachment_read: bool, pub depth_stencil_attachment_write: bool,
    pub transfer_read: bool, pub transfer_write: bool,
}