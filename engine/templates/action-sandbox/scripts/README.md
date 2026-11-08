# scripts

工程 C# 脚本目录。

- `Spin.cs`：示例脚本骨架（尚未加入 EditorScript.dll 编译流程）。
- 场景 Cube 绑定的是内置 `RotatingEntity`（`engine/scripts/RotatingEntity.cs`，已随编辑器编译）。

后续版本将支持工程脚本编译/热重载；当前建议通过 `Hezhou.Engine` API（见
`engine/scripts/CSHARP_ENGINE_API_0_3.md`）编写玩法逻辑。
