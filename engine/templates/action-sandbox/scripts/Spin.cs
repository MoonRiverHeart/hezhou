// 示例工程脚本（action-sandbox 模板，0.4）
// 注意：当前工程脚本尚未加入 EditorScript.dll 的编译流程，此文件用于说明结构；
// 场景中 Cube 绑定的是已编译的内置脚本 RotatingEntity（engine/scripts/RotatingEntity.cs）。
using Hezhou.Engine;

public class Spin
{
    private float _speed = 45f;

    public void Start()
    {
        Debug.Log("Spin.Start");
    }

    public void Update(float dt)
    {
        _speed += 0f;
        Debug.Log("Spin.Update dt=" + dt);
    }
}
