//! 渲染质量档 → Vulkan 资源配置（0.2）
//!
//! 把 [`hezhou_render_pipeline::RenderSettings`] 翻译成 Vulkan 层需要的关键描述:
//! - [`ShadowPassDesc`]: 阴影深度图的分辨率/格式/用法，并提供 image/view/sampler/
//!   render pass attachment 的 `vk::*CreateInfo` 构造方法
//! - [`BloomPlan`]: Bloom mip 链尺寸
//! - [`RenderPassKind`] / [`RenderPlan`]: 当前设置下的渲染 pass 顺序
//!
//! 本模块只构造 `vk::*CreateInfo`（纯 CPU，可单测），不创建 GPU 资源；
//! 真正的资源创建在 [`crate::shadow_map::ShadowMapResources`]。
//!
//! 约定（与 `shaders/pbr.frag` 一致）:
//! - 阴影格式 `D32_SFLOAT`（16-bit/24-bit 深度在部分驱动上精度不足，暂不提供回退）
//! - 采样器 `LESS_OR_EQUAL` + `CLAMP_TO_BORDER` + `FLOAT_OPAQUE_WHITE`
//!   （越界采样返回深度 1.0 = 完全受光，避免边缘漏光）
//! - 阴影图作为 `sampler2DArrayShadow`，每级 cascade 一层

use ash::vk;
use hezhou_render_pipeline::{
    RenderQualityPreset, RenderSettings, ViewportExtent,
};

/// 阴影深度图格式
pub const SHADOW_DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;
/// HDR 游戏渲染目标格式（Tonemap 之前）
pub const HDR_COLOR_FORMAT: vk::Format = vk::Format::R16G16B16A16_SFLOAT;
/// Bloom mip 链最大层数
pub const MAX_BLOOM_MIPS: u32 = 6;

/// 阴影渲染 pass 的资源描述
///
/// 只保存纯数据；具体的 `vk::*CreateInfo` 通过方法构造，
/// 避免在结构体里保存带生命周期的 Vulkan 结构（便于跨模块传递与缓存）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShadowPassDesc {
    /// 单级 cascade 分辨率（正方形）
    pub extent: vk::Extent2D,
    /// 深度格式（D32_SFLOAT）
    pub format: vk::Format,
    /// cascade 层数（= 阴影贴图 array layers）
    pub cascade_count: u32,
    /// 深度图用途（DEPTH_STENCIL_ATTACHMENT | SAMPLED）
    pub image_usage: vk::ImageUsageFlags,
}

impl ShadowPassDesc {
    /// 深度图创建参数
    pub fn image_create_info(&self) -> vk::ImageCreateInfo<'static> {
        vk::ImageCreateInfo {
            flags: vk::ImageCreateFlags::empty(),
            image_type: vk::ImageType::TYPE_2D,
            format: self.format,
            extent: vk::Extent3D {
                width: self.extent.width,
                height: self.extent.height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: self.cascade_count,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: self.image_usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            ..Default::default()
        }
    }

    /// 整张 array 的 image view 创建参数（shader 采样用，`image` 由调用方填）
    pub fn array_view_create_info(&self, image: vk::Image) -> vk::ImageViewCreateInfo<'static> {
        vk::ImageViewCreateInfo {
            image,
            view_type: vk::ImageViewType::TYPE_2D_ARRAY,
            format: self.format,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: self.cascade_count,
            },
            ..Default::default()
        }
    }

    /// 单层 image view 创建参数（framebuffer 用）
    pub fn layer_view_create_info(
        &self,
        image: vk::Image,
        layer: u32,
    ) -> vk::ImageViewCreateInfo<'static> {
        vk::ImageViewCreateInfo {
            image,
            view_type: vk::ImageViewType::TYPE_2D_ARRAY,
            format: self.format,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: layer,
                layer_count: 1,
            },
            ..Default::default()
        }
    }

    /// 比较采样器创建参数
    pub fn sampler_create_info(&self) -> vk::SamplerCreateInfo<'static> {
        vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            mipmap_mode: vk::SamplerMipmapMode::NEAREST,
            address_mode_u: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_v: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_w: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
            anisotropy_enable: vk::FALSE,
            max_anisotropy: 1.0,
            compare_enable: vk::TRUE,
            compare_op: vk::CompareOp::LESS_OR_EQUAL,
            min_lod: 0.0,
            max_lod: 0.0,
            ..Default::default()
        }
    }

    /// 深度 render pass attachment 描述
    pub fn attachment_description(&self) -> vk::AttachmentDescription {
        vk::AttachmentDescription {
            flags: vk::AttachmentDescriptionFlags::empty(),
            format: self.format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        }
    }
}

