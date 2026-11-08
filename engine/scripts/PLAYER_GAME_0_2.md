# PlayerGame 0.2 — Player C# 入口与 PlayerCSharpContext ABI

**状态**：0.2 task-32（v1 Scene Context）/ task-33（v2 Transform/生命周期）/
task-34（v3 动画控制）/ task-35（v4 资产/音频）/ task-36（v5 角色控制）/
task-37（v6 9 状态动画状态机）  
**文件**：`engine/scripts/PlayerGame.cs`、`engine/player/src/csharp_context.rs`  
**Rust host**：`hezhou-player --features mono-host`（`WindowPlayerHost`）

## 1. 入口

C# 无 `Main()`，Player 通过 Mono 静态调用：

```csharp
namespace Hezhou
{
    public static class PlayerGame
    {
        public static void Initialize(IntPtr context); // PlayerCSharpContext（可为 0）
        public static void Update(float dt);           // 每帧
        public static void Shutdown();                 // 退出
    }
}
```

- `Initialize`/`Shutdown` 幂等；重复调用安全；
- `context == IntPtr.Zero`（旧 Player/无 Scene）时保持安全空转，不抛异常；
- 所有日志走 `Console.Error`，并可选追加到环境变量 `HEZHOU_PLAYER_MONO_LOG`。

## 2. PlayerCSharpContext ABI（version = 6）

Rust：`#[repr(C)] PlayerCSharpContext`（`engine/player/src/csharp_context.rs`）  
C#：`[StructLayout(LayoutKind.Sequential)] PlayerCSharpContextNative`

### 2.1 v1 基础字段（顺序/偏移不变）

| offset | Rust 字段 | C# 字段 | 类型 / delegate |
|---|---|---|---|
| 0  | `version: u32` | `Version` | `uint`（当前 = 2） |
| 4  | `reserved: u32` | `Reserved` | `uint`（0） |
| 8  | `scene: *mut Scene` | `Scene` | `IntPtr` |
| 16 | `scene_entity_count` | `SceneEntityCount` | `uint(IntPtr scene)` |
| 24 | `scene_create_cube` | `SceneCreateCube` | `ulong(IntPtr scene)` |
| 32 | `scene_create_entity` | `SceneCreateEntity` | `ulong(IntPtr scene)` |
| 40 | `scene_set_entity_position` | `SceneSetEntityPosition` | `int(IntPtr, ulong, float, float, float)` |
| 48 | `scene_get_entity_position` | `SceneGetEntityPosition` | `int(IntPtr, ulong, out float, out float, out float)` |
| 56 | `log` | `Log` | `void(IntPtr)`（C 字符串，可选） |

### 2.2 v2 tail 追加（task-33，保持 v1 布局）

| offset | Rust 字段 | 类型 / delegate |
|---|---|---|
| 64 | `scene_set_entity_rotation` | `int(IntPtr, ulong, float x, float y, float z)`（欧拉角，度） |
| 72 | `scene_get_entity_rotation` | `int(IntPtr, ulong, out float, out float, out float)`（欧拉角，度） |
| 80 | `scene_set_entity_scale` | `int(IntPtr, ulong, float, float, float)` |
| 88 | `scene_get_entity_scale` | `int(IntPtr, ulong, out float, out float, out float)` |
| 96 | `scene_entity_exists` | `int(IntPtr, ulong)`（1 存在 / 0 不存在） |
| 104 | `scene_destroy_entity` | `int(IntPtr, ulong)`（0 成功 / -1 失败） |

- C# delegates 必须标 `[UnmanagedFunctionPointer(CallingConvention.Cdecl)]`；
- 用 `Marshal.PtrToStructure` 读取原生结构，`Marshal.GetDelegateForFunctionPointer<T>` 转换函数指针；
- 函数返回：创建成功 = 实体 id（非 0），失败 = 0；set/get 成功 = 0，失败 = -1；
  exists 返回 1/0；destroy 返回 0/-1；
- rotation 为**欧拉角（度）**，与 core `Quaternion::from_euler_degrees`/`to_euler_degrees` 对应；
- Rust 侧所有函数 `extern "C"` + `catch_unwind`；空 scene/实体不存在/输出指针空 **不 panic**；
- `scene` 指针只在 Player 帧循环期间有效，C# 不得在 `Shutdown()` 后继续使用。

