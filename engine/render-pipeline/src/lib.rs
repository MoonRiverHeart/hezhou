//! 可扩展渲染管线 trait 系统
//!
//! 定义 RenderPipeline trait 及相关类型，支持运行时管线切换和资源管理。
//! 核心概念:
//! - RenderPipeline: 5个hook点的管线抽象（prepare/record/post_process/composite/cleanup）
//! - RenderContext: 每帧渲染参数（场景指针、摄像机、视口、选中实体）
//! - PipelineResources: 管线专属GPU资源容器
//! - ResourceManager: 跨管线切换的资源生命周期管理
//! - PipelineRegistry: 运行时管线注册/切换/帧渲染
//! - BvhNode/Bvh: GPU-friendly BVH结构（用于光线追踪加速）

pub mod pipeline;
pub mod context;
pub mod resource;
pub mod registry;
pub mod bvh;
pub mod raster_pipeline;
pub mod raytrace_pipeline;

pub use pipeline::{RenderPipeline, RenderPassOutput};
pub use context::{RenderContext, RenderMode, CameraParams, ViewportExtent};
pub use resource::{PipelineResourceDesc, PipelineResources, ResourceManager};
pub use registry::PipelineRegistry;
pub use bvh::{BvhNode, Bvh};
pub use raster_pipeline::{
    RasterPipeline, PushConstantData, LightUboData,
    PrimitiveMeshRange, CustomMeshInfo, TextureCacheEntry,
    is_custom_mesh, mesh_type_from_path,
};
pub use raytrace_pipeline::{RayTracePipeline, RayTracePushConstant};