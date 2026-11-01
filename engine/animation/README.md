# hezhou-animation（0.2 骨骼/蒙皮动画能力）

`engine/animation` 是 Hezhou 0.2 的引擎级骨骼动画模块，提供**可复用的能力**，不包含任何具体角色或游戏规则。

- **零第三方依赖**：`cargo test -p hezhou-animation` 可离线独立运行；
- **纯 Rust 逻辑**：不创建 Vulkan 资源，GPU 部分只提供数据布局与 shader 接口契约；
- **可组合**：asset 导入 → 运行时骨架/蒙皮/片段 → 状态机采样 → joint palette → render 上传。

## 1. 0.2 能力范围

| 能力 | 状态 | 入口 |
|---|---|---|
| Skeleton / Joint / Bone 层级 | ✅ | `Skeleton`、`Joint`、`Bone`（别名） |
| Skin / 顶点权重 / 逆绑定矩阵 | ✅ | `Skin`、`VertexWeights` |
| AnimationClip / Channel / Keyframe（T/R/S） | ✅ | `AnimationClip`、`Channel`、`Keyframe`、`KeyValue` |
| 采样 / 插值（Linear、Step） / 循环 / 速度 | ✅ | `Channel::sample`、`AnimationClip::resolve_time`、`AnimationPlayer` |
| AnimationStateMachine（基础 6 状态 + Land/Dodge/Block） | ✅ | `AnimationStateMachine`、`AnimationState`、`AnimationState::BASE_STATES` |
| 过渡（交叉淡入淡出）与事件 | ✅ | `Transition`、`StateEvent`、`AnimationEvent` |
| 蒙皮矩阵 / GPU joint palette / 顶点布局 | ✅ | `Skin::skin_matrices`、`JointPalette`、`SkinnedVertex` |
| CPU 蒙皮回退路径 | ✅ | `skin_position`、`skin_normal` |
| glTF 导入数据契约（内存数据） | ✅ | `GltfAsset::into_runtime` → `AnimationAssetData` |
| Blend Tree / 混合空间 | ⛔ 0.3 | `PoseNode` 接口 + TODO |
| IK | ⛔ 0.3 | `IkSolver` / `TwoBoneIk` 占位（返回 `UnsupportedFeature`） |
| CUBICSPLINE 切线采样 | ⛔ 0.3 | 枚举 `Interpolation::CubicSpline`，采样回退线性 |
| 根运动提取 / 动画层 / 遮罩 | ⛔ 0.3 | 未实现 |

## 2. 模块结构

```
engine/animation/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs          # crate 文档 + 公开 re-export
│   ├── math.rs         # Vec3 / Quat / Mat4 / Transform（与 core::math 数值语义一致）
│   ├── skeleton.rs     # Skeleton / Joint / Bone / 逆绑定矩阵 / 拓扑世界矩阵
│   ├── skin.rs         # Skin / VertexWeights / 蒙皮矩阵 / CPU 蒙皮
│   ├── clip.rs         # AnimationClip / Channel / Keyframe / 事件区间
│   ├── player.rs       # AnimationPlayer（循环/速度/事件）
│   ├── pose.rs         # Pose / 姿态混合 / Blend Tree、IK 预留接口
│   ├── state_machine.rs# 9 状态机（基础 6 + Land/Dodge/Block）/ 过渡 / 事件 / 桥接 trait
│   ├── gpu.rs          # joint palette / SkinnedVertex 布局 / shader 契约常量
│   └── gltf.rs         # glTF 内存数据契约与运行时转换
└── tests/
    └── animation_api.rs # 16 个公开 API 验收测试
```

## 3. 快速上手

