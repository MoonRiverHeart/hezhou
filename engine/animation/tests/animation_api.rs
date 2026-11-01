//! 集成验收测试：只通过 crate 公开 API 覆盖 0.2 骨骼动画验收点。
//!
//! 覆盖矩阵（对应 task-6 验收要求）：
//! - 插值（线性 / slerp）
//! - 循环 / 非循环 / 速度
//! - 骨骼层级
//! - 蒙皮矩阵 / CPU 蒙皮
//! - 状态机过渡 / 终止状态
//! - 事件（状态机事件 + clip 事件）
//! - GPU palette 布局
//! - glTF 数据契约转换

use hezhou_animation::{
    AnimationClip, AnimationEvent, AnimationImportError, AnimationPlayer, AnimationState,
    AnimationStateMachine, Channel, GltfAnimation, GltfAsset, GltfChannel, GltfNode, GltfSkeleton,
    GltfSkin, JointPalette, Keyframe, Mat4, Pose, Quat, Skeleton, Skin, SkinnedVertex, StateEvent,
    Transform, TransitionCurve, Vec3, VertexWeights,
};

/// 3 关节骨架：root(0) → spine(1) → hand(2)。
fn chain_skeleton() -> Skeleton {
    let mut skeleton = Skeleton::new("hero");
    let root = skeleton.add_root_joint("root", Transform::IDENTITY);
    let spine = skeleton
        .add_joint(
            "spine",
            root,
            Transform::from_trs(Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY),
        )
        .unwrap();
    skeleton
        .add_joint(
            "hand",
            spine,
            Transform::from_trs(Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY),
        )
        .unwrap();
    skeleton
}

/// 生成一段 1 秒、从 `from` 到 `to` 的平移片段。
fn translation_clip(name: &str, joint: usize, from: Vec3, to: Vec3) -> AnimationClip {
    let mut clip = AnimationClip::new(name, 1.0);
    let mut channel = Channel::translation(joint);
    channel.add_key(Keyframe::translation(0.0, from)).unwrap();
    channel.add_key(Keyframe::translation(1.0, to)).unwrap();
    clip.add_channel(channel);
    clip
}

// === 1. 插值 ===

#[test]
fn public_api_channel_interpolates_translation_and_rotation() {
    let mut translation = Channel::translation(0);
    translation
        .add_key(Keyframe::translation(0.0, Vec3::ZERO))
        .unwrap();
    translation
        .add_key(Keyframe::translation(2.0, Vec3::new(4.0, 0.0, 0.0)))
        .unwrap();
    // t=1.0 位于 0..2 中点。
    let value = translation.sample(1.0).unwrap().as_translation().unwrap();
    assert!(value.approx_eq(Vec3::new(2.0, 0.0, 0.0), 1e-5));

    let mut rotation = Channel::rotation(0);
    rotation
        .add_key(Keyframe::rotation(0.0, Quat::IDENTITY))
        .unwrap();
    rotation
        .add_key(Keyframe::rotation(
            1.0,
            Quat::from_axis_angle(Vec3::UP, std::f32::consts::FRAC_PI_2),
        ))
        .unwrap();
    let mid = rotation.sample(0.5).unwrap().as_rotation().unwrap();
    let expected = Quat::from_axis_angle(Vec3::UP, std::f32::consts::FRAC_PI_4);
    assert!(mid.approx_eq(expected, 1e-4));
}

// === 2. 循环 / 速度 ===

#[test]
fn public_api_looping_player_wraps_time() {
    let clip = translation_clip("walk", 0, Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0));
    let mut player = AnimationPlayer::new(clip);
    player.advance(2.25);
    assert!((player.time - 0.25).abs() < 1e-5, "time={}", player.time);
    assert!(player.normalized_time() < 0.3);
}

#[test]
fn public_api_non_looping_player_finishes_and_speed_scales() {
    let clip = translation_clip("jump", 0, Vec3::ZERO, Vec3::new(0.0, 3.0, 0.0));
    let mut player = AnimationPlayer::new(clip)
        .with_looping(false)
        .with_speed(2.0);
    player.advance(0.4);
    assert!((player.time - 0.8).abs() < 1e-5);
    player.advance(1.0);
    assert!((player.time - 1.0).abs() < 1e-5);
    assert!(player.finished);
}

// === 3. 骨骼层级 ===