/// 根据渲染设置生成阴影 pass 描述
///
/// 阴影未启用（质量档关闭或 `ShadowSettings.enabled=false`）时返回 None，
/// 调用方据此跳过深度图分配与阴影 pass。
pub fn shadow_pass_desc(settings: &RenderSettings) -> Option<ShadowPassDesc> {
    let config = settings.shadow_config()?;
    let cascade_count = config
        .cascade_count
        .clamp(1, hezhou_render_pipeline::MAX_CASCADES as u32);
    Some(ShadowPassDesc {
        extent: vk::Extent2D {
            width: config.resolution,
            height: config.resolution,
        },
        format: SHADOW_DEPTH_FORMAT,
        cascade_count,
        image_usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
            | vk::ImageUsageFlags::SAMPLED,
    })
}

/// 阴影深度图显存占用估算（字节）
pub fn shadow_map_bytes(settings: &RenderSettings) -> Option<usize> {
    let desc = shadow_pass_desc(settings)?;
    // D32_SFLOAT = 4 bytes/texel
    Some(
        desc.extent.width as usize
            * desc.extent.height as usize
            * desc.cascade_count as usize
            * 4,
    )
}

/// Bloom mip 链计划
#[derive(Clone, Debug)]
pub struct BloomPlan {
    /// mip 层数
    pub mip_count: u32,
    /// 各层尺寸（第 0 层为半分辨率）
    pub mip_extents: Vec<vk::Extent2D>,
    /// Bloom 纹理格式（HDR，半精度浮点）
    pub format: vk::Format,
    /// 降采样 pass 数
    pub downsample_passes: u32,
    /// 上采样 pass 数
    pub upsample_passes: u32,
}

/// 根据渲染设置与视口尺寸生成 Bloom 计划
///
/// Bloom 未启用时返回 None；mip 从半分辨率开始逐级折半到 1x1。
pub fn bloom_plan(viewport: ViewportExtent, settings: &RenderSettings) -> Option<BloomPlan> {
    if !settings.effective_bloom() {
        return None;
    }
    let bloom = settings.bloom.validate();
    let mut width = (viewport.width / 2).max(1);
    let mut height = (viewport.height / 2).max(1);
    let mut mip_extents = Vec::new();
    let mut mip_count = 0u32;
    while mip_count < bloom.mip_count.min(MAX_BLOOM_MIPS) {
        mip_extents.push(vk::Extent2D { width, height });
        mip_count += 1;
        if width == 1 && height == 1 {
            break;
        }
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
    Some(BloomPlan {
        mip_count,
        mip_extents,
        format: HDR_COLOR_FORMAT,
        // 第 0 层由 bloom_prefilter 产生，其余层由降采样产生
        downsample_passes: mip_count,
        upsample_passes: mip_count.saturating_sub(1),
    })
}

/// 渲染 pass 类型（决定录制顺序）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderPassKind {
    /// 阴影深度 pass（每级 cascade 一次）
    Shadow,
    /// 游戏不透明几何（HDR 目标）
    GameOpaque,
    /// Bloom 亮度提取 + 降采样
    BloomPrefilter,
    /// Bloom 合成 + Tonemap + Gamma
    TonemapBloom,
    /// FXAA 抗锯齿（LDR）
    Fxaa,
    /// UI 合成（swapchain）
    Ui,
}

impl RenderPassKind {
    /// pass 名称（日志/调试）
    pub fn label(self) -> &'static str {
        match self {
            RenderPassKind::Shadow => "shadow",
            RenderPassKind::GameOpaque => "game_opaque",
            RenderPassKind::BloomPrefilter => "bloom_prefilter",
            RenderPassKind::TonemapBloom => "tonemap_bloom",
            RenderPassKind::Fxaa => "fxaa",
            RenderPassKind::Ui => "ui",
        }
    }
}

/// 当前设置下的渲染 pass 顺序
///
/// 顺序: Shadow → Game(HDR) → Bloom → Tonemap → FXAA → UI
/// - 无阴影时跳过 Shadow
/// - Bloom 关闭时 Tonemap 直接采样 HDR 场景（无 Bloom 纹理）
/// - Tonemap 关闭（legacy 默认）时跳过 TonemapBloom，FXAA 直接处理 LDR 游戏画面
pub fn pass_order(settings: &RenderSettings) -> Vec<RenderPassKind> {
    let mut passes = Vec::new();
    if settings.effective_shadows() {
        passes.push(RenderPassKind::Shadow);
    }
    passes.push(RenderPassKind::GameOpaque);
    if settings.effective_bloom() {
        passes.push(RenderPassKind::BloomPrefilter);
    }
    if settings.effective_tonemap() {
        passes.push(RenderPassKind::TonemapBloom);
    }
    if settings.quality.fxaa_enabled {
        passes.push(RenderPassKind::Fxaa);
    }
    passes.push(RenderPassKind::Ui);
    passes
}

