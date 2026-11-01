//! Shader 契约测试 — 校验 GLSL 布局与 Rust 结构体/常量严格一致
//!
//! 覆盖:
//! - `shaders/pbr.vert` 的 push constant（PbrPushConstant, 96B）与 UBO（PbrFrameUbo, 480B）
//! - `shaders/shadow_depth.vert` 的 push constant（ShadowPushConstant, 128B）
//! - `shaders/post_tonemap.frag` / `shaders/bloom_prefilter.frag` 的 push constant（16B）
//! - 契约常量（MAX_CASCADES / PBR_PI / DIELECTRIC_F0 / MIN_ROUGHNESS / TONEMAP_*）
//! - `.spv` 产物存在且为合法 SPIR-V
//! - 若本机有 `glslc`（Vulkan SDK），额外做一次真实编译验证；无 glslc 时跳过

use std::mem::{offset_of, size_of};

use hezhou_render_pipeline::pbr::{
    PbrFrameUbo, PbrPushConstant, DIELECTRIC_F0, MIN_ROUGHNESS, PBR_PI,
};
use hezhou_render_pipeline::post::{
    BloomPrefilterPushConstant, PostTonemapPushConstant, TonemapOperator,
};
use hezhou_render_pipeline::shadow::{ShadowPushConstant, MAX_CASCADES};

// === 编译期校验 .spv 产物存在 ===
const PBR_VERT_SPV: &[u8] = include_bytes!("../../shaders/pbr.vert.spv");
const PBR_FRAG_SPV: &[u8] = include_bytes!("../../shaders/pbr.frag.spv");
const SHADOW_VERT_SPV: &[u8] = include_bytes!("../../shaders/shadow_depth.vert.spv");
const SHADOW_FRAG_SPV: &[u8] = include_bytes!("../../shaders/shadow_depth.frag.spv");
const POST_VERT_SPV: &[u8] = include_bytes!("../../shaders/post.vert.spv");
const POST_TONEMAP_SPV: &[u8] = include_bytes!("../../shaders/post_tonemap.frag.spv");
const BLOOM_PREFILTER_SPV: &[u8] = include_bytes!("../../shaders/bloom_prefilter.frag.spv");

const PBR_VERT_SRC: &str = include_str!("../../shaders/pbr.vert");
const PBR_FRAG_SRC: &str = include_str!("../../shaders/pbr.frag");
const SHADOW_VERT_SRC: &str = include_str!("../../shaders/shadow_depth.vert");
const POST_TONEMAP_SRC: &str = include_str!("../../shaders/post_tonemap.frag");
const BLOOM_PREFILTER_SRC: &str = include_str!("../../shaders/bloom_prefilter.frag");

/// 解析出的 GLSL 字段布局
#[derive(Debug, Clone)]
struct FieldLayout {
    name: String,
    offset: usize,
    size: usize,
}

/// 类型 → (对齐, 大小)；只覆盖本工程契约用到的类型
fn type_layout(ty: &str) -> (usize, usize) {
    match ty {
        "float" => (4, 4),
        "vec2" => (8, 8),
        "vec3" => (16, 12),
        "vec4" => (16, 16),
        "mat4" => (16, 64),
        other => panic!("契约解析未覆盖的 GLSL 类型: {other}"),
    }
}

fn align_up(value: usize, align: usize) -> usize {
    ((value + align - 1) / align) * align
}

/// 提取 `uniform <BlockName> { ... }` 块并按 std140/std430 计算字段偏移
///
/// 约定: 每个字段独占一行（与 shaders/*.glsl 的写法一致）。
fn parse_block(source: &str, block_name: &str) -> Vec<FieldLayout> {
    let marker = format!("uniform {block_name}");
    let mut lines = source.lines();
    let mut found = false;
    for line in lines.by_ref() {
        if line.contains(&marker) && line.contains('{') {
            found = true;
            break;
        }
    }
    assert!(found, "未在 shader 中找到 uniform {block_name} 块");

    let mut fields = Vec::new();
    let mut cursor = 0usize;
    for line in lines {
        let line = line.split("//").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('}') {
            break;
        }
        if line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let ty = parts.next().expect("字段类型");
        let raw_name = parts.next().unwrap_or_else(|| panic!("字段名缺失: {line}"));
        let raw_name = raw_name.trim_end_matches(';');
        let (name, array_len) = match raw_name.split_once('[') {
            Some((n, rest)) => {
                let count = rest.trim_end_matches(']').parse::<usize>().expect("数组长度");
                (n.to_string(), Some(count))
            }
            None => (raw_name.to_string(), None),
        };
        let (align, size) = type_layout(ty);
        let offset = align_up(cursor, align);
        let (size, stride) = match array_len {
            // std140: 数组元素步长向上取整到 16 字节
            Some(_) => (size, align_up(size, 16)),
            None => (size, size),
        };
        fields.push(FieldLayout {
            name,
            offset,
            size,
        });
        cursor = offset + stride * array_len.unwrap_or(1);
    }
    fields
}

