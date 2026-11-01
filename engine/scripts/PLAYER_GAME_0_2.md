# PlayerGame 0.2 — Player C# 入口与 PlayerCSharpContext ABI

**状态**：0.2 task-32（v1 Scene Context）/ task-33（v2 Transform/生命周期）/
task-34（v3 动画控制）  
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

## 2. PlayerCSharpContext ABI（version = 3）

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

### 2.4 版本兼容策略

- v2/v3 只在尾部追加字段，不改旧布局；C# 侧读取 `Version`：
  - `>= 3` 且 `animation_state != 0`：解析全部字段（动画 API 可用）；
  - `== 2`：解析 v1+v2，动画 API 跳过（v2 DLL/旧 Player 兼容）；
  - `== 1`：只解析 v1，rotation/scale/动画跳过；
  - 其他：记录日志后回退空转。
- 旧 **v2 DLL** 遇到 v3 context 属于“版本 > 支持范围”，会记录日志并安全回退
  （不崩溃、不误用尾部指针）；新 C# 对旧 v1/v2 context 自动降级。
- 未来扩展：**尾部追加函数指针 + version 递增**，旧读取方忽略新字段。

## 3. C# 用法（当前实现）

`Initialize(context)`：

1. `context == 0` → 记录日志后空转；
2. `Marshal.PtrToStructure` 解析，校验 version/scene/函数指针（v2 校验 6 个 Transform/
   生命周期指针；v3 额外校验 10 个动画指针 + `animation_state`）；
3. 调用 `EntityCount()` → `CreateCube()` → `SetEntityPosition(cube, 2.5, 0.5, -1.5)`
   → `SetRotation(cube, 0, 90, 0)` → `SetScale(cube, 1.5, 1.5, 1.5)`
   → `EntityCount()`（创建后）→ `GetPosition/GetRotation/GetScale`（读回）；
4. v3 动画：`AnimationClipCount()` 枚举 clip（名称/时长），
   `AnimationPlay(0)` → `AnimationSetTime(0.25)` → `AnimationSetSpeed(1.5)`，
   读回 current/time/speed；
5. 日志输出 `SceneContext: Initialize 创建 …`、`Animation: clips=N [0:name(duration), …]`、
   `Animation: play clip=0 … time=0.250 speed=1.500`（v1/v2 兼容时对应 API 为 -1/跳过）。

`Update(dt)`：每帧读取 `EntityCount()`/`GetPosition()`；v2 读取 `GetRotation()/GetScale()`
与 `EntityExists()`；v3 读取 `AnimationCurrent()/AnimationGetTime()/AnimationGetSpeed()`；
首帧与每 60 帧输出 `SceneContext: frame=…`、`Animation: frame=… current=… time=… speed=…`。

`Shutdown()`：

1. 读取最终实体数/位置；
2. v2：`EntityExists(cube)` → `DestroyEntity(cube)` → `EntityExists(cube)`；
3. v3：读取并输出最终 `Animation: Shutdown current=… time=… speed=…`；
4. 输出 `SceneContext: Shutdown entities=… existsBefore=1 destroyResult=0 existsAfter=0 …`，
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

## 5. 扩展点（后续 gameplay/animation/audio）

- **只加函数指针、不改已有布局**：在结构体尾部追加字段，`version` 递增；
  C# 侧校验 `Version` 后按新布局解析（旧 Player 传 v1 时 C# 回退到 v1 子集）。
- 建议后续 API（按同样 `extern "C"` + `catch_unwind` 约定）：
  - `scene_get_entity_transform`（一次取 T/R/S）、`scene_set_entity_visible`、
    `scene_set_entity_pbr_material`、`scene_get_entity_name/set_entity_name`
  - `scene_create_primitive`（cube/plane/sphere 等）、`scene_get_entity_count` 之外
    的层级/父子 API
  - 动画状态机/混合/事件（当前只有单 clip `play/set_time/set_speed`）、
    `animation_get_pose`/骨骼查询、`gameplay_*`、`audio_*`
  - `log` 扩展为带级别/上下文的日志（或直接用 `HEZHOU_PLAYER_MONO_LOG`）。
- 需要托管对象/字符串时通过 context 提供拷贝/释放函数，避免跨 ABI 裸指针生命周期问题。

## 6. 限制

- 本轮 context 覆盖 scene 的 count/create（cube/空实体）/position/rotation/scale/
  exists/destroy + v3 单 clip 动画（枚举/play/time/speed/playing）+ log；
  无材质/组件/层级/动画混合/状态机/物理/音频 API；
- 动画为**单 clip 切换**，无交叉淡化/混合/事件；时间循环 [0,duration]；
  speed 限制 [-8,8]；
- `Shutdown()` 默认销毁自己创建的 cube（用于验证生命周期）；如需保留可改为配置项；
- `scene`/`animation_state` 指针生命周期仅覆盖 Player 帧循环；C# 不得缓存到
  `Shutdown()` 之后；
- C# 侧异常/context 版本不符/函数指针缺失 → 记录日志后回退纯 Rust/空转，不崩溃；
  旧 v2 DLL 遇到 v3 context 会安全回退（不误用 v3 尾部）；
- `Update` 中 context 读取异常时 C# 会停止继续调用 context（安全回退）；
- 需要 Mono 运行时（系统安装或 Player 包内 `runtime/`）。
