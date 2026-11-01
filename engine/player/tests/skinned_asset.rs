//! Player 蒙皮资产消费测试：ImportedPrimitive → SkinnedVertex、蒙皮 mesh/skeleton/animation
//! 选择、joint palette（静态绑定姿态/动画采样）、回退与错误安全。

mod common;

use std::path::PathBuf;

use common::TempDir;
use hezhou_animation::SkinnedVertex;
use hezhou_asset::cook::{CookEntry, CookManifest};
use hezhou_asset::{
    AnimationInterpolation, AnimationProperty, AssetId, AssetKind, AssetLocator, Aabb,
    ImportSourceFormat, ImportedAnimation, ImportedAnimationChannel, ImportedJoint, ImportedMesh,
    ImportedNode, ImportedPrimitive, ImportedScene, ImportedSkeleton, Mat4, PrimitiveMode, Trs,
    Vec2 as AssetVec2, Vec3 as AssetVec3,
};
use hezhou_player::asset_index::PlayerAssetIndex;
use hezhou_player::imported_assets::ImportedAssetBundle;
use hezhou_player::manifest::PlayerManifest;
use hezhou_player::skinned_asset::{
    compute_joint_palette, primitive_to_skinned_mesh, select_first_skinned_asset,
    SkinnedAssetError,
};
use hezhou_player::{PlayerBuildConfig, TargetPlatform};

fn host() -> TargetPlatform {
    TargetPlatform::host()
}

const PARENT_GLTF: &str = "33333333-3333-4333-8333-333333333333";
const SCENE_ID: &str = "11111111-1111-4111-8111-111111111111";

fn id(text: &str) -> AssetId {
    AssetId::parse(text).expect("AssetId")
}

struct SkinnedFixture {
    _temp: TempDir,
    bundle: ImportedAssetBundle,
    scene: ImportedScene,
    primitive: ImportedPrimitive,
    mesh: AssetId,
    skeleton: AssetId,
    animation: AssetId,
}

