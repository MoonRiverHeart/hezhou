# scripts

工程 C# 脚本目录。示例：

```csharp
using Hezhou.Engine;

public static class MyGame
{
    public static void Start()
    {
        Engine.Initialize();
        Entity cube = Scene.CreateCube();
        cube.Name = "Player";
        cube.Position = new Vector3(0f, 1f, 0f);
    }
}
```

> 0.4 限制：工程脚本加入 `EditorScript.dll` 的编译流程仍在规划中；当前示例脚本用于
> 结构说明，可参考 `engine/scripts/` 下已编译脚本（如 `RotatingEntity.cs`）的绑定方式。
