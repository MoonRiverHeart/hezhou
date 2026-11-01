//! 方向光 Shadow Map（CSM 级联阴影）— 0.2 最小实现
//!
//! 负责 CPU 侧的阴影渲染数据:
//! - [`ShadowSettings`]: 开关/bias/级联分割系数/最大距离/纹素吸附（用户调参）
//! - [`ShadowConfig`]: 质量档解析后的级联数与分辨率（由 `quality::QualitySettings` 提供）
//! - [`ShadowFrameData`]: 每帧计算出的各级 cascade 光源矩阵、分割距离、纹素世界尺寸
//!
//! 约定:
//! - 光源方向使用 `DirectionalLightComponent.direction` 的语义（**光的传播方向**），
//!   阴影相机放在 `center - to_light * radius`（`to_light = -direction`）。
//! - 深度范围使用 Vulkan 约定 `[0, 1]`（RH_ZO 正交投影）。
//! - 阴影贴图格式 `D32_SFLOAT`、采样器 `LESS_OR_EQUAL` + `CLAMP_TO_BORDER`，
//!   由 `hezhou-rhi-vulkan::render_quality::shadow_pass_desc` 生成（见该模块）。
//!
//! 已知限制（0.2）:
//! - 仅支持 1 盏方向光
//! - 不做逐级视锥剔除、不做 PCSS 软阴影（固定 3x3 PCF）
//! - 不做透明/alpha test 阴影

use bytemuck::{Pod, Zeroable};

use crate::context::CameraParams;
use crate::pbr::PbrFrameUbo;

/// 最大 cascade 级联数（受 `PbrFrameUbo.light_view_proj[4]` 限制）
pub const MAX_CASCADES: usize = 4;
/// 默认级联分割系数（0=均匀分割，1=对数分割）
pub const DEFAULT_CASCADE_LAMBDA: f32 = 0.5;
/// 默认深度偏移（归一化深度单位）
pub const DEFAULT_DEPTH_BIAS: f32 = 0.0015;
/// 默认法线偏移（世界单位，用于抑制斜面阴影痤疮）
pub const DEFAULT_NORMAL_BIAS: f32 = 0.02;
/// 默认阴影最大距离（世界单位）
pub const DEFAULT_MAX_DISTANCE: f32 = 100.0;

/// 阴影用户调参
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowSettings {
    /// 是否启用阴影（与质量档开关共同决定，见 `RenderSettings::effective_shadows`）
    pub enabled: bool,
    /// 深度偏移（归一化深度单位，0..=0.1）
    pub depth_bias: f32,
    /// 法线偏移（世界单位，0..=1）
    pub normal_bias: f32,
    /// 级联分割系数: 0=均匀，1=对数（0..=1）
    pub cascade_lambda: f32,
    /// 阴影覆盖的最大距离（世界单位，会被相机 far 截断）
    pub max_distance: f32,
    /// 是否做纹素吸附（减少相机移动时的阴影边缘抖动）
    pub stable: bool,
}

impl Default for ShadowSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            depth_bias: DEFAULT_DEPTH_BIAS,
            normal_bias: DEFAULT_NORMAL_BIAS,
            cascade_lambda: DEFAULT_CASCADE_LAMBDA,
            max_distance: DEFAULT_MAX_DISTANCE,
            stable: true,
        }
    }
}

impl ShadowSettings {
    /// 收敛到合法区间（NaN 回退默认值）
    pub fn validate(mut self) -> Self {
        let def = Self::default();
        self.depth_bias = clamp_finite(self.depth_bias, 0.0, 0.1, def.depth_bias);
        self.normal_bias = clamp_finite(self.normal_bias, 0.0, 1.0, def.normal_bias);
        self.cascade_lambda = clamp_finite(self.cascade_lambda, 0.0, 1.0, def.cascade_lambda);
        self.max_distance = clamp_finite(self.max_distance, 0.1, 10_000.0, def.max_distance);
        self
    }
}

fn clamp_finite(value: f32, lo: f32, hi: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(lo, hi)
    } else {
        fallback
    }
}

/// 质量档解析后的阴影配置
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowConfig {
    /// cascade 级联数（1..=4）
    pub cascade_count: u32,
    /// 单级 cascade 贴图分辨率（正方形，2 的幂）
    pub resolution: u32,
    /// 深度偏移（归一化深度单位）
    pub depth_bias: f32,
    /// 法线偏移（世界单位）
    pub normal_bias: f32,
    /// 级联分割系数 0..=1
    pub cascade_lambda: f32,
    /// 阴影最大距离
    pub max_distance: f32,
    /// 纹素吸附
    pub stable: bool,
}

