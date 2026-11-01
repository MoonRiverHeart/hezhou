//! glTF 2.0 / GLB 导入器（`gltf` feature，默认启用）。
//!
//! 该模块把 `.gltf` / `.glb` 解析为 [`ImportedScene`]，覆盖 0.2 需要的最小子集：
//!
//! | 数据 | 支持 |
//! |---|---|
//! | Mesh | POSITION / NORMAL / TANGENT / TEXCOORD_0 / JOINTS_0 / WEIGHTS_0 / indices |
//! | Material | metallic-roughness：base color / metallic / roughness / normal / occlusion / emissive / alpha |
//! | Texture | 内嵌 bufferView、data URI、外部图片文件；sampler 过滤与环绕 |
//! | Skeleton | skin.joints、逆绑定矩阵、关节层级（父关节索引） |
//! | Animation | translation / rotation / scale / morph weights，Linear / Step / CubicSpline |
//! | Node | TRS 或 matrix（统一转 TRS）、父子层级、mesh/skin 引用 |
//!
//! 已知限制（0.2 首轮，不阻塞主流程）：
//!
//! - 不支持 sparse accessor 的增量数据（只读取基础值并产生 warning）；
//! - 只导入 `TEXCOORD_0`，多 UV 集产生 warning；顶点色 `COLOR_0` 产生 warning 后忽略；
//! - 不支持 morph target 几何（权重动画通道会保留数据但渲染侧 0.2 不消费）；
//! - 不实现 Draco / meshopt / KTX2 等压缩扩展（产生 warning，几何/纹理解码可能失败）；
//! - 外部 URI 按相对源文件路径解析并百分号解码，不做 URI 规范化之外的路径穿越检查
//!   （数据库层会校验逻辑路径合法性）；
//! - 材质 `KHR_materials_unlit` / specular 等扩展被忽略（按标准 PBR 处理）。
//!
//! 关闭 `gltf` feature 时，本模块替换为返回 [`AssetError::ImporterDisabled`] 的桩，
//! 上层代码（数据库/cook）可以无条件编译。

#![cfg(feature = "gltf")]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use gltf::accessor::{Accessor, DataType};
use gltf::buffer::View as BufferView;
use gltf::image::Source as ImageSource;
use gltf::texture::{MagFilter, MinFilter, WrappingMode};

use crate::error::{AssetError, AssetResult};
use crate::math::{Aabb, Mat4, Quat, Trs, Vec2, Vec3, Vec4};
use crate::meta::GltfImportSettings;

use super::{
    validate_imported_scene, AlphaMode, AnimationInterpolation, AnimationProperty,
    ImportSourceFormat, ImportedAnimation, ImportedAnimationChannel, ImportedImage, ImportedJoint,
    ImportedMaterial, ImportedMesh, ImportedNode, ImportedPrimitive, ImportedSampler,
    ImportedScene, ImportedSkeleton, ImportedTexture, ImportedTextureRef, PrimitiveMode,
    TextureFilterMode, TextureWrapMode,
};

/// 是否编译了真实 glTF 导入器（`gltf` feature）。
pub const fn is_enabled() -> bool {
    true
}

/// 从文件路径导入 glTF/GLB。
pub fn import_gltf_file(path: &Path, settings: &GltfImportSettings) -> AssetResult<ImportedScene> {
    let bytes = std::fs::read(path)?;
    import_gltf_bytes(&bytes, path.parent(), settings)
}

