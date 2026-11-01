# hezhou-integration（animation / gameplay / physics 集成适配层）

`engine/integration` 是 Hezhou 0.2 的**能力适配层**：`hezhou-animation`（骨骼/蒙皮/状态机）、
`hezhou-gameplay`（角色/战斗/连招能力）与 `hezhou-physics`（刚体/碰撞/查询）刻意互不依赖，
由本 crate 负责组合。它只做适配，**不包含任何具体游戏规则**（招式数值、关卡、敌人 AI 都在
示例工程/游戏项目里）。

- 依赖方向：`integration → animation + gameplay + physics`；能力 crate 之间互不依赖；
- gameplay 启用其 `physics` feature：查询 trait 由 gameplay-owner 的 `PhysicsQuery` /
  `PhysicsHitQuery` 实现，集成层**不重复实现**；
- 纯 Rust、可离线单测：`cargo test -p hezhou-integration` 不需要 GPU、音频或 Mono；
- 公开 API 英文命名，注释中文。

## 1. 能力一览

| 要求 | 实现 |
|---|---|
| gameplay `ActionStateBridge` | `AnimationActionBridge`（实现 `ActionStateBridge` + `AnimationEventSource`） |
| `ActionState` → `AnimationState` 映射 | `map_action_state`（含 Land/Dodge/Block 兼容分支） |
| 逐招式 `ActionId` clip | `ActionClipRegistry` + 切换前 `set_clip_with`（Attack 状态按招式换片） |
| 事件映射 | `AnimationEventMapper`：`Enter/Exit/Transition/ClipEvent` → gameplay `AnimationEvent` |
| 每帧动画封装 | `AnimationActionAdapter`：`request/update/sample/state_remaining/restart` |
| 循环 / 一次性 | 一次性动作强制不循环；`state_remaining` 区分循环/一次性 |
| Death 终止 | 进入 Death 后 bridge 拒绝一切状态切换 |
| 过渡中同目标 | 同状态分支处理：换 clip + restart，不重新开始过渡 |
| 动作物理组合 | `PhysicsCombatAdapter`：`PhysicsWorld` + `FixedTimestep` + gameplay 查询适配器 |
| `CombatantId ↔ body_id` | `CombatantBodyMap`（一个 combatant 可挂多个 body，命中结果去重） |
| 单一积分来源 | gameplay motor 积分 + `move_capsule` 扫掠；physics 只推进动态刚体（不调用 physics motor） |

## 2. 每帧调用顺序（编辑器 Play 模式 / Player 完全一致）

```text
每帧 update(dt)：
  1. gameplay：motor.step(dt) -> MotorOutput
  2. gameplay：locomotion_state(&output, run_threshold) -> 基础 ActionState
     战斗/闪避/受击层可覆盖：ComboState.action_state() / Dodge / Block / Health
  3. adapter.bridge_mut() 交给 ActionAnimator：
       animator.sync(desired, adapter.bridge_mut())        // 基础状态去重
       animator.play_one_shot(attack_state, adapter.bridge_mut()) // 一次性动作（带 restart）
  4. let events = adapter.update(dt);   // 推进状态机 + 映射 gameplay 事件
  5. gameplay 消费 events：
       HitFrame            -> HitDetectionQuery 查询命中盒
       CancelWindowOpen/Close -> 连招窗口（可选，0.2 默认时间驱动）
       Footstep            -> 音频/特效
       HitReactionStart    -> 受击表现
  6. let pose = adapter.sample(&skeleton);
  7. let palette = JointPalette::from_pose(&skeleton, &skin, &pose);  // render 上传
```

编辑器额外注意：

- 从 Editing 切换到 Running（开始 Play）时：`ActionAnimator::reset()` + 重新注册 clip；
- 停止 Play 时：`adapter.restart()` 或重建 adapter，避免预览停在过渡中间；
- 动画预览面板可直接用 `adapter.drain_state_events()` 显示原始状态机事件（含 `TransitionFinished`）。

完整最小示例：

