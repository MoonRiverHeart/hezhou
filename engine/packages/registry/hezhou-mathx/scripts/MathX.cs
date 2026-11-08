// hezhou-mathx 示例脚本（工程脚本编译流程 0.4 未接入，仅作包内容演示）
public static class MathX
{
    public static float Clamp01(float value)
    {
        return value < 0f ? 0f : (value > 1f ? 1f : value);
    }
}