/// 从内存字节导入 glTF/GLB。
///
/// `base_dir` 用于解析外部 `buffer` / `image` 的相对 URI；GLB 内嵌数据不受影响。
pub fn import_gltf_bytes(
    bytes: &[u8],
    base_dir: Option<&Path>,
    settings: &GltfImportSettings,
) -> AssetResult<ImportedScene> {
    let format = if bytes.starts_with(b"glTF") {
        ImportSourceFormat::Glb
    } else {
        ImportSourceFormat::Gltf
    };

    let gltf = gltf::Gltf::from_slice(bytes)
        .map_err(|e| AssetError::Import(format!("glTF 解析失败: {e}")))?;
    let gltf::Gltf { document, blob } = gltf;

    let mut warnings: Vec<String> = Vec::new();
    let mut external_files: BTreeSet<String> = BTreeSet::new();
    let buffers = resolve_buffers(
        &document,
        blob.as_deref(),
        base_dir,
        &mut external_files,
        &mut warnings,
    )?;

    let nodes = import_nodes(&document, settings);
    let node_parents: Vec<Option<usize>> = nodes.iter().map(|node| node.parent).collect();
    let skeletons = if settings.import_skins {
        import_skeletons(&document, &buffers, &nodes, &node_parents, &mut warnings)?
    } else {
        Vec::new()
    };
    let node_to_joint = build_node_to_joint(&skeletons);

    let textures = if settings.import_textures {
        import_textures(&document, &buffers, &mut external_files, &mut warnings)?
    } else {
        Vec::new()
    };
    let materials = if settings.import_materials {
        import_materials(&document, settings)
    } else {
        Vec::new()
    };
    let meshes = if settings.import_meshes {
        import_meshes(
            &document,
            &buffers,
            settings,
            materials.len(),
            &mut warnings,
        )?
    } else {
        Vec::new()
    };
    let animations = if settings.import_animations {
        import_animations(&document, &buffers, &node_to_joint, &mut warnings)?
    } else {
        Vec::new()
    };

    let mut scene = ImportedScene {
        format,
        generator: document.as_json().asset.generator.clone(),
        nodes,
        meshes,
        materials,
        textures,
        skeletons,
        animations,
        external_files: external_files.into_iter().collect(),
        warnings,
    };

    // 节点引用的 skin 绑定到网格 primitive，供渲染侧做 GPU 蒙皮。
    for node in &scene.nodes {
        if let (Some(mesh_index), Some(skin_index)) = (node.mesh, node.skin) {
            if let Some(mesh) = scene.meshes.get_mut(mesh_index) {
                for primitive in &mut mesh.primitives {
                    if primitive.skeleton.is_none() {
                        primitive.skeleton = Some(skin_index);
                    }
                }
            }
        }
    }

    validate_imported_scene(&scene)?;
    Ok(scene)
}

/// 读取并解析缓冲区（GLB 二进制块 / 外部 `.bin` / data URI）。
fn resolve_buffers(
    document: &gltf::Document,
    blob: Option<&[u8]>,
    base_dir: Option<&Path>,
    external_files: &mut BTreeSet<String>,
    warnings: &mut Vec<String>,
) -> AssetResult<Vec<Vec<u8>>> {
    let mut buffers = Vec::with_capacity(document.buffers().count());
    for buffer in document.buffers() {
        let data = match buffer.source() {
            gltf::buffer::Source::Bin => match blob {
                Some(blob) => blob.to_vec(),
                None => {
                    return Err(AssetError::Import(format!(
                        "缓冲区 {} 没有 uri 也没有 GLB 二进制块",
                        buffer.index()
                    )))
                }
            },
            gltf::buffer::Source::Uri(uri) => decode_uri(uri, base_dir, external_files, "缓冲区")?,
        };
        if data.len() < buffer.length() {
            warnings.push(format!(
                "缓冲区 {} 声明 {} 字节，实际 {} 字节（按实际数据继续）",
                buffer.index(),
                buffer.length(),
                data.len()
            ));
        }
        buffers.push(data);
    }
    Ok(buffers)
}

