//! glTF/GLB 导入器集成测试：Mesh/Material/Texture/Skeleton/Animation 全链路。

mod common;

use hezhou_asset::import::gltf::{import_gltf_file, is_enabled};
use hezhou_asset::meta::GltfImportSettings;
use hezhou_asset::path::LogicalPath;
use hezhou_asset::{
    validate_imported_scene, AlphaMode, AnimationInterpolation, AnimationProperty, AssetDatabase,
    AssetKind, ImportedImage, PrimitiveMode, TextureFilterMode, TextureWrapMode,
};

fn write_glb(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("hero.glb");
    std::fs::write(&path, common::minimal_glb()).unwrap();
    path
}

#[test]
fn glb_import_covers_mesh_material_texture_skeleton_animation() {
    if !is_enabled() {
        eprintln!("跳过：未启用 gltf feature");
        return;
    }
    let temp = common::tempdir();
    let path = write_glb(temp.path());
    let scene = import_gltf_file(&path, &GltfImportSettings::default()).unwrap();
    validate_imported_scene(&scene).unwrap();

    assert_eq!(scene.format, hezhou_asset::ImportSourceFormat::Glb);
    assert_eq!(scene.generator.as_deref(), Some("hezhou-asset-test"));
    assert!(scene.warnings.is_empty(), "告警: {:?}", scene.warnings);

    // 节点层级 + mesh/skin 引用
    assert_eq!(scene.nodes.len(), 3);
    assert_eq!(scene.nodes[0].name.as_deref(), Some("Root"));
    assert_eq!(scene.nodes[0].mesh, Some(0));
    assert_eq!(scene.nodes[0].skin, Some(0));
    assert_eq!(scene.nodes[1].parent, Some(0));
    assert_eq!(scene.nodes[2].parent, Some(1));
    assert_eq!(scene.nodes[1].local_transform.translation.y, 1.0);

    // 网格
    assert_eq!(scene.meshes.len(), 1);
    let mesh = &scene.meshes[0];
    assert_eq!(mesh.name.as_deref(), Some("HeroMesh"));
    let primitive = &mesh.primitives[0];
    assert_eq!(primitive.mode, PrimitiveMode::Triangles);
    assert_eq!(primitive.positions.len(), 3);
    assert_eq!(primitive.normals.len(), 3);
    assert_eq!(primitive.uvs.len(), 3);
    assert_eq!(primitive.joints.len(), 3);
    assert_eq!(primitive.joints[1], [0, 1, 0, 0]);
    assert_eq!(primitive.weights[1], [0.5, 0.5, 0.0, 0.0]);
    assert_eq!(primitive.indices, vec![0, 1, 2]);
    assert_eq!(primitive.material, Some(0));
    assert_eq!(primitive.skeleton, Some(0));
    assert_eq!(primitive.bounds.min, hezhou_asset::Vec3::new(0.0, 0.0, 0.0));
    assert_eq!(primitive.bounds.max, hezhou_asset::Vec3::new(1.0, 1.0, 0.0));

    // 材质
    let material = &scene.materials[0];
    assert_eq!(material.name.as_deref(), Some("HeroMat"));
    assert!((material.metallic_factor - 0.5).abs() < 1e-6);
    assert!((material.roughness_factor - 0.25).abs() < 1e-6);
    assert!(material.double_sided);
    assert_eq!(material.alpha_mode, AlphaMode::Opaque);
    let base_color = material.base_color_texture.expect("应有基础色纹理");
    assert_eq!(base_color.texture, 0);
    assert_eq!(base_color.tex_coord, 0);

    // 纹理（内嵌图片保留原始字节）
    let texture = &scene.textures[0];
    assert_eq!(texture.name.as_deref(), Some("HeroTex"));
    assert_eq!(texture.sampler.mag_filter, TextureFilterMode::Linear);
    assert_eq!(texture.sampler.wrap_s, TextureWrapMode::Repeat);
    match &texture.image {
        ImportedImage::Embedded { mime_type, data } => {
            assert_eq!(mime_type.as_deref(), Some("image/png"));
            assert_eq!(data, b"PNGDATA!");
        }
        other => panic!("期望内嵌图片，实际 {other:?}"),
    }

    // 骨骼
    assert_eq!(scene.skeletons.len(), 1);
    let skeleton = &scene.skeletons[0];
    assert_eq!(skeleton.name.as_deref(), Some("HeroSkin"));
    assert_eq!(skeleton.joints.len(), 2);
    assert_eq!(skeleton.joints[0].node, 1);
    assert_eq!(skeleton.joints[0].parent, None);
    assert_eq!(skeleton.joints[1].node, 2);
    assert_eq!(skeleton.joints[1].parent, Some(0));
    assert_eq!(skeleton.root_joint, Some(0));
    assert_eq!(
        skeleton.joints[0].inverse_bind_matrix,
        hezhou_asset::Mat4::IDENTITY
    );

    // 动画：通道按 node 索引 -> joint 索引映射
    assert_eq!(scene.animations.len(), 1);
    let animation = &scene.animations[0];
    assert_eq!(animation.name.as_deref(), Some("Idle"));
    assert!((animation.duration - 1.0).abs() < 1e-6);
    assert_eq!(animation.channels.len(), 2);
    let translation = &animation.channels[0];
    assert_eq!(translation.target_node, 1);
    assert_eq!(translation.target_joint, Some(0));
    assert_eq!(translation.property, AnimationProperty::Translation);
    assert_eq!(translation.interpolation, AnimationInterpolation::Linear);
    assert_eq!(translation.times, vec![0.0, 1.0]);
    assert_eq!(translation.values.len(), 6);
    let rotation = &animation.channels[1];
    assert_eq!(rotation.target_node, 2);
    assert_eq!(rotation.target_joint, Some(1));
    assert_eq!(rotation.property, AnimationProperty::Rotation);
    assert_eq!(rotation.values.len(), 8);
    assert!(scene.animation_by_name("Idle").is_some());
}

