//! RenderContext 与 0.2 渲染设置集成测试
//!
//! 校验编辑器每帧传入的 `RenderContext` 能直接派生:
//! - 质量档 → 阴影配置
//! - 摄像机 + 光源方向 → 阴影矩阵
//! - PBR 每帧 UBO（含阴影级联数据）

use hezhou_render_pipeline::{
    BloomSettings, CameraParams, PbrLight, PbrMaterial, PbrMaterialOverride,
    RenderContext, RenderQualityPreset, RenderSettings, ShadingModel, ShadowSettings,
    TonemapOperator, ViewportExtent,
};

fn camera() -> CameraParams {
    CameraParams {
        position: [0.0, 3.0, 10.0],
        yaw: 0.2,
        pitch: -0.25,
        fov: 60.0_f32.to_radians(),
        near: 0.1,
        far: 120.0,
    }
}

fn context_with(preset: RenderQualityPreset) -> RenderContext {
    RenderContext {
        camera: camera(),
        viewport_extent: ViewportExtent::new(1920, 1080),
        render_settings: RenderSettings::for_preset(preset),
        ..RenderContext::default()
    }
}

#[test]
fn default_context_is_legacy_and_has_no_shadow() {
    let ctx = RenderContext::default();
    assert!(ctx.render_settings.is_legacy_compatible());
    assert_eq!(ctx.render_settings.shading, ShadingModel::BlinnPhong);
    assert!(ctx.shadow_config().is_none());
    assert!(ctx
        .shadow_frame([-0.5, -1.0, -0.5])
        .expect("legacy 路径不应报错")
        .is_none());
}

#[test]
fn aspect_uses_viewport_extent() {
    let mut ctx = RenderContext::default();
    assert!((ctx.aspect() - 1280.0 / 720.0).abs() < 1e-6);
    ctx.viewport_extent = ViewportExtent::new(0, 0);
    assert_eq!(ctx.aspect(), 1.0);
}

#[test]
fn high_preset_context_produces_three_cascades() {
    let ctx = context_with(RenderQualityPreset::High);
    let config = ctx.shadow_config().expect("高配应启用阴影");
    assert_eq!(config.cascade_count, 3);
    assert_eq!(config.resolution, 2048);

    let frame = ctx
        .shadow_frame([-0.4, -1.0, -0.3])
        .expect("阴影计算不应失败")
        .expect("高配应返回阴影数据");
    assert_eq!(frame.cascade_count, 3);
    assert!(frame.cascade_splits[2] > frame.cascade_splits[1]);
    assert!(frame.cascade_splits[1] > frame.cascade_splits[0]);
    for i in 0..3 {
        assert!(frame.light_view_proj[i]
            .iter()
            .flatten()
            .all(|v| v.is_finite()));
    }
}

#[test]
fn low_preset_context_has_no_shadow_frame() {
    let ctx = context_with(RenderQualityPreset::Low);
    assert!(ctx.shadow_config().is_none());
    assert!(ctx
        .shadow_frame([-0.4, -1.0, -0.3])
        .expect("低配无需阴影")
        .is_none());
}

#[test]
fn shadow_frame_rejects_invalid_camera() {
    let mut ctx = context_with(RenderQualityPreset::Medium);
    ctx.camera.near = 0.0;
    assert!(ctx.shadow_frame([-0.4, -1.0, -0.3]).is_err());
}

#[test]
fn pbr_frame_ubo_contains_shadow_data_when_enabled() {
    let ctx = context_with(RenderQualityPreset::High);
    let frame = ctx
        .shadow_frame([-0.4, -1.0, -0.3])
        .unwrap()
        .expect("应有阴影数据");
    let light = PbrLight::default();
    let view = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, -10.0, 1.0]];
    let proj = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
    let ubo = ctx.pbr_frame_ubo(&light, view, proj, Some(&frame));
    assert_eq!(ubo.cascade_count, 3.0);
    assert!((ubo.shadow_strength - 1.0).abs() < 1e-6);
    assert_eq!(ubo.light_view_proj[0], frame.light_view_proj[0]);
    // 摄像机位置/光源方向透传
    assert_eq!(ubo.camera_pos, [0.0, 3.0, 10.0, 0.0]);
    assert_eq!(ubo.direction, light.direction);
    // 正切与投影矩阵透传
    assert_eq!(ubo.view, view);
    assert_eq!(ubo.proj, proj);
}

#[test]
fn pbr_frame_ubo_without_shadow_has_cascade_zero() {
    let ctx = context_with(RenderQualityPreset::Low);
    let ubo = ctx.pbr_frame_ubo(
        &PbrLight::default(),
        [[0.0; 4]; 4],
        [[0.0; 4]; 4],
        None,
    );
    assert_eq!(ubo.cascade_count, 0.0);
    assert_eq!(ubo.shadow_strength, 0.0);
}

#[test]
fn quality_presets_map_to_expected_switches() {
    let high = context_with(RenderQualityPreset::High);
    assert!(high.render_settings.effective_shadows());
    assert!(high.render_settings.effective_bloom());
    assert!(high.render_settings.effective_tonemap());
    assert_eq!(high.render_settings.shading, ShadingModel::Pbr);
    assert_eq!(high.render_settings.quality.preset, RenderQualityPreset::High);

    let medium = context_with(RenderQualityPreset::Medium);
    assert!(medium.render_settings.effective_shadows());
    assert!(!medium.render_settings.effective_bloom());
    assert_eq!(medium.render_settings.quality.shadow_cascade_count, 2);
}

#[test]
fn render_settings_defaults_are_sane() {
    let settings = RenderSettings::default();
    assert_eq!(settings.tonemap.operator, TonemapOperator::Aces);
    assert!((settings.tonemap.gamma - 2.2).abs() < 1e-6);
    assert_eq!(settings.shadows, ShadowSettings { enabled: false, ..ShadowSettings::default() });
    assert_eq!(settings.bloom, BloomSettings { enabled: false, ..BloomSettings::default() });
    assert!(settings.clone().validate() == settings);
}

#[test]
fn shadow_settings_invalid_values_do_not_break_frame() {
    let mut ctx = context_with(RenderQualityPreset::Medium);
    ctx.render_settings.shadows.depth_bias = -1.0;
    ctx.render_settings.shadows.cascade_lambda = 9.0;
    let frame = ctx
        .shadow_frame([0.0, -1.0, 0.0])
        .expect("非法调参不应导致错误（会被 validate 收敛）")
        .expect("中配应返回阴影数据");
    assert_eq!(frame.cascade_count, 2);
}

#[test]
fn render_context_resolves_per_entity_material() {
    let mut ctx = context_with(RenderQualityPreset::High);
    let red_metal = PbrMaterial::metal([0.9, 0.1, 0.1], 0.2);
    ctx.pbr_material_overrides.set(
        PbrMaterialOverride::new(1, red_metal).with_texture_path("textures/red.png"),
    );
    // 覆盖命中
    assert_eq!(ctx.resolve_pbr_material(1), &red_metal);
    assert_eq!(ctx.resolve_pbr_texture_path(1), Some("textures/red.png"));
    // 未覆盖实体回退 render_settings.default_material
    let default_material = ctx.render_settings.default_material;
    assert_eq!(ctx.resolve_pbr_material(2), &default_material);
    assert!(ctx.resolve_pbr_texture_path(2).is_none());
    // 覆盖可删除后回退
    assert!(ctx.pbr_material_overrides.remove(1));
    assert_eq!(ctx.resolve_pbr_material(1), &default_material);
}