```rust
use hezhou_animation::{
    AnimationClip, AnimationState, AnimationStateMachine, Channel, Keyframe,
    JointPalette, Quat, Skeleton, Skin, Transform, Vec3,
};

// 1) 骨架：父先子后构建；逆绑定矩阵可自动反算
let mut skeleton = Skeleton::new("hero");
let root = skeleton.add_root_joint("root", Transform::IDENTITY);
let hand = skeleton.add_joint(
    "hand",
    root,
    Transform::from_trs(Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY),
).unwrap();
skeleton.rebuild_inverse_bind_matrices().unwrap();
let skin = Skin::from_skeleton(&skeleton);

// 2) 动画片段
let mut attack = AnimationClip::new("Attack", 1.0);
let mut channel = Channel::translation(hand);
channel.add_key(Keyframe::translation(0.0, Vec3::new(0.0, 1.0, 0.0))).unwrap();
channel.add_key(Keyframe::translation(1.0, Vec3::new(0.0, 1.0, 2.0))).unwrap();
attack.add_channel(channel);

// 3) 状态机
let mut machine = AnimationStateMachine::new();
machine.set_clip_with(AnimationState::Attack, attack, false, 1.0);
machine.transition_to(AnimationState::Attack, 0.0);
machine.update(0.5);
let pose = machine.sample(&skeleton);

// 4) 每帧生成 palette（数据交给 render 上传）
let palette = JointPalette::from_pose(&skeleton, &skin, &pose);
let bytes = palette.to_le_bytes(); // 每个关节 64 bytes，列主序 mat4
```

## 4. 关键约定

### 4.1 坐标系与矩阵
- `Mat4::cols[i]` 是第 i 列，与 GLSL/Vulkan `mat4` 内存布局一致（column-major）；
- 世界矩阵 = 父世界矩阵 × 本关节局部 TRS；
- 蒙皮矩阵 `palette[i] = world_joint[i] * inverse_bind[i]`，在**模型空间**完成蒙皮，
  之后 render 侧再乘实体 model matrix；
- 绑定姿态下所有蒙皮矩阵应为单位矩阵（可用 `Skin::bind_skin_matrices` 自检）。

### 4.2 时间与事件
- 时间为秒；循环用 `rem_euclid(duration)`，非循环钳制到 `[0, duration]`；
- clip 事件触发区间为**半开区间 `[start, end)`**，相邻两次推进不会重复派发；
- 一次推进跨越多个完整循环时，每个事件只派发一次（避免卡顿帧刷屏）；
- 事件由 `AnimationStateMachine::update` / `AnimationPlayer::advance` 返回，动画模块不执行游戏逻辑。

### 4.3 状态机规则（0.2）

**状态表（9 个，旧 6 状态语义不变，task-11 追加 Land/Dodge/Block）：**

| 状态 | 默认循环 | 终止 | 说明 |
|---|---|---|---|
| `Idle` | ✅ | ❌ | 待机 |
| `Run` | ✅ | ❌ | 移动/奔跑（Move 也映射到这里） |
| `Jump` | ❌ | ❌ | 跳跃/空中（Fall 也映射到这里） |
| `Attack` | ❌ | ❌ | 攻击（轻/重攻击通过 clip 区分） |
| `Hit` | ❌ | ❌ | 受击 |
| `Death` | ❌ | ✅ | 死亡，进入后不可切出 |
| `Land` | ❌ | ❌ | 落地（task-11 新增） |
| `Dodge` | ❌ | ❌ | 闪避（task-11 新增） |
| `Block` | ❌ | ❌ | 格挡（task-11 新增） |

- 兼容性：`AnimationState::ALL` 前 6 项与旧版本顺序一致，可用 `BASE_STATES` 单独取旧 6 状态；
- 同状态重复请求返回 `false`（重播用 `restart()`）；
- `Death` 是终止状态：可进入，不可切出（新增状态同样受此限制）；
- 过渡进行中允许打断；跨状态过渡期间旧片段时间继续推进；
- 状态机事件：`Enter` / `Exit` / `TransitionStarted` / `TransitionFinished` / `ClipEvent`；
- 默认循环策略：`Idle`/`Run` 循环，其余（含 `Land`/`Dodge`/`Block`）一次性动作不循环。

## 5. GPU 集成点（render-owner 需要做的事）

本模块只交付数据/接口契约，**不创建 Vulkan 资源**。具体见 `src/gpu.rs` 与 `engine/shaders/skinning.*`。

### 5.1 顶点布局（stride 64 bytes）

| location | 属性 | 格式 | offset |
|---|---|---|---|
| 0 | position | `VK_FORMAT_R32G32B32_SFLOAT` | 0 |
| 1 | normal | `VK_FORMAT_R32G32B32_SFLOAT` | 12 |
| 2 | uv | `VK_FORMAT_R32G32_SFLOAT` | 24 |
| 3 | joint_indices | `VK_FORMAT_R32G32B32A32_UINT` | 32 |
| 4 | joint_weights | `VK_FORMAT_R32G32B32A32_SFLOAT` | 48 |