impl Default for ShadowConfig {
    fn default() -> Self {
        Self {
            cascade_count: 2,
            resolution: 1024,
            depth_bias: DEFAULT_DEPTH_BIAS,
            normal_bias: DEFAULT_NORMAL_BIAS,
            cascade_lambda: DEFAULT_CASCADE_LAMBDA,
            max_distance: DEFAULT_MAX_DISTANCE,
            stable: true,
        }
    }
}

impl ShadowConfig {
    /// 收敛到合法区间:
    /// - cascade_count → 1..=4
    /// - resolution → 256..=4096 的 2 的幂
    /// - bias/lambda/max_distance 见各默认值
    pub fn validate(self) -> Self {
        let settings = ShadowSettings {
            enabled: true,
            depth_bias: self.depth_bias,
            normal_bias: self.normal_bias,
            cascade_lambda: self.cascade_lambda,
            max_distance: self.max_distance,
            stable: self.stable,
        }
        .validate();
        Self {
            cascade_count: self.cascade_count.clamp(1, MAX_CASCADES as u32),
            resolution: crate::quality::normalize_shadow_resolution(self.resolution),
            depth_bias: settings.depth_bias,
            normal_bias: settings.normal_bias,
            cascade_lambda: settings.cascade_lambda,
            max_distance: settings.max_distance,
            stable: settings.stable,
        }
    }

    /// 单个纹素覆盖的世界尺寸
    pub fn texel_world_size(&self, radius: f32) -> f32 {
        let res = self.resolution.max(1) as f32;
        (2.0 * radius / res).max(1e-6)
    }
}

/// `shaders/shadow_depth.vert` 的 push constant（128 bytes）
///
/// 深度阶段每个 cascade 一次 draw，只传光源矩阵 + 模型矩阵。
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct ShadowPushConstant {
    /// 当前 cascade 的光源 view-projection 矩阵（column-major）
    pub light_view_proj: [[f32; 4]; 4],
    /// 模型矩阵（column-major）
    pub model: [[f32; 4]; 4],
}

/// 阴影计算错误
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShadowError {
    /// 相机参数非法（near<=0、far<=near、fov 越界、aspect<=0）
    InvalidCamera(String),
    /// 光源方向非法（零向量/NaN）
    InvalidLightDirection,
    /// 阴影配置非法
    InvalidConfig(String),
}

impl std::fmt::Display for ShadowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShadowError::InvalidCamera(msg) => write!(f, "invalid shadow camera: {msg}"),
            ShadowError::InvalidLightDirection => write!(f, "invalid light direction"),
            ShadowError::InvalidConfig(msg) => write!(f, "invalid shadow config: {msg}"),
        }
    }
}

impl std::error::Error for ShadowError {}

/// 每帧阴影数据 — 由 [`ShadowFrameData::from_camera`] 生成
///
/// `light_view_proj[i]` 是第 i 级 cascade 的 `proj * view`（column-major），
/// 未启用的级联填 0。`cascade_splits[i]` 是第 i 级 cascade 的远平面 **view 空间距离**。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowFrameData {
    /// 启用的级联数（1..=4）
    pub cascade_count: u32,
    /// 各级 cascade 远平面距离（view 空间）
    pub cascade_splits: [f32; MAX_CASCADES],
    /// 各级 cascade 的光源 view-projection 矩阵（column-major）
    pub light_view_proj: [[[f32; 4]; 4]; MAX_CASCADES],
    /// 各级 cascade 的纹素世界尺寸
    pub texel_world_size: [f32; MAX_CASCADES],
    /// 各级 cascade 包围球半径
    pub radius: [f32; MAX_CASCADES],
}

impl Default for ShadowFrameData {
    fn default() -> Self {
        Self {
            cascade_count: 0,
            cascade_splits: [0.0; MAX_CASCADES],
            light_view_proj: [[[0.0; 4]; 4]; MAX_CASCADES],
            texel_world_size: [0.0; MAX_CASCADES],
            radius: [0.0; MAX_CASCADES],
        }
    }
}

