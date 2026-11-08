# Hezhou 入门指南（1.0 方向，适用于当前 0.4 RC）

> 目标：让**非团队开发者**从 0 到「用模板新建工程 → 编辑场景/材质 → 导出 Player → 启动检查」完成一次闭环。
> 基线：仓库当前版本 **0.4.0（RC）**；最近已发布版本 **v0.3.0**；1.0 尚未发布。
> 文档中命令均已在本仓库/本机核对（见 §11 验证记录）。平台边界见 [PLATFORM_SUPPORT_1_0.md](PLATFORM_SUPPORT_1_0.md)。

---

## 1. 前置条件

| 项目 | 要求 |
|---|---|
| 操作系统 | Windows 10/11 x64 或 Linux x64（Ubuntu 20.04+ 推荐） |
| GPU | 支持 Vulkan 1.1+ 的显卡与驱动（编辑器渲染必需） |
| 安装包 | Windows：MSI（推荐）或 MSIX；Linux：DEB |
| 源码构建（可选） | Rust 1.95、Mono 6.12（mcs）、Python 3.11、CMake/Ninja、Vulkan/GLFW/X11/Wayland 开发包（Linux） |

> 安装版**不需要**单独安装 Rust/Mono：安装包内置 Mono runtime；Windows MSIX 为自签名包，需先信任随附证书。

---

## 2. 下载与安装

发布页：<https://github.com/MoonRiverHeart/hezhou/releases>

当前已发布的最新版本是 **v1.0.1**；包含项目模板、包管理器、插件 API、网络与第三人称 Player demo。

### 2.1 Windows

| 文件 | 说明 |
|---|---|
| `HezhouEditor-<version>.msi` | 推荐；开始菜单快捷方式指向 `hezhou_launcher.exe`（无控制台） |
| `HezhouEditor-<version>.msix` + `HezhouEditor.cer` | MSIX 应用包；安装前需把 `.cer` 导入「受信任的根证书颁发机构」或使用 `-SkipBuild` 本地打包时禁用签名 |
| `hezhou-third-person-player-windows.zip` | 第三人称 Player demo（含 Mono runtime，WASD/Space） |

```powershell
# MSI（示例：0.3.0）
msiexec /i HezhouEditor-0.3.0.msi /qn

# MSIX（先信任证书）
Import-Certificate -FilePath HezhouEditor.cer -CertStoreLocation Cert:\LocalMachine\Root
Add-AppxPackage -Path HezhouEditor-<version>.msix
```

启动：开始菜单 → **Hezhou Editor**（实际入口 `hezhou_launcher.exe`）。开发调试可直接运行
`engine/target/release/mono_editor_demo.exe`（会保留控制台）。

### 2.2 Linux

```bash
sudo dpkg -i hezhou-editor_<version>_amd64.deb
# 若缺依赖：
sudo apt-get -f install
hezhou-editor
```

DEB 安装到 `/opt/hezhou/editor`，内置 Mono runtime；需要系统 Vulkan 驱动与图形桌面会话。
第三人称 Player：`hezhou-third-person-player-linux.tar.gz`（解压后执行 `./third-person-demo --run --host window ...`）。

---

## 3. 从源码构建（1.0.1 开发版）

已发布 1.0.1 安装包包含模板/包管理器/插件 Tab；从源码构建用于开发：

```bash
git clone https://github.com/MoonRiverHeart/hezhou.git
cd hezhou/engine

# 1) C# 编辑器程序集（mcs，必须）
pwsh -File scripts/build_mono.ps1           # Windows
# Linux: bash ../installer/linux/build_deb.sh 或手动 mcs 后 cargo build

# 2) 编辑器（release，mono feature）
cargo build --release --features mono

# 3) 启动编辑器（开发调试；安装版用 launcher）
cargo run --bin mono_editor_demo --features mono --release
```

环境变量/依赖排障见 §10。

---

## 4. 用模板新建工程（0.4+）

1. 启动编辑器，左侧资产面板切到 **「工程」** Tab；
2. 点 **「新建项目（模板）」**；
3. 选择模板：
   - `empty-3d`：空场景 + project.json + build.settings.json + assets/scripts 占位；
   - `action-sandbox`：Cube（PBR 橙色材质 + `RotatingEntity` 脚本绑定）+ 方向光 + 场景；