```rust
use hezhou_animation::{AnimationClip, Skeleton};
use hezhou_gameplay::{
    locomotion_state, ActionAnimator, ActionId, ActionState, MotorOutput,
};
use hezhou_integration::AnimationActionAdapter;

// 资源加载阶段：逐招式 clip（HeavyAttack 也走逐招式 id）
let mut adapter = AnimationActionAdapter::new().with_default_transition(0.12);
adapter
    .set_clip(hezhou_animation::AnimationState::Idle, AnimationClip::new("Idle", 2.0))
    .set_clip(hezhou_animation::AnimationState::Run, AnimationClip::new("Run", 0.8))
    .set_action_clip(ActionId::new(1), AnimationClip::new("Attack_1", 0.6))
    .set_action_clip(ActionId::new(2), AnimationClip::new("Attack_2", 0.7))
    .set_dodge_clip(AnimationClip::new("Dodge", 0.5))
    .set_block_clip(AnimationClip::new("Block", 1.0));

let mut animator = ActionAnimator::new();
let skeleton = Skeleton::new("hero");
let motor_output = MotorOutput::default();

// 每帧（伪代码）
let desired = locomotion_state(&motor_output, 4.0);
animator.sync(desired, adapter.bridge_mut());
let _events = adapter.update(1.0 / 60.0);
let _pose = adapter.sample(&skeleton);
```

## 3. 状态映射

`map_action_state`（`src/mapping.rs`）：

| gameplay `ActionState` | animation `AnimationState`（默认，task-11 已合入） | `--no-default-features` 兼容模式 |
|---|---|---|
| `Idle` | `Idle` | `Idle` |
| `Move` / `Run` | `Run` | `Run` |
| `Jump` / `Fall` | `Jump` | `Jump` |
| `Land` | `Land` | `Jump` |
| `Attack(ActionId)` / `HeavyAttack(ActionId)` | `Attack`（逐招式换 clip） | `Attack` |
| `Dodge` | `Dodge` | `Attack`（用专用 clip） |
| `Block` | `Block` | `Attack`（用专用 clip） |
| `Hit` | `Hit` | `Hit` |
| `Death` | `Death` | `Death` |

- animation 0.2 在 task-11 中已交付 Land/Dodge/Block 扩展状态（`AnimationState::ALL` 共 9 个），
  集成层默认开启 `extended-states` feature，做**一一映射**；
- 兼容分支保留在 `--no-default-features` 下（Land→Jump、Dodge/Block→Attack），
  用于旧 animation 版本/回归对照，保证本 crate 在扩展未合入时也能编译测试；
- 兼容模式下 `Dodge`/`Block` 复用 `Attack` 状态槽位，因此必须用
  `set_dodge_clip` / `set_block_clip` 指定专用 clip；未指定时会回退 `Attack` 槽位已绑定的 clip。
- 非攻击动作在事件里使用保留 `ActionId`（`reserved_action_ids`，约定 `< 32`）；
  游戏自定义招式请从 32 开始分配，避免冲突。

## 4. 逐招式 clip（"Attack 状态 + 按招式换片"）

0.2 的 `AnimationStateMachine` 每个状态只能绑定一个 clip，而动作游戏需要
"同一 Attack 状态播放不同招式"。集成层的做法是**切换前重绑 clip**：

```rust
// 方式 A：逐招式注册（推荐）
adapter.set_action_clip(ActionId::new(1), clip_attack_1);
adapter.set_action_clip(ActionId::new(2), clip_attack_2);
adapter.request_one_shot(ActionState::Attack(ActionId::new(2))); // 自动 set_clip(Attack, clip_attack_2)

// 方式 B：直接绑定状态槽位（兼容用法，所有招式共用同一 clip）
adapter.set_clip(hezhou_animation::AnimationState::Attack, clip_for_action);
adapter.request_one_shot(ActionState::Attack(ActionId::new(9)));
```

解析优先级：**逐招式 clip → 虚拟槽位（Land/Dodge/Block）→ 状态槽位**。
攻击类动作既没有逐招式 clip 也没有状态槽位 clip 时，`request_transition` 返回 `false`
（视为资产缺失），gameplay 的 `ActionAnimator` 会保持旧状态并重试/降级。

