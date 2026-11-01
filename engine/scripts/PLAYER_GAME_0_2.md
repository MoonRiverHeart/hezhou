# PlayerGame 0.2 — Player C# 入口与 PlayerCSharpContext ABI

**状态**：0.2 task-32（Player C# Scene Context API）  
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

## 2. PlayerCSharpContext ABI（version = 1）

Rust：`#[repr(C)] PlayerCSharpContext`（`engine/player/src/csharp_context.rs`）  
C#：`[StructLayout(LayoutKind.Sequential)] PlayerCSharpContextNative`

| offset | Rust 字段 | C# 字段 | 类型 / delegate |
|---|---|---|---|
| 0  | `version: u32` | `Version` | `uint`（必须 = 1） |
| 4  | `reserved: u32` | `Reserved` | `uint`（0） |
| 8  | `scene: *mut Scene` | `Scene` | `IntPtr` |
| 16 | `scene_entity_count` | `SceneEntityCount` | `uint(IntPtr scene)` |
| 24 | `scene_create_cube` | `SceneCreateCube` | `ulong(IntPtr scene)` |
| 32 | `scene_create_entity` | `SceneCreateEntity` | `ulong(IntPtr scene)` |
| 40 | `scene_set_entity_position` | `SceneSetEntityPosition` | `int(IntPtr, ulong, float, float, float)` |
| 48 | `scene_get_entity_position` | `SceneGetEntityPosition` | `int(IntPtr, ulong, out float, out float, out float)` |
| 56 | `log` | `Log` | `void(IntPtr)`（C 字符串，可选） |

- C# delegates 必须标 `[UnmanagedFunctionPointer(CallingConvention.Cdecl)]`；
- 用 `Marshal.PtrToStructure` 读取原生结构，`Marshal.GetDelegateForFunctionPointer<T>` 转换函数指针；
- 函数返回：创建成功 = 实体 id（非 0），失败 = 0；set/get 成功 = 0，失败 = -1；
- Rust 侧所有函数 `extern "C"` + `catch_unwind`；空 scene/实体不存在/输出指针空 **不 panic**；
- `scene` 指针只在 Player 帧循环期间有效，C# 不得在 `Shutdown()` 后继续使用。

## 3. C# 用法（当前实现）

`Initialize(context)`：

1. `context == 0` → 记录日志后空转；
2. `Marshal.PtrToStructure` 解析，校验 version/scene/函数指针；
3. 调用 `EntityCount()` → `CreateCube()` → `SetEntityPosition(cube, 2.5, 0.5, -1.5)`
   → `EntityCount()`（创建后）→ `GetEntityPosition(cube)`；
4. 日志输出 `SceneContext: Initialize 创建 cube=… entities N->N+1 position=(…)`。

`Update(dt)`：每帧读取 `EntityCount()`/`GetEntityPosition()`；首帧与每 60 帧输出
`SceneContext: frame=… entities=… position=(…)`。

`Shutdown()`：读取最终实体数/位置后输出统计并重置静态状态。

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

本机实测（task-32，debug mono-host，cooked fixture 场景 3 实体，5 帧）：

```text
mono-host：Scene context 已提供（初始 3 实体，ABI v1，scene=0x...）
C# [PlayerGame] SceneContext: Initialize 创建 cube=1 setResult=0 getResult=0 entities 3->4 position=(2.500,0.500,-1.500)
C# [PlayerGame] SceneContext: frame=1 entities=4 cube=1 getResult=0 position=(2.500,0.500,-1.500)
C# [PlayerGame] SceneContext: Shutdown entities=4 cube=1 getResult=0 position=(2.500,0.500,-1.500)
mono-host：Shutdown 完成（…）；C# Shutdown 后 Scene 实体 4
窗口 host 结束：5 帧
```

## 5. 扩展点（后续 gameplay/animation/audio）

- **只加函数指针、不改已有布局**：在结构体尾部追加字段，`version` 递增；
  C# 侧校验 `Version` 后按新布局解析（旧 Player 传 v1 时 C# 回退到 v1 子集）。
- 建议后续 API（按同样 `extern "C"` + `catch_unwind` 约定）：
  - `scene_get_entity_transform` / `scene_set_entity_rotation/scale`
  - `scene_set_entity_visible` / `scene_set_entity_pbr_material`
  - `animation_play`/`animation_get_pose`、`gameplay_*`、`audio_*`
  - `log` 扩展为带级别/上下文的日志（或直接用 `HEZHOU_PLAYER_MONO_LOG`）。
- 需要托管对象/字符串时通过 context 提供拷贝/释放函数，避免跨 ABI 裸指针生命周期问题。

## 6. 限制

- 本轮仅 scene 的 count/create/position 五个操作 + log；无 transform 旋转/缩放、
  无删除实体、无材质/组件/动画/物理/音频 API；
- `scene` 指针生命周期仅覆盖 Player 帧循环；C# 不得缓存到 `Shutdown()` 之后；
- C# 侧异常/context 版本不符/函数指针缺失 → 记录日志后回退纯 Rust/空转，不崩溃；
- `Update` 中 context 读取异常时 C# 会停止继续调用 context（安全回退）；
- 需要 Mono 运行时（系统安装或 Player 包内 `runtime/`）。