4. 填**项目名**（如 `MyGame`）与**目标目录**（如 `D:/Projects/MyGame`，必须不存在或为空目录）；
5. 点「创建」：编辑器会调用模板创建、自动加载工程并刷新场景/资产/Build Settings。

生成的文件：

```text
<项目>/
├── project.json          # 工程（名称/实体/相机状态）
├── build.settings.json   # Player 导出配置（产品名/窗口/质量/启动场景）
├── README.md
├── assets/
│   ├── scenes/main.scene
│   └── README.md
└── scripts/              # 工程脚本（0.4 编译流程尚未接入，见 scripts/README）
```

手动/已安装版本：编辑器「工程 → 打开工程」选择工程目录或 `project.json` 也可加载 0.3 格式工程。

---

## 5. 场景与实体编辑

- **层级面板**：右键/工具栏创建 Cube、Plane、方向光、空实体；拖拽设置父子层级；
- **选择**：左键选中；`Ctrl+点击` 多选；`Shift+点击` 范围选择；`Delete` 删除（确认）；
- **属性面板**：
  - 「位置」Tab：Position/Rotation/Scale；
  - 「渲染」Tab：管线、PBR 材质（albedo RGBA、metallic、roughness、normal_scale、ao、emissive、strength、纹理路径）、清除材质；
  - 「运动」Tab：C# 脚本绑定；
- **变换/Gizmo**：预览区 Gizmo 轴条拖拽；`Ctrl+G` 循环平移/缩放/旋转；
- **撤销/重做**：`Ctrl+Z` / `Ctrl+Y`；
- **保存**：`Ctrl+S`（工程）或 `Ctrl+Shift+S` 另存；「工程」Tab 可保存当前场景到场景槽。

### 5.1 PBR 与纹理

- 「渲染」Tab 修改 PBR 参数即时写回 core 并同步 renderer；
- 「材质」Tab → **PBR albedo 纹理（AssetDatabase）**：点纹理按钮赋给选中实体；「清除 albedo 纹理」移除；
- 当前仅支持 **albedo 单纹理槽**；纹理路径随 Project/Prefab 保存（旧工程兼容）。

### 5.2 动画预览 / 状态机（0.3+）

- 「动画」Tab：输入/浏览 glTF，加载后列出 clip/skeleton；
- 时间轴 Slider 拖动/播放/暂停；9 状态机按钮（Idle/Run/Jump/Attack/Hit/Death/Land/Dodge/Block），显示当前状态与过渡 alpha；
- **本轮不渲染角色**（预览状态/时间，不驱动蒙皮）。

### 5.3 音频 / 物理 / 角色能力

- 音频：工具栏「音频测试」播放 AssetDatabase 第一个音频资产；状态栏显示后端（cpal/null）；
- 物理/角色/动画/资产/Player 的完整组合示例是 headless 能力演示：

```bash
cd engine
cargo run -p hezhou-examples --bin capability_demo
# 可选：--steps 300 --out target/demo-report.json
cargo test -p hezhou-examples --bin capability_demo
```

输出覆盖 physics/gameplay/animation/integration/asset/audio/player 七项，`RESULT: PASS` 后退出码 0。
蒙皮 Player 产物的 fixture 与测试：

```bash
cd engine
cargo test -p hezhou-player --test skinned_asset
```

---

## 6. Build Settings 与导出 Player

1. 「工程」Tab 下方或资产面板 **「Build」** Tab：编辑 `product_name/executable_name/version/start_scene/quality/window/targets/log_dir/required_font`；
2. 「保存设置」→ 校验通过后写 `<工程>/build.settings.json`；
3. 「工程」Tab 导出：
   - **导出 Player（DryRun）**：生成双平台 plan/manifest（不复制二进制）；
   - **导出 Player（Build）**：真实构建**当前平台**（Windows 或 Linux）到 `<工程>/exports/<product>/<platform>/`；
4. 构建成功会生成 `manifest.json`、`assets/`、`scripts/`、`runtime/`（若配置）与可执行文件；失败会清理半成品并只写 Console/状态栏。

跨平台：`Build` 只构建当前平台；另一平台需在对应平台构建（`targets` 仅用于校验/提示）。
CLI dry-run 计划（可选，验证配置）：

