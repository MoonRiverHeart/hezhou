//! 渲染质量档与渲染设置
//!
//! 0.2 渲染能力（PBR / Shadow Map / Tonemap / Bloom）通过 [`RenderSettings`] 聚合配置。
//! 设计原则:
//! - **默认保持向后兼容**: `RenderSettings::default()` 等价于 0.1 的 legacy 光栅路径
//!   （Blinn-Phong + FXAA，无阴影、无 Bloom、无 Tonemap）。
//! - **质量档是唯一入口**: 编辑器通过 [`RenderQualityPreset`] 选择低/中/高档，
//!   由 [`QualitySettings::for_preset`] 解析成具体开关与分辨率。
//! - **阴影级联数与分辨率由质量档决定**（`QualitySettings`），
//!   而 bias/lambda/距离等调参放在 [`crate::shadow::ShadowSettings`]。
//!
//! 编辑器接入点（第一轮只提供 Rust API，C#/FFI 由 Lead 统一接入）:
//! - 运行时: `hezhou_rhi_vulkan::UIVulkanRenderer::set_render_quality_preset`
//! - 每帧参数: `hezhou_render_pipeline::RenderContext::render_settings`
//! - FFI 建议: `renderer_set_quality_preset(i32)`，取值见 [`RenderQualityPreset::as_u32`]

use crate::pbr::{PbrMaterial, ShadingModel};
use crate::post::{BloomSettings, TonemapOperator, TonemapSettings};
use crate::shadow::ShadowSettings;

/// 阴影贴图最小分辨率（像素，单级 cascade）
pub const MIN_SHADOW_RESOLUTION: u32 = 256;
/// 阴影贴图最大分辨率（像素，单级 cascade）
pub const MAX_SHADOW_RESOLUTION: u32 = 4096;

/// 渲染质量档位
///
/// 低/中/高为预设档，Custom 允许编辑器逐项覆盖（覆盖后仍需调用
/// [`RenderSettings::validate`] 做范围收敛）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderQualityPreset {
    /// 低配: 关闭阴影与 Bloom，单级阴影分辨率 512（阴影关闭时不再分配深度图）
    Low,
    /// 中配: 2 级 cascade，阴影分辨率 1024，关闭 Bloom
    Medium,
    /// 高配: 3 级 cascade，阴影分辨率 2048，开启 Bloom
    High,
    /// 自定义: 以中配为基线，逐项覆盖
    Custom,
}

impl RenderQualityPreset {
    /// 所有档位（便于 UI 枚举与测试遍历）
    pub const ALL: [RenderQualityPreset; 4] = [
        RenderQualityPreset::Low,
        RenderQualityPreset::Medium,
        RenderQualityPreset::High,
        RenderQualityPreset::Custom,
    ];

    /// 转整数（C#/FFI 传输用）
    pub fn as_u32(self) -> u32 {
        match self {
            RenderQualityPreset::Low => 0,
            RenderQualityPreset::Medium => 1,
            RenderQualityPreset::High => 2,
            RenderQualityPreset::Custom => 3,
        }
    }

    /// 从整数解析（FFI 入口，非法值回退为 Medium）
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => RenderQualityPreset::Low,
            1 => RenderQualityPreset::Medium,
            2 => RenderQualityPreset::High,
            3 => RenderQualityPreset::Custom,
            _ => RenderQualityPreset::Medium,
        }
    }

    /// 显示名（日志/UI）
    pub fn label(self) -> &'static str {
        match self {
            RenderQualityPreset::Low => "low",
            RenderQualityPreset::Medium => "medium",
            RenderQualityPreset::High => "high",
            RenderQualityPreset::Custom => "custom",
        }
    }
}

/// 质量档解析后的具体渲染开关
///
/// 这是"档位 → 具体数值"的唯一映射表，避免各渲染后端各自硬编码。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualitySettings {
    /// 档位标识
    pub preset: RenderQualityPreset,
    /// 是否启用 PBR 着色（false 时回退 Blinn-Phong）
    pub pbr_shading: bool,
    /// 是否启用方向光阴影
    pub shadows_enabled: bool,
    /// 阴影 cascade 级联数（1..=4，阴影关闭时忽略）
    pub shadow_cascade_count: u8,
    /// 单级 cascade 阴影贴图分辨率（正方形，2 的幂）
    pub shadow_resolution: u32,
    /// 是否启用 Bloom
    pub bloom_enabled: bool,
    /// 是否启用 Tonemap + Gamma
    pub tonemap_enabled: bool,
    /// 是否启用 FXAA（0.1 已存在，保留开关以便质量档关闭）
    pub fxaa_enabled: bool,
}

