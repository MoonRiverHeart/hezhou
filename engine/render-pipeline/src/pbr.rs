//! PBR 材质与着色（0.2 最小实现）
//!
//! 内容:
//! - [`PbrMaterial`]: albedo / metallic / roughness / normal / AO / emissive 参数与范围校验
//! - [`PbrLight`]: 方向光参数（方向 + 颜色 + 强度 + 环境项）
//! - CPU 参考 BRDF（GGX + Smith + Schlick），用于单元测试、编辑器预览与离线校验
//! - GPU 数据布局: [`PbrPushConstant`]（96 bytes）与 [`PbrFrameUbo`]（480 bytes）
//!
//! **与 legacy 路径的关系**:
//! 着色模型由 [`ShadingModel`] 显式选择，默认 `BlinnPhong`（0.1 行为）。
//! PBR 只是新增路径，`rotation.vert/frag` 与现有光栅管线不受影响。
//!
//! **GPU 布局约定**（必须与 `shaders/pbr.vert` / `shaders/pbr.frag` 严格一致，
//! 由 `tests/shader_contract.rs` 校验）:
//! - Push constant: `mat4 model; vec4 base_color; vec4 material_params;` = 96 bytes
//! - Frame UBO(std140): direction/intensity、color/shadow_strength、ambient/cascade_count、
//!   cascade_splits、shadow_params、view、proj、light_view_proj[4]、camera_pos = 480 bytes

use bytemuck::{Pod, Zeroable};

/// 介电质基础反射率（非金属 F0）
pub const DIELECTRIC_F0: f32 = 0.04;
/// 粗糙度下限 — 避免 GGX 分布函数在高光核心处除零/过曝
pub const MIN_ROUGHNESS: f32 = 0.045;
/// 圆周率（与 GLSL 端 `PBR_PI` 一致）
pub const PBR_PI: f32 = 3.14159265;
/// 着色模型
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShadingModel {
    /// 0.1 现有路径 — Blinn-Phong（默认，可回退）
    #[default]
    BlinnPhong,
    /// 0.2 PBR（GGX + Smith + Schlick）
    Pbr,
}

impl ShadingModel {
    /// 转整数（FFI/日志）
    pub fn as_u32(self) -> u32 {
        match self {
            ShadingModel::BlinnPhong => 0,
            ShadingModel::Pbr => 1,
        }
    }

    /// 从整数解析（非法值回退 Blinn-Phong）
    pub fn from_u32(value: u32) -> Self {
        match value {
            1 => ShadingModel::Pbr,
            _ => ShadingModel::BlinnPhong,
        }
    }

    /// 显示名
    pub fn label(self) -> &'static str {
        match self {
            ShadingModel::BlinnPhong => "blinn-phong",
            ShadingModel::Pbr => "pbr",
        }
    }
}

/// PBR 材质参数
///
/// 字段语义:
/// - `base_color`: 线性空间 albedo（RGBA，alpha 供透明材质扩展，0.2 不使用）
/// - `metallic`: 0=介电质，1=金属
/// - `roughness`: 0=镜面，1=完全粗糙
/// - `normal_scale`: 法线贴图强度系数
/// - `ao`: 环境光遮蔽（0=全遮蔽，1=无遮蔽）
/// - `emissive` / `emissive_strength`: 自发光（线性空间）
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PbrMaterial {
    /// 线性空间 albedo（RGBA）
    pub base_color: [f32; 4],
    /// 金属度 0..=1
    pub metallic: f32,
    /// 粗糙度 0..=1
    pub roughness: f32,
    /// 法线贴图强度（0 = 无扰动）
    pub normal_scale: f32,
    /// 环境光遮蔽 0..=1
    pub ao: f32,
    /// 自发光颜色（线性）
    pub emissive: [f32; 3],
    /// 自发光强度
    pub emissive_strength: f32,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            base_color: [0.8, 0.8, 0.8, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            normal_scale: 1.0,
            ao: 1.0,
            emissive: [0.0, 0.0, 0.0],
            emissive_strength: 0.0,
        }
    }
}

impl PbrMaterial {
    /// 便捷构造: 给定 albedo 的介电质材质
    pub fn dielectric(albedo: [f32; 3], roughness: f32) -> Self {
        Self {
            base_color: [albedo[0], albedo[1], albedo[2], 1.0],
            metallic: 0.0,
            roughness,
            ..Self::default()
        }
    }