## 5. 事件映射

`AnimationEventMapper`（`src/event_map.rs`）：

| animation `StateEvent` | gameplay `AnimationEvent` |
|---|---|
| `Enter(state)` | `ActionStarted(id)`；`Hit` 状态额外 `HitReactionStart(id)` |
| `Exit(state)` | `ActionEnded(id)` |
| `TransitionStarted { from, to }` | `ActionEnded(from)` + `ActionStarted(to)`（与同批 `Enter/Exit` 去重，保证只派发一组） |
| `TransitionFinished { state }` | gameplay 0.2 无对应枚举：保留在 `drain_state_events()` 原始流 |
| `ClipEvent { state, event }` | 按下表映射 |

clip 事件名约定（来自动画资产命名，可用 `mapper_mut().register_rule(name, target)` 覆盖）：

| 事件名 | gameplay 事件 |
|---|---|
| `attack_hit`、`hit_frame`、`hit_2` | `HitFrame { action, hit_index }`（末尾数字为 hit_index） |
| `footstep`、`footstep_l`、`footstep_r` | `Footstep { foot }` |
| `cancel_window_open` / `cancel_open` / `combo_window_open` | `CancelWindowOpen(action)` |
| `cancel_window_close` / `cancel_close` | `CancelWindowClose(action)` |
| `hit_reaction` | `HitReactionStart(action)` |
| 其它未知事件 | 忽略并计数（`mapper().ignored_clip_event_count()`） |

`Attack` 状态被所有招式共用，映射器会记住"最近一次请求的攻击 `ActionId`"，
因此 `HitFrame { action }` 能准确指向触发它的招式；`HeavyAttack` 同样使用逐招式 id。

## 6. restart / 过渡中同目标 / Death

- **restart**：gameplay `ActionAnimator::play_one_shot` 对同一动作再次调用时，
  bridge 走"同 animation 状态"分支 → `machine.restart()`（片段时间归零）并补发
  `ActionStarted` 事件；进行中的过渡不被取消；
- **过渡中同目标**：状态机 `current` 在过渡开始时已切到目标，因此再次请求同一目标时
  bridge 不会重复开始过渡，只换 clip / 按需重启时间；
- **Death 终止**：`Death` 是 animation 的终止状态，bridge 在 `request_transition`
  入口直接拒绝之后的所有切换（包括再次进入 Death），`is_terminal()` 可用于
  停止输入/AI 并让 palette 停在最后一帧；
- **一次性动作不循环**：`ActionState::is_one_shot()`（Land/Attack/HeavyAttack/Dodge/Hit/Death）
  即使注册的 binding 标记循环也会被强制 `looping = false`；`Block` 是保持型动作，默认循环；
- **state_remaining**：一次性动作返回 `Some(duration - time)`；循环动作/未绑定 clip 返回
  `None`（表示由 gameplay 时间驱动），符合 gameplay `ActionStateBridge` 的约定。

## 7. physics ↔ gameplay 集成（`PhysicsCombatAdapter`）

gameplay-owner 已在 `hezhou-gameplay` 的 `physics` feature 中实现 `PhysicsQuery` /
`PhysicsHitQuery`（地面探测、胶囊扫掠、相机球扫、视线、命中盒），集成层**直接复用**这些
查询适配器，只负责组合与映射：

```text
integration::physics::PhysicsCombatAdapter
├── PhysicsWorld                 # 刚体/碰撞体/查询
├── FixedTimestep                # 固定步累加器（默认 60Hz）
├── CombatantBodyMap             # CombatantId ↔ body_id（一个 combatant 可多 body）
├── character/camera/hit filters # QueryFilter：层掩码/触发器/排除自身
└── spawn_* / sync_* / advance   # 场景搭建、角色 transform 同步、固定步推进
```

### 7.1 每帧顺序（单一积分来源）

