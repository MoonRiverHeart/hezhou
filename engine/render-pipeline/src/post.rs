//! 后处理设置: Tonemap + Gamma + Bloom（0.2 最小实现）
//!
//! 提供 CPU 参考实现与 GPU 参数布局:
//! - [`TonemapOperator`]: None / Reinhard / ACES / Filmic(Hable)
//! - [`TonemapSettings`]: exposure + operator + gamma
//! - [`BloomSettings`]: 阈值/软膝/强度
//! - [`PostTonemapPushConstant`]（16 bytes，匹配 `shaders/post_tonemap.frag`）
//! - [`BloomPrefilterPushConstant`]（16 bytes，匹配 `shaders/bloom_prefilter.frag`）
//!
//! 渲染顺序（文档见 `docs/RENDER_0_2.md`）:
//! `Game(HDR) → BloomPrefilter → Tonemap(+Gamma) → FXAA → UI`
//!
//! 注意: 0.1 的 FXAA 保留；Tonemap 关闭时（legacy 默认）FXAA 仍直接处理 LDR 游戏画面。

use bytemuck::{Pod, Zeroable};

use crate::pbr::PBR_PI;

/// Tonemap 算子
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TonemapOperator {
    /// 不做 Tonemap（仅 exposure/gamma）
    None,
    /// Reinhard: c/(1+c)
    Reinhard,
    /// ACES 近似（Narkowicz 拟合，0.2 默认）
    #[default]
    Aces,
    /// Hable Uncharted2 风格曲线
    Filmic,
}

impl TonemapOperator {
    /// 转整数（与 `shaders/post_tonemap.frag` 的 `TONEMAP_*` 宏一致）
    pub fn as_u32(self) -> u32 {
        match self {
            TonemapOperator::None => 0,
            TonemapOperator::Reinhard => 1,
            TonemapOperator::Aces => 2,
            TonemapOperator::Filmic => 3,
        }
    }

    /// 从整数解析（非法值回退 ACES）
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => TonemapOperator::None,
            1 => TonemapOperator::Reinhard,
            2 => TonemapOperator::Aces,
            3 => TonemapOperator::Filmic,
            _ => TonemapOperator::Aces,
        }
    }

    /// 显示名
    pub fn label(self) -> &'static str {
        match self {
            TonemapOperator::None => "none",
            TonemapOperator::Reinhard => "reinhard",
            TonemapOperator::Aces => "aces",
            TonemapOperator::Filmic => "filmic",
        }
    }

    /// 单通道 tonemap（输入线性 HDR，输出 [0,1]）
    pub fn apply_scalar(self, x: f32) -> f32 {
        let x = if x.is_finite() { x.max(0.0) } else { 0.0 };
        let out = match self {
            TonemapOperator::None => x,
            TonemapOperator::Reinhard => x / (1.0 + x),
            TonemapOperator::Aces => {
                let a = 2.51;
                let b = 0.03;
                let c = 2.43;
                let d = 0.59;
                let e = 0.14;
                ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
            }
            TonemapOperator::Filmic => {
                // Hable Uncharted2，归一化到 [0,1]
                let curve = |v: f32| {
                    let a = 0.15;
                    let b = 0.50;
                    let c = 0.10;
                    let d = 0.20;
                    let e = 0.02;
                    let f = 0.30;
                    ((v * (a * v + c * b) + d * e) / (v * (a * v + b) + d * f)) - e / f
                };
                let white = 11.2;
                (curve(x) / curve(white)).clamp(0.0, 1.0)
            }
        };
        if out.is_finite() {
            out
        } else {
            0.0
        }
    }

    /// RGB tonemap
    pub fn apply(self, rgb: [f32; 3]) -> [f32; 3] {
        [
            self.apply_scalar(rgb[0]),
            self.apply_scalar(rgb[1]),
            self.apply_scalar(rgb[2]),
        ]
    }
}

/// Tonemap + Gamma 设置
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TonemapSettings {
    /// 是否启用（质量档与这里共同决定，见 `RenderSettings::effective_tonemap`）
    pub enabled: bool,
    /// tonemap 算子
    pub operator: TonemapOperator,
    /// 曝光倍数（线性空间乘子，>0）
    pub exposure: f32,
    /// Gamma 值（2.2 为 sRGB 近似；<=1 表示不做 gamma 校正）
    pub gamma: f32,
}

impl Default for TonemapSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            operator: TonemapOperator::Aces,
            exposure: 1.0,
            gamma: 2.2,
        }
    }
}