/// 解析外部 URI（外部文件或 data URI）。
fn decode_uri(
    uri: &str,
    base_dir: Option<&Path>,
    external_files: &mut BTreeSet<String>,
    label: &str,
) -> AssetResult<Vec<u8>> {
    if let Some(rest) = uri.strip_prefix("data:") {
        let (meta, payload) = rest
            .split_once(',')
            .ok_or_else(|| AssetError::Import(format!("{label} data URI 缺少 ',' 分隔: {uri}")))?;
        if meta.ends_with(";base64") {
            use base64::Engine as _;
            base64::engine::general_purpose::STANDARD
                .decode(payload.as_bytes())
                .map_err(|e| AssetError::Import(format!("{label} data URI base64 解码失败: {e}")))
        } else {
            // 非 base64 data URI：按百分号编码解码为原始字节。
            Ok(percent_decode_bytes(payload))
        }
    } else {
        external_files.insert(uri.to_string());
        let path = resolve_relative_path(base_dir, &percent_decode(uri))?;
        std::fs::read(&path).map_err(|e| {
            AssetError::Import(format!("{label} 外部文件读取失败 {}: {e}", path.display()))
        })
    }
}

/// 把 glTF 相对 URI 拼到源文件目录上。
fn resolve_relative_path(base_dir: Option<&Path>, relative: &str) -> AssetResult<PathBuf> {
    if relative.is_empty() {
        return Err(AssetError::Import("外部 URI 为空".to_string()));
    }
    match base_dir {
        Some(dir) => Ok(dir.join(relative)),
        None => Ok(PathBuf::from(relative)),
    }
}

/// 百分号解码（用于 URI）；非法序列原样保留。
fn percent_decode(input: &str) -> String {
    String::from_utf8_lossy(&percent_decode_bytes(input)).into_owned()
}

fn percent_decode_bytes(input: &str) -> Vec<u8> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = &input[index + 1..index + 3];
            if let Ok(value) = u8::from_str_radix(hex, 16) {
                out.push(value);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    out
}

/// 取 bufferView 的字节切片。
fn view_bytes<'b>(buffers: &'b [Vec<u8>], view: &BufferView<'_>) -> AssetResult<&'b [u8]> {
    let buffer_index = view.buffer().index();
    let data = buffers.get(buffer_index).ok_or_else(|| {
        AssetError::Import(format!("bufferView 引用的 buffer {buffer_index} 越界"))
    })?;
    let start = view.offset();
    let end = start
        .checked_add(view.length())
        .ok_or_else(|| AssetError::Import("bufferView 长度溢出".to_string()))?;
    data.get(start..end).ok_or_else(|| {
        AssetError::Import(format!(
            "bufferView 越界: {start}..{end} / buffer 长度 {}",
            data.len()
        ))
    })
}

/// 读取 accessor 为扁平 f32 序列（每元素 `dimensions().multiplicity()` 个分量）。
#[allow(clippy::needless_range_loop)]
fn read_accessor(
    buffers: &[Vec<u8>],
    accessor: &Accessor<'_>,
    warnings: &mut Vec<String>,
    label: &str,
) -> AssetResult<Vec<f32>> {
    if accessor.sparse().is_some() {
        warnings.push(format!("{label}: 暂不支持稀疏 accessor，仅读取基础值"));
    }

    let count = accessor.count();
    let multiplicity = accessor.dimensions().multiplicity();
    let component_size = accessor.data_type().size();
    let normalized = accessor.normalized();
    let mut out = vec![0.0f32; count * multiplicity];

    let Some(view) = accessor.view() else {
        return Ok(out);
    };
    let data = view_bytes(buffers, &view)?;
    let stride = view.stride().unwrap_or(component_size * multiplicity);
    let base = accessor.offset();

    for element in 0..count {
        for component in 0..multiplicity {
            let position = base + element * stride + component * component_size;
            out[element * multiplicity + component] =
                read_component(data, position, accessor.data_type(), normalized)?;
        }
    }
    Ok(out)
}