impl ShadowFrameData {
    /// 从相机与光线方向计算所有 cascade 的数据
    ///
    /// - `aspect` = 视口宽/高
    /// - `light_direction` = 光的传播方向（与 `DirectionalLightComponent.direction` 一致）
    /// - `config` 会先 [`ShadowConfig::validate`]
    pub fn from_camera(
        camera: &CameraParams,
        aspect: f32,
        light_direction: [f32; 3],
        config: ShadowConfig,
    ) -> Result<Self, ShadowError> {
        let config = config.validate();
        if !(aspect.is_finite() && aspect > 0.0) {
            return Err(ShadowError::InvalidCamera(format!("aspect={aspect}")));
        }
        if !(camera.near.is_finite() && camera.near > 0.0) {
            return Err(ShadowError::InvalidCamera(format!(
                "near={}",
                camera.near
            )));
        }
        if !(camera.far.is_finite() && camera.far > camera.near) {
            return Err(ShadowError::InvalidCamera(format!(
                "far={} (near={})",
                camera.far, camera.near
            )));
        }
        if !(camera.fov.is_finite() && camera.fov > 0.0 && camera.fov < std::f32::consts::PI) {
            return Err(ShadowError::InvalidCamera(format!("fov={}", camera.fov)));
        }
        let to_light = normalize3([
            -light_direction[0],
            -light_direction[1],
            -light_direction[2],
        ]);
        if to_light == [0.0, 0.0, 0.0] {
            return Err(ShadowError::InvalidLightDirection);
        }

        let near = camera.near;
        let far = camera.far.min(config.max_distance);
        if far <= near * 1.001 {
            return Err(ShadowError::InvalidConfig(format!(
                "阴影最大距离 {far} 小于相机近平面 {near}"
            )));
        }

        let splits = compute_cascade_splits(near, far, config.cascade_count, config.cascade_lambda)?;
        let (right, up, forward) = camera_basis(camera.yaw, camera.pitch);
        let eye = camera.position;

        let mut out = ShadowFrameData {
            cascade_count: config.cascade_count,
            ..ShadowFrameData::default()
        };

        let mut cascade_near = near;
        for i in 0..config.cascade_count as usize {
            let cascade_far = splits.splits[i];
            let corners =
                sub_frustum_corners(eye, right, up, forward, camera.fov, aspect, cascade_near, cascade_far);
            let (center, radius) = bounding_sphere(&corners);
            let radius = radius.max(1e-4);
            let texel = config.texel_world_size(radius);

            // 1. 先做不吸附的 look-at，取得光照空间基向量（用于纹素吸附）
            let up_axis = choose_up_axis(to_light);
            let view_unsnapped = look_at_rh(sub3(center, scale3(to_light, radius)), center, up_axis);
            let center_snapped = if config.stable {
                let right_axis = [view_unsnapped[0][0], view_unsnapped[1][0], view_unsnapped[2][0]];
                let up_axis_v = [view_unsnapped[0][1], view_unsnapped[1][1], view_unsnapped[2][1]];
                snap_center_to_texel(center, right_axis, up_axis_v, texel)
            } else {
                center
            };

            // 2. 吸附后的光源相机与正交投影
            let light_eye = sub3(center_snapped, scale3(to_light, radius));
            let view = look_at_rh(light_eye, center_snapped, up_axis);
            let proj = orthographic_rh_zo(-radius, radius, -radius, radius, 0.0, 2.0 * radius);
            out.light_view_proj[i] = mul4(&proj, &view);
            out.cascade_splits[i] = cascade_far;
            out.texel_world_size[i] = texel;
            out.radius[i] = radius;
            cascade_near = cascade_far;
        }

        // 未使用的级联沿用最后一级，避免 shader 读到 0 矩阵
        for i in config.cascade_count as usize..MAX_CASCADES {
            let last = (config.cascade_count as usize - 1).max(0);
            out.cascade_splits[i] = out.cascade_splits[last];
            out.light_view_proj[i] = out.light_view_proj[last];
            out.texel_world_size[i] = out.texel_world_size[last];
            out.radius[i] = out.radius[last];
        }

        Ok(out)
    }

    /// 根据 view 空间深度选择 cascade 索引
    ///
    /// 返回值 clamp 到 `0..cascade_count-1`（未启用阴影时返回 0）。
    pub fn select_cascade(&self, view_depth: f32) -> u32 {
        if self.cascade_count == 0 {
            return 0;
        }
        for i in 0..self.cascade_count as usize {
            if view_depth <= self.cascade_splits[i] {
                return i as u32;
            }
        }
        self.cascade_count - 1
    }

