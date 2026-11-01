# hezhou-examples

本目录是 Hezhou 的示例/演示程序集合（编辑器、Vulkan/GLFW demo、Mono 热重载等）。
它们用于开发调试与能力验证，不是发布入口；安装版入口是 `hezhou_launcher.exe`。

## 0.2 能力演示工程（headless）：`capability_demo`

`src/capability_demo.rs` 是 0.2 引擎能力的**组合示例**，用来证明
`physics / gameplay / animation / integration / asset / audio / player` 六个模块
可以协同工作。它不是具体游戏：没有关卡、剧情、敌人 AI 或成品数值，示例中的
位置/伤害/时间轴只是为了让各能力模块产生可观测输出。

```bash
cd engine
# 运行（默认 6 秒 @60Hz，报告写到仓库根目录 test-results/capability-demo/report.json）
cargo run -p hezhou-examples --bin capability_demo

# 自定义步数/输出路径
cargo run -p hezhou-examples --bin capability_demo -- --steps 300 --out target/demo-report.json

# 只跑 demo 的 headless 单元测试（不打开窗口、不初始化 Vulkan）
cargo test -p hezhou-examples --bin capability_demo
```

退出码：全部能力项通过为 `0`，任一失败为 `1`，运行错误为 `2`。

### 组合了哪些能力

| 能力 | 用法 |
|---|---|
| physics | `PhysicsCombatAdapter`：地面/墙/角色/动态道具、固定 60Hz 步进、胶囊扫掠/相机球扫/raycast 查询 |
| gameplay | `CharacterMotor`、`ThirdPersonCamera`、`LockOnState`、`ComboGraph/ComboState`、`Health`、`resolve_damage` |
| animation | 程序化骨架 + clip + `AnimationStateMachine`，经 integration 驱动 |
| integration | `AnimationActionAdapter`（ActionState 映射、逐招式 clip、动画事件透传）+ `PhysicsCombatAdapter`（单一积分来源） |
| asset | 生成 fixture 工程 → `AssetDatabase` 扫描 → `Cooker` cook → `CookManifest` 读取与哈希校验 |
| audio | `AudioEngine::null()` 无声后端：加载/播放/暂停/恢复/停止、3D 播放（不需要声卡） |
| player | `PlayerBuilder` Windows+Linux dry-run 计划、`PlayerManifest` 落盘/回读校验 |

### 每帧顺序（60Hz 固定步）

```text
input -> motor.step(fixed_dt, &input, &adapter.character_query_for(player))
      -> adapter.sync_character_transform(player, output.position)
      -> combo.press/update（连招窗口）
      -> 判定窗口内 HitboxRuntime.detect(&adapter.combat_hit_query_for(player), ...)
      -> DamageEvent -> resolve_damage
      -> 动画：combat.action_state() / locomotion_state -> ActionAnimator -> ActionAdapter.update
      -> camera.update(..., &adapter.camera_query_for(player)) + LockOn
      -> adapter.advance(frame_dt)   // 只推进动态刚体；角色由 gameplay motor 积分
      -> audio.update(fixed_dt)
```

### 报告

`test-results/capability-demo/report.json`（相对仓库根目录）包含：

- `capabilities[]`：每项能力的 `pass/fail` 与细节；
- `physics`：角色最终位置/是否着地/相机是否避障/锁定目标/动态道具高度；
- `animation`：出现过的动画状态、命中帧事件、`ActionStarted` 事件；
- `combat`：起手/排队/连招衔接/命中/伤害/敌人剩余生命/攻击阶段；
- `asset`：fixture 工程、cook manifest、拷贝文件数、哈希校验结果；
- `audio`：后端、是否无声、播放 voice 数、主音量；
- `player`：manifest 校验、双平台计划文件数、指纹、启动场景；
- `passed` / `failures`：总结果。

同一份 fixture 在相同步数下，`combat/animation/physics/player.fingerprint` 是确定性的，
可直接用于 CI 回归。

### 边界

- 无窗口、无 GPU、无 Mono、无编辑器依赖，headless CI 可跑；
- 不修改任何能力模块源码，只通过公开 API 组合；
- 示例中的攻击时间轴/伤害/位置是演示数据，真实项目应替换为项目自己的配置。
