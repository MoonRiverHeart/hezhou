# hezhou-gameplay

Hezhou 引擎的 **3D 动作游戏通用能力组件**（0.2 能力切片）。本 crate 只提供可复用的引擎能力，
不包含任何具体游戏的关卡、敌人或数值：招式、时间轴、伤害、生命值等数据全部由使用方（示例工程 /
游戏项目）提供。

- 纯 Rust、**默认零第三方依赖**：物理查询、动画状态机均通过 trait 注入；
- 所有能力都可以在无渲染 / 无物理 / 无动画 crate 的情况下做确定性单元测试；
- 提供可选 `physics` feature，把查询 trait 直接接到 `hezhou-physics`（默认关闭，不引入依赖）；
- 公开 API 使用英文命名，注释使用中文。

## 目录结构

```
engine/gameplay/
├── src/
│   ├── lib.rs                # 模块导出 + prelude
│   ├── math.rs               # 轻量 Vec3 / 角度工具（不依赖 core::math）
│   ├── ids.rs                # CombatantId
│   ├── query.rs              # 物理查询 trait + FlatGroundQuery 参考实现
│   ├── motor.rs              # CharacterMotor
│   ├── camera.rs             # ThirdPersonCamera + LockOnState
│   ├── animation_bridge.rs   # 与 animation 的状态机桥接 trait
│   ├── physics_adapter.rs    # 可选 physics 适配层（feature = "physics"）
│   └── combat/               # 连招、命中盒、伤害、生命、闪避、格挡、重攻击
├── tests/                    # 集成测试（含 mock 物理/动画实现）
└── examples/compose.rs       # 组合用法示例
```

## 能力与 API

### 1. CharacterMotor（角色移动）

`src/motor.rs`：移动 / 奔跑 / 跳跃 / 重力 / 地面吸附的纯逻辑状态机。
物理查询注入为 `&dyn CharacterCollisionQuery`，便于测试与替换实现。

```rust
use hezhou_gameplay::prelude::*;

let world = FlatGroundQuery::new(0.0);          // 或 physics 提供的实现
let mut motor = CharacterMotor::new(MotorConfig::default());
let input = MotorInput::with_direction(Vec3::FORWARD);
let output = motor.step(1.0 / 60.0, &input, &world);

println!("位置 {:?} 速度 {:?} 在地面={}", output.position, output.velocity, output.grounded);
```

| 类型 | 说明 |
|---|---|
| `MotorConfig` | 步行/奔跑速度、加减速、重力、跳跃初速、土狼时间、跳跃缓冲、地面吸附距离、最大坡度、胶囊尺寸 |
| `MotorInput` | 世界空间移动方向、冲刺、跳跃按下/按住、外部速度（闪避/前冲/击退）、速度倍率 |
| `MotorOutput` | 脚底位置、速度、是否在地面、起跳/落地事件、速度、移动强度（供动画） |
| `CharacterMotor` | `step` 推进、`teleport`、`set_velocity`、`position`/`velocity`/`is_grounded` |

特性：

- 加速度式移动，帧率无关（固定时间步下 60/120Hz 位移一致，有测试覆盖）；
- 土狼时间（`coyote_time`）+ 跳跃输入缓冲（`jump_buffer_time`）；
- 可变跳跃高度（松开跳跃键削减上升速度）；
- 地面吸附（下坡 / 小台阶），坡度上限由 `max_slope_angle_deg` 控制；
- 天花板、墙体接触法线解算；
- 外部速度通道不污染持久速度，适合闪避/攻击位移/击退。

### 2. ThirdPersonCamera + LockOnState（第三人称相机与锁定）

```rust
use hezhou_gameplay::prelude::*;

let mut camera = ThirdPersonCamera::new(ThirdPersonCameraConfig::default());
camera.apply_look(&CameraLookInput { yaw_delta: 10.0, pitch_delta: 0.0, zoom_delta: 0.0 });
let view = camera.update(dt, character_position, &camera_collision_query);

let mut lock_on = LockOnState::new();
let candidates = [LockTargetCandidate::new(CombatantId(2), enemy_position)];
lock_on.select(camera_position, camera.forward(), &candidates, &LockOnConfig::default(), Some(&visibility));
```

