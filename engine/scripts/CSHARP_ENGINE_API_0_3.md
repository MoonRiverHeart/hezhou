# C# 稳定引擎 API 外观（0.3 task-48）

`engine/scripts/EngineApi.cs` 提供 **`Hezhou.Engine`** 命名空间，让游戏脚本不直接依赖
`UI`/`UI.Scene` 等内部 C# FFI 包装。与 `EditorScript.dll` 同程序集（非独立 DLL），零新增 FFI。

## 1. 命名空间 / 类型表

| 类型 | 成员 | 说明 |
|---|---|---|
| `Hezhou.Engine.Engine` | `Version`（"0.3.0"）、`IsInitialized`、`Initialize()`、`Shutdown()`、`CurrentScenePtr`、`BindScene(IntPtr)`、`Log(string)` | 引擎入口；场景指针默认取编辑器现有场景，可显式绑定 |
| `Hezhou.Engine.Scene` | `EntityCount`、`CreateEntity()`、`CreateCube()`、`CreatePlane()`、`CreateDirectionalLight()`、`DestroyEntity(ulong/Entity)`、`EntityExists(ulong)`、`FindByName(string)`、`SetParent(ulong,ulong)`、`GetParent(ulong)` | 场景/实体 CRUD 与查询 |
| `Hezhou.Engine.Entity` | `Id`、`Name`、`Position`、`Rotation`（只读 Euler）、`Scale`、`ParentId`、`SetParent(Entity)`、`IsAlive`、`Visible`（本地字段） | 轻量实体句柄，属性即时读写场景 |
| `Hezhou.Engine.Vector3` | `X/Y/Z`、`ToString()` | 外观层简单值类型 |
| `Hezhou.Engine.Time` | `DeltaTime`、`FrameCount`、`ElapsedTime`、`Tick(float)`、`Reset()` | 由编辑器 `Update` 每帧驱动 |
| `Hezhou.Engine.Audio` | `PlayFirst(bool)`、`Pause()`、`Resume()`、`Stop()`、`SetVolume(float)`、`IsPlaying`、`Backend` | 封装 audio FFI；`Backend` 解析摘要（cpal/null/off） |
| `Hezhou.Engine.Animation` | `PlayClip(string)`、`SetClipTime(float)`、`State`、`TransitionAlpha` | 编辑器动画预览（状态机/时间轴） |
| `Hezhou.Engine.Debug` | `Log`/`LogWarning`/`LogError` | 转发 DFX `Log`（module="Game"） |

## 2. 示例：创建 Cube 并移动

```csharp
using Hezhou.Engine;

public static class GameEntry
{
    public static void Start()
    {
        Engine.Initialize();
        Entity cube = Scene.CreateCube();
        cube.Name = "Player";
        cube.Position = new Vector3(0f, 1f, 0f);
        cube.Scale = new Vector3(1.0f, 1.0f, 1.0f);

        Entity found = Scene.FindByName("Player");
        Debug.Log("found=" + (found != null ? found.Id.ToString() : "null"));

        Audio.SetVolume(0.8f);
        Debug.Log("audio backend=" + Audio.Backend);

        Animation.PlayClip("Idle");     // 需先在动画 Tab 初始化状态图
        Debug.Log("state=" + Animation.State + " alpha=" + Animation.TransitionAlpha);
    }
}
```

编辑器脚本可在 `Update` 中直接用 `Time.DeltaTime`（编辑器已在每帧自动 `Time.Tick`）。

## 3. 与 UI 内部 API 的边界

- **只做外观**：所有实际调用转发到现有 `UI.*`/`UI.Scene.*` 静态包装；不复制 FFI 逻辑、不新增 FFI 字段；
- **不破坏兼容**：现有 UI API 保持原样；游戏脚本新代码建议只用 `Hezhou.Engine`；
- **场景指针**：`Engine.CurrentScenePtr` 优先显式 `BindScene`，否则用 `UI.SceneGetExistingPtr()`
  （编辑器当前场景），因此编辑器内无需额外绑定；
- **时间**：`Time.Tick(deltaTime)` 已在 `EditorScript.Update` 每帧调用（一行接入），
  `FrameCount/ElapsedTime` 由 API 内累计；`Reset()` 供测试/场景重载。

## 4. 已知限制（0.3）

1. 位于 `EditorScript.dll` 内，**不是独立 DLL**；Player 独立 context/程序集后续统一。
2. `Entity.Visible` 尚无 C# FFI 包装：当前仅本地字段，**未同步渲染层**（后续补 `scene_*` 包装）。
3. `Entity.Rotation` 目前**只读**（从四元数换算 Euler）；缺少 C# setter 包装。
4. `Audio.PlayFirst` 播放 AssetDatabase 第一个音频资产（无按名/路径选择参数）；`IsPlaying` 解析摘要 voices。
5. `Animation` 依赖动画 Tab 的预览状态图（`animation_preview_init_graph`）；未初始化时 `State` 为空、alpha=0。
6. 无坐标系/单位转换（与编辑器一致：Y 上、单位为米、Euler 角度）。

## 5. 测试证据

- 编辑器启动自检：**PASS=102, FAIL=0**（task-47 的 96 + Engine API 6 项），证据
  （`engine/logs/hezhou_2026-11-08.log` 行 2736-2744）：
  - `Engine.Initialize/IsInitialized (Version=0.3.0)`
  - `Scene.CreateCube 实体数 +1 (id=5)`
  - `Name/Position/Scale + FindByName roundtrip`
  - `Time.Tick 更新 DeltaTime/FrameCount/ElapsedTime`
  - `Audio.Backend 非空且 Animation.TransitionAlpha 安全 (backend=cpal, alpha=0)`
  - `DestroyEntity 后 EntityExists=false`
- `build_mono.ps1` exit 0（DLL 418,304 字节 / 408.5 KB）；FFI contract 不变 **334/334/334 OK**；
  `ci_local.ps1 -SkipBuild` 通过（需可写 TMP/TEMP 供 glfw-sys CMake/MSBuild）。