前 32 字节与现有非蒙皮顶点完全一致；render 侧可共用顶点结构，只把 stride 从 32 扩到 64 并加 2 个属性。
对应 Rust 结构：`SkinnedVertex`（`size_of = 64`）；属性表：`SKINNED_VERTEX_ATTRIBUTES`。

### 5.2 joint palette

- 数据：`JointPalette { matrices: Vec<Mat4> }`，元素 i 对应 `Skin::joints[i]`；
- 上传：`JointPalette::to_le_bytes()`（连续 64 字节/元素）或 `to_f32_vec()`；
- 建议绑定：`layout(std430, set = 0, binding = 1) readonly buffer JointPalette { mat4 joints[]; };`
  （LightData 占用 set=0 binding=0，不冲突）；
- 小型骨架（≤ 256 关节）可改用 UBO（16 KB），更大骨骼建议 storage buffer / 分块；
- 每帧流程：`machine.sample(&skeleton)` → `JointPalette::from_pose(...)` → 写入 buffer → descriptor set。

### 5.3 shader 文件

- `engine/shaders/skinning.vert`：蒙皮顶点着色器（palette 蒙皮 + 与 `rotation.vert` 相同的
  144 bytes push constant 布局 + 相同 varying location 0..4）；
- `engine/shaders/skinning.frag`：参考片元着色器；由于 varying 与 `rotation.frag` 完全一致，
  **推荐 render 侧直接复用 `rotation.frag.spv`**，避免重复维护；
- 仓库没有自动化 shader 编译流程，需手动：
  ```bash
  glslc engine/shaders/skinning.vert -o engine/shaders/skinning.vert.spv
  glslc engine/shaders/skinning.frag -o engine/shaders/skinning.frag.spv
  ```
- render-pipeline 改动（新 pipeline、descriptor、vertex input、push constant 路径）由 render-owner
  负责；本 crate 不修改 `rhi-vulkan` / `render-pipeline` 源文件。

## 6. asset 集成点

`engine/asset` 导入 glTF 后，把结果填成 `gltf::GltfAsset`（所有字段都是普通内存数据），再调用一次转换：

```rust
let runtime: AnimationAssetData = gltf_asset.into_runtime()?;
// runtime.skeleton: Skeleton
// runtime.skins:    Vec<Skin>
// runtime.clips:    Vec<AnimationClip>
runtime.bind_to_state_machine(&mut machine, &[
    (AnimationState::Idle, "Idle"),
    (AnimationState::Run, "Run"),
    (AnimationState::Attack, "Attack"),
]);
```

约定细节：
- `GltfNode.parent` / `GltfChannel.target_node` 使用 glTF 节点索引；
- `GltfSkeleton.joint_nodes` 的顺序即运行时 `JointIndex`；
- glTF 的 `inverseBindMatrices` 缺失时，本模块会用绑定姿态自动反算（skeleton 与 skin 同步补全）；
- glTF 本身没有动画事件；asset 层可从自定义扩展/命名约定生成 `AnimationEvent` 填入 `GltfAnimation.events`；
- 0.2 只支持 LINEAR/STEP 通道；CUBICSPLINE 会返回 `AnimationImportError::UnsupportedInterpolation`。

## 7. gameplay 集成点

gameplay 模块不要依赖状态机内部结构，只用桥接 trait：

```rust
use hezhou_animation::{AnimationState, AnimationStateDriver};

fn drive(driver: &mut dyn AnimationStateDriver, want_attack: bool) {
    if want_attack {
        driver.request_state(AnimationState::Attack);
    }
    let _ = driver.current_state();
}
```

状态机事件（脚步/攻击命中帧）在 `AnimationStateMachine::update(dt)` 返回值中消费；
连招窗口/伤害判定等游戏规则仍在 gameplay 模块（`ComboGraph` 等），本模块只透传事件。

**集成层（`engine/integration`）的 `ActionStateBridge` 适配映射（0.2 可用）：**