/// 渲染计划 — 一帧所需的 pass 与资源描述
#[derive(Clone, Debug)]
pub struct RenderPlan {
    /// 渲染 pass 顺序
    pub passes: Vec<RenderPassKind>,
    /// 阴影 pass 描述（None=无阴影）
    pub shadow: Option<ShadowPassDesc>,
    /// Bloom 计划（None=无 Bloom）
    pub bloom: Option<BloomPlan>,
    /// 游戏渲染目标是否需要 HDR 浮点格式
    pub hdr_target: bool,
}

/// 构造完整渲染计划
pub fn build_render_plan(viewport: ViewportExtent, settings: &RenderSettings) -> RenderPlan {
    let hdr_target = settings.effective_tonemap()
        || settings.effective_bloom()
        || settings.shading == hezhou_render_pipeline::ShadingModel::Pbr;
    RenderPlan {
        passes: pass_order(settings),
        shadow: shadow_pass_desc(settings),
        bloom: bloom_plan(viewport, settings),
        hdr_target,
    }
}

/// 质量档 → 阴影分辨率映射（编辑器 UI 展示用）
pub fn quality_summary(settings: &RenderSettings) -> String {
    let preset = settings.quality.preset;
    let shader = match settings.shading {
        hezhou_render_pipeline::ShadingModel::Pbr => "PBR",
        hezhou_render_pipeline::ShadingModel::BlinnPhong => "Blinn-Phong",
    };
    let shadow = match settings.shadow_config() {
        Some(config) => format!("ON {}cascade@{}", config.cascade_count, config.resolution),
        None => "OFF".to_string(),
    };
    format!(
        "quality={} shading={} shadows={} bloom={} tonemap={} fxaa={}",
        preset.label(),
        shader,
        shadow,
        settings.effective_bloom(),
        settings.effective_tonemap(),
        settings.quality.fxaa_enabled
    )
}