impl TonemapSettings {
    /// 收敛到合法区间（NaN 回退默认值）
    pub fn validate(mut self) -> Self {
        let def = Self::default();
        self.exposure = if self.exposure.is_finite() {
            self.exposure.clamp(0.01, 100.0)
        } else {
            def.exposure
        };
        self.gamma = if self.gamma.is_finite() {
            self.gamma.clamp(0.1, 5.0)
        } else {
            def.gamma
        };
        self
    }

    /// 是否产生任何颜色变化（关闭 / 曝光=1 且 gamma=1 且算子 None）
    pub fn is_identity(&self) -> bool {
        !self.enabled
            || (self.operator == TonemapOperator::None
                && (self.exposure - 1.0).abs() < 1e-6
                && (self.gamma - 1.0).abs() < 1e-6)
    }

    /// CPU 参考: exposure → tonemap → gamma
    pub fn process(&self, rgb: [f32; 3]) -> [f32; 3] {
        if !self.enabled {
            return rgb;
        }
        let settings = self.validate();
        let exposed = [
            rgb[0] * settings.exposure,
            rgb[1] * settings.exposure,
            rgb[2] * settings.exposure,
        ];
        let mapped = settings.operator.apply(exposed);
        apply_gamma(mapped, settings.gamma)
    }

    /// 生成 `shaders/post_tonemap.frag` 的 push constant
    pub fn push_constant(&self, bloom_intensity: f32) -> PostTonemapPushConstant {
        PostTonemapPushConstant {
            params: [
                self.exposure.max(0.01),
                self.gamma.max(0.0001),
                self.operator.as_u32() as f32,
                bloom_intensity.max(0.0),
            ],
        }
    }
}

/// Gamma 校正（`gamma <= 1` 视为关闭）
pub fn apply_gamma(rgb: [f32; 3], gamma: f32) -> [f32; 3] {
    let g = if gamma.is_finite() && gamma > 1.0 {
        gamma
    } else {
        1.0
    };
    if (g - 1.0).abs() < 1e-6 {
        return rgb;
    }
    let inv = 1.0 / g;
    [
        rgb[0].max(0.0).powf(inv),
        rgb[1].max(0.0).powf(inv),
        rgb[2].max(0.0).powf(inv),
    ]
}

/// Bloom 设置
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BloomSettings {
    /// 是否启用（质量档与这里共同决定，见 `RenderSettings::effective_bloom`）
    pub enabled: bool,
    /// 亮度阈值（低于该亮度不参与 Bloom）
    pub threshold: f32,
    /// 软膝宽度（0=硬阈值，>0 平滑过渡）
    pub soft_knee: f32,
    /// Bloom 合成强度
    pub intensity: f32,
    /// 降采样 mip 层数上限（1..=6）
    pub mip_count: u32,
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 1.0,
            soft_knee: 0.5,
            intensity: 0.6,
            mip_count: 5,
        }
    }
}

impl BloomSettings {
    /// 收敛到合法区间（NaN 回退默认值）
    pub fn validate(mut self) -> Self {
        let def = Self::default();
        self.threshold = if self.threshold.is_finite() {
            self.threshold.clamp(0.0, 100.0)
        } else {
            def.threshold
        };
        self.soft_knee = if self.soft_knee.is_finite() {
            self.soft_knee.clamp(0.0, 1.0)
        } else {
            def.soft_knee
        };
        self.intensity = if self.intensity.is_finite() {
            self.intensity.clamp(0.0, 10.0)
        } else {
            def.intensity
        };
        self.mip_count = self.mip_count.clamp(1, 6);
        self
    }

    /// CPU 参考: 亮度阈值 + 软膝（返回过滤后的 HDR 颜色）
    pub fn prefilter(&self, rgb: [f32; 3]) -> [f32; 3] {
        let s = self.validate();
        // 感知亮度（Rec.709 系数）
        let luma = rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
        if luma <= 0.0 {
            return [0.0, 0.0, 0.0];
        }
        let knee = (s.threshold * s.soft_knee).max(1e-6);
        let soft = (luma - s.threshold + knee).clamp(0.0, 2.0 * knee);
        let contribution = (soft * soft / (4.0 * knee + 1e-6)).max(luma - s.threshold);
        let factor = if luma > 0.0 { contribution / luma } else { 0.0 };
        [
            (rgb[0] * factor).max(0.0),
            (rgb[1] * factor).max(0.0),
            (rgb[2] * factor).max(0.0),
        ]
    }

    /// CPU 参考: Bloom 合成（HDR + bloom * intensity）
    pub fn combine(&self, hdr: [f32; 3], bloom: [f32; 3]) -> [f32; 3] {
        let s = self.validate();
        if !s.enabled {
            return hdr;
        }
        [
            hdr[0] + bloom[0] * s.intensity,
            hdr[1] + bloom[1] * s.intensity,
            hdr[2] + bloom[2] * s.intensity,
        ]
    }
}