| gameplay `ActionState` | animation `AnimationState` | 备注 |
|---|---|---|
| `Idle` | `Idle` | |
| `Move` / `Run` | `Run` | 循环 |
| `Jump` / `Fall` | `Jump` | 空中阶段共用 |
| `Land` | `Land` | task-11 新增，一次性 |
| `Attack(ActionId)` | `Attack` | 切状态前 `set_clip(Attack, clip_for_action)` |
| `HeavyAttack(ActionId)` | `Attack` | 用 `set_clip_with(..., speed)` 区分重击节奏 |
| `Dodge` | `Dodge` | task-11 新增，一次性 |
| `Block` | `Block` | task-11 新增；需要持续格挡时可 `set_clip_with(Block, clip, true, 1.0)` 改成循环 |
| `Hit` | `Hit` | |
| `Death` | `Death` | 终止状态 |

- adapter 由 **`engine/integration`** 统一实现（animation/gameplay 互不依赖）；
- 逐招式 `ActionId` 不需要扩展 animation 状态：集成层切换前替换 `Attack` 状态的 clip 即可；
- `TransitionSpec.restart=true` 时请调用 `AnimationStateMachine::restart()`（同状态 `request_state` 会返回 false）；
- `state_remaining()` 用 `current_time()` + `clip(state).duration` 计算，循环状态返回 `None`。

## 8. 与现有 core 动画组件的关系

- `core/src/ecs/components/animation.rs` 里的 `AnimationPlayer` / `AnimationClip` / `AnimationSystem`
  是 v0.1 的 **Transform 级关键帧动画**（无骨骼、无蒙皮），本任务**未修改**该文件，旧测试不受影响；
- 新的 `hezhou-animation` 是独立的骨骼动画 crate，输出完整 `Pose` 与 GPU palette；
- 后续如需 ECS 集成，建议在 core 增加一个持有 `hezhou_animation::AnimationStateMachine + Skeleton + Skin`
  的组件，由 render 侧消费 `JointPalette`（这属于 0.2 Lead 集成阶段，本 crate 不反向依赖 core）。

## 9. 测试

```bash
cd engine
cargo test -p hezhou-animation
```

当前测试覆盖（共 52 个，含 doctest）：
- 数学：矩阵求逆、四元数 slerp；
- 采样/插值：平移线性、旋转 slerp、Step、关键帧排序/类型校验；
- 循环/速度：循环回绕、非循环钳制 + finished、speed 缩放、跨循环边界事件；
- 骨骼层级：世界矩阵累乘、拓扑顺序、环/非法父索引校验、绑定蒙皮矩阵为身份；
- 蒙皮：逆绑定矩阵、权重归一化/前 4 影响、CPU 蒙皮混合、palette 与姿态一致；
- 状态机：过渡混合与完成、Death 终止、Enter/Exit/ClipEvent、桥接 trait；
- 状态扩展（task-11）：Land/Dodge/Block 注册、默认非循环、名称解析、新状态互切、
  Death 终止不被破坏、旧 6 状态顺序与语义回归；
- GPU：顶点布局 offset/stride、palette 字节布局；
- glTF：契约转换、缺失 IBM 自动补全、未知关节/值数量/CUBICSPLINE 错误。

## 10. 当前限制与 0.3 TODO

1. **Blend Tree / 混合空间**：仅预留 `PoseNode` 接口与 `Pose::blend_many`（简化实现），0.3 实装。
2. **IK**：`IkSolver` / `TwoBoneIk` 占位，调用返回 `UnsupportedFeature`；0.3 实装 two-bone/look-at。
3. **CUBICSPLINE**：数据契约可表达，采样回退线性；0.3 增加 in/out tangent 支持。
4. **根运动**：0.2 不提取 root motion，位移由角色控制器驱动；0.3 提供根运动曲线与 delta 提取。
5. **动画层/遮罩/加法动画**：未实现。
6. **GPU 上传**：本 crate 不创建 buffer/descriptor，也不做 GPU 测试（无 GPU CI）。
7. **多 skin/多 mesh 部件**：0.2 假设单角色一套 skin；多部件拆分留待 asset/render 集成阶段。
8. **过渡打断**：过渡进行中切到第三个状态时，旧过渡立即结束（不做“过渡的过渡”混合）；
   0.3 可引入过渡队列/分层混合。
9. **性能**：状态机采样每帧分配 `Pose`/`Vec`；0.3 可引入 pose 双缓冲与 palette 复用。