    /// 世界坐标 → cascade 阴影贴图坐标 `[u, v, depth]`
    ///
    /// 未启用该 cascade 时返回 None。
    pub fn shadow_uv_depth(&self, cascade: u32, world: [f32; 3]) -> Option<[f32; 3]> {
        if cascade >= self.cascade_count {
            return None;
        }
        let clip = transform_point4(&self.light_view_proj[cascade as usize], world);
        if clip[3].abs() < 1e-8 {
            return None;
        }
        let ndc = [clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]];
        // Vulkan: NDC xy ∈ [-1,1] → UV [0,1]；depth 已在 [0,1]
        Some([ndc[0] * 0.5 + 0.5, ndc[1] * 0.5 + 0.5, ndc[2]])
    }

    /// 生成 `pbr.frag` 的阴影参数 `vec4(depth_bias, normal_bias, texel_size, 0)`
    pub fn shadow_params(&self, config: &ShadowConfig) -> [f32; 4] {
        let texel = self.texel_world_size[0];
        [config.depth_bias, config.normal_bias, texel, 0.0]
    }

    /// 把阴影数据写入 PBR 每帧 UBO
    pub fn write_into_ubo(&self, config: &ShadowConfig, ubo: &mut PbrFrameUbo) {
        if self.cascade_count == 0 {
            ubo.set_shadow_data(0.0, [0.0; 4], [0.0; 4], [[[0.0; 4]; 4]; 4], 0);
            return;
        }
        ubo.set_shadow_data(
            1.0,
            self.cascade_splits,
            self.shadow_params(config),
            self.light_view_proj,
            self.cascade_count,
        );
    }
}

/// 计算 cascade 分割距离（view 空间远平面）
///
/// 混合公式: `split(i) = lambda * near*(far/near)^(i/n) + (1-lambda)*(near + (far-near)*i/n)`
/// - `lambda=0` → 均匀分割；`lambda=1` → 对数分割（近处更精细）
pub fn compute_cascade_splits(
    near: f32,
    far: f32,
    count: u32,
    lambda: f32,
) -> Result<CascadeSplits, ShadowError> {
    if !(near.is_finite() && near > 0.0) {
        return Err(ShadowError::InvalidCamera(format!("near={near}")));
    }
    if !(far.is_finite() && far > near) {
        return Err(ShadowError::InvalidCamera(format!(
            "far={far} (near={near})"
        )));
    }
    let count = count.clamp(1, MAX_CASCADES as u32);
    let lambda = if lambda.is_finite() {
        lambda.clamp(0.0, 1.0)
    } else {
        DEFAULT_CASCADE_LAMBDA
    };
    let n = count as f32;
    let mut splits = [far; MAX_CASCADES];
    for i in 1..=count as usize {
        let t = i as f32 / n;
        let log_split = near * (far / near).powf(t);
        let uniform_split = near + (far - near) * t;
        splits[i - 1] = lambda * log_split + (1.0 - lambda) * uniform_split;
    }
    Ok(CascadeSplits { count, splits })
}

/// cascade 分割结果
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CascadeSplits {
    /// 级联数量
    pub count: u32,
    /// 各级远平面距离（未使用项 = far）
    pub splits: [f32; MAX_CASCADES],
}

/// 相机基向量（与 `shaders/rotation.vert` 的 `viewMatrix()` 推导一致）
///
/// 返回 `(right, up, forward)`；`forward` 为相机观察方向（世界空间）。
pub fn camera_basis(yaw: f32, pitch: f32) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let cy = yaw.cos();
    let sy = yaw.sin();
    let cp = pitch.cos();
    let sp = pitch.sin();
    let forward = [sy * cp, -sp, -cy * cp];
    let right = [cy, 0.0, sy];
    let up = [sy * sp, cp, -cy * sp];
    (right, up, forward)
}

/// 子视锥 8 个角点（世界空间）
///
/// 顺序: near 4 点（左下、右下、右上、左上）后 far 4 点。
pub fn sub_frustum_corners(
    eye: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    forward: [f32; 3],
    fov: f32,
    aspect: f32,
    near: f32,
    far: f32,
) -> [[f32; 3]; 8] {
    let tan_half = (fov * 0.5).tan();
    let xs = tan_half * aspect;
    let ys = tan_half;
    let mut out = [[0.0f32; 3]; 8];
    let mut write = |idx_base: usize, depth: f32| {
        let center = add3(eye, scale3(forward, depth));
        let rx = scale3(right, xs * depth);
        let uy = scale3(up, ys * depth);
        out[idx_base] = add3(add3(center, rx), scale3(uy, -1.0));
        out[idx_base + 1] = add3(sub3(center, rx), scale3(uy, -1.0));
        out[idx_base + 2] = add3(sub3(center, rx), uy);
        out[idx_base + 3] = add3(add3(center, rx), uy);
    };
    write(0, near);
    write(4, far);
    out
}

