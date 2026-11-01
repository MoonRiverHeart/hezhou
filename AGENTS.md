# HEZHOU PROJECT KNOWLEDGE BASE

**Generated:** 2026-10-31
**Commit:** 09d60d5
**Branch:** main

## OVERVIEW
跨平台游戏引擎与编辑器：Rust core + C# scripting（Mono JIT）+ Vulkan 渲染 + 自绘 Widget UI。
当前发布版本 `v0.1.0`，Windows 同时提供 **MSI + MSIX**，Linux 提供 **DEB**。

## STRUCTURE
```
hezhou/
├── engine/          # Rust workspace — 20 crates（详见 engine/AGENTS.md）
│   ├── examples/    # mono_editor_demo（编辑器主程序）
│   │   └── src/hezhou_launcher.rs  # Windows GUI 启动器（无终端）
│   ├── audio/       # 0.2 音频能力（Null/cpal 后端、混音、3D）
│   ├── asset/       # 0.2 资产管线（AssetId/Meta/glTF/cook）
│   ├── gameplay/    # 0.2 动作能力（Motor/Camera/Combat）
│   ├── animation/   # 0.2 骨骼/蒙皮/状态机
│   ├── integration/ # animation/gameplay/physics 适配层
│   ├── player/      # Windows/Linux Player 导出
│   ├── scripts/     # 34 个 C# 脚本 + build_mono.ps1
│   ├── fonts/       # HarmonyOS_Sans_SC.ttf（已入库，安装包必须包含）
│   └── target/      # 构建产物（gitignore）
├── installer/
│   ├── windows/     # MSI: build_msi.ps1 + HezhouEditor.wxs (WiX v5)
│   │                # MSIX: build_msix.ps1 + msix/AppxManifest.xml
│   └── linux/       # build_deb.sh + build_deb.py（ar/tar，无需 dpkg-deb）
├── .github/workflows/ci.yml  # CI + 标签触发打包/发布
├── dist/            # 本地打包输出（gitignore）
├── demo/            # 旧版互操作示例（Rust↔C#/Harmony），非发布路径
├── release/         # 旧版发布目录（历史遗留，当前以 installer/ 为准）
├── design/ docs/    # 设计文档与截图
└── scripts/         # 顶层辅助脚本
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| 编辑器入口 | `engine/examples/src/mono_editor_demo.rs` | `main()` → `editor::run()` |
| Windows 无终端启动 | `engine/examples/src/hezhou_launcher.rs` | GUI 子系统，`CREATE_NO_WINDOW` 启动编辑器 |
| 安装目录只读适配 | `engine/examples/src/editor/mod.rs` → `prepare_runtime_dir()` | 复制 scripts/assets/fonts 到用户数据目录 |
| 编辑器主体 | `engine/examples/src/editor/` | mod/ffi/hot_reload/mcp/scene_ffi |
| C# 编辑器 | `engine/scripts/` | 34 个 `.cs`，只能用 Mono `mcs` |
| 字体 | `engine/fonts/HarmonyOS_Sans_SC.ttf` | 缺失会导致 UI 文字空白，必须随包发布 |
| Windows MSI | `installer/windows/build_msi.ps1` | WiX v5，Start Menu 指向 launcher |
| Windows MSIX | `installer/windows/build_msix.ps1` | MakeAppx + signtool，自签名证书 |
| Linux DEB | `installer/linux/build_deb.sh` | Python 生成 ar/control/data |
| 0.2 能力模块 | `engine/{audio,asset,gameplay,animation,integration,player}/` | 先跑各自 `cargo test -p ...` |
| CI / 发布 | `.github/workflows/ci.yml` | tag `v*` 或 workflow_dispatch |
| 引擎完整 KB | `engine/AGENTS.md` | 架构、FFI、Widget、测试细节 |
| 0.2 项目计划 | `docs/PROJECT_PLAN.md` | 路线图、验收、ADR、当前进度 |

## CONVENTIONS
- Cargo workspace 在 `engine/`，任何 cargo 命令先 `cd engine`。
- 安装后的正常入口是 **`hezhou_launcher.exe`**（MSI 快捷方式/MSIX 应用入口）。
  直接运行 `mono_editor_demo.exe` 会保留控制台，仅用于开发调试。
- 安装目录（Program Files/WindowsApps）只读；运行时写操作走
  `%LOCALAPPDATA%\Hezhou\Editor`（Linux 为 `$XDG_DATA_HOME/hezhou/editor`）。
- 安装包必须包含 `fonts/`；打包脚本已内置 `engine/fonts`。
- MSIX 是自签名包，发布时同时上传匹配的 `.cer`；安装前需信任根证书。
- `dist/`、`installer/stage/`、`build-temp/`、`download/` 都是构建产物，不提交。
- 版本号来源：`engine/Cargo.toml` 的 `[workspace.package].version`。

## ANTI-PATTERNS (THIS PROJECT)
- **NEVER** 从仓库根目录运行 `cargo build`，必须进入 `engine/`。
- **NEVER** 让已安装应用直接双击 `mono_editor_demo.exe`；应使用 launcher/MSIX 入口。
- **NEVER** 删除或漏打包 `engine/fonts/HarmonyOS_Sans_SC.ttf`。
- **NEVER** 假设安装目录可写；所有运行态文件必须写用户数据目录。
- **NEVER** 用 .NET 8 编译的 DLL 配 Mono；必须用 `mcs`。
- **NEVER** 把 MSIX 证书私钥（`.pfx`）提交到仓库；CI 临时生成。
- **NEVER** 提交 `dist/`、`installer/stage/`、`download/` 等构建/测试产物。

## COMMANDS
```bash
cd engine