- `ThirdPersonCamera`：yaw/pitch 轨道、缩放、跟随平滑、右肩偏移、相机避障（`CameraCollisionQuery`）；
  `camera_relative_move` 把输入方向转成世界方向，`aim_at_point` 用于锁定平滑转向。
- `LockOnState`（导出别名 `LockOnTarget`）：按"前方夹角 + 距离"评分选目标，支持 `VisibilityQuery` 视线过滤、目标保持迟滞、
  超出 `break_distance` 自动解除，`aim_point` 提供瞄准点。
- `FlatGroundQuery` 是无遮挡的参考实现；正式项目用 physics 的射线/球体扫掠实现。

### 3. 战斗能力（`src/combat/`）

| 类型 | 能力 |
|---|---|
| `ComboGraph` / `AttackDefinition` | 招式数据：时间轴（startup/active/recovery）、连招窗口、后续段、命中盒、伤害、前冲速度 |
| `ComboState` / `ComboAction` | 连招状态机：起手、窗口判定、预输入缓冲、阶段（Startup/Active/Recovery/Finished）、`action_state`、`forward_push_velocity` |
| `Hitbox` / `HitboxShape` / `HitboxPose` | 命中盒数据（球/胶囊/盒 + 局部偏移 + 攻击者姿态） |
| `HitboxRuntime` + `HitDetectionQuery` | 命中查询与"一次攻击只命中同一目标一次"去重 |
| `DamageEvent` / `resolve_damage` | 伤害事件与结算管线（死亡 → 闪避 → 格挡 → 生命值） |
| `Health` | 生命值、死亡判定、回血/复活 |
| `Dodge` / `DodgeConfig` | 闪避状态机、无敌帧窗口、冷却、位移速度 |
| `Block` / `BlockConfig` | 格挡减伤、护盾消耗、破防硬直、护盾恢复、正面角度判定 |
| `HeavyAttack` / `HeavyAttackConfig` | 重攻击蓄力、最短/满蓄力、伤害倍率、蓄力期移速倍率 |

连招时间轴：

```text
|<-- startup -->|<-- active -->|<---- recovery ---->|
                              |<-- combo window -->|    下一段可在窗口内取消/衔接
```

```rust
use hezhou_gameplay::prelude::*;

let mut graph = ComboGraph::new();
graph.add_attack(
    AttackDefinition::new(AttackId(1), AttackKind::Light)
        .with_timing(0.12, 0.10, 0.25)
        .with_next([AttackId(2)])
        .with_damage(10.0, DamageKind::Light),
);
graph.add_attack(
    AttackDefinition::new(AttackId(2), AttackKind::Light)
        .with_timing(0.10, 0.08, 0.20)
        .as_combo_only(),
);

let mut combo = ComboState::new();
let _ = combo.press(AttackId(1), &graph, &ComboConfig::default());
let update = combo.update(dt, &graph);
if update.hitbox_active {
    // 1) 查询命中（physics 实现 HitDetectionQuery）
    // 2) definition.damage_events(...) 生成 DamageEvent
    // 3) resolve_damage(&mut health, Some(&mut block), Some(&dodge), forward, &event)
}
```

伤害结算顺序固定为：**已死亡 → 闪避无敌帧 → 格挡（是否可格挡/是否正面）→ 扣血**，
`DamageOutcome` 给出实际伤害、护盾消耗、破防、死亡、击退等结果。

### 4. 动画状态机桥接（`src/animation_bridge.rs`）

gameplay 不依赖 animation crate 的具体实现，只定义桥接契约：

```rust
pub trait ActionStateBridge {
    fn request_transition(&mut self, transition: ActionTransition) -> bool;
    fn state_remaining(&self) -> Option<f32> { None }
}

pub trait AnimationEventSource {
    fn drain_events(&mut self, out: &mut Vec<AnimationEvent>);
}
```

- `ActionState`：Idle / Move / Run / Jump / Fall / Land / Attack(ActionId) / HeavyAttack / Dodge / Block / Hit / Death；
- `ActionAnimator`：逻辑侧同步器，避免每帧重复请求；一次性动作（攻击/受击/闪避）带 `restart`；
- `locomotion_state(&MotorOutput, run_threshold)`：把移动输出映射为基础动画状态；
- `AnimationEvent`：命中帧、取消窗口、脚步、受击等，动画侧通过 `drain_events` 交给 gameplay 消费。