### 2.3 v3 tail 追加（task-34，保持 v1/v2 布局）

`animation_state` 指向 host 栈上的 `PlayerAnimationState`（窗口 host 维护 clip 列表/
当前索引/时间/速度/playing）；无动画时为 0 或空状态，所有函数安全返回 0/-1/NaN。

| offset | Rust 字段 | 类型 / delegate |
|---|---|---|
| 112 | `animation_state` | `IntPtr`（0 = 无动画） |
| 120 | `animation_clip_count` | `uint(IntPtr state)` |
| 128 | `animation_clip_name` | `int(IntPtr, uint index, u8* buf, uint len)`（两段式：buf=0 返回所需字节数） |
| 136 | `animation_clip_duration` | `float(IntPtr, uint index)`（失败 -1） |
| 144 | `animation_current` | `int(IntPtr)`（当前 clip 下标；无 -1） |
| 152 | `animation_play` | `int(IntPtr, uint index)`（0/-1） |
| 160 | `animation_set_time` | `int(IntPtr, float seconds)`（0/-1，clamp 到 [0,duration]） |
| 168 | `animation_get_time` | `float(IntPtr)`（失败 NaN） |
| 176 | `animation_set_speed` | `int(IntPtr, float speed)`（0/-1，clamp [-8,8]） |
| 184 | `animation_get_speed` | `float(IntPtr)`（失败 NaN） |
| 192 | `animation_set_playing` | `int(IntPtr, int playing)`（0/-1） |

窗口 host 每帧 `PlayerAnimationState::advance(dt)`，再用当前 clip/time 调用
`SelectedSkinnedAsset::palette_for_clip()` 生成 joint palette 上传 GPU；C# 设置
clip/time/speed 后下一帧 palette 即由新状态驱动。

### 2.4 v4 tail 追加（task-35，保持 v1/v2/v3 布局）

| offset | Rust 字段 | 类型 / delegate |
|---|---|---|
| 200 | `assets` | `IntPtr` → `PlayerAssetCatalog`（可空） |
| 208 | `asset_count` | `uint(IntPtr assets)` |
| 216 | `asset_id` | `int(IntPtr, uint index, u8* buf, uint len)`（UTF-8 UUID） |
| 224 | `asset_kind` | `int(IntPtr, uint index, u8* buf, uint len)`（`scene`/`audio`/…） |
| 232 | `asset_path` | `int(IntPtr, uint index, u8* buf, uint len)`（包内 output） |
| 240 | `audio` | `IntPtr` → `PlayerAudioRuntime`（可空） |
| 248 | `audio_play_first` | `int(IntPtr)`（0/-1） |
| 256 | `audio_pause` | `int(IntPtr)`（0/-1） |
| 264 | `audio_resume` | `int(IntPtr)`（0/-1） |
| 272 | `audio_stop` | `int(IntPtr)`（0/-1） |
| 280 | `audio_set_volume` | `int(IntPtr, float volume)`（0/-1） |
| 288 | `audio_is_playing` | `int(IntPtr)`（1/0） |
| 296 | `audio_backend` | `int(IntPtr, u32 unused, u8* buf, uint len)`（UTF-8，如 `null`/`cpal`） |

资产目录来自 `PlayerAssetIndex`（包含 glTF 子资产）；音频运行时默认 **null backend**
（未启用 `audio-cpal`），只加载/播放包内第一个 `AssetKind::Audio`（WAV/OGG），
每帧 `engine.update(dt)` 推进状态机。C# 字符串均采用两段式：`buf=0/len=0` 返回所需字节数。

### 2.5 v5 tail 追加（task-36，保持 v1..v4 布局）

| offset | Rust 字段 | 类型 / delegate |
|---|---|---|
| 304 | `gameplay_world` | `IntPtr` → `PlayerGameplayRuntime`（可空） |
| 312 | `character_create` | `int(IntPtr, ulong entity, float radius, float total_height)`（0/-1） |
| 320 | `character_step` | `int(IntPtr, ulong entity, float input_x, float input_z, int jump, float dt)`（0/-1） |
| 328 | `character_grounded` | `int(IntPtr, ulong entity)`（1/0） |
| 336 | `character_position` | `int(IntPtr, ulong entity, out float x, out float y, out float z)`（0/-1） |
| 344 | `character_remove` | `int(IntPtr, ulong entity)`（0/-1） |