/// 读取索引 accessor 为 u32。
#[allow(clippy::needless_range_loop)]
fn read_indices(
    buffers: &[Vec<u8>],
    accessor: &Accessor<'_>,
    warnings: &mut Vec<String>,
    label: &str,
) -> AssetResult<Vec<u32>> {
    if accessor.sparse().is_some() {
        warnings.push(format!("{label}: 暂不支持稀疏 accessor，仅读取基础值"));
    }
    let count = accessor.count();
    let component_size = accessor.data_type().size();
    let mut out = vec![0u32; count];
    let Some(view) = accessor.view() else {
        return Ok(out);
    };
    let data = view_bytes(buffers, &view)?;
    let stride = view.stride().unwrap_or(component_size);
    let base = accessor.offset();
    for element in 0..count {
        let position = base + element * stride;
        let bytes = data
            .get(position..position + component_size)
            .ok_or_else(|| AssetError::Import(format!("{label}: 索引 accessor 越界")))?;
        out[element] = match accessor.data_type() {
            DataType::U8 => bytes[0] as u32,
            DataType::U16 => u16::from_le_bytes([bytes[0], bytes[1]]) as u32,
            DataType::U32 => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            other => {
                return Err(AssetError::Import(format!(
                    "{label}: 不支持的索引组件类型 {other:?}"
                )))
            }
        };
    }
    Ok(out)
}

/// 读取单个数值分量并按 normalized 规则反量化。
fn read_component(
    data: &[u8],
    position: usize,
    data_type: DataType,
    normalized: bool,
) -> AssetResult<f32> {
    let size = data_type.size();
    let bytes = data
        .get(position..position + size)
        .ok_or_else(|| AssetError::Import("accessor 读取越界".to_string()))?;
    let value = match data_type {
        DataType::I8 => normalize_signed(bytes[0] as i8 as f32, 127.0, normalized),
        DataType::U8 => normalize_unsigned(bytes[0] as f32, 255.0, normalized),
        DataType::I16 => normalize_signed(
            i16::from_le_bytes([bytes[0], bytes[1]]) as f32,
            32767.0,
            normalized,
        ),
        DataType::U16 => normalize_unsigned(
            u16::from_le_bytes([bytes[0], bytes[1]]) as f32,
            65535.0,
            normalized,
        ),
        DataType::U32 => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f32,
        DataType::F32 => f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
    };
    Ok(value)
}

fn normalize_signed(value: f32, max: f32, normalized: bool) -> f32 {
    if normalized {
        (value / max).clamp(-1.0, 1.0)
    } else {
        value
    }
}

fn normalize_unsigned(value: f32, max: f32, normalized: bool) -> f32 {
    if normalized {
        (value / max).clamp(0.0, 1.0)
    } else {
        value
    }
}

/// 导入节点层级。
fn import_nodes(document: &gltf::Document, settings: &GltfImportSettings) -> Vec<ImportedNode> {
    let mut nodes: Vec<ImportedNode> = Vec::with_capacity(document.nodes().count());
    for node in document.nodes() {
        let (translation, rotation, scale) = node.transform().decomposed();
        nodes.push(ImportedNode {
            name: node.name().map(String::from),
            parent: None,
            children: node.children().map(|child| child.index()).collect(),
            local_transform: Trs {
                translation: Vec3::from_array(translation).scaled(settings.scale),
                rotation: Quat::from_array(rotation),
                scale: Vec3::from_array(scale),
            },
            mesh: node.mesh().map(|mesh| mesh.index()),
            skin: if settings.import_skins {
                node.skin().map(|skin| skin.index())
            } else {
                None
            },
        });
    }
    for node in document.nodes() {
        for child in node.children() {
            if let Some(entry) = nodes.get_mut(child.index()) {
                entry.parent = Some(node.index());
            }
        }
    }
    nodes
}

