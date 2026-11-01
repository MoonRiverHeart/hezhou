//! `animation-bridge` feature 测试：ImportedScene -> hezhou_animation::gltf::GltfAsset。

#![cfg(feature = "animation-bridge")]

mod common;

use hezhou_asset::animation_bridge::imported_scene_to_gltf_asset;
use hezhou_asset::import::gltf::is_enabled;
use hezhou_asset::meta::GltfImportSettings;

#[test]
fn bridge_builds_animation_gltf_contract_and_runtime_data() {
    if !is_enabled() {
        eprintln!("跳过：未启用 gltf feature");
        return;
    }
    let temp = common::tempdir();
    let path = temp.path().join("hero.glb");
    std::fs::write(&path, common::minimal_glb()).unwrap();
    let scene = hezhou_asset::import::gltf::import_gltf_file(&path, &GltfImportSettings::default())
        .unwrap();

    let bridge = imported_scene_to_gltf_asset(&scene);
    assert!(bridge.has_skeleton());
    assert!(bridge.warnings.is_empty(), "告警: {:?}", bridge.warnings);

    let skeleton = bridge.asset.skeleton.as_ref().unwrap();
    assert_eq!(skeleton.joint_nodes, vec![1, 2]);
    assert_eq!(skeleton.inverse_bind_matrices.len(), 2);
    // 节点表按 glTF 节点索引，动画通道可直接映射
    assert_eq!(skeleton.nodes.len(), 3);
    assert_eq!(skeleton.nodes[1].parent, Some(0));
    assert_eq!(skeleton.nodes[1].translation.y, 1.0);

    assert_eq!(bridge.asset.skins.len(), 1);
    assert_eq!(bridge.asset.skins[0].joint_nodes, vec![1, 2]);
    assert_eq!(bridge.asset.animations.len(), 1);
    let animation = &bridge.asset.animations[0];
    assert_eq!(animation.name, "Idle");
    assert_eq!(animation.channels.len(), 2);
    assert_eq!(animation.channels[0].target_node, 1);
    assert_eq!(animation.channels[0].times, vec![0.0, 1.0]);
    assert_eq!(animation.channels[0].values.len(), 6);

    // animation crate 的运行时转换入口
    let runtime = bridge.into_runtime().unwrap();
    assert_eq!(runtime.skeleton.joints.len(), 2);
    assert_eq!(runtime.skins.len(), 1);
    assert_eq!(runtime.clips.len(), 1);
    assert_eq!(runtime.clips[0].name, "Idle");
}

#[test]
fn bridge_skips_morph_target_weight_channels_with_warning() {
    if !is_enabled() {
        return;
    }
    let temp = common::tempdir();
    let path = temp.path().join("hero.glb");
    std::fs::write(&path, common::minimal_glb()).unwrap();
    let mut scene =
        hezhou_asset::import::gltf::import_gltf_file(&path, &GltfImportSettings::default())
            .unwrap();
    scene.animations[0].channels[0].property = hezhou_asset::AnimationProperty::MorphTargetWeights;

    let bridge = imported_scene_to_gltf_asset(&scene);
    assert!(!bridge.warnings.is_empty());
    assert_eq!(bridge.asset.animations[0].channels.len(), 1);
}