# 构建
cargo build --release --features mono
powershell -ExecutionPolicy Bypass -File scripts/build_mono.ps1

# 开发运行（会显示控制台，正常）
cargo run --bin mono_editor_demo --features mono --release

# 测试
powershell -ExecutionPolicy Bypass -File scripts/run_tests.ps1 -Offline
powershell -ExecutionPolicy Bypass -File scripts/run_autotest.ps1 -Mode intent
powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1 -SkipBuild

# Windows MSI
dotnet tool install --global wix --version 5.0.2
$env:PATH = "$env:USERPROFILE\.dotnet\tools;$env:PATH"
powershell -ExecutionPolicy Bypass -File installer/windows/build_msi.ps1

# Windows MSIX（不带证书为未签名包）
powershell -ExecutionPolicy Bypass -File installer/windows/build_msix.ps1

# Linux DEB
bash installer/linux/build_deb.sh
```

## RELEASE / CI
- 触发：push tag `v*`（例如 `v0.1.0`）或手动 `workflow_dispatch`。
- 作业：`rust-tests`、`csharp-build`、`package-windows`、`package-linux`、
  `publish-release`。
- Windows 附件：`HezhouEditor-<version>.msix`、`HezhouEditor.cer`、
  `HezhouEditor-<version>.msi`。
- Linux 附件：`hezhou-editor_<version>_amd64.deb`。
- 发布前必须确认五个作业全部 success，且 GitHub Release 中三种安装包都存在。
- `v0.1.0` 发布页：https://github.com/MoonRiverHeart/hezhou/releases/tag/v0.1.0

## NOTES
- CI 使用 Rust 1.95.0、Mono 6.12.0.199、Python 3.11；Windows 打包安装 WiX 5.0.2。
- Linux 打包需要 CMake/Ninja + GLFW/X11/Wayland/Vulkan/Mono 依赖。
- 编辑器启动时会做 C# 自检并输出 `PASS=35, FAIL=0`。
- 主要运行日志：`logs/`（开发目录）或 `%LOCALAPPDATA%\Hezhou\Editor\logs`（安装版）。
- `demo/` 与 `release/` 是历史目录，不参与当前 CI/发布流程。

## LANGUAGE POLICY
- **全程使用中文** — 所有思考、回答、注释、对话必须用中文。
- 代码本身（Rust/C#/GLSL）保持英文，解释、诊断、计划和 todo 描述用中文。
