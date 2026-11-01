# PlayerGame — Hezhou Player 的 C# 游戏入口（0.2 task-30）

`engine/scripts/PlayerGame.cs` 是 **Player 程序集**（非编辑器 UI）的 C# 入口点。
Rust Player host 加载 C# 程序集后，可通过 Mono 反射/静态调用：

```csharp
namespace Hezhou
{
    public static class PlayerGame
    {
        public static void Initialize(IntPtr context); // Player 上下文（本轮忽略，保留签名）
        public static void Update(float dt);           // 每帧
        public static void Shutdown();                 // 退出
    }
}
```

## 设计约定

- **零依赖**：不引用 UI/FfiContext/EditorScript，Player 程序集可独立加载；
  与编辑器 DLL 同一次 `mcs` 编译（`build_mono.ps1 -All`），互不干扰。
- **幂等**：重复 `Initialize` 只告警不重置；未初始化时 `Update` 安全跳过，`Shutdown` 幂等。
- **健壮**：`dt` 为 NaN/∞/负数时按 0 处理；日志经 `SafeLog` 包装（`Console.Error` 失败不抛异常）。
- **可观测**：`IsInitialized` / `FrameCount` / `ElapsedSeconds` 只读属性；每 60 帧输出一次
  `[PlayerGame] frame=... elapsed=... avgDt=...`。
- **上下文预留**：`Initialize(IntPtr context)` 保存的 `_context` 目前未使用，后续通过它
  （或 FfiContext）暴露 gameplay/animation API，签名保持稳定。

## 构建与验证

```powershell
cd engine
powershell -ExecutionPolicy Bypass -File scripts/build_mono.ps1
# → scripts/bin/Mono/Release/net8.0/<prefix>_<ts>.dll
# → 部署 scripts/bin/Mono/EditorScript.dll
```

`PlayerGame.cs` 位于 `build_mono.ps1` 的 `$AllSourceFiles`（全量编译）；
`$UiSourceFiles`（`-UiOnly`）不包含它，确保 UI-only 编译不引入 Player 入口。
编译后可用 Mono 静态调用验证：

```bash
mono PlayerGameSmoke.exe   # 引用 EditorScript.dll，调用 Initialize/Update/Shutdown
```

## 已知限制 / 后续

1. 本轮 Player host 尚未通过 Mono 调用该入口（host/window 集成在 player 侧后续任务）；
2. `context` 未使用；gameplay/animation 的 API 需随 player context 契约一起定义；
3. 尚无脚本热重载/生命周期回调（Awake/Start/OnDestroy 等），后续按 Player 脚本模型扩展。
