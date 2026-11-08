# Hezhou 平台支持矩阵（1.0 方向，2026-11）

> 基线：仓库 **0.4.0 RC**；最近发布 **v0.3.0**；1.0 尚未发布。
> 本文件明确「已支持 / 实验 / 未实现」，避免把路线图当承诺。实际验收以 [1.0_RELEASE_CHECKLIST.md](1.0_RELEASE_CHECKLIST.md) 为准。

## 1. 平台矩阵总览

| 平台 | 编辑器 | Player 导出 | 安装包 | 状态 | 说明 |
|---|---|---|---|---|---|
| **Windows 10/11 x64** | ✅ 支持 | ✅ 支持（当前平台构建） | ✅ MSI + MSIX | **发布平台** | 主开发/CI 平台；Vulkan 渲染 + Mono 内置 |
| **Linux x64** | ✅ 支持 | ✅ 支持（当前平台构建） | ✅ DEB | **发布平台** | Ubuntu 20.04+；内置 Mono runtime；X11/Wayland + Vulkan |
| **macOS** | ❌ 未实现 | ❌ 未实现 | ❌ | 未实现 | 无 Metal 后端/打包；不在 0.4/1.0 承诺范围 |
| **Android** | ❌ 未实现 | ❌ 未实现 | ❌ | 未实现 | 无 Vulkan/NDK/触摸/输入适配；路线待定 |
| **Web (WASM)** | ❌ 未实现 | ❌ 未实现 | ❌ | 未实现 | 无 WebGPU/WASM 打包；路线待定 |
| **HarmonyOS** | ⚠️ 实验 | ❌ | ❌ | 实验 | `harmony` crate 提供 OpenHarmony 原生窗口实验接口，未进入发布/CI 矩阵 |

> **0.4/1.0 承诺范围**：Windows x64 与 Linux x64 的编辑器 + Player（当前平台构建）。
> macOS/Android/Web 不在 1.0 承诺内；HarmonyOS 仅保留实验接口。

## 2. Windows（发布平台）

**已支持**
- 编辑器：Vulkan 渲染、UI、Mono C# 脚本/热重载、Project/Prefab/SceneManager、PBR/阴影/Bloom/Tonemap/FXAA、Profiler、Build Settings、动画时间轴/9 状态机、材质纹理、模板/包/插件、Profiler/诊断。
- Player：单平台构建（exe + assets + scripts + runtime + manifest），`--validate` 启动检查通过；`window-host` feature 可选窗口 host。
- 安装：MSI（WiX v5，开始菜单指向 `hezhou_launcher.exe`）；MSIX（自签名，需信任 `.cer`）。
- CI：`package-windows`、`player-build`、`player-window-host`、`package-player` 等 Windows runner job。

**构建依赖**
- Rust 1.95（CI 固定）、Mono 6.12（`mcs`，编译 EditorScript.dll）、WiX v5（MSI）、Windows SDK（MakeAppx/signtool，MSIX）。
- 运行时：Vulkan 1.1+ 驱动、VC++ Runtime（安装包内置）；编辑器安装包内置 Mono runtime。

**Player 产物**
- 编辑器内导出：`<工程>/exports/<product>/windows/`（exe/assets/scripts/runtime/manifest）。
- CI 能力演示：`hezhou-capability-player-windows.zip`（含 Mono runtime、skinned fixture、`hezhou-capability-demo.exe`）。

**已知限制**
- 只构建当前平台；跨平台需在对应平台构建（PlayerBuilder 限制）。
- 默认 Player host 尚未接入完整游戏循环（`--run` 报 pending，退出码 3）；普通项目导出的包以 `--validate` 为当前验收。
- 安装目录只读：运行态写 `%LOCALAPPDATA%\Hezhou\Editor`。

## 3. Linux（发布平台）

**已支持**
- 编辑器：与 Windows 同能力（Vulkan + Mono + C# UI/脚本）；DEB 安装。
- Player：当前平台构建；`--validate` 启动检查；能力演示 tar.gz。
- CI：`package-linux`、`player-build`（ubuntu runner）等。

**构建依赖（CI/本地）**
```bash
sudo apt-get install -y build-essential cmake ninja-build pkg-config python3 \
  libvulkan-dev libglfw3-dev libx11-dev libxrandr-dev libxi-dev libxcursor-dev \
  libxinerama-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
  wayland-protocols libdecor-0-dev libx11-xcb-dev libxcb1-dev libxcb-randr0-dev \
  libxcb-xinerama0-dev libxcb-xkb-dev libxcb-shape0-dev libxcb-xfixes0-dev libasound2-dev \
  mono-devel libmono-2.0-dev
```
运行时：Vulkan 1.1+ 驱动、桌面会话（X11/Wayland）、系统音频（可选）；DEB 内置 Mono 到 `/opt/hezhou/runtime/mono`。