角色运行时内部为 `hezhou_physics::PhysicsWorld` + y=0 地面无限 `Plane` + 每 entity 一个
`CharacterMotor`（胶囊）；`create` 从 Scene transform 取脚底位置，`step` 的
input_x/input_z 是水平期望速度（m/s）、y 由重力/跳跃接管，结果脚底位置写回 Scene。

### 2.6 v6 tail 追加（task-37，保持 v1..v5 布局）

| offset | Rust 字段 | 类型 / delegate |
|---|---|---|
| 352 | `animation_state_machine` | `IntPtr` → `PlayerAnimationGraph`（可空） |
| 360 | `animation_set_state` | `int(IntPtr, const char* name)`（0/-1） |
| 368 | `animation_trigger` | `int(IntPtr, const char* name)`（0/-1；同状态 restart） |
| 376 | `animation_current_state` | `int(IntPtr, u8* buf, uint len)`（UTF-8 状态名） |
| 384 | `animation_is_transitioning` | `int(IntPtr)`（1/0） |
| 392 | `animation_transition_alpha` | `float(IntPtr)`（失败 NaN） |

`PlayerAnimationGraph` 启用 `hezhou-asset/animation-bridge`：`imported_scene_to_gltf_asset`
→ `into_runtime()` 得到运行时 Skeleton/Skin/AnimationClip，按 clip 名（大小写不敏感）
映射 9 状态（Idle/Run/Jump/Attack/Hit/Death/Land/Dodge/Block）；无匹配时 Idle 用首个
clip、其余状态空 clip → 绑定姿态。窗口 host 每帧 `graph.update(dt)` → `sample()` →
`JointPalette::from_pose` → 上传 palette。

### 2.7 版本兼容策略

- v2..v6 只在尾部追加字段，不改旧布局；C# 侧读取 `Version`：
  - `>= 6` 且 `animation_state_machine != 0`：全部 API（含状态机）；
  - `== 5`：v1..v5，状态机跳过；
  - `== 4`：v1..v4，角色/状态机跳过；
  - `== 3`：v1..v3，资产/音频/角色/状态机跳过；
  - `== 2`：v1+v2，动画/资产/音频/角色/状态机跳过；
  - `== 1`：只解析 v1（兼容旧 Player/DLL）；
  - 其他：记录日志后回退空转。
- 旧 DLL 遇到更高 version 属于“版本 > 支持范围”，会记录日志并安全回退
  （不崩溃、不误用尾部指针）；新 C# 对旧 v1..v5 context 自动降级。
- 未来扩展：**尾部追加函数指针 + version 递增**，旧读取方忽略新字段。

## 3. C# 用法（当前实现）

`Initialize(context)`：

1. `context == 0` → 记录日志后空转；
2. `Marshal.PtrToStructure` 解析，校验 version/scene/函数指针（v2 校验 6 个 Transform/
   生命周期指针；v3 校验 10 个动画指针 + `animation_state`；v4 校验 4 个资产 +
   7 个音频指针 + `assets`/`audio`）；
3. 调用 `EntityCount()` → `CreateCube()` → `SetEntityPosition(cube, 2.5, 0.5, -1.5)`
   → `SetRotation(cube, 0, 90, 0)` → `SetScale(cube, 1.5, 1.5, 1.5)`
   → `EntityCount()`（创建后）→ `GetPosition/GetRotation/GetScale`（读回）；
4. v3 动画：`AnimationClipCount()` 枚举 clip（名称/时长），
   `AnimationPlay(0)` → `AnimationSetTime(0.25)` → `AnimationSetSpeed(1.5)`，
   读回 current/time/speed；
5. v4 资产：`AssetCount()` + 前 8 个 `AssetIdString/AssetKindString/AssetPathString`，
   日志 `Assets: count=N [i:kind:path:id, …]`；