集成方向：animation crate（或集成层）实现 `ActionStateBridge` / `AnimationEventSource`，
gameplay 只持有 `&mut dyn ActionStateBridge`，不会产生对动画实现的编译期依赖。

### 5. 物理查询 trait（`src/query.rs`）

physics crate 完成自研动作物理后需要提供的能力（也可放在集成适配层）：

```rust
pub trait CharacterCollisionQuery {
    fn ground_probe(&self, feet: Vec3, radius: f32, max_distance: f32) -> Option<GroundHit>;
    fn move_capsule(&self, feet: Vec3, delta: Vec3, radius: f32, height: f32) -> CapsuleMoveResult;
}

pub trait CameraCollisionQuery {
    fn sweep_camera(&self, from: Vec3, to: Vec3, radius: f32) -> CameraSweep;
}

pub trait VisibilityQuery {
    fn has_line_of_sight(&self, from: Vec3, to: Vec3) -> bool;
}

pub trait HitDetectionQuery {
    fn query_hitbox(&self, hitbox: &Hitbox, pose: &HitboxPose) -> Vec<HitContact>;
}
```

实现约定：

- `GroundHit.point` 是脚底应吸附到的位置；`normal` 为世界空间单位法线；
- `CapsuleMoveResult.position` 是解算后的脚底位置，`contacts` 为本次扫掠接触面（法线指向角色外侧）；
- `sweep_camera` 返回 `to` 之前最近的安全位置，`hit=true` 表示发生遮挡；
- `query_hitbox` 返回与命中盒重叠的目标（同一目标只返回一次），命中盒局部坐标约定为
  x 右 / y 上 / z 前，绕 Y 轴随 `HitboxPose::forward` 旋转；
- `Vec3` 是 gameplay 自带类型，与 `hezhou_core::math::Vec3` 可通过 `[f32; 3]` 互转，
  适配层建议在 physics 实现中做一次行列转换。

## 测试

```bash
cd engine
cargo test -p hezhou-gameplay                    # 默认（零依赖）
cargo test -p hezhou-gameplay --features physics # 额外包含 physics 适配测试
```

默认共 **53 个测试**；启用 `physics` feature 后额外 **7 个适配测试**（合计 60）。覆盖：

- 移动/奔跑/跳跃/可变跳跃高度/土狼时间/跳跃缓冲/地面吸附/墙体/天花板/帧率无关/外部位移；
- 相机跟随/旋转/缩放/避障、锁定选择/背向过滤/视线过滤/迟滞/超距解除/转向；
- 连招时间轴与阶段、命中窗口、窗口内衔接、预输入缓冲、非法衔接、结束重置；
- 命中盒查询与去重、命中/受击/死亡、闪避无敌帧、格挡减伤/背向不格挡/破防/护盾恢复/不可格挡伤害；
- 重攻击蓄力倍率、取消与过早释放；
- 动画状态映射、桥接去重、一次性动作重启、动画事件拉取；
- physics 适配：地面探测/吸附点、胶囊落地、贴墙滑动、相机扫掠、视线遮挡、命中盒 body→CombatantId 映射、层/排除过滤器。

## 示例

```bash
cd engine
cargo run -p hezhou-gameplay --example compose
```

`examples/compose.rs` 把角色、相机、锁定、连招、命中结算、动画桥接串成一个 2 秒的固定时间步循环，
仅演示 API 组合方式。真实项目应把这些逻辑拆成 ECS 系统。

## 与现有 crate 的对接（集成点）

本 crate 不直接依赖 `hezhou-physics` / `hezhou-animation`，两者通过下面两层适配接入。

### physics → 查询 trait

本 crate 提供**可选** `physics` feature（默认关闭，默认测试保持零依赖）：
启用后 `physics_adapter` 模块把 4 个查询 trait 直接实现到 `hezhou-physics` 定稿 API 上：

```bash
cd engine
cargo test -p hezhou-gameplay --features physics
```