**Player 产物**
- 编辑器导出：`exports/<product>/linux/`（无后缀 exe、assets/scripts/runtime/manifest）。
- CI：`hezhou-capability-player-linux.tar.gz`。

**已知限制**
- 只构建当前平台；Linux 产物需在 Linux 构建（CI ubuntu runner）。
- 打包脚本不生成 `.deb` 签名/仓库；需手动分发。
- 不同发行版 glibc/GLFW/驱动差异需自测；CI 仅覆盖 Ubuntu。

## 4. macOS / Android / Web（未实现）

| 平台 | 现状 | 需要的后端/工作 | 1.0 计划 |
|---|---|---|---|
| macOS | 无 Metal/窗口/打包；Rust 代码可能部分编译但未验证 | Metal 或 MoltenVK 后端、Cocoa 窗口、.app/pkg 打包、CI runner、Mono runtime 适配 | ❌ 不在 1.0 承诺 |
| Android | 无 NDK/Vulkan/触摸/生命周期适配 | Android Vulkan surface、JNI、触摸/输入、Gradle/AAB、Mono/IL2CPP 方案 | ❌ 不在 1.0 承诺 |
| Web | 无 WASM/WebGPU/打包 | WASM 构建、WebGPU 或软件渲染、浏览器输入/音频、资源按需加载 | ❌ 不在 1.0 承诺 |

> 这些平台可能在 1.0 之后的路线图出现；当前不应在项目计划中承诺时间表。

## 5. HarmonyOS（实验）

- `engine/harmony` 提供 OpenHarmony 原生窗口/平台实验接口；
- 未进入 CI、未提供安装包/Player 导出，编辑器也未在该平台验证；
- 仅适合内部实验，**不作为 1.0 发布平台**。

## 6. 能力 × 平台（当前实现）

| 能力 | Windows | Linux | 说明 |
|---|---|---|---|
| Vulkan 渲染（PBR/阴影/后处理） | ✅ | ✅ | 需 Vulkan 驱动 |
| 编辑器 UI/Dock/Inspector | ✅ | ✅ | Mono C# UI |
| C# 脚本/热重载 | ✅ | ✅ | 需 mcs 编译 |
| 资产管线（glTF/cook/AssetId） | ✅ | ✅ | headless 可用 |
| 骨骼动画/蒙皮数据 | ✅（数据/预览） | ✅ | 角色渲染接入仍有限 |
| 物理/角色/动作（数据层） | ✅ | ✅ | headless 能力演示可跑 |
| 音频（cpal/null） | ✅ | ✅ | Linux 需 ALSA/Pulse |
| 网络（UDP/TCP/快照/RPC） | ✅（实验） | ✅（实验） | 无加密/预测/AOI；1.0 前保持实验 |
| Player 导出 + validate | ✅ | ✅ | 当前平台 |
| Player 完整游戏 host | ⚠️ 部分 | ⚠️ 部分 | 默认 host pending；1.0 待完成 |
| 项目模板/包管理器/插件 | ✅ | ✅ | 0.4 起；插件为声明式 |
| 诊断包/soak/性能基线 | ✅ | ✅ | PowerShell 7 跨平台 |

## 7. 平台相关限制（汇总）

1. **交叉编译不支持**：Player 只能构建当前平台；Windows 构建的编辑器不能直接导出 Linux Player。
2. **GPU/驱动**：编辑器与窗口 Player 需 Vulkan；headless 能力演示/CI 不需要 GPU。
3. **Mono 版本**：必须用 Mono `mcs` 编译 C#（不要用 .NET 8 DLL）；安装包内置 Mono，源码构建需自备 6.12。
4. **安装签名**：MSIX 自签名需手动信任；DEB 未签名。
5. **macOS/Android/Web 无承诺**：项目计划中出现相关条目之前，视为未实现。

## 8. 验证依据

- CI job（`.github/workflows/ci.yml`）：`rust-tests`、`perf-baseline`、`soak-quick`、`diagnostics-smoke`（ubuntu+windows matrix）、`csharp-build`、`package-windows`、`package-linux`、`player-build`、`player-window-host`、`package-player`、`publish-release`。
- 本机 2026-11-08 验证：diagnostics self-test PASS、capability_demo PASS、soak quick PASS、perf baseline PASS、player-build dry-run PASS、skinned_asset 8 tests PASS；详见 [GETTING_STARTED_1_0.md](GETTING_STARTED_1_0.md) §11。