6. v4 音频：`AudioBackendString()` → `AudioPlayFirst()` → `AudioPause()` →
   `AudioIsPlaying()` → `AudioResume()` → `AudioIsPlaying()` → `AudioSetVolume(0.5)`，
   日志 `Audio: backend=null playResult=0 … isPlayingAfterResume=1 volume=0.500`；
7. v5 角色：`CharacterCreate(cube, 0.3, 1.8)`（地面 y=0 胶囊角色），日志
   `Character: create entity=… radius=0.300 height=1.800 result=0 position=(…) grounded=…`；
8. v6 状态机：读取 `AnimationCurrentStateString()`/`AnimationIsTransitioning()`/
   `AnimationTransitionAlpha()`，日志 `AnimGraph: initial state=Idle …`；
9. 日志输出 `SceneContext: Initialize 创建 …`、`Animation: clips=N`、
   `Assets: count=…`、`Audio: backend=…`、`Character: create …`、`AnimGraph: initial …`。

`Update(dt)`：每帧读取 `EntityCount()`/`GetPosition()`；v2 读取 `GetRotation()/GetScale()`
与 `EntityExists()`；v3 读取 `AnimationCurrent()/AnimationGetTime()/AnimationGetSpeed()`；
v4 读取 `AudioIsPlaying()`；v5 每帧调用 `CharacterStep(cube, 1.0, 0.0, false, dt)`
（+X 1m/s，无跳跃）并读取 `CharacterPosition/CharacterGrounded`；v6 在 frame1 触发
`Run`、frame15/每 60 帧触发 `Attack`，读取 current/transitioning/alpha；首帧与每 60 帧
输出 `SceneContext: frame=…`、`Animation: frame=…`、`Audio: frame=…`、
`Character: frame=…`、`AnimGraph: trigger=…` / `AnimGraph: frame=… state=… alpha=…`。

`Shutdown()`：

1. v6：读取并输出 `AnimGraph: Shutdown state=… transitioning=… alpha=…`；
2. v5：读取角色位置/grounded → `CharacterRemove`，日志 `Character: Shutdown … removeResult=0`；
3. 读取最终实体数/位置；
4. v2：`EntityExists(cube)` → `DestroyEntity(cube)` → `EntityExists(cube)`；
5. v3：读取并输出最终 `Animation: Shutdown …`；
6. v4：`AudioIsPlaying()` → `AudioStop()` → `AudioIsPlaying()`，日志 `Audio: Shutdown …`；
7. 输出 `SceneContext: Shutdown entities=… existsBefore=1 destroyResult=0 existsAfter=0 …`，
   重置静态状态。

## 4. 构建/运行

```bash
cd engine
# 编译 C#（包含 PlayerGame.cs；mcs）
powershell -ExecutionPolicy Bypass -File scripts/build_mono.ps1

# 构建 Player（需 Mono 依赖 + 窗口 host）
cargo build -p hezhou-player --features mono-host

# 打包（scripts 目录放入编译产物）
./target/debug/hezhou-player-build --config <build-config.json> --cook <cook_manifest.json> \
  --scripts-dir <dir with PlayerGame/EditorScript.dll> --runtime-dir <runtime> \
  --exe ./target/debug/hezhou-player --build-out ../dist/player

# 运行（窗口 + C# 调用；HEZHOU_PLAYER_WINDOW_FRAMES 控制帧数）
HEZHOU_PLAYER_WINDOW_FRAMES=30 ../dist/player/windows/<exe> --run --host window \
  --manifest ../dist/player/windows/manifest.json --log-dir ../dist/logs
```

本机实测（task-33，debug mono-host，cooked fixture 场景 3 实体，30 帧）：

```text
mono-host：Scene context 已提供（初始 3 实体，ABI v2，scene=0x...）
C# [PlayerGame] SceneContext: Initialize 创建 cube=4 ... entities 3->4 position=(2.500,0.500,-1.500)
                 rotationSet=0 rotationGet=0 rotation=(0.000,89.980,0.000)
                 scaleSet=0 scaleGet=0 scale=(1.500,1.500,1.500)
C# [PlayerGame] SceneContext: frame=1 entities=4 ... position=(2.500,0.500,-1.500) rotation=(0.000,89.980,0.000) scale=(1.500,1.500,1.500)
C# [PlayerGame] SceneContext: Shutdown entities=3 cube=4 existsBefore=1 destroyResult=0 existsAfter=0 ...
mono-host：Shutdown 完成（…）；C# Shutdown 后 Scene 实体 3
窗口 host 结束：30 帧
```