impl QualitySettings {
    /// 按质量档解析具体参数
    pub fn for_preset(preset: RenderQualityPreset) -> Self {
        match preset {
            RenderQualityPreset::Low => Self {
                preset,
                pbr_shading: true,
                shadows_enabled: false,
                shadow_cascade_count: 1,
                shadow_resolution: 512,
                bloom_enabled: false,
                tonemap_enabled: true,
                fxaa_enabled: true,
            },
            RenderQualityPreset::Medium => Self {
                preset,
                pbr_shading: true,
                shadows_enabled: true,
                shadow_cascade_count: 2,
                shadow_resolution: 1024,
                bloom_enabled: false,
                tonemap_enabled: true,
                fxaa_enabled: true,
            },
            RenderQualityPreset::High => Self {
                preset,
                pbr_shading: true,
                shadows_enabled: true,
                shadow_cascade_count: 3,
                shadow_resolution: 2048,
                bloom_enabled: true,
                tonemap_enabled: true,
                fxaa_enabled: true,
            },
            RenderQualityPreset::Custom => Self {
                preset,
                pbr_shading: true,
                shadows_enabled: true,
                shadow_cascade_count: 2,
                shadow_resolution: 1024,
                bloom_enabled: true,
                tonemap_enabled: true,
                fxaa_enabled: true,
            },
        }
    }

    /// 0.1 行为兼容档：Blinn-Phong + FXAA，无阴影/Bloom/Tonemap
    ///
    /// 用于 `RenderSettings::default()`，保证未显式选择质量档时视觉不回归。
    pub fn legacy_compatible() -> Self {
        Self {
            preset: RenderQualityPreset::Custom,
            pbr_shading: false,
            shadows_enabled: false,
            shadow_cascade_count: 1,
            shadow_resolution: 1024,
            bloom_enabled: false,
            tonemap_enabled: false,
            fxaa_enabled: true,
        }
    }

    /// 阴影是否真正可用（质量档层面）
    pub fn shadows_available(&self) -> bool {
        self.shadows_enabled && self.shadow_cascade_count > 0 && self.shadow_resolution > 0
    }

    /// 把非法值收敛到合法区间（cascade 1..=4，分辨率 clamp 到 2 的幂）
    pub fn validate(self) -> Self {
        let mut out = self;
        out.shadow_cascade_count = out
            .shadow_cascade_count
            .clamp(1, crate::shadow::MAX_CASCADES as u8);
        out.shadow_resolution = normalize_shadow_resolution(out.shadow_resolution);
        out
    }
}

/// 把任意分辨率收敛到 [MIN_SHADOW_RESOLUTION, MAX_SHADOW_RESOLUTION] 内的 2 的幂
///
/// 向下取整（宁小勿大，避免质量档配置异常导致显存超预算）。
pub fn normalize_shadow_resolution(requested: u32) -> u32 {
    let clamped = requested.clamp(MIN_SHADOW_RESOLUTION, MAX_SHADOW_RESOLUTION);
    // 向下取到最近的 2 的幂
    let mut pot = 1u32;
    while pot * 2 <= clamped {
        pot *= 2;
    }
    pot
}

/// 完整渲染设置 — 0.2 渲染能力的配置聚合
///
/// 由渲染后端每帧从 `RenderContext` 读取；未接入时使用 `Default`（legacy 兼容）。
#[derive(Clone, Debug, PartialEq)]
pub struct RenderSettings {
    /// 质量档具体开关
    pub quality: QualitySettings,
    /// 着色模型（Pbr 为 0.2 新路径，BlinnPhong 为可回退路径）
    pub shading: ShadingModel,
    /// 阴影调参（级联数与分辨率来自 `quality`）
    pub shadows: ShadowSettings,
    /// Tonemap + Gamma 设置
    pub tonemap: TonemapSettings,
    /// Bloom 设置
    pub bloom: BloomSettings,
    /// 默认 PBR 材质（实体未指定材质时使用）
    pub default_material: PbrMaterial,
}

impl Default for RenderSettings {
    /// 默认 = 0.1 legacy 行为（不改变现有编辑器视觉）
    fn default() -> Self {
        Self::legacy_compatible()
    }
}

impl RenderSettings {
    /// 0.1 行为兼容设置
    pub fn legacy_compatible() -> Self {
        Self {
            quality: QualitySettings::legacy_compatible(),
            shading: ShadingModel::BlinnPhong,
            shadows: ShadowSettings {
                enabled: false,
                ..ShadowSettings::default()
            },
            tonemap: TonemapSettings {
                enabled: false,
                ..TonemapSettings::default()
            },
            bloom: BloomSettings {
                enabled: false,
                ..BloomSettings::default()
            },
            default_material: PbrMaterial::default(),
        }
    }