    /// 便捷构造: 给定 albedo 的金属材质
    pub fn metal(albedo: [f32; 3], roughness: f32) -> Self {
        Self {
            base_color: [albedo[0], albedo[1], albedo[2], 1.0],
            metallic: 1.0,
            roughness,
            ..Self::default()
        }
    }

    /// 是否所有字段都是有限值且在合法范围
    pub fn is_valid(&self) -> bool {
        let range_ok = |v: f32, lo: f32, hi: f32| v.is_finite() && v >= lo && v <= hi;
        range_ok(self.base_color[0], 0.0, 1.0)
            && range_ok(self.base_color[1], 0.0, 1.0)
            && range_ok(self.base_color[2], 0.0, 1.0)
            && range_ok(self.base_color[3], 0.0, 1.0)
            && range_ok(self.metallic, 0.0, 1.0)
            && range_ok(self.roughness, 0.0, 1.0)
            && range_ok(self.normal_scale, 0.0, 4.0)
            && range_ok(self.ao, 0.0, 1.0)
            && self.emissive.iter().all(|c| c.is_finite() && *c >= 0.0)
            && range_ok(self.emissive_strength, 0.0, 100.0)
    }

    /// 返回收敛后的材质（NaN 回退默认值，越界 clamp）
    pub fn sanitized(&self) -> Self {
        let default = Self::default();
        let mut out = *self;
        for i in 0..4 {
            out.base_color[i] = clamp_or(out.base_color[i], 0.0, 1.0, default.base_color[i]);
        }
        out.metallic = clamp_or(out.metallic, 0.0, 1.0, default.metallic);
        out.roughness = clamp_or(out.roughness, 0.0, 1.0, default.roughness);
        out.normal_scale = clamp_or(out.normal_scale, 0.0, 4.0, default.normal_scale);
        out.ao = clamp_or(out.ao, 0.0, 1.0, default.ao);
        for i in 0..3 {
            out.emissive[i] = clamp_or(out.emissive[i], 0.0, 1.0, default.emissive[i]);
        }
        out.emissive_strength =
            clamp_or(out.emissive_strength, 0.0, 100.0, default.emissive_strength);
        // 粗糙度下限：避免 GGX 高光除零（0 保留为 0，由着色函数内部 clamp）
        out.roughness = out.roughness.max(0.0);
        out
    }
}

/// NaN 用默认值、越界 clamp
fn clamp_or(value: f32, lo: f32, hi: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(lo, hi)
    } else {
        fallback
    }
}

/// 方向光参数
///
/// `direction` 是**光的传播方向**（与 `DirectionalLightComponent.direction` 一致），
/// 着色时朝向光源的方向为 `-direction`。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PbrLight {
    /// 光的传播方向（非单位向量会在使用时归一化）
    pub direction: [f32; 3],
    /// 光颜色（线性空间）
    pub color: [f32; 3],
    /// 光强度
    pub intensity: f32,
    /// 环境光颜色（线性空间，BRDF 之外的常量项）
    pub ambient: [f32; 3],
}

impl Default for PbrLight {
    /// 与现有 `LightUboData` 的 fallback 一致: direction=(-0.5,-1,-0.5), 白光, 强度 1
    fn default() -> Self {
        Self {
            direction: [-0.5, -1.0, -0.5],
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            ambient: [0.03, 0.03, 0.03],
        }
    }
}

impl PbrLight {
    /// 单位化的"朝向光源"方向；非法方向返回默认光方向
    pub fn light_dir(&self) -> [f32; 3] {
        let d = normalize3(self.direction);
        if d == [0.0, 0.0, 0.0] {
            normalize3(PbrLight::default().direction)
        } else {
            [-d[0], -d[1], -d[2]]
        }
    }
}

/// PBR 顶点/片元共用 push constant（96 bytes，匹配 `shaders/pbr.vert`）
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct PbrPushConstant {
    /// 模型矩阵（column-major）
    pub model: [[f32; 4]; 4],
    /// albedo RGBA
    pub base_color: [f32; 4],
    /// x=metallic, y=roughness, z=normal_scale, w=ao
    pub material_params: [f32; 4],
}