本机实测（task-34，debug mono-host，skinned fixture 1 个 `Spin` clip、30 帧，
ABI v3 动画控制）：

```text
mono-host：蒙皮动画运行时：clip 0/1 `Spin` t=0.000/1.000s speed=1.00 playing=true
C# [PlayerGame] Animation: clips=1 [0:Spin(1.000s)]
C# [PlayerGame] Animation: play clip=0 playResult=0 setTimeResult=0 time=0.250 setSpeedResult=0 speed=1.500
C# [PlayerGame] Animation: frame=1 current=0 time=0.600 speed=1.500
C# [PlayerGame] Animation: Shutdown current=0 time=0.335 speed=1.500
mono-host：蒙皮绘制统计：3 顶点 / 1 三角形 / 2 关节；palette 更新 30 次（动画驱动）；动画 clip 0/1 `Spin` t=0.335 speed=1.50 playing=true
窗口 host 结束：30 帧
```

即 C# 通过 ABI v3 枚举 1 个 clip、播放并设置 time=0.25s/speed=1.5；窗口 host
每帧按该状态推进时间并重算 palette（实际 t 随帧变化）。

本机实测（task-35，debug mono-host，skinned fixture + 1s WAV，30 帧，ABI v4 资产/音频）：

```text
mono-host：资产目录：6 条（首个 scene / assets/scenes/...）
mono-host：音频引擎：backend=null（silent=true）
mono-host：音频：加载首个 Audio 资产 dddddddd-...（backend=null，source=.../assets/audio/test.wav）
C# [PlayerGame] Assets: count=6 [0:scene:...:1111..., ..., 5:audio:assets/audio/test.wav:dddd...]
C# [PlayerGame] Audio: backend=null playResult=0 pauseResult=0 isPlayingAfterPause=0
                 resumeResult=0 isPlayingAfterResume=1 volumeResult=0 volume=0.500
C# [PlayerGame] Audio: frame=1 isPlaying=1
C# [PlayerGame] Audio: Shutdown isPlaying=1 stopResult=0 isPlayingAfterStop=0
mono-host：音频统计：音频：backend=null（silent=true）loaded=dddd... state=stopped volume=0.50 ...
窗口 host 结束：30 帧
```

即 C# 枚举 6 个资产（含 WAV），通过 null backend 播放/暂停/恢复/设音量/停止音频；
默认构建无需声卡，`audio-cpal` feature 可切真实 cpal 输出（失败自动降级 null）。

本机实测（task-36，debug mono-host，cooked fixture，30 帧，ABI v5 角色控制）：

```text
C# [PlayerGame] Character: create entity=4 radius=0.300 height=1.800 result=0
                 posResult=0 position=(2.500,0.500,-1.500) grounded=0
C# [PlayerGame] Character: frame=1 stepResult=0 posResult=0
                 position=(2.676,0.197,-1.500) grounded=0
C# [PlayerGame] Character: Shutdown posResult=0 position=(3.169,0.006,-1.500)
                 grounded=1 removeResult=0
player.log: 角色运行时：0 个角色，地面 body=1
            角色统计：角色运行时：0 个角色，地面 body=1
            窗口 host 结束：30 帧，687.3 ms
run_exit=0
```

即 C# 通过 v5 为 cube 建立胶囊角色，每帧 `step(+X 1m/s)` 并写回 Scene；
30 帧内从 (2.5,0.5,-1.5) 移动到 (3.169,0.006,-1.5)（落地 grounded=1，位移 +0.669m）。

本机实测（task-37，debug mono-host，skinned fixture 3 clip（Idle/Run/Attack），30 帧，
ABI v6 9 状态机）：