#[test]
fn public_api_bone_hierarchy_composes_world_matrices() {
    let skeleton = chain_skeleton();
    assert_eq!(skeleton.topological_order(), vec![0, 1, 2]);
    assert_eq!(skeleton.depth(2), Some(2));
    assert_eq!(skeleton.find("hand"), Some(2));
    let world = skeleton.world_bind_matrices();
    let hand_position = world[2].transform_point(Vec3::ZERO);
    assert!(
        hand_position.approx_eq(Vec3::new(0.0, 2.0, 0.0), 1e-5),
        "hand world = {hand_position:?}"
    );
    // 父子关系：spine 的父是 root，root 无父。
    assert_eq!(skeleton.joints[1].parent, Some(0));
    assert_eq!(skeleton.joints[0].parent, None);
}

// === 4. 蒙皮矩阵 / 逆绑定 ===

#[test]
fn public_api_skin_matrices_are_identity_at_bind_pose() {
    let mut skeleton = chain_skeleton();
    skeleton.rebuild_inverse_bind_matrices().unwrap();
    let skin = Skin::from_skeleton(&skeleton);
    for matrix in skin.bind_skin_matrices(&skeleton) {
        assert!(matrix.approx_eq(&Mat4::IDENTITY, 1e-4));
    }
}

#[test]
fn public_api_cpu_skinning_follows_palette() {
    let mut skeleton = chain_skeleton();
    skeleton.rebuild_inverse_bind_matrices().unwrap();
    let skin = Skin::from_skeleton(&skeleton);
    // 绑定姿态下蒙皮不改变顶点（hand 附近点）。
    let bind_pose = Pose::from_locals(skeleton.default_pose());
    let bind_palette = JointPalette::from_pose(&skeleton, &skin, &bind_pose);
    let weights = VertexWeights::from_pairs(&[(0, 0.25), (2, 0.75)]);
    let vertex = Vec3::new(0.0, 1.0, 0.0);
    let skinned_at_bind = hezhou_animation::skin_position(vertex, &weights, &bind_palette.matrices);
    assert!(
        skinned_at_bind.approx_eq(vertex, 1e-4),
        "bind 姿态应保持顶点位置"
    );

    // 根关节整体平移后，蒙皮结果同步平移。
    let mut moved_pose = skeleton.default_pose();
    moved_pose[0].translation = Vec3::new(5.0, 0.0, 0.0);
    let moved_palette = JointPalette::from_pose(&skeleton, &skin, &Pose::from_locals(moved_pose));
    let skinned = hezhou_animation::skin_position(vertex, &weights, &moved_palette.matrices);
    assert!(
        skinned.approx_eq(vertex + Vec3::new(5.0, 0.0, 0.0), 1e-4),
        "got {skinned:?}"
    );
}

// === 5. 状态机 ===

#[test]
fn public_api_state_machine_registers_nine_states_and_crossfades() {
    let skeleton = chain_skeleton();
    let mut machine =
        AnimationStateMachine::new().with_default_transition(0.4, TransitionCurve::Linear);
    // 6 个基础状态 + task-11 新增 Land/Dodge/Block。
    assert_eq!(AnimationState::ALL.len(), 9);
    assert_eq!(AnimationState::BASE_STATES.len(), 6);
    for state in AnimationState::ALL {
        assert_eq!(AnimationState::from_name(state.name()), Some(state));
        assert!(machine.entry(state).is_some());
    }
    // Idle 恒 0，Run 恒 10，过渡中点应为 5。
    let mut idle = AnimationClip::new("Idle", 1.0);
    let mut idle_channel = Channel::translation(0);
    idle_channel
        .add_key(Keyframe::translation(0.0, Vec3::ZERO))
        .unwrap();
    idle_channel
        .add_key(Keyframe::translation(1.0, Vec3::ZERO))
        .unwrap();
    idle.add_channel(idle_channel);
    let mut run = AnimationClip::new("Run", 1.0);
    let mut run_channel = Channel::translation(0);
    run_channel
        .add_key(Keyframe::translation(0.0, Vec3::new(10.0, 0.0, 0.0)))
        .unwrap();
    run_channel
        .add_key(Keyframe::translation(1.0, Vec3::new(10.0, 0.0, 0.0)))
        .unwrap();
    run.add_channel(run_channel);
    machine.set_clip(AnimationState::Idle, idle);
    machine.set_clip(AnimationState::Run, run);

    assert!(machine.request_state(AnimationState::Run));
    machine.update(0.2);
    assert!((machine.transition_alpha() - 0.5).abs() < 1e-5);
    let pose = machine.sample(&skeleton);
    assert!(pose.locals[0]
        .translation
        .approx_eq(Vec3::new(5.0, 0.0, 0.0), 1e-4));
    let events = machine.update(0.25);
    assert!(events.iter().any(|e| matches!(
        e,
        StateEvent::TransitionFinished {
            state: AnimationState::Run
        }
    )));
}

