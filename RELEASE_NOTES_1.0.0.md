# Hezhou 1.0.0 发布说明

> 状态：1.0 Release / 双平台 CI + release-check 全绿。基线：0.4.0。

## 1.0 交付

- 引擎能力：音频、资产/glTF、骨骼动画、自研物理、动作组件、PBR/阴影、Prefab/场景、Player
- 编辑器：Profiler、Build Settings、动画时间轴/9 状态预览、材质纹理、包管理器、插件、模板、Player 导出
- C# `Hezhou.Engine` API 外观
- 网络：UDP/TCP 传输、会话/房间、权威快照/Delta/ack、客户端插值、RPC
- 生态：本地包 registry、声明式插件、项目模板
- 稳定性：60s soak CI、30 分钟 soak、诊断包、性能基线
- 文档：Getting Started、平台矩阵、API 稳定性、Release Checklist

## 发布产物

- Windows：MSI / MSIX / .cer / Capability Player zip
- Linux：DEB / Capability Player tar.gz

## 验证

- CI：rust-tests、csharp-build、package-windows/linux、player-build、player-window-host、package-player、perf-baseline、soak-quick、diagnostics-smoke、release-check、publish-release 全绿
- release-check：版本/文档/FFI/关键测试/capability/perf/soak/Player validate/fresh-start 通过
- 编辑器启动自检 PASS=117；FFI 343
- 网络 58 tests；包管理 6；插件 5；物理 52
- 本地 30 分钟 soak：1800s / 107999 ticks / 0 overruns / PASS

## 平台

- Windows x64、Linux x64：支持
- macOS / Android / Web：未实现，路线见 PLATFORM_SUPPORT_1_0.md
- HarmonyOS：实验

## 已知限制

- 网络无预测/回滚/加密/可靠 UDP/AOI
- 插件声明式，无动态代码/权限沙箱/市场
- 包管理器仅本地 registry，无远程/签名/完整 semver
- Player C# context 为 ABI v6，完整游戏 host/PlayerGame 端到端仍需真实项目验收
- 30 分钟 soak 绝对内存 +1.77MB，相对低基数偏高，需持续观察
- 非团队开发者按文档 demo 需外部验证
