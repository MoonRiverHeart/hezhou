#version 450

// PBR 前向着色 — 片元阶段（0.2）
// 输出线性 HDR 颜色，Tonemap/Gamma 由 post_tonemap.frag 负责。
// 契约常量（与 Rust 端 render-pipeline::pbr / post / shadow 一致）:
#define MAX_CASCADES 4
#define PBR_PI 3.14159265
#define DIELECTRIC_F0 0.04
#define MIN_ROUGHNESS 0.045
#define SHADOW_PCF_RADIUS 1

layout(location = 0) in vec3 inWorldPos;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inUV;

layout(location = 0) out vec4 out_color;

// push constant: PbrPushConstant (96 bytes)
layout(push_constant) uniform PbrPushConstants {
    mat4 model;
    vec4 base_color;
    vec4 material_params;
} pc;

// 每帧 UBO: PbrFrameUbo (480 bytes, std140)
layout(std140, set = 0, binding = 0) uniform PbrFrameUbo {
    vec3 direction;
    float intensity;
    vec3 color;
    float shadow_strength;
    vec3 ambient;
    float cascade_count;
    vec4 cascade_splits;
    vec4 shadow_params;
    mat4 view;
    mat4 proj;
    mat4 light_view_proj[4];
    vec4 camera_pos;
} frame;

// 阴影贴图: 2D 数组深度图（每级 cascade 一层），硬件比较采样 → 3x3 PCF
layout(set = 0, binding = 1) uniform sampler2DArrayShadow shadow_map;

// 材质贴图（set 1）: albedo / normal / metallic-roughness(G=roughness,B=metallic) / AO
layout(set = 1, binding = 0) uniform sampler2D albedo_map;
layout(set = 1, binding = 1) uniform sampler2D normal_map;
layout(set = 1, binding = 2) uniform sampler2D metallic_roughness_map;
layout(set = 1, binding = 3) uniform sampler2D ao_map;

// === PBR BRDF 函数（与 CPU 参考实现 pbr.rs 对齐） ===

float distribution_ggx(float ndoth, float roughness) {
    float r = max(roughness, MIN_ROUGHNESS);
    float a = r * r;
    float a2 = a * a;
    float d = ndoth * ndoth * (a2 - 1.0) + 1.0;
    return a2 / (PBR_PI * d * d);
}

float geometry_schlick_ggx(float ndotx, float roughness) {
    float r = max(roughness, MIN_ROUGHNESS);
    float k = (r + 1.0) * (r + 1.0) / 8.0;
    return ndotx / max(ndotx * (1.0 - k) + k, 1e-6);
}

float geometry_smith(float ndotv, float ndotl, float roughness) {
    return geometry_schlick_ggx(ndotv, roughness) * geometry_schlick_ggx(ndotl, roughness);
}

vec3 fresnel_schlick(float cos_theta, vec3 f0) {
    float c = clamp(cos_theta, 0.0, 1.0);
    return f0 + (vec3(1.0) - f0) * pow(1.0 - c, 5.0);
}

// 由屏幕空间导数构造切线空间（0.2 无顶点切线，最小实现）
mat3 cotangent_frame(vec3 n, vec3 p, vec2 uv) {
    vec3 dp1 = dFdx(p);
    vec3 dp2 = dFdy(p);
    vec2 duv1 = dFdx(uv);
    vec2 duv2 = dFdy(uv);
    vec3 dp2perp = cross(dp2, n);
    vec3 dp1perp = cross(n, dp1);
    vec3 t = dp2perp * duv1.x + dp1perp * duv2.x;
    vec3 b = dp2perp * duv1.y + dp1perp * duv2.y;
    float inv_max = inversesqrt(max(dot(t, t), dot(b, b)));
    return mat3(t * inv_max, b * inv_max, n);
}

// === 阴影采样 ===

