# Hezhou 0.3.0 发布说明

> 状态：Release Candidate / 待双平台 CI 验收。基线：0.2.0。

## Highlights

- 编辑器工具链第一批：Profiler、Build Settings、动画时间轴、9 状态动画预览、材质 albedo 纹理编辑
- C# 稳定 API 外观 `Hezhou.Engine`
- 自研物理进阶：动态刚体、冲量/摩擦、CCD、距离/固定关节、空间索引宽相、旋转盒保守接触
- Headless 性能基线 + CI 性能门禁
- 现有 0.2 能力与 Player 全保留

## 编辑器

- Profiler Tab：FPS/frame ms/p95/启动/实体/PBR/蒙皮/资产/音频/内存
- Build Settings：`build.settings.json` 编辑/校验/保存，Player 导出使用
- 动画 Tab：glTF clip/skeleton、时间轴、播放/暂停/拖动、9 状态机预览与过渡 alpha
- 材质 Tab：Texture 资产列表、albedo 纹理赋值/清除、持久化
- 启动自检：PASS=102

## C# API

`Hezhou.Engine`：

```csharp
var cube = Hezhou.Engine.Scene.CreateCube();
cube.Position = new Hezhou.Engine.Vector3(1, 0, 0);
Hezhou.Engine.Time.DeltaTime
Hezhou.Engine.Audio.PlayFirst()
Hezhou.Engine.Animation.PlayClip(0)
```

限制：位于 EditorScript.dll 内，Player context 后续统一。

## 物理

- 动态刚体质量/阻尼/角速度/冲量
- Plane/AABB 接触、恢复系数、摩擦、位置修正
- 球/胶囊 CCD，动态-动态 CCD
- DistanceJoint / FixedJoint
- uniform grid 空间索引（200 体约 7.7× 加速）
- 旋转盒保守 AABB 接触

## 性能

`perf_smoke` 5 项 headless 指标 + 阈值门禁；CI `perf-baseline` 成功。
本机 release 基线：ECS 10k 9.99ms、物理 200 体 48.27ms、动画 10k 9.98ms、音频 1.59ms、资产 1000 1.22ms。

## Validation

- 本地 `ci_local.ps1 -Offline` 全绿
- 0.3 CI run 42470825264 全绿
- 编辑器冒烟 PASS=102
- 物理 52 tests；Player/编辑器/资产/动画回归
- 双平台 Editor + Player 构建待 0.3.0 tag CI

## Known Issues / 0.4

- 动画预览不渲染角色、无关键帧编辑/混合树/事件轨
- 材质仅 albedo 纹理槽
- C# API 非独立程序集，Player context 待统一
- 物理旋转碰撞为保守 AABB，无精确 OBB/布娃娃/多级 BVH
- 性能门禁阈值基于本机 release，共享 runner 需持续校准