```bash
cd engine
cargo run -p hezhou-player --bin hezhou-player-build -- \
  --config player/tests/fixtures/minimal/build-config.json \
  --cook player/tests/fixtures/minimal/cook-manifest.json \
  --plan-out target/player-plan --target both
```

---

## 7. 运行/检查 Player 包

```bash
# 包内可执行文件（产物目录下）
<product-exe> --validate --player-root <包目录> --manifest <包目录>/manifest.json
```

- `--validate`：启动检查（manifest/资产/脚本/运行时/字体/资产索引），通过退出码 0；
- `--run` / `--host default|headless|null`：默认 host 当前会明确报告「等待 game host 接入」并退出 **3**
  （不是崩溃；完整游戏循环 host 是 1.0 待完成项，见 [1.0_RELEASE_CHECKLIST.md](1.0_RELEASE_CHECKLIST.md)）；
- 构建 Player 时启用 `window-host` feature 会创建窗口 host（CI 能力演示包使用），但普通游戏逻辑接入仍未完成。

CI 的第三人称 Player 包可直接运行：

```bash
third-person-demo --run --host window --manifest <解压目录>/manifest.json --log-dir <日志目录>
```

WASD/方向键移动，Space 跳跃，关闭窗口退出。默认运行到用户关闭窗口；CI/冒烟可用 `HEZHOU_PLAYER_WINDOW_FRAMES=30` 限帧。

---

## 8. 日志与工程位置

| 内容 | 位置 |
|---|---|
| 编辑器日志 | 开发目录 `engine/logs/hezhou_*.log`；安装版 `%LOCALAPPDATA%\Hezhou\Editor\logs`（Linux `$XDG_DATA_HOME/hezhou/editor/logs`） |
| Player 日志 | 包内 `logs/player.log`（或 `--log-dir` 指定） |
| 工程/场景 | `project.json`、`assets/scenes/*.scene`、`build.settings.json` |
| 安装目录 | Windows `Program Files\Hezhou`；Linux `/opt/hezhou/editor`（**只读**，运行数据写用户数据目录） |

---

## 9. 常见问题（FAQ）

1. **界面文字空白**：`engine/fonts/HarmonyOS_Sans_SC.ttf` 缺失；安装包应包含字体，源码构建需保留该文件。
2. **C# 编辑器无 UI / 方法数 0**：`EditorScript.dll` 未用 `mcs` 编译或未部署；执行 `pwsh -File engine/scripts/build_mono.ps1`。
3. **Vulkan 初始化失败**：更新显卡驱动、确认 `vulkaninfo --summary` 可运行；远程桌面/虚拟机可能无 Vulkan。
4. **安装目录写文件失败**：安装目录只读；编辑器会把 scripts/assets/fonts 复制到用户数据目录，请勿手动改安装目录。
5. **Player `--validate` 报引擎版本不兼容**：用与 `manifest.json` 同版本构建的 `hezhou-player`；升级版本后重建 `cargo build -p hezhou-player`（旧 target 产物会版本不符）。
6. **导出 Player 失败**：先确认 `build.settings.json` 校验通过、`assets/` 中有场景文件；查看 Console/状态栏与 `logs/player_dry_run`/输出的错误摘要。
7. **模板列表为空**：项目模板来自仓库 `engine/templates`（源码构建需在 `engine/` 目录启动编辑器）；安装版模板目录随包分发仍在 1.0 打包项中。插件来自 `<工程>/plugins/`，两者不要混淆。
8. **性能/卡顿**：打开「Profiler」Tab 看 FPS/帧时间/实体/PBR/蒙皮/资产统计；详见 [PERF_BASELINE_0_3.md](PERF_BASELINE_0_3.md)。

### 9.1 诊断包（报障必交）

```powershell
# Windows（在仓库根目录；安装版可用 -ProjectRoot/-PlayerRoot 指向你的工程/Player 包）
pwsh -File engine/scripts/diagnostics.ps1 `
    -OutDir "$env:USERPROFILE\Desktop\hezhou-diagnostics" `
    -ProjectRoot "D:\Projects\MyGame" `
    -PlayerRoot "D:\Projects\MyGame\exports\MyGame\windows"