```text
player.log: 动画状态图：3 个 clip / 9 状态（映射 3），current=Idle ...
C# [PlayerGame] AnimGraph: initial state=Idle transitioning=0 alpha=1.000
C# [PlayerGame] AnimGraph: trigger=Run result=0
C# [PlayerGame] AnimGraph: frame=1 state=Run transitioning=1 alpha=0.000
C# [PlayerGame] AnimGraph: trigger=Attack result=0
C# [PlayerGame] AnimGraph: frame=15 state=Attack transitioning=1 alpha=0.000
C# [PlayerGame] AnimGraph: Shutdown state=Attack transitioning=0 alpha=1.000
player.log: 动画状态图统计：... current=Attack time=0.253s ... transitions=4
player.log: 蒙皮绘制统计：... palette 更新 30 次（动画驱动）；动画 clip 0/3 `Idle` ...
            窗口 host 结束：30 帧，723.4 ms
run_exit=0
```

即 9 状态状态机按 clip 名映射 3 个状态，C# 触发 Run→Attack，每帧 update/sample/palette
由状态图驱动；真实窗口渲染退出 0。

## 5. 扩展点（后续 gameplay/animation/audio）

- **只加函数指针、不改已有布局**：在结构体尾部追加字段，`version` 递增；
  C# 侧校验 `Version` 后按新布局解析（旧 Player 传 v1 时 C# 回退到 v1 子集）。
- 建议后续 API（按同样 `extern "C"` + `catch_unwind` 约定）：
  - `scene_get_entity_transform`（一次取 T/R/S）、`scene_set_entity_visible`、
    `scene_set_entity_pbr_material`、`scene_get_entity_name/set_entity_name`
  - `scene_create_primitive`（cube/plane/sphere 等）、`scene_get_entity_count` 之外
    的层级/父子 API
  - 动画事件消费/自定义混合树/遮罩/IK/根运动（当前 9 状态机仅状态切换+过渡，
    未把 clip 事件接入 gameplay）；
  - 角色扩展：斜坡/台阶（0.2 已含基础 step height）/多地形/输入加速度/
    相机方向/跳跃参数/动画混合；
  - `audio_play_named`/3D 位置/总线/循环（当前仅首个音频 2D play_first）；
  - `log` 扩展为带级别/上下文的日志（或直接用 `HEZHOU_PLAYER_MONO_LOG`）。
- 需要托管对象/字符串时通过 context 提供拷贝/释放函数，避免跨 ABI 裸指针生命周期问题。

## 6. 限制

- 本轮 context 覆盖 scene 的 count/create（cube/空实体）/position/rotation/scale/
  exists/destroy + v3 单 clip 动画 + v4 资产目录枚举/音频控制 + v5 角色控制
  + v6 9 状态动画状态机（set_state/trigger/current/transitioning/alpha）+ log；
  无材质/组件/层级/物理 API；
- 状态机按 clip 名称关键词映射 9 状态；无匹配 clip 时 Idle 用首个 clip、其余状态
  空 clip 回退绑定姿态；无动画事件消费/自定义混合树/遮罩/IK/根运动；
- 角色为单个无限地面 Plane + 直立胶囊，input_x/input_z 为直接速度（无加速度/相机方向），
  无复杂地形/斜坡限制之外的碰撞体；每帧 step 的 dt ≤ 0/非有限值安全返回错误；
- 音频仅加载/播放第一个 WAV/OGG（2D，无循环/3D 位置/总线）；默认 null backend
  （无真实声音），启用 `audio-cpal` feature 才尝试真实输出设备；加载失败安全空转；
- 单 clip 时间/速度 API（v3）仍保留用于诊断；状态图驱动 palette 时以 v6 状态为准；
- `Shutdown()` 默认移除角色、销毁自己创建的 cube、停止音频；
- `scene`/`animation_state`/`assets`/`audio`/`gameplay_world`/`animation_state_machine`
  指针生命周期仅覆盖 Player 帧循环；C# 不得缓存到 `Shutdown()` 之后；
  `Shutdown()` 之后；
- C# 侧异常/context 版本不符/函数指针缺失 → 记录日志后回退纯 Rust/空转，不崩溃；
  旧 v2 DLL 遇到 v3 context 会安全回退（不误用 v3 尾部）；
- `Update` 中 context 读取异常时 C# 会停止继续调用 context（安全回退）；
- 需要 Mono 运行时（系统安装或 Player 包内 `runtime/`）。