    /// 按质量档生成设置（PBR + 阴影/Bloom 按档位开关）
    pub fn for_preset(preset: RenderQualityPreset) -> Self {
        let quality = QualitySettings::for_preset(preset).validate();
        Self {
            quality,
            shading: if quality.pbr_shading {
                ShadingModel::Pbr
            } else {
                ShadingModel::BlinnPhong
            },
            shadows: ShadowSettings {
                enabled: quality.shadows_enabled,
                ..ShadowSettings::default()
            },
            tonemap: TonemapSettings {
                enabled: quality.tonemap_enabled,
                operator: TonemapOperator::Aces,
                ..TonemapSettings::default()
            },
            bloom: BloomSettings {
                enabled: quality.bloom_enabled,
                ..BloomSettings::default()
            },
            default_material: PbrMaterial::default(),
        }
    }

    /// 切换质量档（同时同步着色模型/阴影/Bloom/Tonemap 的开关）
    pub fn set_quality_preset(&mut self, preset: RenderQualityPreset) {
        *self = Self::for_preset(preset);
    }

    /// 范围收敛：所有子设置 clamp 到合法区间
    ///
    /// 注意: 质量档的 cascade/分辨率是唯一真源，阴影子设置不再重复保存这两项。
    pub fn validate(mut self) -> Self {
        self.quality = self.quality.validate();
        self.shadows = self.shadows.validate();
        self.tonemap = self.tonemap.validate();
        self.bloom = self.bloom.validate();
        self.default_material = self.default_material.sanitized();
        self
    }

    /// 返回收敛后的副本（不消耗 self）
    pub fn validated(&self) -> Self {
        self.clone().validate()
    }

    /// 阴影是否实际启用（质量档 + ShadowSettings 双开关）
    pub fn effective_shadows(&self) -> bool {
        self.quality.shadows_available() && self.shadows.enabled
    }

    /// Bloom 是否实际启用（质量档 + BloomSettings 双开关）
    pub fn effective_bloom(&self) -> bool {
        self.quality.bloom_enabled && self.bloom.enabled
    }

    /// Tonemap 是否实际启用（质量档 + TonemapSettings 双开关）
    pub fn effective_tonemap(&self) -> bool {
        self.quality.tonemap_enabled && self.tonemap.enabled
    }

    /// 是否仍处于 0.1 legacy 路径（Blinn-Phong + FXAA）
    pub fn is_legacy_compatible(&self) -> bool {
        self.shading == ShadingModel::BlinnPhong
            && !self.effective_shadows()
            && !self.effective_bloom()
            && !self.effective_tonemap()
            && self.quality.fxaa_enabled
    }

    /// 生成阴影渲染配置（阴影未启用时返回 None）
    pub fn shadow_config(&self) -> Option<crate::shadow::ShadowConfig> {
        if !self.effective_shadows() {
            return None;
        }
        Some(crate::shadow::ShadowConfig {
            cascade_count: self.quality.shadow_cascade_count as u32,
            resolution: self.quality.shadow_resolution,
            depth_bias: self.shadows.depth_bias,
            normal_bias: self.shadows.normal_bias,
            cascade_lambda: self.shadows.cascade_lambda,
            max_distance: self.shadows.max_distance,
            stable: self.shadows.stable,
        })
    }

    /// 质量档显示名（日志/状态栏）
    pub fn quality_label(&self) -> &'static str {
        self.quality.preset.label()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_legacy_compatible() {
        let s = RenderSettings::default();
        assert_eq!(s.shading, ShadingModel::BlinnPhong);
        assert!(!s.effective_shadows());
        assert!(!s.effective_bloom());
        assert!(!s.effective_tonemap());
        assert!(s.quality.fxaa_enabled);
        assert!(s.is_legacy_compatible());
        assert!(s.shadow_config().is_none());
    }

    #[test]
    fn legacy_preset_has_no_pbr_shadow_bloom() {
        let s = RenderSettings::for_preset(RenderQualityPreset::Low);
        // 低配仍然使用 PBR 材质着色，但关闭阴影与 Bloom
        assert_eq!(s.shading, ShadingModel::Pbr);
        assert!(s.effective_tonemap());
        assert!(!s.effective_shadows());
        assert!(!s.effective_bloom());
        assert!(!s.is_legacy_compatible());
        assert!(s.shadow_config().is_none());
    }

    #[test]
    fn medium_preset_enables_two_cascades_no_bloom() {
        let s = RenderSettings::for_preset(RenderQualityPreset::Medium);
        assert!(s.effective_shadows());
        assert!(!s.effective_bloom());
        let cfg = s.shadow_config().expect("中配应启用阴影");
        assert_eq!(cfg.cascade_count, 2);
        assert_eq!(cfg.resolution, 1024);
    }

    #[test]
    fn high_preset_enables_three_cascades_and_bloom() {
        let s = RenderSettings::for_preset(RenderQualityPreset::High);
        assert!(s.effective_shadows());
        assert!(s.effective_bloom());
        let cfg = s.shadow_config().expect("高配应启用阴影");
        assert_eq!(cfg.cascade_count, 3);
        assert_eq!(cfg.resolution, 2048);
    }