/// 导入骨骼 / 蒙皮。
fn import_skeletons(
    document: &gltf::Document,
    buffers: &[Vec<u8>],
    nodes: &[ImportedNode],
    node_parents: &[Option<usize>],
    warnings: &mut Vec<String>,
) -> AssetResult<Vec<ImportedSkeleton>> {
    let mut skeletons = Vec::new();
    for skin in document.skins() {
        let joint_nodes: Vec<usize> = skin.joints().map(|node| node.index()).collect();
        let inverse_binds = match skin.inverse_bind_matrices() {
            Some(accessor) => read_accessor(
                buffers,
                &accessor,
                warnings,
                &format!("skin {} 逆绑定矩阵", skin.index()),
            )?,
            None => {
                warnings.push(format!(
                    "skin {} 缺少 inverseBindMatrices，使用单位矩阵",
                    skin.index()
                ));
                Vec::new()
            }
        };

        let mut joints = Vec::with_capacity(joint_nodes.len());
        for (joint_index, node_index) in joint_nodes.iter().copied().enumerate() {
            let inverse_bind_matrix = {
                let start = joint_index * 16;
                match inverse_binds.get(start..start + 16) {
                    Some(values) => {
                        let mut array = [0.0f32; 16];
                        array.copy_from_slice(values);
                        Mat4::from_cols_array(array)
                    }
                    None => Mat4::IDENTITY,
                }
            };
            joints.push(ImportedJoint {
                node: node_index,
                name: nodes.get(node_index).and_then(|node| node.name.clone()),
                parent: find_parent_joint(node_parents, &joint_nodes, node_index),
                inverse_bind_matrix,
            });
        }

        let root_joint = skin
            .skeleton()
            .map(|node| node.index())
            .and_then(|node_index| joint_nodes.iter().position(|&index| index == node_index));

        skeletons.push(ImportedSkeleton {
            name: skin.name().map(String::from),
            joints,
            root_joint,
        });
    }
    Ok(skeletons)
}

/// 沿节点父链找到最近的“也是关节”的祖先。
fn find_parent_joint(
    node_parents: &[Option<usize>],
    joint_nodes: &[usize],
    node_index: usize,
) -> Option<usize> {
    let mut current = node_parents.get(node_index).copied().flatten();
    while let Some(parent) = current {
        if let Some(joint_index) = joint_nodes.iter().position(|&index| index == parent) {
            return Some(joint_index);
        }
        current = node_parents.get(parent).copied().flatten();
    }
    None
}

/// node 索引 -> (skeleton 索引, joint 索引)。
fn build_node_to_joint(skeletons: &[ImportedSkeleton]) -> BTreeMap<usize, (usize, usize)> {
    let mut map = BTreeMap::new();
    for (skeleton_index, skeleton) in skeletons.iter().enumerate() {
        for (joint_index, joint) in skeleton.joints.iter().enumerate() {
            map.entry(joint.node)
                .or_insert((skeleton_index, joint_index));
        }
    }
    map
}

/// 导入纹理与采样器。
fn import_textures(
    document: &gltf::Document,
    buffers: &[Vec<u8>],
    external_files: &mut BTreeSet<String>,
    _warnings: &mut Vec<String>,
) -> AssetResult<Vec<ImportedTexture>> {
    let mut textures = Vec::new();
    for texture in document.textures() {
        let sampler = texture.sampler();
        let sampler_settings = ImportedSampler {
            mag_filter: match sampler.mag_filter() {
                Some(MagFilter::Nearest) => TextureFilterMode::Nearest,
                Some(MagFilter::Linear) | None => TextureFilterMode::Linear,
            },
            min_filter: match sampler.min_filter() {
                Some(MinFilter::Nearest)
                | Some(MinFilter::NearestMipmapNearest)
                | Some(MinFilter::NearestMipmapLinear) => TextureFilterMode::Nearest,
                Some(MinFilter::Linear)
                | Some(MinFilter::LinearMipmapNearest)
                | Some(MinFilter::LinearMipmapLinear)
                | None => TextureFilterMode::Linear,
            },
            wrap_s: map_wrapping(sampler.wrap_s()),
            wrap_t: map_wrapping(sampler.wrap_t()),
        };

        let image = match texture.source().source() {
            ImageSource::Uri { uri, mime_type } => {
                if let Some(rest) = uri.strip_prefix("data:") {
                    let (meta, payload) = rest.split_once(',').ok_or_else(|| {
                        AssetError::Import(format!("纹理 data URI 格式非法: {uri}"))
                    })?;
                    let data = if meta.ends_with(";base64") {
                        use base64::Engine as _;
                        base64::engine::general_purpose::STANDARD
                            .decode(payload.as_bytes())
                            .map_err(|e| {
                                AssetError::Import(format!("纹理 data URI base64 解码失败: {e}"))
                            })?
                    } else {
                        percent_decode_bytes(payload)
                    };
                    ImportedImage::Embedded {
                        mime_type: mime_type.map(String::from).or_else(|| data_uri_mime(meta)),
                        data,
                    }
                } else {
                    external_files.insert(uri.to_string());
                    ImportedImage::ExternalFile {
                        uri: uri.to_string(),
                        mime_type: mime_type.map(String::from),
                    }
                }
            }
            ImageSource::View { view, mime_type } => ImportedImage::Embedded {
                mime_type: Some(mime_type.to_string()),
                data: view_bytes(buffers, &view)?.to_vec(),
            },
        };

        textures.push(ImportedTexture {
            name: texture.name().map(String::from),
            sampler: sampler_settings,
            image,
        });
    }
    Ok(textures)
}