/// `shaders/post_tonemap.frag` 的 push constant（16 bytes）
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct PostTonemapPushConstant {
    /// x=exposure, y=gamma, z=tonemap_operator, w=bloom_intensity
    pub params: [f32; 4],
}

/// `shaders/bloom_prefilter.frag` 的 push constant（16 bytes）
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct BloomPrefilterPushConstant {
    /// x=threshold, y=soft_knee, z=texel_size_x, w=texel_size_y
    pub params: [f32; 4],
}

impl BloomPrefilterPushConstant {
    /// 从 Bloom 设置与源图尺寸构造
    pub fn from_settings(settings: &BloomSettings, source_extent: [u32; 2]) -> Self {
        let s = settings.validate();
        let w = source_extent[0].max(1) as f32;
        let h = source_extent[1].max(1) as f32;
        Self {
            params: [s.threshold, s.soft_knee, 1.0 / w, 1.0 / h],
        }
    }
}

/// 后处理阶段顺序（文档/调试用）
pub const POST_PASS_ORDER: &[&str] = &["bloom_prefilter", "post_tonemap", "fxaa"];

/// 校验 PBR 相关常量与 GLSL 端一致（供测试与运行时断言使用）
pub fn pi_matches_shader_contract() -> bool {
    (PBR_PI - std::f32::consts::PI).abs() < 1e-5
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn tonemap_operator_roundtrip() {
        for op in [
            TonemapOperator::None,
            TonemapOperator::Reinhard,
            TonemapOperator::Aces,
            TonemapOperator::Filmic,
        ] {
            assert_eq!(TonemapOperator::from_u32(op.as_u32()), op);
        }
        assert_eq!(TonemapOperator::from_u32(99), TonemapOperator::Aces);
        assert_eq!(TonemapOperator::default(), TonemapOperator::Aces);
    }

    #[test]
    fn tonemap_outputs_are_bounded_and_monotonic() {
        for op in [
            TonemapOperator::None,
            TonemapOperator::Reinhard,
            TonemapOperator::Aces,
            TonemapOperator::Filmic,
        ] {
            let mut prev = -1.0;
            for x in [0.0f32, 0.1, 0.5, 1.0, 2.0, 4.0, 16.0] {
                let y = op.apply_scalar(x);
                assert!(y.is_finite());
                if op == TonemapOperator::None {
                    // None 只做恒等映射（可能 >1，legacy 路径不启用时保持原值）
                    assert!((y - x).abs() < 1e-6);
                    continue;
                }
                assert!((0.0..=1.0).contains(&y), "{op:?} x={x} y={y}");
                assert!(y >= prev, "{op:?} 应单调递增");
                prev = y;
            }
        }
    }

    #[test]
    fn reinhard_formula_matches() {
        assert!((TonemapOperator::Reinhard.apply_scalar(1.0) - 0.5).abs() < 1e-6);
        assert!((TonemapOperator::Reinhard.apply_scalar(0.0)).abs() < 1e-6);
    }

    #[test]
    fn aces_maps_zero_to_zero_and_is_less_than_one() {
        assert!(TonemapOperator::Aces.apply_scalar(0.0).abs() < 1e-6);
        assert!(TonemapOperator::Aces.apply_scalar(1.0) < 1.0);
        assert!(TonemapOperator::Aces.apply_scalar(1e6) <= 1.0);
    }

    #[test]
    fn tonemap_handles_negative_and_nan_input() {
        assert_eq!(TonemapOperator::Aces.apply_scalar(-1.0), 0.0);
        assert_eq!(TonemapOperator::Aces.apply_scalar(f32::NAN), 0.0);
        let rgb = TonemapOperator::Reinhard.apply([f32::NAN, -1.0, 1.0]);
        assert_eq!(rgb[0], 0.0);
        assert_eq!(rgb[1], 0.0);
        assert!((rgb[2] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn gamma_2_2_known_value() {
        let out = apply_gamma([0.5, 0.5, 0.5], 2.2);
        let expected = 0.5f32.powf(1.0 / 2.2);
        for c in out {
            assert!((c - expected).abs() < 1e-6);
        }
        // gamma<=1 不改变颜色
        assert_eq!(apply_gamma([0.3, 0.4, 0.5], 1.0), [0.3, 0.4, 0.5]);
        assert_eq!(apply_gamma([0.3, 0.4, 0.5], 0.0), [0.3, 0.4, 0.5]);
    }

    #[test]
    fn tonemap_settings_process_pipeline() {
        let s = TonemapSettings {
            enabled: true,
            operator: TonemapOperator::Reinhard,
            exposure: 2.0,
            gamma: 1.0,
        };
        // exposure 2 * 1 = 2 → reinhard 2/3
        let out = s.process([1.0, 0.0, 0.0]);
        assert!((out[0] - 2.0 / 3.0).abs() < 1e-5);
        assert_eq!(out[1], 0.0);
    }

    #[test]
    fn disabled_tonemap_is_identity() {
        let s = TonemapSettings {
            enabled: false,
            ..TonemapSettings::default()
        };
        assert!(s.is_identity());
        assert_eq!(s.process([2.0, 3.0, 4.0]), [2.0, 3.0, 4.0]);
    }

    #[test]
    fn tonemap_settings_validate_clamps() {
        let s = TonemapSettings {
            exposure: 0.0,
            gamma: 100.0,
            ..TonemapSettings::default()
        }
        .validate();
        assert!(s.exposure >= 0.01);
        assert!(s.gamma <= 5.0);
        let s = TonemapSettings {
            exposure: f32::NAN,
            gamma: f32::NAN,
            ..TonemapSettings::default()
        }
        .validate();
        assert!((s.exposure - 1.0).abs() < 1e-6);
        assert!((s.gamma - 2.2).abs() < 1e-6);
    }

    #[test]
    fn bloom_prefilter_thresholds_bright_pixels() {
        let s = BloomSettings {
            enabled: true,
            threshold: 1.0,
            soft_knee: 0.0,
            intensity: 1.0,
            mip_count: 5,
        };
        // 暗于阈值 → 0
        assert_eq!(s.prefilter([0.2, 0.2, 0.2]), [0.0, 0.0, 0.0]);
        // 亮于阈值 → 保留超出部分
        let bright = s.prefilter([3.0, 3.0, 3.0]);
        assert!(bright[0] > 0.0 && bright[0] < 3.0);
        // 纯黑安全
        assert_eq!(s.prefilter([0.0, 0.0, 0.0]), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn bloom_combine_respects_intensity_and_switch() {
        let s = BloomSettings {
            enabled: true,
            threshold: 1.0,
            soft_knee: 0.5,
            intensity: 0.5,
            mip_count: 5,
        };
        let out = s.combine([1.0, 1.0, 1.0], [0.4, 0.4, 0.4]);
        assert!((out[0] - 1.2).abs() < 1e-6);
        let off = BloomSettings {
            enabled: false,
            ..s
        };
        assert_eq!(off.combine([1.0, 1.0, 1.0], [0.4, 0.4, 0.4]), [1.0, 1.0, 1.0]);
    }

    #[test]
    fn bloom_settings_validate_clamps() {
        let s = BloomSettings {
            enabled: true,
            threshold: -5.0,
            soft_knee: 9.0,
            intensity: -1.0,
            mip_count: 0,
        }
        .validate();
        assert_eq!(s.threshold, 0.0);
        assert_eq!(s.soft_knee, 1.0);
        assert_eq!(s.intensity, 0.0);
        assert_eq!(s.mip_count, 1);
        let s = BloomSettings {
            mip_count: 100,
            ..BloomSettings::default()
        }
        .validate();
        assert_eq!(s.mip_count, 6);
    }

    #[test]
    fn push_constant_layouts_are_16_bytes() {
        assert_eq!(size_of::<PostTonemapPushConstant>(), 16);
        assert_eq!(size_of::<BloomPrefilterPushConstant>(), 16);
        assert_eq!(std::mem::offset_of!(PostTonemapPushConstant, params), 0);
        assert_eq!(std::mem::offset_of!(BloomPrefilterPushConstant, params), 0);

        let s = TonemapSettings {
            enabled: true,
            operator: TonemapOperator::Filmic,
            exposure: 1.5,
            gamma: 2.2,
        };
        let pc = s.push_constant(0.25);
        assert_eq!(pc.params[0], 1.5);
        assert_eq!(pc.params[1], 2.2);
        assert_eq!(pc.params[2], 3.0);
        assert_eq!(pc.params[3], 0.25);
    }

    #[test]
    fn bloom_prefilter_push_constant_uses_inverse_extent() {
        let s = BloomSettings::default();
        let pc = BloomPrefilterPushConstant::from_settings(&s, [1920, 1080]);
        assert!((pc.params[2] - 1.0 / 1920.0).abs() < 1e-9);
        assert!((pc.params[3] - 1.0 / 1080.0).abs() < 1e-9);
    }

    #[test]
    fn post_pass_order_is_documented() {
        assert_eq!(
            POST_PASS_ORDER,
            &["bloom_prefilter", "post_tonemap", "fxaa"]
        );
        assert!(pi_matches_shader_contract());
    }
}