fn parsed_size(fields: &[FieldLayout]) -> usize {
    let last = fields.last().expect("至少一个字段");
    align_up(last.offset + last.size, 16)
}

fn assert_field_names(fields: &[FieldLayout], expected: &[&str]) {
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names, expected,
        "GLSL 字段顺序变化，必须同步 Rust 结构体"
    );
}

fn assert_offsets(fields: &[FieldLayout], expected: &[(&str, usize)]) {
    for (field, (name, expected_offset)) in fields.iter().zip(expected) {
        assert_eq!(field.name, *name, "字段名不一致");
        assert_eq!(
            field.offset, *expected_offset,
            "字段 {} 的 GLSL 偏移 {} != Rust 偏移 {}",
            name, field.offset, expected_offset
        );
    }
}

/// 解析 `#define NAME value` 数值
fn parse_define_f32(source: &str, name: &str) -> f32 {
    let prefix = format!("#define {name} ");
    for line in source.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(&prefix) {
            let token = rest.split_whitespace().next().expect("define 值");
            return token.parse::<f32>().unwrap_or_else(|_| {
                panic!("define {name} 的值不是数值: {token}")
            });
        }
    }
    panic!("未找到 #define {name}");
}

// === 测试 ===

#[test]
fn pbr_push_constant_matches_shader() {
    let fields = parse_block(PBR_VERT_SRC, "PbrPushConstants");
    assert_field_names(&fields, &["model", "base_color", "material_params"]);
    assert_offsets(
        &fields,
        &[
            ("model", offset_of!(PbrPushConstant, model)),
            ("base_color", offset_of!(PbrPushConstant, base_color)),
            ("material_params", offset_of!(PbrPushConstant, material_params)),
        ],
    );
    assert_eq!(parsed_size(&fields), size_of::<PbrPushConstant>());
    assert_eq!(size_of::<PbrPushConstant>(), 96);
}

#[test]
fn pbr_frame_ubo_matches_shader() {
    let fields = parse_block(PBR_VERT_SRC, "PbrFrameUbo");
    assert_field_names(
        &fields,
        &[
            "direction",
            "intensity",
            "color",
            "shadow_strength",
            "ambient",
            "cascade_count",
            "cascade_splits",
            "shadow_params",
            "view",
            "proj",
            "light_view_proj",
            "camera_pos",
        ],
    );
    assert_offsets(
        &fields,
        &[
            ("direction", offset_of!(PbrFrameUbo, direction)),
            ("intensity", offset_of!(PbrFrameUbo, intensity)),
            ("color", offset_of!(PbrFrameUbo, color)),
            ("shadow_strength", offset_of!(PbrFrameUbo, shadow_strength)),
            ("ambient", offset_of!(PbrFrameUbo, ambient)),
            ("cascade_count", offset_of!(PbrFrameUbo, cascade_count)),
            ("cascade_splits", offset_of!(PbrFrameUbo, cascade_splits)),
            ("shadow_params", offset_of!(PbrFrameUbo, shadow_params)),
            ("view", offset_of!(PbrFrameUbo, view)),
            ("proj", offset_of!(PbrFrameUbo, proj)),
            ("light_view_proj", offset_of!(PbrFrameUbo, light_view_proj)),
            ("camera_pos", offset_of!(PbrFrameUbo, camera_pos)),
        ],
    );
    assert_eq!(parsed_size(&fields), size_of::<PbrFrameUbo>());
    assert_eq!(size_of::<PbrFrameUbo>(), 480);
    // 顶点与片元 shader 的 UBO 块必须一致
    let frag_fields = parse_block(PBR_FRAG_SRC, "PbrFrameUbo");
    assert_field_names(
        &frag_fields,
        &fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
    );
    assert_eq!(parsed_size(&frag_fields), parsed_size(&fields));
}

#[test]
fn shadow_push_constant_matches_shader() {
    let fields = parse_block(SHADOW_VERT_SRC, "ShadowPushConstants");
    assert_field_names(&fields, &["light_view_proj", "model"]);
    assert_offsets(
        &fields,
        &[
            ("light_view_proj", offset_of!(ShadowPushConstant, light_view_proj)),
            ("model", offset_of!(ShadowPushConstant, model)),
        ],
    );
    assert_eq!(parsed_size(&fields), size_of::<ShadowPushConstant>());
    assert_eq!(size_of::<ShadowPushConstant>(), 128);
}

#[test]
fn post_tonemap_push_constant_matches_shader() {
    let fields = parse_block(POST_TONEMAP_SRC, "PostTonemapPushConstants");
    assert_field_names(&fields, &["params"]);
    assert_offsets(
        &fields,
        &[("params", offset_of!(PostTonemapPushConstant, params))],
    );
    assert_eq!(parsed_size(&fields), size_of::<PostTonemapPushConstant>());
    assert_eq!(size_of::<PostTonemapPushConstant>(), 16);
}