#[test]
fn import_settings_can_disable_parts_and_scale_positions() {
    if !is_enabled() {
        return;
    }
    let temp = common::tempdir();
    let path = write_glb(temp.path());
    let settings = GltfImportSettings {
        import_animations: false,
        import_skins: false,
        import_textures: false,
        import_materials: false,
        scale: 2.0,
        ..Default::default()
    };
    let scene = import_gltf_file(&path, &settings).unwrap();
    assert!(scene.animations.is_empty());
    assert!(scene.skeletons.is_empty());
    assert!(scene.textures.is_empty());
    assert!(scene.materials.is_empty());
    // 节点上的 skin 引用也应关闭
    assert!(scene.nodes.iter().all(|node| node.skin.is_none()));
    // 位置按 scale 缩放
    assert_eq!(
        scene.meshes[0].primitives[0].positions[1],
        hezhou_asset::Vec3::new(2.0, 0.0, 0.0)
    );
}

#[test]
fn database_import_registers_sub_assets_with_stable_ids() {
    if !is_enabled() {
        return;
    }
    let temp = common::tempdir();
    let project = temp.path();
    common::write_project_asset(project, "assets/models/hero.glb", &common::minimal_glb());

    let mut db = AssetDatabase::open(project).unwrap();
    let hero_id = db
        .id_of(&LogicalPath::new("assets/models/hero.glb").unwrap())
        .unwrap();
    let outcome = db.import_gltf(hero_id).unwrap();

    // 5 类子资产各 1 个
    for kind in [
        AssetKind::Mesh,
        AssetKind::Material,
        AssetKind::Texture,
        AssetKind::Skeleton,
        AssetKind::Animation,
    ] {
        assert_eq!(outcome.sub_assets_of_kind(kind).count(), 1, "kind={kind:?}");
    }
    let mesh = outcome.sub_asset(AssetKind::Mesh, 0).unwrap();
    assert_eq!(mesh.key, "mesh:0");
    assert_eq!(db.entry(mesh.id).unwrap().kind, AssetKind::Mesh);
    assert_eq!(
        db.locator_of(mesh.id).unwrap().to_string(),
        "assets/models/hero.glb#mesh:0"
    );
    // 子资产依赖父文件
    assert!(
        db.dependencies(mesh.id).unwrap().contains(&hero_id),
        "mesh 应依赖父 glTF"
    );
    // Mesh -> Material 依赖已登记
    let material = outcome.sub_asset(AssetKind::Material, 0).unwrap();
    assert!(db.dependencies(mesh.id).unwrap().contains(&material.id));
    // 材质 -> 纹理依赖
    let texture = outcome.sub_asset(AssetKind::Texture, 0).unwrap();
    assert!(db.dependencies(material.id).unwrap().contains(&texture.id));

    // 重新导入应复用同一批子资产 ID
    let again = db.import_gltf(hero_id).unwrap();
    assert_eq!(again.sub_asset(AssetKind::Mesh, 0).unwrap().id, mesh.id);

    // 重新打开工程：子资产 ID 从 .meta 恢复
    drop(db);
    let db2 = AssetDatabase::open(project).unwrap();
    assert_eq!(db2.entry(mesh.id).unwrap().kind, AssetKind::Mesh);
    assert_eq!(
        db2.locator_of(mesh.id).unwrap().to_string(),
        "assets/models/hero.glb#mesh:0"
    );
}

#[test]
fn external_gltf_resolves_buffers_images_and_dependencies() {
    if !is_enabled() {
        return;
    }
    let temp = common::tempdir();
    let project = temp.path();
    let (gltf_path, _bin_path, _image_path) = common::minimal_external_gltf(project);
    let scene = import_gltf_file(&gltf_path, &GltfImportSettings::default()).unwrap();

    assert_eq!(scene.format, hezhou_asset::ImportSourceFormat::Gltf);
    assert_eq!(scene.meshes[0].primitives[0].positions.len(), 3);
    assert_eq!(scene.meshes[0].primitives[0].indices, vec![0, 1, 2]);
    assert_eq!(scene.textures.len(), 1);
    match &scene.textures[0].image {
        ImportedImage::ExternalFile { uri, .. } => assert_eq!(uri, "albedo.png"),
        other => panic!("期望外部图片，实际 {other:?}"),
    }
    assert!(scene.external_files.contains(&"model.bin".to_string()));
    assert!(scene.external_files.contains(&"albedo.png".to_string()));

    // 数据库：外部文件被登记为资产，纹理子资产依赖外部图片
    let mut db = AssetDatabase::open(project).unwrap();
    let gltf_id = db
        .id_of(&LogicalPath::new("assets/models/model.gltf").unwrap())
        .unwrap();
    let outcome = db.import_gltf(gltf_id).unwrap();
    let texture = outcome.sub_asset(AssetKind::Texture, 0).unwrap();
    let image_id = db
        .id_of(&LogicalPath::new("assets/models/albedo.png").unwrap())
        .unwrap();
    assert!(
        db.dependencies(texture.id).unwrap().contains(&image_id),
        "纹理子资产应依赖外部图片资产"
    );
    // 源文件依赖外部 bin + png
    let bin_id = db
        .id_of(&LogicalPath::new("assets/models/model.bin").unwrap())
        .unwrap();
    let file_dependencies = db.dependencies(gltf_id).unwrap();
    assert!(file_dependencies.contains(&bin_id));
    assert!(file_dependencies.contains(&image_id));
}
