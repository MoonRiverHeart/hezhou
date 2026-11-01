//! 集成测试共享工具：构造最小 glTF/GLB 测试数据与工程目录。

#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// 创建自动清理的临时目录（位于 crate 的 `target/test-tmp`）。
///
/// 不直接使用系统临时目录：部分 Windows 环境 `%TEMP%` 不可写。
pub fn tempdir() -> tempfile::TempDir {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test-tmp");
    std::fs::create_dir_all(&base).expect("创建测试临时根目录失败");
    tempfile::tempdir_in(&base).expect("创建测试临时目录失败")
}

/// 把 JSON 字符串与 BIN 数据打包为 GLB 字节。
pub fn build_glb(json: &str, bin: &[u8]) -> Vec<u8> {
    let mut json_bytes = json.as_bytes().to_vec();
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(b' ');
    }
    let mut bin_bytes = bin.to_vec();
    while bin_bytes.len() % 4 != 0 {
        bin_bytes.push(0);
    }

    let total_length = 12 + 8 + json_bytes.len() + 8 + bin_bytes.len();
    let mut out = Vec::with_capacity(total_length);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total_length as u32).to_le_bytes());
    out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&json_bytes);
    out.extend_from_slice(&(bin_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin_bytes);
    out
}

/// 最小但覆盖 Mesh/Material/Texture/Skeleton/Animation 的 GLB。
///
/// 场景节点：
/// - node 0 `Root`：mesh 0 + skin 0
/// - node 1 `Hips`：node 2 的父节点，joint 0
/// - node 2 `Spine`：joint 1
///
/// 动画 `Idle`：Hips 平移 + Spine 旋转，各 2 帧。
pub fn minimal_glb() -> Vec<u8> {
    let json = r#"{
      "asset": { "version": "2.0", "generator": "hezhou-asset-test" },
      "scene": 0,
      "scenes": [ { "nodes": [0] } ],
      "nodes": [
        { "name": "Root", "children": [1], "mesh": 0, "skin": 0 },
        { "name": "Hips", "children": [2], "translation": [0.0, 1.0, 0.0] },
        { "name": "Spine" }
      ],
      "meshes": [
        {
          "name": "HeroMesh",
          "primitives": [
            {
              "attributes": {
                "POSITION": 0,
                "NORMAL": 1,
                "TEXCOORD_0": 2,
                "JOINTS_0": 3,
                "WEIGHTS_0": 4
              },
              "indices": 5,
              "material": 0
            }
          ]
        }
      ],
      "materials": [
        {
          "name": "HeroMat",
          "pbrMetallicRoughness": {
            "baseColorFactor": [0.8, 0.5, 0.25, 1.0],
            "baseColorTexture": { "index": 0 },
            "metallicFactor": 0.5,
            "roughnessFactor": 0.25
          },
          "doubleSided": true
        }
      ],
      "textures": [ { "name": "HeroTex", "source": 0, "sampler": 0 } ],
      "images": [ { "bufferView": 10, "mimeType": "image/png" } ],
      "samplers": [
        { "magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497 }
      ],
      "skins": [
        {
          "name": "HeroSkin",
          "joints": [1, 2],
          "inverseBindMatrices": 6,
          "skeleton": 1
        }
      ],
      "animations": [
        {
          "name": "Idle",
          "channels": [
            { "sampler": 0, "target": { "node": 1, "path": "translation" } },
            { "sampler": 1, "target": { "node": 2, "path": "rotation" } }
          ],
          "samplers": [
            { "input": 7, "interpolation": "LINEAR", "output": 8 },
            { "input": 7, "interpolation": "LINEAR", "output": 9 }
          ]
        }
      ],
      "accessors": [
        {
          "bufferView": 0,
          "componentType": 5126,
          "count": 3,
          "type": "VEC3",
          "min": [0.0, 0.0, 0.0],
          "max": [1.0, 1.0, 0.0]
        },
        { "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3" },
        { "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC2" },
        { "bufferView": 3, "componentType": 5123, "count": 3, "type": "VEC4" },
        { "bufferView": 4, "componentType": 5126, "count": 3, "type": "VEC4" },
        { "bufferView": 5, "componentType": 5123, "count": 3, "type": "SCALAR" },
        { "bufferView": 6, "componentType": 5126, "count": 2, "type": "MAT4" },
        { "bufferView": 7, "componentType": 5126, "count": 2, "type": "SCALAR" },
        { "bufferView": 8, "componentType": 5126, "count": 2, "type": "VEC3" },
        { "bufferView": 9, "componentType": 5126, "count": 2, "type": "VEC4" }
      ],
      "bufferViews": [
        { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
        { "buffer": 0, "byteOffset": 36, "byteLength": 36 },
        { "buffer": 0, "byteOffset": 72, "byteLength": 24 },
        { "buffer": 0, "byteOffset": 96, "byteLength": 24 },
        { "buffer": 0, "byteOffset": 120, "byteLength": 48 },
        { "buffer": 0, "byteOffset": 168, "byteLength": 6 },
        { "buffer": 0, "byteOffset": 176, "byteLength": 128 },
        { "buffer": 0, "byteOffset": 304, "byteLength": 8 },
        { "buffer": 0, "byteOffset": 312, "byteLength": 24 },
        { "buffer": 0, "byteOffset": 336, "byteLength": 32 },
        { "buffer": 0, "byteOffset": 368, "byteLength": 8 }
      ],
      "buffers": [ { "byteLength": 376 } ]
    }"#;

    let mut bin = Vec::new();
    // POSITION: 三角形
    for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    // NORMAL
    for _ in 0..3 {
        for value in [0.0f32, 0.0, 1.0] {
            bin.extend_from_slice(&value.to_le_bytes());
        }
    }
    // TEXCOORD_0
    for value in [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0] {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    // JOINTS_0 (u16)
    for value in [0u16, 1, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0] {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    // WEIGHTS_0
    for value in [
        1.0f32, 0.0, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0,
    ] {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    assert_eq!(bin.len(), 168, "属性区长度必须为 168");
    // indices (u16) + 2 字节对齐
    for value in [0u16, 1, 2] {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    bin.extend_from_slice(&[0, 0]);
    assert_eq!(bin.len(), 176);
    // 逆绑定矩阵 x2（单位矩阵）
    for _ in 0..2 {
        for value in [
            1.0f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ] {
            bin.extend_from_slice(&value.to_le_bytes());
        }
    }
    // 动画时间 [0, 1]
    for value in [0.0f32, 1.0] {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    // 平移关键帧
    for value in [0.0f32, 1.0, 0.0, 0.0, 2.0, 0.0] {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    // 旋转关键帧（xyzw）
    for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.3826834, 0.0, 0.9238795] {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    // 内嵌图片字节（导入器不解码，仅保存原始字节）
    bin.extend_from_slice(b"PNGDATA!");
    assert_eq!(bin.len(), 376, "BIN 长度必须与 buffer.byteLength 一致");

    build_glb(json, &bin)
}

/// 最小 `.gltf` + 外部 `.bin` + 外部图片的工程资产。
///
/// 返回 `(gltf 文件路径, bin 文件路径, 图片文件路径)`，三者都在 `assets/models/` 下。
pub fn minimal_external_gltf(project_root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let models_dir = project_root.join("assets").join("models");
    std::fs::create_dir_all(&models_dir).unwrap();

    let mut bin = Vec::new();
    for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0u16, 1, 2] {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    assert_eq!(bin.len(), 42);

    let bin_path = models_dir.join("model.bin");
    std::fs::write(&bin_path, &bin).unwrap();
    let image_path = models_dir.join("albedo.png");
    std::fs::write(&image_path, b"PNGDATA-external").unwrap();

    let json = r#"{
      "asset": { "version": "2.0", "generator": "hezhou-asset-test" },
      "scene": 0,
      "scenes": [ { "nodes": [0] } ],
      "nodes": [ { "name": "Quad", "mesh": 0 } ],
      "meshes": [
        {
          "name": "QuadMesh",
          "primitives": [
            { "attributes": { "POSITION": 0 }, "indices": 1, "material": 0 }
          ]
        }
      ],
      "materials": [
        {
          "name": "QuadMat",
          "pbrMetallicRoughness": {
            "baseColorTexture": { "index": 0 }
          }
        }
      ],
      "textures": [ { "source": 0 } ],
      "images": [ { "uri": "albedo.png" } ],
      "accessors": [
        {
          "bufferView": 0,
          "componentType": 5126,
          "count": 3,
          "type": "VEC3",
          "min": [0.0, 0.0, 0.0],
          "max": [1.0, 1.0, 0.0]
        },
        { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
      ],
      "bufferViews": [
        { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
        { "buffer": 0, "byteOffset": 36, "byteLength": 6 }
      ],
      "buffers": [ { "byteLength": 42, "uri": "model.bin" } ]
    }"#;
    let gltf_path = models_dir.join("model.gltf");
    std::fs::write(&gltf_path, json).unwrap();

    (gltf_path, bin_path, image_path)
}

/// 在工程中创建一个任意内容的资产文件（用于扫描/cook 测试）。
pub fn write_project_asset(project_root: &Path, relative: &str, bytes: &[u8]) -> PathBuf {
    let path = project_root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, bytes).unwrap();
    path
}