impl PbrPushConstant {
    /// 从模型矩阵 + 材质构造
    pub fn from_material(model: [[f32; 4]; 4], material: &PbrMaterial) -> Self {
        let m = material.sanitized();
        Self {
            model,
            base_color: m.base_color,
            material_params: [m.metallic, m.roughness, m.normal_scale, m.ao],
        }
    }
}

/// PBR 每帧 UBO（480 bytes，std140，匹配 `shaders/pbr.vert` 的 `PbrFrameUbo`）
///
/// 布局:
/// ```text
/// vec3 direction;  float intensity;      // 0
/// vec3 color;      float shadow_strength; // 16
/// vec3 ambient;    float cascade_count;  // 32
/// vec4 cascade_splits;                   // 48
/// vec4 shadow_params;  // (depth_bias, normal_bias, texel_size, 0)  // 64
/// mat4 view;                             // 80
/// mat4 proj;                             // 144
/// mat4 light_view_proj[4];               // 208 .. 464
/// vec4 camera_pos;                       // 464 .. 480
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct PbrFrameUbo {
    /// 光的传播方向
    pub direction: [f32; 3],
    /// 光强度
    pub intensity: f32,
    /// 光颜色
    pub color: [f32; 3],
    /// 阴影强度（0=无阴影，1=全阴影）
    pub shadow_strength: f32,
    /// 环境光颜色
    pub ambient: [f32; 3],
    /// 启用的 cascade 数量（0=无阴影）
    pub cascade_count: f32,
    /// 各级 cascade 的远平面距离（view 空间）
    pub cascade_splits: [f32; 4],
    /// x=depth_bias, y=normal_bias, z=texel_size, w=保留
    pub shadow_params: [f32; 4],
    /// 摄像机 view 矩阵（column-major）
    pub view: [[f32; 4]; 4],
    /// 摄像机 projection 矩阵（column-major）
    pub proj: [[f32; 4]; 4],
    /// 各级 cascade 的光源 view-projection 矩阵
    pub light_view_proj: [[[f32; 4]; 4]; 4],
    /// 摄像机世界坐标（w=保留）
    pub camera_pos: [f32; 4],
}

impl PbrFrameUbo {
    /// 从光源参数 + 相机矩阵构造（阴影部分默认关闭）
    pub fn from_light(light: &PbrLight, view: [[f32; 4]; 4], proj: [[f32; 4]; 4], camera_pos: [f32; 3]) -> Self {
        Self {
            direction: light.direction,
            intensity: light.intensity,
            color: light.color,
            shadow_strength: 0.0,
            ambient: light.ambient,
            cascade_count: 0.0,
            cascade_splits: [0.0; 4],
            shadow_params: [0.0; 4],
            view,
            proj,
            light_view_proj: [[[0.0; 4]; 4]; 4],
            camera_pos: [camera_pos[0], camera_pos[1], camera_pos[2], 0.0],
        }
    }

    /// 写入阴影数据（cascade 矩阵、分割距离、bias/texel）
    ///
    /// 该函数由 `shadow::ShadowFrameData::write_into_ubo` 调用，保持 pbr.rs 不反向依赖 shadow.rs 的类型细节。
    pub fn set_shadow_data(
        &mut self,
        shadow_strength: f32,
        cascade_splits: [f32; 4],
        shadow_params: [f32; 4],
        light_view_proj: [[[f32; 4]; 4]; 4],
        cascade_count: u32,
    ) {
        self.shadow_strength = shadow_strength.clamp(0.0, 1.0);
        self.cascade_splits = cascade_splits;
        self.shadow_params = shadow_params;
        self.light_view_proj = light_view_proj;
        self.cascade_count = cascade_count as f32;
    }
}

// === CPU 参考 BRDF（GGX / Smith / Schlick） ===

/// Schlick 菲涅尔（标量）
pub fn fresnel_schlick_scalar(cos_theta: f32, f0: f32) -> f32 {
    let c = cos_theta.clamp(0.0, 1.0);
    f0 + (1.0 - f0) * (1.0 - c).powi(5)
}