fn map_wrapping(mode: WrappingMode) -> TextureWrapMode {
    match mode {
        WrappingMode::ClampToEdge => TextureWrapMode::ClampToEdge,
        WrappingMode::MirroredRepeat => TextureWrapMode::MirroredRepeat,
        WrappingMode::Repeat => TextureWrapMode::Repeat,
    }
}

fn data_uri_mime(meta: &str) -> Option<String> {
    let mime = meta.trim_end_matches(";base64").trim();
    if mime.is_empty() {
        None
    } else {
        Some(mime.to_string())
    }
}

/// 导入材质。
fn import_materials(
    document: &gltf::Document,
    settings: &GltfImportSettings,
) -> Vec<ImportedMaterial> {
    let mut materials = Vec::new();
    for material in document.materials() {
        let pbr = material.pbr_metallic_roughness();
        let texture_ref = |info: Option<gltf::texture::Info<'_>>| -> Option<ImportedTextureRef> {
            if !settings.import_textures {
                return None;
            }
            info.map(|info| ImportedTextureRef {
                texture: info.texture().index(),
                tex_coord: info.tex_coord(),
            })
        };

        let normal_texture = if settings.import_textures {
            material.normal_texture().map(|normal| ImportedTextureRef {
                texture: normal.texture().index(),
                tex_coord: normal.tex_coord(),
            })
        } else {
            None
        };
        let occlusion_texture = if settings.import_textures {
            material
                .occlusion_texture()
                .map(|occlusion| ImportedTextureRef {
                    texture: occlusion.texture().index(),
                    tex_coord: occlusion.tex_coord(),
                })
        } else {
            None
        };

        let base_color = pbr.base_color_factor();
        materials.push(ImportedMaterial {
            name: material.name().map(String::from),
            base_color_factor: Vec4::from_array(base_color),
            base_color_texture: texture_ref(pbr.base_color_texture()),
            metallic_factor: pbr.metallic_factor(),
            roughness_factor: pbr.roughness_factor(),
            metallic_roughness_texture: texture_ref(pbr.metallic_roughness_texture()),
            normal_texture,
            normal_scale: material
                .normal_texture()
                .map(|normal| normal.scale())
                .unwrap_or(1.0),
            occlusion_texture,
            occlusion_strength: material
                .occlusion_texture()
                .map(|occlusion| occlusion.strength())
                .unwrap_or(1.0),
            emissive_factor: Vec3::from_array(material.emissive_factor()),
            emissive_texture: texture_ref(material.emissive_texture()),
            alpha_mode: match material.alpha_mode() {
                gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
                gltf::material::AlphaMode::Mask => AlphaMode::Mask,
                gltf::material::AlphaMode::Blend => AlphaMode::Blend,
            },
            alpha_cutoff: material.alpha_cutoff().unwrap_or(0.5),
            double_sided: material.double_sided(),
        });
    }
    materials
}