int select_cascade(float view_depth) {
    for (int i = 0; i < MAX_CASCADES; ++i) {
        if (float(i) >= frame.cascade_count) {
            break;
        }
        if (view_depth <= frame.cascade_splits[i]) {
            return i;
        }
    }
    return int(max(frame.cascade_count - 1.0, 0.0));
}

float sample_shadow(vec3 world_pos, vec3 normal) {
    if (frame.cascade_count < 0.5 || frame.shadow_strength < 0.001) {
        return 1.0;
    }
    float view_depth = -(frame.view * vec4(world_pos, 1.0)).z;
    int cascade = select_cascade(view_depth);

    // 法线偏移抑制阴影痤疮
    vec4 clip = frame.light_view_proj[cascade] * vec4(world_pos + normal * frame.shadow_params.y, 1.0);
    vec3 ndc = clip.xyz / clip.w;
    vec2 uv = ndc.xy * 0.5 + 0.5;
    float depth = ndc.z - frame.shadow_params.x;

    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || depth > 1.0) {
        return 1.0;
    }

    vec3 size = vec3(textureSize(shadow_map, 0));
    vec2 texel = 1.0 / size.xy;
    float lit = 0.0;
    for (int y = -SHADOW_PCF_RADIUS; y <= SHADOW_PCF_RADIUS; ++y) {
        for (int x = -SHADOW_PCF_RADIUS; x <= SHADOW_PCF_RADIUS; ++x) {
            vec2 offset = vec2(float(x), float(y)) * texel;
            lit += texture(shadow_map, vec4(uv + offset, float(cascade), depth));
        }
    }
    float pcf = float((SHADOW_PCF_RADIUS * 2 + 1) * (SHADOW_PCF_RADIUS * 2 + 1));
    return mix(1.0, lit / pcf, clamp(frame.shadow_strength, 0.0, 1.0));
}

void main() {
    // === 材质解包 ===
    vec3 albedo = pc.base_color.rgb * texture(albedo_map, inUV).rgb;
    vec4 mr = texture(metallic_roughness_map, inUV);
    float metallic = clamp(pc.material_params.x * mr.b, 0.0, 1.0);
    float roughness = clamp(pc.material_params.y * mr.g, MIN_ROUGHNESS, 1.0);
    float ao = clamp(pc.material_params.w * texture(ao_map, inUV).r, 0.0, 1.0);

    // 法线贴图（0.2 使用导数构造 TBN）
    vec3 n_geom = normalize(inNormal);
    vec3 n_tangent = texture(normal_map, inUV).xyz * 2.0 - 1.0;
    n_tangent.xy *= pc.material_params.z;
    vec3 n = normalize(cotangent_frame(n_geom, inWorldPos, inUV) * n_tangent);

    // === 方向光直接光照（Cook-Torrance） ===
    vec3 v = normalize(frame.camera_pos.xyz - inWorldPos);
    vec3 l = normalize(-frame.direction);
    float ndotl = max(dot(n, l), 0.0);
    float ndotv = max(dot(n, v), 1e-4);

    vec3 f0 = mix(vec3(DIELECTRIC_F0), albedo, metallic);
    vec3 h = normalize(l + v);
    vec3 f = fresnel_schlick(max(dot(h, v), 0.0), f0);
    float d = distribution_ggx(max(dot(n, h), 0.0), roughness);
    float g = geometry_smith(ndotv, ndotl, roughness);
    vec3 specular = (d * g * f) / max(4.0 * ndotv * ndotl, 1e-4);

    vec3 kd = (vec3(1.0) - f) * (1.0 - metallic);
    vec3 radiance = frame.color * frame.intensity;
    float shadow = sample_shadow(inWorldPos, n_geom);
    vec3 direct = (kd * albedo / PBR_PI + specular) * radiance * ndotl * shadow;

    // === 环境项（0.2 无 IBL，仅常量环境光 + AO） ===
    vec3 ambient = frame.ambient * albedo * ao;

    out_color = vec4(direct + ambient, pc.base_color.a);
}