/// 从整数应用质量档（编辑器 FFI 入口的便捷封装）
pub fn settings_with_preset(preset_u32: u32) -> RenderSettings {
    RenderSettings::for_preset(RenderQualityPreset::from_u32(preset_u32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hezhou_render_pipeline::{
        BloomSettings, RenderQualityPreset, ShadowSettings,
    };

    fn high() -> RenderSettings {
        RenderSettings::for_preset(RenderQualityPreset::High)
    }

    fn legacy() -> RenderSettings {
        RenderSettings::default()
    }

    #[test]
    fn legacy_has_no_shadow_or_bloom() {
        let s = legacy();
        assert!(shadow_pass_desc(&s).is_none());
        assert!(bloom_plan(ViewportExtent::new(1920, 1080), &s).is_none());
        assert!(shadow_map_bytes(&s).is_none());
        assert_eq!(
            pass_order(&s),
            vec![RenderPassKind::GameOpaque, RenderPassKind::Fxaa, RenderPassKind::Ui]
        );
    }

    #[test]
    fn high_shadow_desc_shape() {
        let s = high();
        let desc = shadow_pass_desc(&s).expect("高配应有阴影描述");
        assert_eq!(desc.format, vk::Format::D32_SFLOAT);
        assert_eq!(desc.cascade_count, 3);
        assert_eq!(desc.extent.width, 2048);
        assert_eq!(desc.extent.height, 2048);
        assert!(desc
            .image_usage
            .contains(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT));
        assert!(desc.image_usage.contains(vk::ImageUsageFlags::SAMPLED));

        let image = desc.image_create_info();
        assert_eq!(image.format, vk::Format::D32_SFLOAT);
        assert_eq!(image.image_type, vk::ImageType::TYPE_2D);
        assert_eq!(image.array_layers, 3);
        assert_eq!(image.mip_levels, 1);
        assert_eq!(image.samples, vk::SampleCountFlags::TYPE_1);
        assert_eq!(image.tiling, vk::ImageTiling::OPTIMAL);
        assert_eq!(image.initial_layout, vk::ImageLayout::UNDEFINED);
        assert_eq!(image.extent.width, 2048);
        assert_eq!(image.extent.height, 2048);
        assert_eq!(image.extent.depth, 1);
    }

    #[test]
    fn high_shadow_view_is_array() {
        let desc = shadow_pass_desc(&high()).unwrap();
        let fake_image = vk::Image::null();
        let view = desc.array_view_create_info(fake_image);
        assert_eq!(view.image, fake_image);
        assert_eq!(view.view_type, vk::ImageViewType::TYPE_2D_ARRAY);
        assert_eq!(view.format, vk::Format::D32_SFLOAT);
        assert_eq!(
            view.subresource_range.aspect_mask,
            vk::ImageAspectFlags::DEPTH
        );
        assert_eq!(view.subresource_range.layer_count, 3);
        assert_eq!(view.subresource_range.base_array_layer, 0);
        assert_eq!(view.subresource_range.level_count, 1);

        let layer_view = desc.layer_view_create_info(fake_image, 2);
        assert_eq!(layer_view.subresource_range.base_array_layer, 2);
        assert_eq!(layer_view.subresource_range.layer_count, 1);
    }

    #[test]
    fn high_shadow_sampler_is_comparison_clamp_border() {
        let desc = shadow_pass_desc(&high()).unwrap();
        let sampler = desc.sampler_create_info();
        assert_eq!(sampler.compare_enable, vk::TRUE);
        assert_eq!(sampler.compare_op, vk::CompareOp::LESS_OR_EQUAL);
        assert_eq!(
            sampler.address_mode_u,
            vk::SamplerAddressMode::CLAMP_TO_BORDER
        );
        assert_eq!(
            sampler.address_mode_v,
            vk::SamplerAddressMode::CLAMP_TO_BORDER
        );
        assert_eq!(
            sampler.address_mode_w,
            vk::SamplerAddressMode::CLAMP_TO_BORDER
        );
        assert_eq!(sampler.border_color, vk::BorderColor::FLOAT_OPAQUE_WHITE);
        assert_eq!(sampler.max_lod, 0.0);
        assert_eq!(sampler.mag_filter, vk::Filter::LINEAR);
        assert_eq!(sampler.mipmap_mode, vk::SamplerMipmapMode::NEAREST);
    }

    #[test]
    fn high_shadow_attachment_layouts() {
        let desc = shadow_pass_desc(&high()).unwrap();
        let attachment = desc.attachment_description();
        assert_eq!(attachment.load_op, vk::AttachmentLoadOp::CLEAR);
        assert_eq!(attachment.store_op, vk::AttachmentStoreOp::STORE);
        assert_eq!(attachment.initial_layout, vk::ImageLayout::UNDEFINED);
        assert_eq!(
            attachment.final_layout,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        );
        assert_eq!(attachment.format, vk::Format::D32_SFLOAT);
        assert_eq!(attachment.samples, vk::SampleCountFlags::TYPE_1);
        assert_eq!(
            attachment.stencil_load_op,
            vk::AttachmentLoadOp::DONT_CARE
        );
    }

    #[test]
    fn shadow_bytes_scales_with_cascades_and_resolution() {
        let high_bytes = shadow_map_bytes(&high()).unwrap();
        assert_eq!(high_bytes, 2048 * 2048 * 3 * 4);
        let medium = RenderSettings::for_preset(RenderQualityPreset::Medium);
        assert_eq!(shadow_map_bytes(&medium).unwrap(), 1024 * 1024 * 2 * 4);
    }

    #[test]
    fn shadow_disabled_by_settings_switch() {
        let mut s = high();
        s.shadows.enabled = false;
        assert!(shadow_pass_desc(&s).is_none());
        assert!(!pass_order(&s).contains(&RenderPassKind::Shadow));
    }

    #[test]
    fn custom_quality_can_disable_shadow_but_keep_bloom() {
        let mut s = RenderSettings::for_preset(RenderQualityPreset::Custom);
        s.quality.shadows_enabled = false;
        assert!(shadow_pass_desc(&s).is_none());
        assert!(s.effective_bloom());
        assert!(bloom_plan(ViewportExtent::new(1280, 720), &s).is_some());
    }

    #[test]
    fn bloom_plan_mip_chain_shrinks_each_level() {
        let s = high();
        let plan = bloom_plan(ViewportExtent::new(1920, 1080), &s).expect("高配应有 Bloom");
        assert_eq!(plan.mip_extents[0].width, 960);
        assert_eq!(plan.mip_extents[0].height, 540);
        for pair in plan.mip_extents.windows(2) {
            assert!(pair[1].width <= pair[0].width);
            assert!(pair[1].height <= pair[0].height);
        }
        // 默认 bloom.mip_count = 5，半分辨率开始逐级折半
        assert_eq!(plan.mip_count, 5);
        assert_eq!(plan.mip_count as usize, plan.mip_extents.len());
        let last = plan.mip_extents.last().unwrap();
        assert_eq!(last.width, 60);
        assert_eq!(last.height, 33);
        assert!(last.width < plan.mip_extents[0].width);
        assert_eq!(plan.format, vk::Format::R16G16B16A16_SFLOAT);
        assert_eq!(plan.downsample_passes, plan.mip_count);
        assert_eq!(plan.upsample_passes, plan.mip_count - 1);
    }

    #[test]
    fn bloom_plan_can_reach_single_pixel_with_more_mips() {
        let mut s = high();
        s.bloom = BloomSettings {
            enabled: true,
            mip_count: 6,
            ..BloomSettings::default()
        };
        let plan = bloom_plan(ViewportExtent::new(64, 64), &s).unwrap();
        assert_eq!(plan.mip_count, 6);
        let last = plan.mip_extents.last().unwrap();
        assert_eq!(last.width, 1);
        assert_eq!(last.height, 1);
    }

    #[test]
    fn bloom_plan_clamps_mip_count() {
        let mut s = high();
        s.bloom = BloomSettings {
            enabled: true,
            mip_count: 100,
            ..BloomSettings::default()
        };
        let plan = bloom_plan(ViewportExtent::new(64, 64), &s).unwrap();
        assert!(plan.mip_count <= MAX_BLOOM_MIPS);
        // 64x64 半分辨率 32x32: 32,16,8,4,2,1 = 6 层
        assert_eq!(plan.mip_count, 6);
    }

    #[test]
    fn pass_order_is_correct_for_high() {
        let order = pass_order(&high());
        assert_eq!(
            order,
            vec![
                RenderPassKind::Shadow,
                RenderPassKind::GameOpaque,
                RenderPassKind::BloomPrefilter,
                RenderPassKind::TonemapBloom,
                RenderPassKind::Fxaa,
                RenderPassKind::Ui,
            ]
        );
    }

    #[test]
    fn pass_order_keeps_fxaa_in_legacy() {
        let order = pass_order(&legacy());
        assert!(order.contains(&RenderPassKind::Fxaa));
        assert_eq!(order.last(), Some(&RenderPassKind::Ui));
    }

    #[test]
    fn pass_order_skips_fxaa_when_disabled() {
        let mut s = high();
        s.quality.fxaa_enabled = false;
        assert!(!pass_order(&s).contains(&RenderPassKind::Fxaa));
    }

    #[test]
    fn render_plan_high_is_hdr() {
        let plan = build_render_plan(ViewportExtent::new(1920, 1080), &high());
        assert!(plan.hdr_target);
        assert!(plan.shadow.is_some());
        assert!(plan.bloom.is_some());
        assert_eq!(plan.passes.len(), 6);
    }

    #[test]
    fn render_plan_legacy_is_ldr() {
        let plan = build_render_plan(ViewportExtent::new(1920, 1080), &legacy());
        assert!(!plan.hdr_target);
        assert!(plan.shadow.is_none());
        assert!(plan.bloom.is_none());
        assert_eq!(plan.passes.len(), 3);
    }

    #[test]
    fn quality_summary_contains_key_fields() {
        let summary = quality_summary(&high());
        assert!(summary.contains("quality=high"));
        assert!(summary.contains("PBR"));
        assert!(summary.contains("3cascade@2048"));
        assert!(summary.contains("bloom=true"));
    }

    #[test]
    fn settings_with_preset_maps_ffi_integers() {
        assert_eq!(
            settings_with_preset(2).quality.preset,
            RenderQualityPreset::High
        );
        assert_eq!(
            settings_with_preset(0).quality.preset,
            RenderQualityPreset::Low
        );
        // 非法值回退中配
        assert_eq!(
            settings_with_preset(99).quality.preset,
            RenderQualityPreset::Medium
        );
    }

    #[test]
    fn shadow_settings_default_when_used_directly() {
        // ShadowSettings::default() 默认 enabled=true，但质量档仍决定最终开关
        assert!(ShadowSettings::default().enabled);
        let mut s = RenderSettings::for_preset(RenderQualityPreset::Medium);
        s.shadows.enabled = false;
        assert!(shadow_pass_desc(&s).is_none());
    }
}