/// 导入网格。
fn import_meshes(
    document: &gltf::Document,
    buffers: &[Vec<u8>],
    settings: &GltfImportSettings,
    material_count: usize,
    warnings: &mut Vec<String>,
) -> AssetResult<Vec<ImportedMesh>> {
    let mut meshes = Vec::new();
    for mesh in document.meshes() {
        let mut primitives = Vec::new();
        for primitive in mesh.primitives() {
            let mut positions: Vec<Vec3> = Vec::new();
            let mut normals: Vec<Vec3> = Vec::new();
            let mut tangents: Vec<Vec4> = Vec::new();
            let mut uvs: Vec<Vec2> = Vec::new();
            let mut joints: Vec<[u16; 4]> = Vec::new();
            let mut weights: Vec<[f32; 4]> = Vec::new();

            for (semantic, accessor) in primitive.attributes() {
                match semantic {
                    gltf::Semantic::Positions => {
                        let flat = read_accessor(
                            buffers,
                            &accessor,
                            warnings,
                            &format!("mesh {} primitive POSITION", mesh.index()),
                        )?;
                        positions = flat
                            .chunks_exact(3)
                            .map(|chunk| {
                                Vec3::new(chunk[0], chunk[1], chunk[2]).scaled(settings.scale)
                            })
                            .collect();
                    }
                    gltf::Semantic::Normals => {
                        let flat = read_accessor(
                            buffers,
                            &accessor,
                            warnings,
                            &format!("mesh {} primitive NORMAL", mesh.index()),
                        )?;
                        normals = flat
                            .chunks_exact(3)
                            .map(|chunk| Vec3::new(chunk[0], chunk[1], chunk[2]))
                            .collect();
                    }
                    gltf::Semantic::Tangents => {
                        let flat = read_accessor(
                            buffers,
                            &accessor,
                            warnings,
                            &format!("mesh {} primitive TANGENT", mesh.index()),
                        )?;
                        tangents = flat
                            .chunks_exact(4)
                            .map(|chunk| Vec4::new(chunk[0], chunk[1], chunk[2], chunk[3]))
                            .collect();
                    }
                    gltf::Semantic::TexCoords(set) => {
                        if set != 0 {
                            warnings.push(format!(
                                "mesh {}: 只导入 TEXCOORD_0，忽略 TEXCOORD_{set}",
                                mesh.index()
                            ));
                            continue;
                        }
                        let flat = read_accessor(
                            buffers,
                            &accessor,
                            warnings,
                            &format!("mesh {} primitive TEXCOORD_0", mesh.index()),
                        )?;
                        uvs = flat
                            .chunks_exact(2)
                            .map(|chunk| Vec2::new(chunk[0], chunk[1]))
                            .collect();
                    }
                    gltf::Semantic::Joints(set) => {
                        if set != 0 {
                            warnings.push(format!(
                                "mesh {}: 只导入 JOINTS_0，忽略 JOINTS_{set}",
                                mesh.index()
                            ));
                            continue;
                        }
                        let flat = read_accessor(
                            buffers,
                            &accessor,
                            warnings,
                            &format!("mesh {} primitive JOINTS_0", mesh.index()),
                        )?;
                        joints = flat
                            .chunks_exact(4)
                            .map(|chunk| {
                                [
                                    chunk[0].max(0.0) as u16,
                                    chunk[1].max(0.0) as u16,
                                    chunk[2].max(0.0) as u16,
                                    chunk[3].max(0.0) as u16,
                                ]
                            })
                            .collect();
                    }
                    gltf::Semantic::Weights(set) => {
                        if set != 0 {
                            warnings.push(format!(
                                "mesh {}: 只导入 WEIGHTS_0，忽略 WEIGHTS_{set}",
                                mesh.index()
                            ));
                            continue;
                        }
                        let flat = read_accessor(
                            buffers,
                            &accessor,
                            warnings,
                            &format!("mesh {} primitive WEIGHTS_0", mesh.index()),
                        )?;
                        weights = flat
                            .chunks_exact(4)
                            .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
                            .collect();
                    }
                    gltf::Semantic::Colors(set) => {
                        warnings.push(format!(
                            "mesh {}: 0.2 暂不导入顶点色 COLOR_{set}，已忽略",
                            mesh.index()
                        ));
                    }
                }
            }

            let indices = match primitive.indices() {
                Some(accessor) => read_indices(
                    buffers,
                    &accessor,
                    warnings,
                    &format!("mesh {} primitive indices", mesh.index()),
                )?,
                None => Vec::new(),
            };

            let material = primitive
                .material()
                .index()
                .filter(|index| *index < material_count);

            let mut bounds = Aabb::EMPTY;
            for position in &positions {
                bounds.expand_point(*position);
            }

            primitives.push(ImportedPrimitive {
                positions,
                normals,
                tangents,
                uvs,
                joints,
                weights,
                indices,
                material,
                skeleton: None,
                mode: map_primitive_mode(primitive.mode()),
                bounds,
            });
        }
        meshes.push(ImportedMesh {
            name: mesh.name().map(String::from),
            primitives,
        });
    }
    Ok(meshes)
}