#[test]
fn public_api_death_state_is_terminal() {
    let mut machine = AnimationStateMachine::new();
    assert!(machine.request_state(AnimationState::Death));
    machine.update(1.0);
    assert!(!machine.request_state(AnimationState::Idle));
    assert_eq!(machine.current_state(), AnimationState::Death);
}

// === 6. 事件 ===

#[test]
fn public_api_clip_events_fire_through_state_machine() {
    let mut machine =
        AnimationStateMachine::new().with_default_transition(0.0, TransitionCurve::Linear);
    let mut attack = translation_clip("Attack", 0, Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0));
    attack.add_event(AnimationEvent::new(0.1, "attack_hit"));
    attack.add_event(AnimationEvent::new(0.5, "attack_combo_window"));
    machine.set_clip_with(AnimationState::Attack, attack, false, 1.0);
    assert!(machine.request_state(AnimationState::Attack));
    let _pending = machine.update(0.0);
    let events = machine.update(0.15);
    let hit = events.iter().any(|event| {
        matches!(
            event,
            StateEvent::ClipEvent { state, event }
                if *state == AnimationState::Attack && event.name == "attack_hit"
        )
    });
    assert!(hit, "events = {events:?}");
    let events = machine.update(0.4);
    assert!(events.iter().any(|event| matches!(
        event,
        StateEvent::ClipEvent { event, .. } if event.name == "attack_combo_window"
    )));
}

#[test]
fn public_api_player_events_survive_loop_boundary() {
    let mut clip = translation_clip("Run", 0, Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0));
    clip.add_event(AnimationEvent::new(0.25, "footstep"));
    let mut player = AnimationPlayer::new(clip);
    player.seek(0.9);
    // 从 0.9 推进 0.5 秒（循环后 0.4）应命中下一圈的 0.25 处事件。
    let events = player.advance(0.5);
    assert_eq!(events.len(), 1, "events={events:?}");
    assert_eq!(events[0].name, "footstep");
}

// === 7. GPU palette ===

#[test]
fn public_api_gpu_palette_layout_is_stable() {
    assert_eq!(std::mem::size_of::<SkinnedVertex>(), 64);
    let skeleton = chain_skeleton();
    let skin = Skin::from_skeleton(&skeleton);
    let pose = Pose::from_locals(skeleton.default_pose());
    let palette = JointPalette::from_pose(&skeleton, &skin, &pose);
    assert_eq!(palette.len(), 3);
    assert_eq!(palette.byte_len(), 3 * 64);
    assert_eq!(palette.to_f32_vec().len(), 3 * 16);
    assert_eq!(palette.to_le_bytes().len(), palette.byte_len());
    assert_eq!(SkinnedVertex::SIZE, hezhou_animation::SKINNED_VERTEX_STRIDE);
}

// === 8. glTF 数据契约 ===

#[test]
fn public_api_gltf_contract_converts_to_runtime_animation() {
    let mut gltf_skeleton = GltfSkeleton::new("hero");
    gltf_skeleton.nodes = vec![GltfNode::new("root", None), {
        let mut node = GltfNode::new("spine", Some(0));
        node.translation = Vec3::new(0.0, 1.0, 0.0);
        node
    }];
    gltf_skeleton.joint_nodes = vec![0, 1];
    gltf_skeleton.inverse_bind_matrices = vec![
        Mat4::IDENTITY,
        Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0)),
    ];
    let mut asset = GltfAsset::new(gltf_skeleton);
    let mut gltf_skin = GltfSkin::new("hero_skin");
    gltf_skin.joint_nodes = vec![0, 1];
    gltf_skin.inverse_bind_matrices = vec![
        Mat4::IDENTITY,
        Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0)),
    ];
    asset.skins.push(gltf_skin);
    let mut animation = GltfAnimation::new("Idle");
    let mut channel = GltfChannel::rotation(1);
    channel.times = vec![0.0, 1.0];
    // 两个关键帧：单位四元数 → 绕 Y 90°（xyzw 顺序）
    let half = std::f32::consts::FRAC_PI_4.sin();
    let w = std::f32::consts::FRAC_PI_4.cos();
    channel.values = vec![0.0, 0.0, 0.0, 1.0, 0.0, half, 0.0, w];
    animation.channels.push(channel);
    asset.animations.push(animation);

    let runtime = asset.into_runtime().unwrap();
    assert_eq!(runtime.skeleton.joint_count(), 2);
    assert_eq!(runtime.skins.len(), 1);
    let clip = runtime.clip("Idle").expect("Idle 片段应存在");
    assert!((clip.duration - 1.0).abs() < 1e-5);
    let pose = clip.sample(1.0, false, &runtime.skeleton);
    // 采样末帧应回到 90° 旋转（四元数 y=sin45）。
    assert!((pose.locals[1].rotation.w - w).abs() < 1e-4);
}

