//! 可扩展渲染管线 trait 系统
//!
//! 定义 RenderPipeline trait 及相关类型，支持运行时管线切换和资源管理。
//! 核心概念:
//! - RenderPipeline: 5个hook点的管线抽象（prepare/record/post_process/composite/cleanup）
//! - RenderContext: 每帧渲染参数（场景指针、摄像机、视口、选中实体、0.2 渲染设置）
//! - PipelineResources: 管线专属GPU资源容器
//! - ResourceManager: 跨管线切换的资源生命周期管理
//! - PipelineRegistry: 运行时管线注册/切换/帧渲染
//! - BvhNode/Bvh: GPU-friendly BVH结构（用于光线追踪加速）
//!
//! 0.2 渲染能力模块（与 legacy Blinn-Phong 路径并存，可回退）:
//! - [`pbr`]: PBR 材质/参考 BRDF/PushConstant 与每帧 UBO 布局
//! - [`shadow`]: 方向光 CSM 级联阴影（分割/bias/纹素吸附/光源矩阵）
//! - [`post`]: Tonemap + Gamma + Bloom 设置与 CPU 参考实现
//! - [`quality`]: 低/中/高质量档与 [`RenderSettings`] 聚合

pub mod pipeline;
pub mod context;
pub mod resource;
pub mod registry;
pub mod bvh;
pub mod raster_pipeline;
pub mod raytrace_pipeline;
pub mod pbr;
pub mod shadow;
pub mod post;
pub mod quality;

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

// === 0.2 渲染能力（PBR / Shadow / Post / Quality） ===
pub use pbr::{
    PbrMaterial, PbrLight, PbrPushConstant, PbrFrameUbo, ShadingModel,
    DIELECTRIC_F0, MIN_ROUGHNESS, PBR_PI,
};
pub use shadow::{
    ShadowFrameData, ShadowConfig, ShadowSettings, ShadowError, CascadeSplits,
    ShadowPushConstant, MAX_CASCADES, compute_cascade_splits,
};
pub use post::{
    TonemapOperator, TonemapSettings, BloomSettings,
    PostTonemapPushConstant, BloomPrefilterPushConstant, POST_PASS_ORDER,
};
pub use quality::{
    RenderSettings, RenderQualityPreset, QualitySettings,
    MIN_SHADOW_RESOLUTION, MAX_SHADOW_RESOLUTION, normalize_shadow_resolution,
};
