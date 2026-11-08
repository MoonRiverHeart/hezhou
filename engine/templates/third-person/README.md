# Third Person（第三人称模板，1.0）

第三人称动作/探索原型模板：20x20 地面、方向光、`Player` 角色占位、相机锚点、障碍与台阶，
并附带可直接导出的 `build.settings.json`。

## 场景内容

| 实体 | 说明 |
|---|---|
| `Ground` | 20x20 地面（Plane，y=-0.25，灰蓝 PBR，roughness 0.9） |
| `Player` | 角色占位（Cube 0.8×1.8×0.8，y=0.9，蓝色 PBR） |
| `CameraAnchor` | 角色子实体（空实体，局部 y=2.0），第三人称相机跟随点 |
| `Obstacle_Box_01/02` | 两个障碍 Cube（±4, 0.5, -2/-3） |
| `Step_01/02` | 台阶（2×0.5×1、2×0.5×1 叠高），便于测试跳跃/物理 |
| `Sun` | 方向光（direction [-0.4,-1,-0.5]、intensity 1.3、暖白） |

- 相机：`current_state.camera_position ≈ (0, 3.2, -7)`、`camera_rotation = (-12°, 0, 0)`，
  形成**角色背后略俯视**的第三人称构图；
- 环境/天空：渲染器默认天空盒 + 方向光；0.x/1.0 暂无独立天空配置文件（后续可扩展 Project 环境字段）；
- 物理/碰撞：Cube/Plane 已带 mesh bounds，可在编辑器中直接观察 Gizmo/拾取与物理接入。

## 操作约定（1.0 目标）

| 输入 | 行为 |
|---|---|
| `W/A/S/D` | 角色前后左右移动（相对相机朝向） |
| `Space` | 跳跃（落地/台阶可测） |
| 鼠标 | 视角旋转（**本轮模板/编辑器无鼠标视角实现**，仅保留输入约定） |
| `Ctrl+G` | 编辑器 Gizmo 循环平移/缩放/旋转 |

> 当前编辑器预览可查看场景/材质/相机参数，但**不生成可操作角色**；角色/相机控制由
> **PlayerGame** 在 1.0 Player 入口生成（完整角色 + 第三人称相机）。

## 在编辑器中打开

1. 启动编辑器 → 资产面板「工程」Tab → **「新建项目（模板）」**；
2. 模板选择 `Third Person`，填项目名/目标目录 → 创建；
   或「打开工程」选择本模板复制出的工程目录；
3. 左侧层级可见 Ground/Player/CameraAnchor/Obstacle/Step/Sun；选中 Player 可在「渲染」Tab
   调整 PBR，在「位置」Tab 调整 Transform；
4. 预览窗使用模板保存的第三人称相机参数；保存工程（Ctrl+S）持久化。

## 导出 Player

1. 资产面板「Build」Tab：已预置
   `product_name = "Third Person Demo"`、`executable_name = "third-person-demo"`、
   `window 1280x720`、`quality = high`、`targets = [windows, linux]`、
   `start_scene = "scenes/main.scene"`；按需修改后「保存设置」；
2. 「工程」Tab → **导出 Player（Build）**：当前平台产物写入
   `<工程>/exports/Third Person Demo/<platform>/`（Windows/Linux 需在对应平台构建）；
3. 启动检查：

```bash
<exports>/Third Person Demo/<platform>/third-person-demo --validate \
  --player-root "<exports>/Third Person Demo/<platform>"
```

PlayerGame（1.0）会在此包内生成完整角色/相机；当前默认 Player host 仍报告 pending（退出码 3），
`--validate` 用于确认资产/脚本/运行时/manifest 完整。详见
[`docs/GETTING_STARTED_1_0.md`](../../../docs/GETTING_STARTED_1_0.md) 与
[`docs/PLATFORM_SUPPORT_1_0.md`](../../../docs/PLATFORM_SUPPORT_1_0.md)。

## 从源码验证模板

```bash
cd engine
# 模板 JSON 可扫描/创建（project_templates 测试会扫描 engine/templates）
cargo test -p hezhou-examples --bin mono_editor_demo --features mono project_templates
# 工程 JSON 可被 core 加载：Project::restore_from_json
# （测试/脚本见任务校验记录；模板扫描也会校验 template.json 白名单/路径）
```

## 限制

- 本轮**无鼠标视角/角色控制**：仅提供场景、相机构图与输入约定；
- PlayerGame 完整角色/相机与游戏 host 仍在 1.0 接入中（当前 Player `--run` 默认 pending）；
- 天空/环境为渲染默认值，无独立环境配置字段；
- 地面/障碍为 Cube/Plane 占位，尚无碰撞体/物理材质配置（物理接入后续任务）；
- 工程脚本编译流程未接入（`scripts/` 为说明占位）。