/// 点集包围球 `(center, radius)`
pub fn bounding_sphere(points: &[[f32; 3]]) -> ([f32; 3], f32) {
    if points.is_empty() {
        return ([0.0; 3], 0.0);
    }
    let mut center = [0.0f32; 3];
    for p in points {
        center = add3(center, *p);
    }
    center = scale3(center, 1.0 / points.len() as f32);
    let mut radius = 0.0f32;
    for p in points {
        radius = radius.max(length3(sub3(*p, center)));
    }
    (center, radius)
}

/// 选择光源相机 up 轴（避免与光线方向共线导致退化）
pub fn choose_up_axis(to_light: [f32; 3]) -> [f32; 3] {
    let world_up = [0.0, 1.0, 0.0];
    if dot3(to_light, world_up).abs() > 0.99 {
        let x_axis = [1.0, 0.0, 0.0];
        if dot3(to_light, x_axis).abs() > 0.99 {
            [0.0, 0.0, 1.0]
        } else {
            x_axis
        }
    } else {
        world_up
    }
}

/// 把阴影相机中心吸附到纹素网格（减少相机移动时的阴影抖动）
pub fn snap_center_to_texel(
    center: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    texel: f32,
) -> [f32; 3] {
    if !(texel.is_finite() && texel > 0.0) {
        return center;
    }
    let cx = dot3(right, center);
    let cy = dot3(up, center);
    let snapped_x = (cx / texel).round() * texel;
    let snapped_y = (cy / texel).round() * texel;
    add3(center, add3(scale3(right, snapped_x - cx), scale3(up, snapped_y - cy)))
}

/// 右手系 look-at 视图矩阵（column-major，`data[col][row]`）
///
/// 与 `shaders/rotation.vert` 的视图矩阵语义一致；矩阵乘法请配套使用 [`mul4`]。
pub fn look_at_rh(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let forward = normalize3(sub3(target, eye));
    let right = normalize3(cross3(forward, up));
    let true_up = cross3(right, forward);
    [
        [right[0], true_up[0], -forward[0], 0.0],
        [right[1], true_up[1], -forward[1], 0.0],
        [right[2], true_up[2], -forward[2], 0.0],
        [
            -dot3(right, eye),
            -dot3(true_up, eye),
            dot3(forward, eye),
            1.0,
        ],
    ]
}

/// 4x4 矩阵乘法（column-major: `data[col][row]`）
///
/// 语义与 GLSL `mat4 * mat4` 一致；不要用 `hezhou_core::Mat4::multiply`
/// （其存储约定与 GPU/GLSL 的 column-major 不一致）。
pub fn mul4(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0f32; 4]; 4];
    for col in 0..4 {
        for row in 0..4 {
            out[col][row] = a[0][row] * b[col][0]
                + a[1][row] * b[col][1]
                + a[2][row] * b[col][2]
                + a[3][row] * b[col][3];
        }
    }
    out
}

/// 右手系正交投影，深度映射到 Vulkan 的 `[0, 1]`（RH_ZO）
pub fn orthographic_rh_zo(
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    near: f32,
    far: f32,
) -> [[f32; 4]; 4] {
    let rl = (right - left).max(1e-6);
    let tb = (top - bottom).max(1e-6);
    // 注意: far > near，n/(n-f) 与 1/(n-f) 使用带符号的 (near - far)，保证深度落在 [0,1]
    let signed_nf = (near - far).min(-1e-6);
    [
        [2.0 / rl, 0.0, 0.0, 0.0],
        [0.0, 2.0 / tb, 0.0, 0.0],
        [0.0, 0.0, 1.0 / signed_nf, 0.0],
        [
            -(right + left) / rl,
            -(top + bottom) / tb,
            near / signed_nf,
            1.0,
        ],
    ]
}