#[test]
fn public_api_gltf_missing_inverse_bind_is_rebuilt() {
    let mut gltf_skeleton = GltfSkeleton::new("no_ibm");
    gltf_skeleton.nodes = vec![GltfNode::new("root", None), {
        let mut node = GltfNode::new("child", Some(0));
        node.translation = Vec3::new(0.0, 2.0, 0.0);
        node
    }];
    gltf_skeleton.joint_nodes = vec![0, 1];
    // 不提供 inverseBindMatrices → 应自动用绑定姿态反算。
    let asset = GltfAsset::new(gltf_skeleton);
    let runtime = asset.into_runtime().unwrap();
    let skin = Skin::from_skeleton(&runtime.skeleton);
    for matrix in skin.bind_skin_matrices(&runtime.skeleton) {
        assert!(matrix.approx_eq(&Mat4::IDENTITY, 1e-4));
    }
}

#[test]
fn public_api_gltf_missing_skeleton_is_rejected() {
    let asset = GltfAsset::default();
    assert!(matches!(
        asset.into_runtime(),
        Err(AnimationImportError::MissingSkeleton)
    ));
}

// === 9. 姿态混合（过渡底层能力）===

#[test]
fn public_api_pose_blend_is_weighted() {
    let a = Pose::from_locals(vec![Transform::from_trs(
        Vec3::new(0.0, 0.0, 0.0),
        Quat::IDENTITY,
    )]);
    let b = Pose::from_locals(vec![Transform::from_trs(
        Vec3::new(8.0, 0.0, 0.0),
        Quat::IDENTITY,
    )]);
    let blended = Pose::blend(&a, &b, 0.25);
    assert!(blended.locals[0]
        .translation
        .approx_eq(Vec3::new(2.0, 0.0, 0.0), 1e-5));
}

// === 10. task-11：Land / Dodge / Block 动作状态扩展 ===

#[test]
fn public_api_land_dodge_block_states_are_additive_and_usable() {
    let skeleton = chain_skeleton();
    let mut machine =
        AnimationStateMachine::new().with_default_transition(0.0, TransitionCurve::Linear);
    // 集成层可直接按名称把 gameplay 的 ActionState 映射到动画状态。
    let mapping = [
        (AnimationState::Land, "Land"),
        (AnimationState::Dodge, "Dodge"),
        (AnimationState::Block, "Block"),
    ];
    for (state, clip_name) in mapping {
        assert_eq!(AnimationState::from_name(clip_name), Some(state));
        assert!(
            !state.default_looping(),
            "{} 默认应为一次性动作",
            state.name()
        );
        let clip = translation_clip(
            clip_name,
            0,
            Vec3::ZERO,
            Vec3::new(state as usize as f32, 0.0, 0.0),
        );
        machine.set_clip(state, clip);
    }
    // 落地 → 闪避 → 格挡 连续切换（覆盖新状态之间的过渡路径）。
    assert!(machine.request_state(AnimationState::Land));
    machine.update(0.5);
    assert!(machine.request_state(AnimationState::Dodge));
    machine.update(0.5);
    assert!(machine.request_state(AnimationState::Block));
    // Block 片段时长 1 秒，推进到末尾后采样终值。
    machine.update(1.0);
    assert_eq!(machine.current_state(), AnimationState::Block);
    let pose = machine.sample(&skeleton);
    // 每段 clip 的平移终值是状态的枚举序号，验证 clip 确实按状态绑定。
    assert!(
        pose.locals[0].translation.approx_eq(
            Vec3::new(AnimationState::Block as usize as f32, 0.0, 0.0),
            1e-5
        ),
        "pose={:?}",
        pose.locals[0].translation
    );
    // 进入 Death 后新状态同样不可切出。
    assert!(machine.request_state(AnimationState::Death));
    machine.update(1.0);
    assert!(!machine.request_state(AnimationState::Land));
    assert!(!machine.request_state(AnimationState::Dodge));
    assert!(!machine.request_state(AnimationState::Block));
}