    #[test]
    fn quality_tiers_are_monotonic() {
        let low = QualitySettings::for_preset(RenderQualityPreset::Low);
        let mid = QualitySettings::for_preset(RenderQualityPreset::Medium);
        let high = QualitySettings::for_preset(RenderQualityPreset::High);
        assert!(low.shadow_resolution <= mid.shadow_resolution);
        assert!(mid.shadow_resolution <= high.shadow_resolution);
        assert!(low.shadow_cascade_count <= mid.shadow_cascade_count);
        assert!(mid.shadow_cascade_count <= high.shadow_cascade_count);
        assert!(!low.bloom_enabled && !mid.bloom_enabled && high.bloom_enabled);
    }

    #[test]
    fn quality_preset_roundtrip_u32() {
        for preset in RenderQualityPreset::ALL {
            assert_eq!(RenderQualityPreset::from_u32(preset.as_u32()), preset);
        }
        assert_eq!(
            RenderQualityPreset::from_u32(99),
            RenderQualityPreset::Medium
        );
    }

    #[test]
    fn normalize_shadow_resolution_clamps_and_rounds_down() {
        assert_eq!(normalize_shadow_resolution(0), MIN_SHADOW_RESOLUTION);
        assert_eq!(normalize_shadow_resolution(100), MIN_SHADOW_RESOLUTION);
        assert_eq!(normalize_shadow_resolution(1000), 512);
        assert_eq!(normalize_shadow_resolution(1024), 1024);
        assert_eq!(normalize_shadow_resolution(3000), 2048);
        assert_eq!(normalize_shadow_resolution(99_999), MAX_SHADOW_RESOLUTION);
    }

    #[test]
    fn quality_validate_clamps_cascade_count() {
        let mut q = QualitySettings::for_preset(RenderQualityPreset::Custom);
        q.shadow_cascade_count = 0;
        assert_eq!(q.validate().shadow_cascade_count, 1);
        q.shadow_cascade_count = 99;
        assert_eq!(q.validate().shadow_cascade_count, 4);
    }

    #[test]
    fn settings_validate_sanitizes_material_and_subsystems() {
        let mut s = RenderSettings::for_preset(RenderQualityPreset::High);
        s.shadows.depth_bias = -5.0;
        s.shadows.cascade_lambda = 3.0;
        s.bloom.intensity = -1.0;
        s.tonemap.exposure = f32::NAN;
        s.default_material.metallic = 7.0;
        let s = s.validate();
        assert!(s.shadows.depth_bias >= 0.0);
        assert!((0.0..=1.0).contains(&s.shadows.cascade_lambda));
        assert!(s.bloom.intensity >= 0.0);
        assert!(s.tonemap.exposure.is_finite() && s.tonemap.exposure > 0.0);
        assert!((0.0..=1.0).contains(&s.default_material.metallic));
    }

    #[test]
    fn effective_flags_require_both_switches() {
        let mut s = RenderSettings::for_preset(RenderQualityPreset::High);
        assert!(s.effective_shadows());
        s.shadows.enabled = false;
        assert!(!s.effective_shadows());
        // 质量档打开但子设置关闭 Bloom
        s.shadows.enabled = true;
        s.bloom.enabled = false;
        assert!(!s.effective_bloom());
    }

    #[test]
    fn custom_preset_allows_overrides() {
        let mut s = RenderSettings::for_preset(RenderQualityPreset::Custom);
        s.quality.shadow_cascade_count = 1;
        s.quality.shadow_resolution = 4096;
        let s = s.validate();
        assert_eq!(s.quality.shadow_cascade_count, 1);
        assert_eq!(s.quality.shadow_resolution, 4096);
        assert_eq!(s.quality.preset, RenderQualityPreset::Custom);
    }

    #[test]
    fn set_quality_preset_replaces_settings() {
        let mut s = RenderSettings::default();
        assert!(s.is_legacy_compatible());
        s.set_quality_preset(RenderQualityPreset::High);
        assert_eq!(s.quality.preset, RenderQualityPreset::High);
        assert!(s.effective_shadows() && s.effective_bloom());
        s.set_quality_preset(RenderQualityPreset::Low);
        assert!(!s.effective_shadows() && !s.effective_bloom());
    }

    #[test]
    fn quality_labels_match_presets() {
        assert_eq!(RenderQualityPreset::Low.label(), "low");
        assert_eq!(RenderQualityPreset::Medium.label(), "medium");
        assert_eq!(RenderQualityPreset::High.label(), "high");
        assert_eq!(RenderQualityPreset::Custom.label(), "custom");
        assert_eq!(
            RenderSettings::for_preset(RenderQualityPreset::High).quality_label(),
            "high"
        );
    }
}