```rust
use hezhou_gameplay::physics_adapter::{PhysicsHitQuery, PhysicsQuery};

// 角色移动、相机、视线：注意排除角色自身刚体
let physics_query = PhysicsQuery::new(&physics_world)
    .with_layer_mask(hezhou_physics::layers::WORLD | hezhou_physics::layers::ENEMY)
    .exclude_body(player_body_id);
let output = motor.step(dt, &input, &physics_query);
let view = camera.update(dt, output.position, &physics_query);

// 攻击命中盒：body_id 自动映射为 CombatantId
let hit_query = PhysicsHitQuery::new(&physics_world)
    .with_layer_mask(hezhou_physics::layers::ENEMY);
let contacts = hit_runtime.detect(&hit_query, &hitbox, &pose);
```

physics 0.2 已提供（`engine/physics/src/query.rs`、`motor.rs`、`hitbox.rs`）：

| gameplay trait | physics 侧可用能力 |
|---|---|
| `CharacterCollisionQuery::ground_probe` | `PhysicsWorld::ground_probe` |
| `CharacterCollisionQuery::move_capsule` | `PhysicsWorld::move_capsule`（无重力/无台阶/无吸附，由 gameplay 负责手感） |
| `CameraCollisionQuery::sweep_camera` | `PhysicsWorld::sphere_cast` |
| `VisibilityQuery::has_line_of_sight` | `PhysicsWorld::raycast_filtered`（无命中即可见） |
| `HitDetectionQuery::query_hitbox` | `physics::Hitbox::overlaps` |

适配时把 `physics::Hit` / `OverlapHit` 的 `body_id` 映射为 `CombatantId`；
`hezhou_core::math::Vec3` 与 gameplay 的 `Vec3` 在 `physics_adapter` 中提供双向 `From`。
`physics::CharacterMotor` 与 gameplay 的 `CharacterMotor` 职责不同：前者负责扫掠/解算，
后者负责手感状态机（土狼时间、跳跃缓冲、地面吸附、冲刺等）；适配层**只调用
`PhysicsWorld::move_capsule`**，不调用 physics 的 `step`，因此不存在双重积分。

physics 0.2 已知限制（已在 `physics_adapter` 文档注明）：
- 旋转后的 Box/胶囊命中盒只能退化到"世界中心 + 轴对齐尺寸"，斜向盒偏保守；
- 胶囊轴固定世界 Y 轴，适用于直立角色；
- `HitboxPose::forward` 只参与偏移计算，不参与形状旋转。

### animation → 状态桥接

`hezhou-animation` 已提供 `AnimationState`（Idle/Run/Jump/Attack/Hit/Death 六状态）、
`AnimationStateMachine`、`AnimationStateDriver` 以及 `StateEvent`（含 `ClipEvent`）。
集成层需要实现 gameplay 的 `ActionStateBridge`，把 `ActionState` 映射到动画状态：

| gameplay `ActionState` | animation `AnimationState` |
|---|---|
| `Idle` | `Idle` |
| `Move` / `Run` | `Run`（Move 可由调用方决定是否降级到 Idle/Run） |
| `Jump` / `Fall` / `Land` | `Jump`（Land 建议由动画层做落地一次性过渡） |
| `Attack(ActionId)` / `HeavyAttack(ActionId)` | `Attack`（具体 clip 由 `AttackDefinition.action` 选择） |
| `Hit` | `Hit` |
| `Death` | `Death` |
| `Dodge` / `Block` | 需要 animation 侧扩展状态或复用 `Attack`/专用 clip 槽位 |

`ActionId` 对应动画资产中的 clip；由于 `AnimationState` 目前不含 Dodge/Block/逐招式区分，
集成层应保留"动画状态 + clip 选择"两层，或将动画状态扩展为与 `ActionState` 同构。
动画侧 `StateEvent::ClipEvent` 可映射为 gameplay 的 `AnimationEvent`（命中帧/脚步/取消窗口），
通过 `AnimationEventSource::drain_events` 交给战斗逻辑消费。

## 边界与非目标

- **不写具体游戏规则**：关卡、敌人 AI、招式数值、胜负条件属于示例工程；
- **不直接依赖** editor / C# / rendering / animation / physics 的具体实现；
- 物理查询与动画状态通过 trait 注入，接口变更由集成层适配；
- 本 crate 不负责状态同步/网络、存档、UI。