fn map_primitive_mode(mode: gltf::mesh::Mode) -> PrimitiveMode {
    match mode {
        gltf::mesh::Mode::Points => PrimitiveMode::Points,
        gltf::mesh::Mode::Lines => PrimitiveMode::Lines,
        gltf::mesh::Mode::LineLoop => PrimitiveMode::LineLoop,
        gltf::mesh::Mode::LineStrip => PrimitiveMode::LineStrip,
        gltf::mesh::Mode::Triangles => PrimitiveMode::Triangles,
        gltf::mesh::Mode::TriangleStrip => PrimitiveMode::TriangleStrip,
        gltf::mesh::Mode::TriangleFan => PrimitiveMode::TriangleFan,
    }
}

/// 导入动画。
fn import_animations(
    document: &gltf::Document,
    buffers: &[Vec<u8>],
    node_to_joint: &BTreeMap<usize, (usize, usize)>,
    warnings: &mut Vec<String>,
) -> AssetResult<Vec<ImportedAnimation>> {
    let mut animations = Vec::new();
    for animation in document.animations() {
        let mut channels = Vec::new();
        let mut duration = 0.0f32;

        for channel in animation.channels() {
            let target = channel.target();
            let target_node = target.node().index();
            let property = map_animation_property(target.property());
            let sampler = channel.sampler();
            let interpolation = match sampler.interpolation() {
                gltf::animation::Interpolation::Linear => AnimationInterpolation::Linear,
                gltf::animation::Interpolation::Step => AnimationInterpolation::Step,
                gltf::animation::Interpolation::CubicSpline => AnimationInterpolation::CubicSpline,
            };

            let times = read_accessor(
                buffers,
                &sampler.input(),
                warnings,
                &format!("动画 {:?} 时间", animation.name()),
            )?;
            let values = read_accessor(
                buffers,
                &sampler.output(),
                warnings,
                &format!("动画 {:?} 关键帧", animation.name()),
            )?;

            if let Some(&last) = times.last() {
                duration = duration.max(last);
            }

            if times.len() < 2 {
                warnings.push(format!(
                    "动画 {:?} 通道 {target_node} 只有 {} 个关键帧，按静态值处理",
                    animation.name(),
                    times.len()
                ));
            }

            channels.push(ImportedAnimationChannel {
                target_node,
                target_joint: node_to_joint.get(&target_node).map(|&(_, joint)| joint),
                property,
                interpolation,
                times,
                values,
            });
        }

        animations.push(ImportedAnimation {
            name: animation.name().map(String::from),
            channels,
            duration,
        });
    }
    Ok(animations)
}

fn map_animation_property(property: gltf::animation::Property) -> AnimationProperty {
    match property {
        gltf::animation::Property::Translation => AnimationProperty::Translation,
        gltf::animation::Property::Rotation => AnimationProperty::Rotation,
        gltf::animation::Property::Scale => AnimationProperty::Scale,
        gltf::animation::Property::MorphTargetWeights => AnimationProperty::MorphTargetWeights,
    }
}