/// Schlick 菲涅尔（RGB）
pub fn fresnel_schlick(cos_theta: f32, f0: [f32; 3]) -> [f32; 3] {
    let c = cos_theta.clamp(0.0, 1.0);
    let k = (1.0 - c).powi(5);
    [
        f0[0] + (1.0 - f0[0]) * k,
        f0[1] + (1.0 - f0[1]) * k,
        f0[2] + (1.0 - f0[2]) * k,
    ]
}

/// GGX / Trowbridge-Reitz 法线分布
pub fn distribution_ggx(ndoth: f32, roughness: f32) -> f32 {
    let r = roughness.max(MIN_ROUGHNESS);
    let a = r * r;
    let a2 = a * a;
    let d = ndoth * ndoth * (a2 - 1.0) + 1.0;
    a2 / (PBR_PI * d * d)
}

/// Smith 几何项的单向 Schlick-GGX 近似
pub fn geometry_schlick_ggx(ndotx: f32, roughness: f32) -> f32 {
    let r = roughness.max(MIN_ROUGHNESS);
    let k = (r + 1.0) * (r + 1.0) / 8.0;
    // 直接光照使用 (r+1)^2/8 的 k，这里保持与 GLSL 端一致
    ndotx / (ndotx * (1.0 - k) + k).max(1e-6)
}

/// Smith 联合几何项（遮蔽 + 阴影）
pub fn geometry_smith(ndotv: f32, ndotl: f32, roughness: f32) -> f32 {
    geometry_schlick_ggx(ndotv, roughness) * geometry_schlick_ggx(ndotl, roughness)
}

/// Cook-Torrance 镜面项（不含 NdotL 与入射光强）
pub fn cook_torrance_specular(
    normal: [f32; 3],
    view: [f32; 3],
    light_dir: [f32; 3],
    material: &PbrMaterial,
) -> [f32; 3] {
    let n = normalize3(normal);
    let v = normalize3(view);
    let l = normalize3(light_dir);
    if n == [0.0; 3] || l == [0.0; 3] {
        return [0.0; 3];
    }
    let ndotl = dot3(n, l).max(0.0);
    let ndotv = dot3(n, v).max(0.0);
    if ndotl <= 0.0 || ndotv <= 0.0 {
        return [0.0; 3];
    }
    let h = normalize3(add3(l, v));
    let ndoth = dot3(n, h).max(0.0);
    let vdoth = dot3(v, h).max(0.0);

    let m = material.sanitized();
    let roughness = m.roughness.max(MIN_ROUGHNESS);
    let diffuse_color = [
        m.base_color[0] * (1.0 - m.metallic),
        m.base_color[1] * (1.0 - m.metallic),
        m.base_color[2] * (1.0 - m.metallic),
    ];
    let f0 = [
        DIELECTRIC_F0 + (diffuse_color[0] - DIELECTRIC_F0) * m.metallic,
        DIELECTRIC_F0 + (diffuse_color[1] - DIELECTRIC_F0) * m.metallic,
        DIELECTRIC_F0 + (diffuse_color[2] - DIELECTRIC_F0) * m.metallic,
    ];
    let f = fresnel_schlick(vdoth, f0);
    let d = distribution_ggx(ndoth, roughness);
    let g = geometry_smith(ndotv, ndotl, roughness);
    let denom = (4.0 * ndotv * ndotl).max(1e-6);
    let spec = d * g / denom;
    [f[0] * spec, f[1] * spec, f[2] * spec]
}

/// CPU 参考: 方向光直接光照（漫反射 + 镜面，不含环境项/自发光）
///
/// 返回线性 HDR 辐亮度。`world_pos` 为着色点世界坐标，`camera_pos` 为相机世界坐标。
pub fn pbr_direct_radiance(
    material: &PbrMaterial,
    normal: [f32; 3],
    world_pos: [f32; 3],
    camera_pos: [f32; 3],
    light: &PbrLight,
) -> [f32; 3] {
    let m = material.sanitized();
    let n = normalize3(normal);
    let l = light.light_dir();
    let ndotl = dot3(n, l).max(0.0);
    if ndotl <= 0.0 || light.intensity <= 0.0 {
        return [0.0; 3];
    }
    let v = normalize3(sub3(camera_pos, world_pos));
    let diffuse_k = (1.0 - m.metallic).max(0.0);
    let diffuse = [
        m.base_color[0] * diffuse_k / PBR_PI,
        m.base_color[1] * diffuse_k / PBR_PI,
        m.base_color[2] * diffuse_k / PBR_PI,
    ];
    let spec = cook_torrance_specular(n, v, l, &m);
    let light_scale = light.intensity * ndotl;
    [
        (diffuse[0] + spec[0]) * light.color[0] * light_scale,
        (diffuse[1] + spec[1]) * light.color[1] * light_scale,
        (diffuse[2] + spec[2]) * light.color[2] * light_scale,
    ]
}