# 自检（CI 同款；无真实用户数据）
pwsh -File engine/scripts/diagnostics.ps1 -SelfTest -OutDir test-results/diagnostics/selftest
```

产物为 `hezhou-diagnostics-<timestamp>.zip`（脱敏、排除敏感文件、大小上限），附 `REPORT.txt`/`summary.json`。
详见 [DIAGNOSTICS_1_0.md](DIAGNOSTICS_1_0.md)。

---

## 10. 开发/CI 常用命令（本仓库核对）

```bash
cd engine

# 构建
cargo build --release --features mono
pwsh -File scripts/build_mono.ps1

# 本地 CI（离线）：Rust 测试 + fuzz smoke + FFI contract + C# 编译 + 图标一致性
pwsh -File scripts/ci_local.ps1 -SkipBuild
pwsh -File scripts/run_tests.ps1 -Offline          # 含 E2E 自动点击测试（较慢）

# 性能/稳定性/诊断/漏测
pwsh -File scripts/perf_baseline.ps1 -Profile debug -Quick -Offline
pwsh -File scripts/soak_test.ps1 -Quick -Offline   # CI 同款 60s soak
pwsh -File scripts/diagnostics.ps1 -SelfTest
pwsh -File scripts/run_autotest.ps1 -Mode intent   # E2E 意图测试（需要可运行编辑器/GPU）

# 能力/Player
cargo run -p hezhou-examples --bin capability_demo
cargo test -p hezhou-player --test skinned_asset
```

> 脚本参数以各脚本 `param(...)` 为准；`ci_local` 支持 `-SkipBuild`/`-Offline`，`run_tests` 支持 `-Offline/-SkipBuild/-SkipStress`，
> `run_autotest` 支持 `-Mode/-Steps/-Duration/-Seed/-ReportDir/-Build`。

### 10.1 安装包构建（维护者）

```powershell
# Windows（需 WiX v5 + MakeAppx/signtool；MSIX 为自签名）
pwsh -File installer/windows/build_msi.ps1            # -Version/-SkipBuild/-StageDir/-MonoRoot/-NoRuntime
pwsh -File installer/windows/build_msix.ps1           # -Version/-SkipBuild/-CertificatePath/-CertificatePassword
```

```bash
# Linux
bash installer/linux/build_deb.sh                     # --skip-build 可跳过构建
```

CI 发布 tag `v*` 时构建并上传 MSI/MSIX/DEB 与能力演示 Player 包（job：`package-windows`/`package-linux`/`package-player`/`publish-release`）。

---

## 11. 本次文档核对记录（2026-11-08，本机 Windows）

| 命令/脚本 | 结果 |
|---|---|
| `pwsh -File engine/scripts/diagnostics.ps1 -SelfTest -OutDir ...` | `SELFTEST PASS`（zip 8,546 B，exit 0） |
| `cargo run -p hezhou-examples --bin capability_demo` | `RESULT: PASS（退出码 0）`，7 项能力输出 |
| `cargo test -p hezhou-player --test skinned_asset` | 8 passed |
| `cargo test -p hezhou-net` | 通过（末组 2 passed，exit 0） |
| `pwsh -File engine/scripts/soak_test.ps1 -Quick -Offline` | `RESULT: PASS`（60s、growth 0.32MB、exit 0） |
| `pwsh -File engine/scripts/perf_baseline.ps1 -Profile debug -Quick -Offline` | 5 项 pass（exit 0） |
| `cargo run -p hezhou-player --bin hezhou-player-build -- ... --plan-out ... --target both` | 计划写入成功（exit 0） |
| `target/debug/hezhou-player --help` | 参数与 §7 一致（exit 0） |
| `pwsh -File engine/scripts/build_mono.ps1` | exit 0（DLL 已部署，本会话多次验证） |
| `pwsh -File engine/scripts/ci_local.ps1 -SkipBuild` | `CI LOCAL OK`（本会话任务验证） |
| 模板→新建工程→加载 | 编辑器启动自检 **PASS=117** 含模板/包/插件；需当前 0.4 RC（已发布 0.3.0 安装包无模板） |

> 注：本机 `target/debug/hezhou-player.exe` 可能是旧版本产物（升级到 0.4.0 后需 `cargo build -p hezhou-player` 重建）。