#[test]
fn bloom_prefilter_push_constant_matches_shader() {
    let fields = parse_block(BLOOM_PREFILTER_SRC, "BloomPrefilterPushConstants");
    assert_field_names(&fields, &["params"]);
    assert_offsets(
        &fields,
        &[("params", offset_of!(BloomPrefilterPushConstant, params))],
    );
    assert_eq!(parsed_size(&fields), size_of::<BloomPrefilterPushConstant>());
    assert_eq!(size_of::<BloomPrefilterPushConstant>(), 16);
}

#[test]
fn shader_define_constants_match_rust() {
    assert_eq!(
        parse_define_f32(PBR_FRAG_SRC, "MAX_CASCADES") as usize,
        MAX_CASCADES
    );
    assert!((parse_define_f32(PBR_FRAG_SRC, "PBR_PI") - PBR_PI).abs() < 1e-5);
    assert!((parse_define_f32(PBR_FRAG_SRC, "DIELECTRIC_F0") - DIELECTRIC_F0).abs() < 1e-6);
    assert!((parse_define_f32(PBR_FRAG_SRC, "MIN_ROUGHNESS") - MIN_ROUGHNESS).abs() < 1e-6);

    assert_eq!(
        parse_define_f32(POST_TONEMAP_SRC, "TONEMAP_NONE") as u32,
        TonemapOperator::None.as_u32()
    );
    assert_eq!(
        parse_define_f32(POST_TONEMAP_SRC, "TONEMAP_REINHARD") as u32,
        TonemapOperator::Reinhard.as_u32()
    );
    assert_eq!(
        parse_define_f32(POST_TONEMAP_SRC, "TONEMAP_ACES") as u32,
        TonemapOperator::Aces.as_u32()
    );
    assert_eq!(
        parse_define_f32(POST_TONEMAP_SRC, "TONEMAP_FILMIC") as u32,
        TonemapOperator::Filmic.as_u32()
    );
}

#[test]
fn shader_bindings_are_documented() {
    // 阴影贴图使用 sampler2DArrayShadow + light_view_proj[4] 级联
    assert!(PBR_FRAG_SRC.contains("sampler2DArrayShadow"));
    assert!(PBR_FRAG_SRC.contains("light_view_proj[cascade]"));
    assert!(PBR_FRAG_SRC.contains("set = 0, binding = 1"));
    // 材质贴图 set 1 binding 0..3
    for binding in 0..4 {
        assert!(
            PBR_FRAG_SRC.contains(&format!("set = 1, binding = {binding}")),
            "缺少材质贴图 binding {binding}"
        );
    }
    // 后处理是 HDR 输入 + 输出 LDR
    assert!(POST_TONEMAP_SRC.contains("hdr_scene"));
    assert!(POST_TONEMAP_SRC.contains("clamp(mapped, 0.0, 1.0)"));
}

#[test]
fn spv_products_are_valid_spirv() {
    for (name, bytes) in [
        ("pbr.vert.spv", PBR_VERT_SPV),
        ("pbr.frag.spv", PBR_FRAG_SPV),
        ("shadow_depth.vert.spv", SHADOW_VERT_SPV),
        ("shadow_depth.frag.spv", SHADOW_FRAG_SPV),
        ("post.vert.spv", POST_VERT_SPV),
        ("post_tonemap.frag.spv", POST_TONEMAP_SPV),
        ("bloom_prefilter.frag.spv", BLOOM_PREFILTER_SPV),
    ] {
        assert!(bytes.len() >= 20, "{name} 过短，SPIR-V 可能不完整");
        // SPIR-V magic number 0x07230203（小端字节序）
        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(magic, 0x0723_0203, "{name} 不是合法 SPIR-V");
    }
}

#[test]
fn shaders_compile_with_glslc_when_available() {
    let shaders = [
        "pbr.vert",
        "pbr.frag",
        "shadow_depth.vert",
        "shadow_depth.frag",
        "post.vert",
        "post_tonemap.frag",
        "bloom_prefilter.frag",
    ];
    let shader_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../shaders");
    // 使用 cargo 为集成测试提供的临时目录（target/tmp），避免依赖系统临时目录权限
    let tmp = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("shader_contract");
    std::fs::create_dir_all(&tmp).expect("创建临时目录");

    for name in shaders {
        let input = shader_dir.join(name);
        assert!(input.exists(), "shader 源文件不存在: {input:?}");
        let output = tmp.join(format!("{name}.spv"));
        let result = std::process::Command::new("glslc")
            .arg("--target-env=vulkan1.1")
            .arg(&input)
            .arg("-o")
            .arg(&output)
            .output();
        match result {
            Ok(out) if out.status.success() => {
                let bytes = std::fs::read(&output).expect("读取编译产物");
                assert!(bytes.len() > 20, "{name} 编译产物为空");
            }
            Ok(out) => panic!(
                "{name} glslc 编译失败:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                // 未安装 Vulkan SDK/glslc 时跳过（CI 允许）
                eprintln!("跳过 glslc 编译校验: 未找到 glslc 可执行文件");
                return;
            }
            Err(err) => panic!("调用 glslc 失败: {err}"),
        }
    }

    let _ = std::fs::remove_dir_all(&tmp);
}