```text
每帧 frame_dt：
  1. input -> gameplay CharacterMotor::step(fixed_dt, &input, &adapter.character_query_for(player))
       - motor 内部只调用 CharacterCollisionQuery::move_capsule / ground_probe 做扫掠
       - 位移/重力/跳跃速度全部由 gameplay motor 积分
  2. adapter.sync_character_transform(player, output.position)
       - 把 motor 的脚底位置写到角色 Kinematic body（physics 不参与角色积分）
  3. adapter.advance(frame_dt)
       - 按 FixedTimestep 只推进动态刚体（道具/敌人受击位移等）
       - 绝不调用 physics::CharacterMotor::step
  4. 命中帧：runtime.detect(&adapter.combat_hit_query_for(player), &hitbox, &pose)
       - 结果 target 已由 body_id 映射为业务 CombatantId，并按目标去重
  5. DamageEvent::from_contact(source, contact, amount, kind)
       -> resolve_damage(&mut health, block, dodge, forward, &event)
  6. 相机：camera.update(..., &adapter.camera_query_for(player)) 做防穿墙，
     锁定视线：adapter.camera_query_for(player).has_line_of_sight(from, to)
```

关键约定：

- **唯一积分来源**：角色运动 = gameplay `CharacterMotor`；physics `advance` 只处理动态刚体。
  角色 body 是 `Kinematic`，不会被 physics `step` 积分，位置只由 `sync_character_transform` 写入；
- **固定步**：`adapter.timestep().step()` 是 gameplay motor 每步的 `dt`；
  `adapter.advance(frame_dt).steps` 是本次物理应跑的步数，`alpha` 用于渲染插值；
- **过滤器透传**：`set_character_filter` / `set_camera_filter` / `set_hit_filter`（或
  `set_*_layer_mask`）直接传给 physics 查询；`*_query_for(combatant)` 会自动排除该
  combatant 的**第一个** body（`QueryFilter` 0.2 只支持排除单个 body）；
- **多部位命中盒**：一个 `CombatantId` 可注册多个 body，`CombatHitQuery` 按映射后的目标去重；
- **场景辅助**：`spawn_static_box[_with_layers]`、`spawn_dynamic_box[_with_layers]`、
  `spawn_sensor_sphere`、`spawn_character`。

最小用法：

```rust
use hezhou_gameplay::{CharacterMotor, CombatantId, MotorConfig, MotorInput, Vec3};
use hezhou_integration::PhysicsCombatAdapter;

let mut adapter = PhysicsCombatAdapter::new(); // 60Hz
adapter.spawn_static_box(Vec3::new(0.0, -0.5, 0.0), Vec3::new(60.0, 1.0, 60.0));
let player = CombatantId::new(1);
adapter.spawn_character(player, Vec3::new(0.0, 2.0, 0.0), 0.35, 1.8);

let mut motor = CharacterMotor::with_position(MotorConfig::default(), Vec3::new(0.0, 2.0, 0.0));
let input = MotorInput::default();
let fixed_dt = adapter.timestep().step();
for _ in 0..120 {
    let output = {
        let query = adapter.character_query_for(player);
        motor.step(fixed_dt, &input, &query)
    };
    adapter.sync_character_transform(player, output.position);
    adapter.advance(fixed_dt);
}
assert!(motor.is_grounded());
```

### 7.2 已知限制

1. **旋转形状**：physics 0.2 的 Box/胶囊命中盒均轴对齐，旋转命中盒偏保守（胶囊轴固定 Y 轴）；
2. **单碰撞体/刚体**：physics 0.2 一个 body 只挂一个 collider；多命中盒请拆多个 body 并注册到
   同一 `CombatantId`（集成层会去重）；
3. **多 body 排除自身**：`QueryFilter` 只支持排除单个 body，`*_query_for` 只自动排除第一个；
   更多部位请用层/掩码或自定义过滤器；
4. **动态刚体分离**：physics 0.2 的 `step` 只做速度/位置积分与简单碰撞检测，不做完整刚体解算、
   关节/CCD/布娃娃（0.3）。

## 8. 测试

```bash
cd engine
cargo test -p hezhou-integration
# 兼容模式（模拟 animation task-11 未合入的旧 6 状态映射）：
cargo test -p hezhou-integration --no-default-features
```