/// 构造合成 ImportedScene + cook manifest/index/bundle。
///
/// 场景包含：2 个关节 node（joint1 是 joint0 的子节点）、1 个蒙皮三角形 mesh、
/// 1 个 skeleton、1 个旋转动画 clip（joint1 绕 Y 轴 0→90°）。
fn make_fixture(skinned: bool) -> SkinnedFixture {
    let temp = TempDir::new(if skinned {
        "skinned-fixture"
    } else {
        "unskinned-fixture"
    });
    let parent = id(PARENT_GLTF);
    let mesh = parent.derive_child(AssetKind::Mesh, "mesh:0");
    let skeleton_id = parent.derive_child(AssetKind::Skeleton, "skeleton:0");
    let animation_id = parent.derive_child(AssetKind::Animation, "animation:0");
    let output = format!("imported/{}.json", parent.simple());

    let joints = if skinned {
        vec![[0u16, 0, 0, 0], [0, 1, 0, 0], [1, 0, 0, 0]]
    } else {
        Vec::new()
    };
    let weights = if skinned {
        vec![
            [1.0f32, 0.0, 0.0, 0.0],
            [0.5, 0.5, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
        ]
    } else {
        Vec::new()
    };
    let primitive = ImportedPrimitive {
        positions: vec![
            AssetVec3::new(0.0, 0.0, 0.0),
            AssetVec3::new(1.0, 0.0, 0.0),
            AssetVec3::new(0.0, 1.0, 0.0),
        ],
        normals: vec![AssetVec3::new(0.0, 0.0, 1.0); 3],
        tangents: Vec::new(),
        uvs: vec![AssetVec2::new(0.0, 0.0); 3],
        joints,
        weights,
        indices: vec![0, 1, 2],
        material: None,
        skeleton: Some(0),
        mode: PrimitiveMode::Triangles,
        bounds: Aabb {
            min: AssetVec3::new(0.0, 0.0, 0.0),
            max: AssetVec3::new(1.0, 1.0, 0.0),
        },
    };
    let scene = ImportedScene {
        format: ImportSourceFormat::Gltf,
        generator: None,
        nodes: vec![
            ImportedNode {
                name: Some("RootJoint".to_string()),
                parent: None,
                children: vec![1],
                local_transform: Trs::IDENTITY,
                mesh: None,
                skin: None,
            },
            ImportedNode {
                name: Some("ChildJoint".to_string()),
                parent: Some(0),
                children: Vec::new(),
                local_transform: Trs {
                    translation: AssetVec3::new(0.0, 1.0, 0.0),
                    ..Trs::IDENTITY
                },
                mesh: None,
                skin: None,
            },
        ],
        meshes: vec![ImportedMesh {
            name: Some("SkinnedTriangle".to_string()),
            primitives: vec![primitive.clone()],
        }],
        materials: Vec::new(),
        textures: Vec::new(),
        skeletons: vec![ImportedSkeleton {
            name: Some("PlayerSkeleton".to_string()),
            joints: vec![
                ImportedJoint {
                    node: 0,
                    name: Some("RootJoint".to_string()),
                    parent: None,
                    inverse_bind_matrix: Mat4::IDENTITY,
                },
                ImportedJoint {
                    node: 1,
                    name: Some("ChildJoint".to_string()),
                    parent: Some(0),
                    inverse_bind_matrix: Mat4::IDENTITY,
                },
            ],
            root_joint: Some(0),
        }],
        animations: vec![ImportedAnimation {
            name: Some("Spin".to_string()),
            channels: vec![ImportedAnimationChannel {
                target_node: 1,
                target_joint: Some(1),
                property: AnimationProperty::Rotation,
                interpolation: AnimationInterpolation::Linear,
                times: vec![0.0, 1.0],
                // identity → 绕 Y 轴 90°（四元数 x,y,z,w = 0,±sin45,0,cos45）。
                values: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.707_106_77, 0.0, 0.707_106_77],
            }],
            duration: 1.0,
        }],
        external_files: Vec::new(),
        warnings: Vec::new(),
    };

    temp.write(
        &format!("assets/{output}"),
        serde_json::to_string(&scene).unwrap().as_bytes(),
    );

    let output_entry = |id: AssetId, kind: AssetKind, locator: &str| CookEntry {
        id,
        kind,
        locator: AssetLocator::parse(locator).unwrap(),
        output: output.clone(),
        dependencies: Vec::new(),
        source_size: 0,
        source_hash: "0".repeat(16),
    };
    let cook = CookManifest {
        format_version: 1,
        roots: vec![id(SCENE_ID), parent],
        entries: vec![
            CookEntry {
                id: id(SCENE_ID),
                kind: AssetKind::Scene,
                locator: AssetLocator::parse("assets/scenes/main.scene").unwrap(),
                output: "scenes/main.scene".to_string(),
                dependencies: Vec::new(),
                source_size: 2,
                source_hash: "0".repeat(16),
            },
            CookEntry {
                id: parent,
                kind: AssetKind::Gltf,
                locator: AssetLocator::parse("assets/models/hero.gltf").unwrap(),
                output: output.clone(),
                dependencies: Vec::new(),
                source_size: 0,
                source_hash: "0".repeat(16),
            },
            output_entry(mesh, AssetKind::Mesh, "assets/models/hero.gltf#mesh:0"),
            output_entry(
                skeleton_id,
                AssetKind::Skeleton,
                "assets/models/hero.gltf#skeleton:0",
            ),
            output_entry(
                animation_id,
                AssetKind::Animation,
                "assets/models/hero.gltf#animation:0",
            ),
        ],
    };
    let config = PlayerBuildConfig::new("Skinned Fixture", "1.0.0", "scenes/main.scene", host())
        .with_required_font(None::<String>);
    let manifest = PlayerManifest::from_parts(
        &config,
        Vec::new(),
        Vec::new(),
        "scenes/main.scene",
        None,
    );
    let index = PlayerAssetIndex::from_cook_manifest(&cook, &manifest).expect("资产索引");
    let bundle = ImportedAssetBundle::load(&temp.join("assets"), &index).expect("加载 bundle");

    SkinnedFixture {
        _temp: temp,
        bundle,
        scene,
        primitive,
        mesh,
        skeleton: skeleton_id,
        animation: animation_id,
    }
}

#[test]
fn converts_imported_primitive_to_skinned_vertices() {
    let fixture = make_fixture(true);
    let (vertices, indices) = primitive_to_skinned_mesh(&fixture.primitive).expect("转换");
    assert_eq!(vertices.len(), 3);
    assert_eq!(indices, vec![0, 1, 2]);
    assert_eq!(SkinnedVertex::SIZE, 64);

    assert_eq!(vertices[0].position, [0.0, 0.0, 0.0]);
    assert_eq!(vertices[0].normal, [0.0, 0.0, 1.0]);
    assert_eq!(vertices[0].uv, [0.0, 0.0]);
    assert_eq!(vertices[0].joint_indices, [0, 0, 0, 0]);
    assert_eq!(vertices[0].joint_weights, [1.0, 0.0, 0.0, 0.0]);

    // 权重归一化：0.5/0.5 保持。
    assert_eq!(vertices[1].joint_indices, [0, 1, 0, 0]);
    let sum: f32 = vertices[1].joint_weights.iter().sum();
    assert!((sum - 1.0).abs() < 1e-6);
    assert!(vertices[2].is_skinned());
}