/// CPU 参考: 环境项（ambient * albedo * ao），不含 IBL
pub fn pbr_ambient(material: &PbrMaterial, light: &PbrLight) -> [f32; 3] {
    let m = material.sanitized();
    let ao = m.ao.clamp(0.0, 1.0);
    [
        light.ambient[0] * m.base_color[0] * ao,
        light.ambient[1] * m.base_color[1] * ao,
        light.ambient[2] * m.base_color[2] * ao,
    ]
}

/// CPU 参考: 自发光项
pub fn pbr_emissive(material: &PbrMaterial) -> [f32; 3] {
    let m = material.sanitized();
    let s = m.emissive_strength;
    [m.emissive[0] * s, m.emissive[1] * s, m.emissive[2] * s]
}

/// CPU 参考: 完整着色（直接光 + 环境 + 自发光）
pub fn pbr_shade(
    material: &PbrMaterial,
    normal: [f32; 3],
    world_pos: [f32; 3],
    camera_pos: [f32; 3],
    light: &PbrLight,
) -> [f32; 3] {
    let direct = pbr_direct_radiance(material, normal, world_pos, camera_pos, light);
    let ambient = pbr_ambient(material, light);
    let emissive = pbr_emissive(material);
    [
        direct[0] + ambient[0] + emissive[0],
        direct[1] + ambient[1] + emissive[1],
        direct[2] + ambient[2] + emissive[2],
    ]
}