/// 4x4 矩阵 * 齐次点（column-major）
pub fn transform_point4(m: &[[f32; 4]; 4], p: [f32; 3]) -> [f32; 4] {
    [
        m[0][0] * p[0] + m[1][0] * p[1] + m[2][0] * p[2] + m[3][0],
        m[0][1] * p[0] + m[1][1] * p[1] + m[2][1] * p[2] + m[3][1],
        m[0][2] * p[0] + m[1][2] * p[1] + m[2][2] * p[2] + m[3][2],
        m[0][3] * p[0] + m[1][3] * p[1] + m[2][3] * p[2] + m[3][3],
    ]
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale3(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn length3(a: [f32; 3]) -> f32 {
    dot3(a, a).sqrt()
}

fn normalize3(a: [f32; 3]) -> [f32; 3] {
    let len = length3(a);
    if len > 1e-8 {
        scale3(a, 1.0 / len)
    } else {
        [0.0, 0.0, 0.0]
    }
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_camera() -> CameraParams {
        CameraParams {
            position: [0.0, 2.0, 8.0],
            yaw: 0.0,
            pitch: -0.2,
            fov: 60.0_f32.to_radians(),
            near: 0.1,
            far: 100.0,
        }
    }

    #[test]
    fn splits_are_monotonic_and_bounded() {
        let splits = compute_cascade_splits(0.1, 100.0, 3, DEFAULT_CASCADE_LAMBDA).unwrap();
        assert_eq!(splits.count, 3);
        assert!(splits.splits[0] > 0.1 && splits.splits[0] < 100.0);
        for i in 1..3 {
            assert!(
                splits.splits[i] > splits.splits[i - 1],
                "分割距离必须递增: {:?}",
                splits.splits
            );
        }
        assert!((splits.splits[2] - 100.0).abs() < 1e-3);
    }

    #[test]
    fn log_split_is_smaller_near_camera_than_uniform() {
        let uniform = compute_cascade_splits(0.1, 100.0, 3, 0.0).unwrap();
        let log = compute_cascade_splits(0.1, 100.0, 3, 1.0).unwrap();
        // 对数分割在第一级更靠近相机（更精细）
        assert!(log.splits[0] < uniform.splits[0]);
        // 最后一级都等于远平面
        assert!((log.splits[2] - uniform.splits[2]).abs() < 1e-3);
    }

    #[test]
    fn splits_single_cascade_covers_full_range() {
        let splits = compute_cascade_splits(1.0, 50.0, 1, 0.5).unwrap();
        assert_eq!(splits.count, 1);
        assert!((splits.splits[0] - 50.0).abs() < 1e-3);
    }

    #[test]
    fn splits_clamp_cascade_count() {
        let splits = compute_cascade_splits(0.1, 100.0, 0, 0.5).unwrap();
        assert_eq!(splits.count, 1);
        let splits = compute_cascade_splits(0.1, 100.0, 99, 0.5).unwrap();
        assert_eq!(splits.count, MAX_CASCADES as u32);
    }

    #[test]
    fn splits_reject_invalid_camera_range() {
        assert!(compute_cascade_splits(0.0, 100.0, 2, 0.5).is_err());
        assert!(compute_cascade_splits(10.0, 5.0, 2, 0.5).is_err());
        assert!(compute_cascade_splits(0.1, f32::NAN, 2, 0.5).is_err());
    }

    #[test]
    fn frame_data_covers_frustum_corners() {
        let camera = test_camera();
        let aspect = 16.0 / 9.0;
        let config = ShadowConfig {
            cascade_count: 3,
            resolution: 1024,
            ..ShadowConfig::default()
        };
        let frame = ShadowFrameData::from_camera(&camera, aspect, [-0.4, -1.0, -0.3], config).unwrap();
        assert_eq!(frame.cascade_count, 3);
        assert_eq!(frame.light_view_proj.len(), MAX_CASCADES);

        // 用同一套参数复算视锥角点，验证全部落在 cascade 阴影贴图内
        let (right, up, forward) = camera_basis(camera.yaw, camera.pitch);
        let mut cascade_near = camera.near;
        for i in 0..frame.cascade_count as usize {
            let cascade_far = frame.cascade_splits[i];
            let corners = sub_frustum_corners(
                camera.position,
                right,
                up,
                forward,
                camera.fov,
                aspect,
                cascade_near,
                cascade_far,
            );
            for c in corners {
                let uvz = frame.shadow_uv_depth(i as u32, c).expect("cascade 有效");
                assert!(
                    (-1e-3..=1.0 + 1e-3).contains(&uvz[0])
                        && (-1e-3..=1.0 + 1e-3).contains(&uvz[1]),
                    "角点超出阴影贴图 UV: {uvz:?}"
                );
                assert!(
                    (-1e-3..=1.0 + 1e-3).contains(&uvz[2]),
                    "角点深度超出 [0,1]: {uvz:?}"
                );
            }
            cascade_near = cascade_far;
        }
    }

    #[test]
    fn frame_center_maps_to_shadow_map_center() {
        let camera = test_camera();
        let config = ShadowConfig {
            cascade_count: 1,
            resolution: 1024,
            stable: false,
            ..ShadowConfig::default()
        };
        let frame = ShadowFrameData::from_camera(&camera, 16.0 / 9.0, [0.0, -1.0, 0.0], config).unwrap();
        // 相机近平面中心应落在阴影贴图中心附近（深度在 0..1 内）
        let near_center = [
            camera.position[0] + camera_basis(camera.yaw, camera.pitch).2[0] * camera.near,
            camera.position[1] + camera_basis(camera.yaw, camera.pitch).2[1] * camera.near,
            camera.position[2] + camera_basis(camera.yaw, camera.pitch).2[2] * camera.near,
        ];
        let uvz = frame.shadow_uv_depth(0, near_center).unwrap();
        assert!(uvz[0].is_finite() && uvz[1].is_finite() && uvz[2].is_finite());
        assert!((-1e-3..=1.0 + 1e-3).contains(&uvz[2]));
    }

    #[test]
    fn frame_handles_light_parallel_to_world_up() {
        let camera = test_camera();
        let frame = ShadowFrameData::from_camera(
            &camera,
            16.0 / 9.0,
            [0.0, -1.0, 0.0],
            ShadowConfig::default(),
        )
        .expect("垂直向下光应可计算阴影矩阵");
        for i in 0..MAX_CASCADES {
            assert!(frame.light_view_proj[i]
                .iter()
                .flatten()
                .all(|v| v.is_finite()));
        }
    }

    #[test]
    fn frame_rejects_zero_light_direction() {
        let camera = test_camera();
        let err =
            ShadowFrameData::from_camera(&camera, 1.0, [0.0, 0.0, 0.0], ShadowConfig::default())
                .unwrap_err();
        assert_eq!(err, ShadowError::InvalidLightDirection);
    }

    #[test]
    fn frame_rejects_invalid_camera() {
        let mut camera = test_camera();
        camera.near = 0.0;
        assert!(ShadowFrameData::from_camera(&camera, 1.0, [0.0, -1.0, 0.0], ShadowConfig::default()).is_err());
        let mut camera = test_camera();
        camera.far = camera.near;
        assert!(ShadowFrameData::from_camera(&camera, 1.0, [0.0, -1.0, 0.0], ShadowConfig::default()).is_err());
        assert!(ShadowFrameData::from_camera(&test_camera(), 0.0, [0.0, -1.0, 0.0], ShadowConfig::default()).is_err());
    }

    #[test]
    fn texel_snapping_is_stable_under_small_camera_motion() {
        let mut camera_a = test_camera();
        let mut camera_b = test_camera();
        // 沿世界 X 轴移动 0.001（远小于纹素尺寸）
        camera_a.position[0] = 0.0;
        camera_b.position[0] = 0.001;
        let config = ShadowConfig {
            cascade_count: 2,
            resolution: 512,
            stable: true,
            ..ShadowConfig::default()
        };
        let a = ShadowFrameData::from_camera(&camera_a, 16.0 / 9.0, [-0.3, -1.0, -0.2], config).unwrap();
        let b = ShadowFrameData::from_camera(&camera_b, 16.0 / 9.0, [-0.3, -1.0, -0.2], config).unwrap();
        // 纹素吸附后，两者光源视图矩阵的平移列应几乎一致（差异 < 1 个纹素对应世界尺寸）
        for i in 0..config.cascade_count as usize {
            let ma = a.light_view_proj[i];
            let mb = b.light_view_proj[i];
            for col in 0..3 {
                for row in 0..3 {
                    assert!((ma[col][row] - mb[col][row]).abs() < 1e-3);
                }
            }
        }
    }

    #[test]
    fn select_cascade_respects_split_boundaries() {
        let camera = test_camera();
        let frame = ShadowFrameData::from_camera(
            &camera,
            16.0 / 9.0,
            [-0.3, -1.0, -0.2],
            ShadowConfig {
                cascade_count: 3,
                ..ShadowConfig::default()
            },
        )
        .unwrap();
        assert_eq!(frame.select_cascade(0.0), 0);
        assert_eq!(frame.select_cascade(frame.cascade_splits[0] - 0.01), 0);
        assert_eq!(frame.select_cascade(frame.cascade_splits[0] + 0.01), 1);
        assert_eq!(frame.select_cascade(frame.cascade_splits[1] + 0.01), 2);
        assert_eq!(frame.select_cascade(1e9), 2);
    }

    #[test]
    fn shadow_params_and_ubo_write() {
        let camera = test_camera();
        let config = ShadowConfig {
            cascade_count: 2,
            resolution: 1024,
            depth_bias: 0.002,
            normal_bias: 0.03,
            ..ShadowConfig::default()
        };
        let frame =
            ShadowFrameData::from_camera(&camera, 16.0 / 9.0, [-0.3, -1.0, -0.2], config).unwrap();
        let params = frame.shadow_params(&config);
        assert!((params[0] - 0.002).abs() < 1e-6);
        assert!((params[1] - 0.03).abs() < 1e-6);
        assert!(params[2] > 0.0);

        let mut ubo = PbrFrameUbo::default();
        frame.write_into_ubo(&config, &mut ubo);
        assert_eq!(ubo.cascade_count, 2.0);
        assert!((ubo.shadow_strength - 1.0).abs() < 1e-6);
        assert_eq!(ubo.light_view_proj[0], frame.light_view_proj[0]);
    }

    #[test]
    fn disabled_frame_writes_zero_shadow() {
        let mut ubo = PbrFrameUbo::default();
        ShadowFrameData::default().write_into_ubo(&ShadowConfig::default(), &mut ubo);
        assert_eq!(ubo.cascade_count, 0.0);
        assert_eq!(ubo.shadow_strength, 0.0);
    }

    #[test]
    fn config_validate_clamps_values() {
        let cfg = ShadowConfig {
            cascade_count: 0,
            resolution: 100,
            depth_bias: -1.0,
            normal_bias: 9.0,
            cascade_lambda: 5.0,
            max_distance: 0.0,
            stable: true,
        }
        .validate();
        assert_eq!(cfg.cascade_count, 1);
        assert_eq!(cfg.resolution, crate::quality::MIN_SHADOW_RESOLUTION);
        assert_eq!(cfg.depth_bias, 0.0);
        assert_eq!(cfg.normal_bias, 1.0);
        assert_eq!(cfg.cascade_lambda, 1.0);
        assert!((cfg.max_distance - 0.1).abs() < 1e-6);
    }

    #[test]
    fn shadow_settings_validate_replaces_nan() {
        let s = ShadowSettings {
            depth_bias: f32::NAN,
            normal_bias: f32::NAN,
            cascade_lambda: f32::NAN,
            max_distance: f32::NAN,
            ..ShadowSettings::default()
        }
        .validate();
        assert!((s.depth_bias - DEFAULT_DEPTH_BIAS).abs() < 1e-6);
        assert!((s.normal_bias - DEFAULT_NORMAL_BIAS).abs() < 1e-6);
        assert!((s.cascade_lambda - DEFAULT_CASCADE_LAMBDA).abs() < 1e-6);
        assert!((s.max_distance - DEFAULT_MAX_DISTANCE).abs() < 1e-6);
    }

    #[test]
    fn ortho_depth_range_is_zero_to_one() {
        let m = orthographic_rh_zo(-1.0, 1.0, -1.0, 1.0, 1.0, 11.0);
        let near_point = transform_point4(&m, [0.0, 0.0, -1.0]); // view depth 1
        let far_point = transform_point4(&m, [0.0, 0.0, -11.0]); // view depth 11
        assert!((near_point[2] / near_point[3]).abs() < 1e-5);
        assert!((far_point[2] / far_point[3] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn camera_basis_is_orthonormal() {
        for (yaw, pitch) in [(0.0, 0.0), (0.3, -0.4), (-1.2, 0.9)] {
            let (right, up, forward) = camera_basis(yaw, pitch);
            assert!((dot3(right, right) - 1.0).abs() < 1e-5);
            assert!((dot3(up, up) - 1.0).abs() < 1e-5);
            assert!((dot3(forward, forward) - 1.0).abs() < 1e-5);
            assert!(dot3(right, forward).abs() < 1e-5);
            assert!(dot3(up, forward).abs() < 1e-5);
            assert!(dot3(right, up).abs() < 1e-5);
        }
    }

    #[test]
    fn shadow_uv_depth_rejects_invalid_cascade() {
        let camera = test_camera();
        let frame = ShadowFrameData::from_camera(
            &camera,
            1.0,
            [0.0, -1.0, 0.0],
            ShadowConfig {
                cascade_count: 1,
                ..ShadowConfig::default()
            },
        )
        .unwrap();
        assert!(frame.shadow_uv_depth(1, [0.0; 3]).is_none());
    }
}