#[test]
fn rejects_skin_attribute_mismatch() {
    let fixture = make_fixture(true);
    let mut primitive = fixture.primitive.clone();
    primitive.joints.pop();
    let error = primitive_to_skinned_mesh(&primitive).expect_err("joints 数量不一致必须失败");
    assert!(
        matches!(error, SkinnedAssetError::SkinAttributeMismatch { .. }),
        "实际错误：{error}"
    );
}

#[test]
fn rejects_non_triangle_topology() {
    let fixture = make_fixture(true);
    let mut primitive = fixture.primitive.clone();
    primitive.mode = PrimitiveMode::Lines;
    let error = primitive_to_skinned_mesh(&primitive).expect_err("非三角形拓扑必须失败");
    assert!(
        matches!(error, SkinnedAssetError::UnsupportedTopology { .. }),
        "实际错误：{error}"
    );
}

#[test]
fn selects_skinned_asset_and_animation_changes_palette() {
    let fixture = make_fixture(true);
    let selected = select_first_skinned_asset(&fixture.bundle)
        .expect("选择蒙皮资产")
        .expect("应有蒙皮资产");

    assert_eq!(selected.mesh_asset_id, fixture.mesh);
    assert_eq!(selected.vertex_count(), 3);
    assert_eq!(selected.triangle_count(), 1);
    assert_eq!(selected.joint_count(), 2);
    assert_eq!(selected.mesh_index, 0);
    assert_eq!(selected.primitive_index, 0);
    assert!(selected.summary().contains("3 顶点"));
    assert!(selected.summary().contains("2 关节"));
    assert_eq!(
        selected.animation.as_ref().map(|a| a.name.as_deref()),
        Some(Some("Spin"))
    );

    let static_palette = selected.palette_at(0.0).expect("palette t=0");
    let animated_palette = selected.palette_at(0.5).expect("palette t=0.5");
    assert_eq!(static_palette.len(), 2);
    assert_eq!(animated_palette.len(), 2);
    // 动画旋转 joint1 → 第二根骨骼矩阵应变化。
    assert_ne!(
        static_palette.matrices[1], animated_palette.matrices[1],
        "动画应改变 joint palette"
    );
    // 矩阵数值有限。
    for matrix in &animated_palette.matrices {
        for column in matrix.cols {
            assert!(column.iter().all(|value| value.is_finite()));
        }
    }
}

#[test]
fn static_bind_pose_palette_matches_joint_world_transforms() {
    let fixture = make_fixture(true);
    let palette = compute_joint_palette(&fixture.scene, 0, None, 0.0).expect("静态 palette");
    assert_eq!(palette.len(), 2);
    // joint0 在原点；joint1 局部 +Y 1.0（IBM 均为单位矩阵）→ palette[1] 平移 (0,1,0)。
    assert_eq!(palette.matrices[0], hezhou_animation::Mat4::IDENTITY);
    assert_eq!(palette.matrices[1].cols[3], [0.0, 1.0, 0.0, 1.0]);
}

#[test]
fn unskinned_mesh_returns_none_for_fallback() {
    let fixture = make_fixture(false);
    let selected = select_first_skinned_asset(&fixture.bundle).expect("选择不报错");
    assert!(selected.is_none(), "无 joints/weights 时应回退（None）");
}

#[test]
fn missing_skeleton_reports_readable_error() {
    let fixture = make_fixture(true);
    let mut selected = select_first_skinned_asset(&fixture.bundle)
        .expect("选择")
        .expect("应有蒙皮资产");
    selected.skeleton = None;
    let error = selected.palette_at(0.0).expect_err("无 skeleton 应失败");
    assert!(
        matches!(error, SkinnedAssetError::NoSkeleton { .. }),
        "实际错误：{error}"
    );
}

#[test]
fn skeleton_and_animation_ids_are_traceable() {
    let fixture = make_fixture(true);
    let selected = select_first_skinned_asset(&fixture.bundle)
        .expect("选择")
        .expect("应有蒙皮资产");
    assert_eq!(
        selected.skeleton.as_ref().and_then(|s| s.asset_id),
        Some(fixture.skeleton)
    );
    assert_eq!(
        selected.animation.as_ref().and_then(|a| a.asset_id),
        Some(fixture.animation)
    );
    assert_eq!(selected.skeleton.as_ref().unwrap().joint_count, 2);
}

// 避免 `PathBuf` 未使用（测试辅助签名占位）。
#[allow(dead_code)]
fn _path_type(_: PathBuf) {}
