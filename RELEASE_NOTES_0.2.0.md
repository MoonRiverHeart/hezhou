# Hezhou 0.2.0 发布说明（草稿）

> 状态：候选/待双平台 CI 验收。基线：0.1.0。

## Highlights

- 3D 动作引擎能力模块：audio / asset / physics / gameplay / animation / integration / player
- glTF 导入、AssetId/Meta、CookManifest
- 自研胶囊角色物理、命中盒、查询
- 9 状态骨骼动画状态机 + GPU 蒙皮
- PBR + CSM 阴影 + Bloom/Tonemap + 天空/outline/Gizmo
- Player：Project JSON 场景、cook imported 资产、真实蒙皮 mesh、窗口 Vulkan、C# PlayerGame
- 编辑器：Prefab/SceneManager、Player 真实导出、PBR 材质 Inspector、音频/资产面板
- C# ABI v6：Scene/Transform、动画状态机、资产/音频、胶囊角色控制

## Player

- Windows: `hezhou-capability-player-windows.zip`
- Linux: `hezhou-capability-player-linux.tar.gz`
- 需要 Vulkan 与（mono-host 构建时）Mono；默认 demo 为 window-host 构建
- 场景：skinned fixture（Project JSON + imported mesh/skeleton/3 clips + WAV）

## Validation

- 本地 `ci_local.ps1 -Offline`: OK
- Player tests: 默认 123 / window-host 126 / mono-host 134
- 编辑器启动自检：PASS=77
- Windows capability demo：validate OK，窗口 5 帧 exit 0
- 双平台 CI：待 workflow 结果

## Known Issues / 0.3

- 示例工程还不是完整可玩关卡（地形/5 音效/角色 7 动作组合仍需资产与玩法）
- Player mono-host 需 Mono 运行时；包内 runtime 尚未统一 bundle
- 蒙皮着色仍为 Blinn-Phong 回退；单 mesh/单 clip 状态机、无混合树/IK
- 无完整 C# gameplay 战斗 API（角色移动已接入）
- Linux Player 仍需 CI/干净机器验收

## Upgrade

- 0.1.0 工程/安装包可直接升级；Project JSON 新字段均有 serde 默认。