`tests/animation_gameplay_bridge.rs` 覆盖：

- 12 种 `ActionState` 的映射（含扩展 feature 两种分支）；
- 逐招式 clip 切换、HeavyAttack 走逐招式 clip、状态槽位 clip 兼容用法、缺 clip 拒绝；
- clip 事件透传（命中帧/连招窗口/脚步/未知事件计数）；
- Enter/Exit/Transition 事件去重、TransitionFinished 保留在原始流；
- Idle 循环回绕、Attack 一次性钳制、`state_remaining`；
- Death 终止后拒绝切换；
- 一次性动作 restart（时间归零 + 补发 ActionStarted）；
- 过渡中请求同一目标（换 clip、重启、不复位过渡）；
- gameplay `ActionAnimator` / `ActionStateBridge` / `AnimationEventSource` 真实接入；
- Land/Dodge/Block 兼容槽位（扩展开启后为一一映射）；
- 姿态采样（过渡混合、攻击片段推进）。

`tests/physics_combat_adapter.rs` 覆盖（复用 gameplay `physics` feature 的查询适配器）：

- 地面探测 / 胶囊扫掠（经 `PhysicsCombatAdapter` 的查询入口）；
- gameplay motor 只用 physics 扫掠落地，physics body 与 motor 位置一致；
- 角色查询排除自身 body、层掩码透传到命中查询；
- 相机防穿墙与视线遮挡；
- 命中盒 body→`CombatantId` 重映射、多 body 去重、未注册 body 回退；
- `HitboxRuntime` 同一窗口去重、新窗口重新命中；
- 固定步 60/144fps 一致性（固定步数对齐、同固定步数下轨迹一致）；
- 单一积分来源：physics step 不积分 Kinematic 角色，但推进动态道具；
- 命中盒 → `DamageEvent::from_contact` → `resolve_damage` 结算闭环；
- 映射表注册/注销/多 body 与默认查询过滤器。

## 9. 已知缺口（0.2）

1. **animation 扩展状态**：task-11 已合入，默认一一映射（Land/Dodge/Block）；
   `--no-default-features` 保留旧 6 状态兼容分支，仅用于回归对照；
2. **无 Walk 状态**：`Move` 与 `Run` 共用 `Run`；需要步行动画时只能注册不同的 Run clip 或等扩展；
3. **过渡打断**：过渡中切第三个状态时，animation 0.2 会直接结束旧过渡（不做"过渡的过渡"混合），
   集成层尊重该行为，不复刻 Blend Tree；
4. **TransitionFinished 无 gameplay 对应事件**：只暴露在原始事件流，留给编辑器/调试；
5. **根运动**：animation 0.2 不提取 root motion，位移仍由 `CharacterMotor` 驱动；
6. **GPU/ECS/FFI**：本 crate 不创建 Vulkan 资源、不新增 ECS 组件、不绑定 C#；
   palette 上传由 render 侧负责，C# API 由 Lead 集成阶段再接；
7. **事件名是命名约定**：资产必须按 §5 命名（或通过 `register_rule` 注册），
   否则事件会被忽略并计入 `ignored_clip_event_count`；
8. **physics 限制**：旋转盒/胶囊、单 body 单碰撞体、多 body 排除自身、
   完整刚体解算均受 physics 0.2 能力限制（详见 §7.2）。

## 10. 模块结构

```
engine/integration/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs           # crate 文档 + re-export + prelude
│   ├── mapping.rs       # ActionState → AnimationState、保留 ActionId、扩展 feature
│   ├── clip_registry.rs # 逐招式 / 状态槽位 / 虚拟槽位 clip 解析
│   ├── event_map.rs     # StateEvent → gameplay AnimationEvent
│   ├── bridge.rs        # ActionStateBridge + AnimationEventSource 实现
│   ├── adapter.rs       # AnimationActionAdapter（动画每帧封装）
│   └── physics.rs       # PhysicsCombatAdapter / CombatantBodyMap / CombatHitQuery
└── tests/
    ├── animation_gameplay_bridge.rs
    └── physics_combat_adapter.rs
```