// === 小型向量工具（避免在 CPU 参考实现里引入额外依赖） ===

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 1e-8 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, 0.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn default_material_is_valid() {
        let m = PbrMaterial::default();
        assert!(m.is_valid());
        assert_eq!(m.sanitized(), m);
        assert_eq!(m.base_color[3], 1.0);
    }

    #[test]
    fn sanitize_clamps_out_of_range_fields() {
        let m = PbrMaterial {
            base_color: [2.0, -1.0, 0.5, 1.0],
            metallic: 3.0,
            roughness: -2.0,
            normal_scale: 10.0,
            ao: 7.0,
            emissive: [5.0, -1.0, 0.2],
            emissive_strength: 1000.0,
        };
        assert!(!m.is_valid());
        let s = m.sanitized();
        assert!(s.is_valid());
        assert_eq!(s.base_color[0], 1.0);
        assert_eq!(s.base_color[1], 0.0);
        assert_eq!(s.metallic, 1.0);
        assert_eq!(s.roughness, 0.0);
        assert_eq!(s.normal_scale, 4.0);
        assert_eq!(s.ao, 1.0);
        assert_eq!(s.emissive[0], 1.0);
        assert_eq!(s.emissive_strength, 100.0);
    }

    #[test]
    fn sanitize_replaces_nan_with_defaults() {
        let m = PbrMaterial {
            base_color: [f32::NAN, 0.5, 0.5, 1.0],
            metallic: f32::NAN,
            roughness: f32::INFINITY,
            ..PbrMaterial::default()
        };
        let s = m.sanitized();
        assert!(s.base_color[0].is_finite());
        assert_eq!(s.base_color[0], PbrMaterial::default().base_color[0]);
        assert_eq!(s.metallic, 0.0);
        assert_eq!(s.roughness, PbrMaterial::default().roughness);
    }

    #[test]
    fn metal_has_no_lambert_diffuse() {
        let white_metal = PbrMaterial::metal([1.0, 1.0, 1.0], 0.2);
        let radiance = pbr_direct_radiance(
            &white_metal,
            [0.0, 1.0, 0.0],
            [0.0; 3],
            [0.0, 0.0, 5.0],
            &PbrLight::default(),
        );
        // 点乘方向可能朝上/下，这里只要求有限且非负
        assert!(radiance.iter().all(|c| c.is_finite() && *c >= 0.0));
        // 金属的漫反射系数为 0：把视角转到掠射，radiance 仍只来自镜面项
        let ndotl = {
            let l = PbrLight::default().light_dir();
            dot3([0.0, 1.0, 0.0], l).max(0.0)
        };
        if ndotl > 0.0 {
            // 漫反射贡献应为 0，故输出应显著小于同等非金属材质
            let dielectric = PbrMaterial::dielectric([1.0, 1.0, 1.0], 0.2);
            let d_rad = pbr_direct_radiance(
                &dielectric,
                [0.0, 1.0, 0.0],
                [0.0; 3],
                [0.0, 0.0, 5.0],
                &PbrLight::default(),
            );
            let metal_lum: f32 = radiance.iter().sum();
            let dielectric_lum: f32 = d_rad.iter().sum();
            assert!(
                dielectric_lum > metal_lum,
                "介电质漫反射应比金属亮: dielectric={dielectric_lum}, metal={metal_lum}"
            );
        }
    }

    #[test]
    fn direct_radiance_zero_when_light_behind_surface() {
        let light = PbrLight {
            direction: [0.0, -1.0, 0.0], // 从上方照下
            ..PbrLight::default()
        };
        // 法线朝下 → NdotL = 0
        let radiance = pbr_direct_radiance(
            &PbrMaterial::default(),
            [0.0, -1.0, 0.0],
            [0.0; 3],
            [0.0, 0.0, 5.0],
            &light,
        );
        assert_eq!(radiance, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn direct_radiance_is_finite_and_non_negative() {
        let lights = [
            PbrLight::default(),
            PbrLight {
                direction: [0.0, 0.0, 1.0],
                intensity: 10.0,
                ..PbrLight::default()
            },
        ];
        for light in lights {
            for normal in [[0.0, 1.0, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]] {
                let r = pbr_direct_radiance(
                    &PbrMaterial::metal([0.9, 0.8, 0.2], 0.15),
                    normal,
                    [1.0, 2.0, 3.0],
                    [1.0, 2.0, 8.0],
                    &light,
                );
                assert!(r.iter().all(|c| c.is_finite() && *c >= 0.0));
            }
        }
    }

    #[test]
    fn ggx_lobe_gets_wider_with_roughness() {
        let smooth = distribution_ggx(1.0, 0.05);
        let rough = distribution_ggx(1.0, 0.8);
        assert!(smooth > rough);
        // 偏离法线方向时，粗糙表面可能反超
        let smooth_off = distribution_ggx(0.5, 0.05);
        let rough_off = distribution_ggx(0.5, 0.8);
        assert!(rough_off > smooth_off);
    }

    #[test]
    fn fresnel_schlick_endpoints() {
        // 法线入射（cos=1）→ F0；掠射（cos=0）→ 1.0
        assert!((fresnel_schlick_scalar(1.0, DIELECTRIC_F0) - DIELECTRIC_F0).abs() < 1e-6);
        assert!((fresnel_schlick_scalar(0.0, 0.04) - 1.0).abs() < 1e-6);
        let rgb = fresnel_schlick(1.0, [0.04, 0.04, 0.04]);
        assert!((rgb[0] - 0.04).abs() < 1e-6);
    }

    #[test]
    fn ambient_respects_ao_and_metallic_independence() {
        let m = PbrMaterial {
            ao: 0.0,
            ..PbrMaterial::dielectric([1.0, 1.0, 1.0], 0.5)
        };
        assert_eq!(pbr_ambient(&m, &PbrLight::default()), [0.0, 0.0, 0.0]);
        let m = PbrMaterial {
            ao: 1.0,
            ..PbrMaterial::dielectric([1.0, 1.0, 1.0], 0.5)
        };
        let amb = pbr_ambient(&m, &PbrLight::default());
        assert!(amb[0] > 0.0);
    }

    #[test]
    fn emissive_contributes_to_final_shading() {
        let m = PbrMaterial {
            emissive: [1.0, 0.0, 0.0],
            emissive_strength: 2.0,
            ..PbrMaterial::default()
        };
        assert_eq!(pbr_emissive(&m), [2.0, 0.0, 0.0]);
        let shaded = pbr_shade(
            &m,
            [0.0, 1.0, 0.0],
            [0.0; 3],
            [0.0, 0.0, 5.0],
            &PbrLight {
                intensity: 0.0,
                ambient: [0.0; 3],
                ..PbrLight::default()
            },
        );
        assert!((shaded[0] - 2.0).abs() < 1e-5);
    }

    #[test]
    fn shading_model_roundtrip() {
        assert_eq!(ShadingModel::default(), ShadingModel::BlinnPhong);
        assert_eq!(ShadingModel::from_u32(1), ShadingModel::Pbr);
        assert_eq!(ShadingModel::from_u32(7), ShadingModel::BlinnPhong);
        assert_eq!(ShadingModel::Pbr.as_u32(), 1);
        assert_eq!(ShadingModel::BlinnPhong.label(), "blinn-phong");
    }

    #[test]
    fn push_constant_layout_is_96_bytes() {
        assert_eq!(size_of::<PbrPushConstant>(), 96);
        assert_eq!(std::mem::offset_of!(PbrPushConstant, model), 0);
        assert_eq!(std::mem::offset_of!(PbrPushConstant, base_color), 64);
        assert_eq!(std::mem::offset_of!(PbrPushConstant, material_params), 80);
        let pc = PbrPushConstant::from_material([[1.0; 4]; 4], &PbrMaterial::metal([0.1, 0.2, 0.3], 0.4));
        assert_eq!(pc.material_params, [1.0, 0.4, 1.0, 1.0]);
        assert_eq!(pc.base_color, [0.1, 0.2, 0.3, 1.0]);
    }

    #[test]
    fn frame_ubo_layout_is_480_bytes() {
        assert_eq!(size_of::<PbrFrameUbo>(), 480);
        assert_eq!(std::mem::offset_of!(PbrFrameUbo, direction), 0);
        assert_eq!(std::mem::offset_of!(PbrFrameUbo, color), 16);
        assert_eq!(std::mem::offset_of!(PbrFrameUbo, ambient), 32);
        assert_eq!(std::mem::offset_of!(PbrFrameUbo, cascade_splits), 48);
        assert_eq!(std::mem::offset_of!(PbrFrameUbo, shadow_params), 64);
        assert_eq!(std::mem::offset_of!(PbrFrameUbo, view), 80);
        assert_eq!(std::mem::offset_of!(PbrFrameUbo, proj), 144);
        assert_eq!(std::mem::offset_of!(PbrFrameUbo, light_view_proj), 208);
        assert_eq!(std::mem::offset_of!(PbrFrameUbo, camera_pos), 464);
    }

    #[test]
    fn frame_ubo_from_light_and_shadow_write() {
        let light = PbrLight {
            direction: [0.0, -1.0, 0.0],
            intensity: 2.0,
            ..PbrLight::default()
        };
        let mut ubo = PbrFrameUbo::from_light(
            &light,
            [[1.0; 4]; 4],
            [[2.0; 4]; 4],
            [1.0, 2.0, 3.0],
        );
        assert_eq!(ubo.direction, [0.0, -1.0, 0.0]);
        assert_eq!(ubo.intensity, 2.0);
        assert_eq!(ubo.cascade_count, 0.0);
        assert_eq!(ubo.camera_pos, [1.0, 2.0, 3.0, 0.0]);
        ubo.set_shadow_data(0.8, [5.0, 15.0, 0.0, 0.0], [0.001, 0.02, 0.0005, 0.0], [[[3.0; 4]; 4]; 4], 2);
        assert!((ubo.shadow_strength - 0.8).abs() < 1e-6);
        assert_eq!(ubo.cascade_count, 2.0);
        assert_eq!(ubo.cascade_splits[1], 15.0);
        assert_eq!(ubo.shadow_params[2], 0.0005);
    }

    #[test]
    fn pbr_light_direction_helper() {
        let l = PbrLight {
            direction: [0.0, -2.0, 0.0],
            ..PbrLight::default()
        };
        let toward = l.light_dir();
        assert!((toward[1] - 1.0).abs() < 1e-6);
        let zero = PbrLight {
            direction: [0.0, 0.0, 0.0],
            ..PbrLight::default()
        };
        assert!(zero.light_dir().iter().all(|c| c.is_finite()));
    }
}
