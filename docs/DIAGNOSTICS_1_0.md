# Hezhou 1.0 诊断包导出

> 状态：1.0 稳定性/支持能力（2026-11-08）
> 入口：`engine/scripts/diagnostics.ps1` + CI `diagnostics-smoke` job
> 目标：用户/开发者**一键**导出可读、可脱敏、可外发的诊断包，用于报障与稳定性分析。

## 1. 定位

`diagnostics.ps1` 把排查一个 Hezhou 工程/Player 问题所需的信息收拢到一个 zip：

- 引擎/编辑器日志、Player `player.log` / `startup-error.log`、DFX/C# 崩溃报告；
- 系统与环境信息（OS/CPU/GPU/内存、Rust/Mono/PowerShell 版本、git、引擎版本、Vulkan 摘要）；
- 工程摘要（`project.json`、`build.settings.json`、`assets/` 文件索引、`cook_manifest.json`）；
- Player 摘要（`manifest.json`、scripts/assets/runtime 文件索引、build config、玩家日志）；
- 错误/异常扫描、被排除文件清单、脱敏统计、人可读 `REPORT.txt`。

它**不会上传任何东西**，所有文件写本地目录；也**不是安全边界**——脱敏是尽力而为，
外发前必须人工检查。

## 2. 快速开始

### 2.1 用户报障（编辑器 / Player 出问题）

```powershell
# Windows（安装版默认日志在 %LOCALAPPDATA%\Hezhou\Editor\logs，脚本会自动尝试）
pwsh -File engine/scripts/diagnostics.ps1 `
    -OutDir "$env:USERPROFILE\Desktop\hezhou-diagnostics" `
    -ProjectRoot "D:\MyGame" `
    -PlayerRoot "D:\MyGame\dist\player\windows"
```

生成后：

1. 打开输出目录里的 `REPORT.txt` 快速浏览；
2. 确认 `summary.json` 的 `redaction.excluded_files`、`warnings` 没有异常；
3. 把 `hezhou-diagnostics-<timestamp>.zip` 发给支持人员（或附 `REPORT.txt`/`summary.json`）。

### 2.2 引擎开发/CI

```powershell
# 仓库根目录：默认收集 logs/、engine/logs/、编辑器用户数据日志，并自动取 git 信息
pwsh -File engine/scripts/diagnostics.ps1

# 自检：构造临时 fixture，断言 zip 内容/脱敏/敏感文件排除/坏路径安全（CI 用）
pwsh -File engine/scripts/diagnostics.ps1 -SelfTest
# CI 指定输出目录（artifact 路径）
pwsh -File engine/scripts/diagnostics.ps1 -SelfTest -OutDir test-results/diagnostics/selftest-ci
```

## 3. 参数

| 参数 | 说明 |
|---|---|
| `-ProjectRoot <dir>` | Hezhou 工程根目录（含 `project.json`/`build.settings.json`/`assets/`）；不存在时只告警不失败 |
| `-PlayerRoot <dir>` | Player 包根目录（含 `manifest.json`）；不存在时只告警不失败 |
| `-OutDir <dir>` | 输出目录；默认 `test-results/diagnostics/hezhou-diagnostics-<timestamp>` |
| `-LogDirs <dir[]>` | 额外日志目录（可多个） |
| `-IncludePaths <path[]>` | 额外要包含的文件/目录（仍做敏感文件过滤、文本-only、大小限制与脱敏） |
| `-EnvAllowlist <name[]>` | 环境变量白名单（默认只记录 OS/CPU/CI/GITHUB_SHA 等，且值仍脱敏） |
| `-MaxFileMB <n>` | 单文件上限（默认 10 MB；文本超限保留尾部并标记 truncated） |
| `-MaxTotalMB <n>` | 诊断包内容总上限（默认 50 MB；超出即停止收集并告警） |
| `-MaxFilesPerDir <n>` | 每个目录最多文件数（默认 300，按修改时间倒序） |
| `-MaxAssetEntries <n>` | 工程/Player 资产索引最多条目（默认 5000） |
| `-SkipSystemInfo` | 跳过系统命令采集（离线/受限环境） |
| `-KeepStaging` | 保留打包前的 staging 目录（调试） |
| `-SelfTest` | 自检模式（见 §7） |

退出码：`0` 成功；`1` 自检失败；`2` 采集/打包失败。

## 4. 收集内容

### 4.1 系统与环境（`system/system.json`）

- OS 名称/版本/架构、PowerShell 版本、CPU（名称/核数）、内存、GPU（名称/驱动）；
- 引擎版本（解析 `engine/Cargo.toml` 的 `[workspace.package].version`）、git commit/branch/dirty；
- 命令输出：`git --version`、`rustc/cargo --version`、`mono --version`、`python --version`、
  `pwsh` 版本、`vulkaninfo --summary`（缺失时记录 `unavailable`，不中断）；
- 环境变量**白名单**（默认 `OS/CI/GITHUB_SHA/GITHUB_RUN_ID/GITHUB_WORKSPACE/RUST_VERSION/MONO_VERSION/HEZHOU_MCP`），
  只记录值且经脱敏；**不会 dump 完整环境变量**；
- 进程信息（PID、CLR、Culture）。

### 4.2 日志

默认候选目录：

| 场景 | 目录 |
|---|---|
| 开发仓库 | `<repo>/logs/`、`<repo>/engine/logs/` |
| Windows 安装版 | `%LOCALAPPDATA%\Hezhou\Editor\logs\`、`...\Hezhou\<产品>\logs` |
| Linux | `$XDG_DATA_HOME/hezhou/editor/logs`、`$HOME/.local/share/hezhou/...` |
| 工程/Player | `<ProjectRoot>/logs`、`<PlayerRoot>/logs`（含 `player.log`、`startup-error.log`） |
| 额外 | `-LogDirs` 指定的目录 |

收集范围：文本扩展名 `.log/.txt/.json/.trace/.csv/.md/.yml/.yaml/.xml/.ini/.cfg/.conf/.scene/.settings`，
每个目录最多 300 个文件、递归深度 ≤3，跳过 `node_modules/target/.git` 等目录。
二进制文件（DLL/EXE/so/DMP 等）不复制内容，只可能出现在索引元数据中。

### 4.3 崩溃 / DFX / 错误扫描（`crashes/`）

- 收集文件名含 `crash`、`dfx`、`startup-error`、`panic` 的文本报告（DFX `crash_report.txt`、
  Player `startup-error.log` 等），脱敏后放入 `crashes/`；
- **崩溃转储（`.dmp/.dump/.mdmp`）默认不收集内容**（可能包含完整内存、token/密码），
  仅在 `crashes/excluded-dumps.json` 记录文件名/大小/时间；
- 从已收集日志中扫描 `error/fatal/panic/exception/crash/failed`，生成 `crashes/error-scan.txt`
  （最多 50 条，已脱敏）。

### 4.4 工程摘要（`project/`）

- `project.json`、`build.settings.json`：脱敏副本 + 摘要字段
  （名称/版本/实体数/资产数/脚本数/窗口/质量档/start_scene/targets/log_dir）；
- `project/assets-index.json`：`assets/` 递归文件列表（相对路径、大小、扩展名），不含内容；
  敏感文件名（证书/令牌等）不会进入索引；
- `cook_manifest.json`（若存在）：脱敏副本 + 条目数/roots 数摘要。

### 4.5 Player 摘要（`player/`、日志目录）

- `manifest.json`：校验 JSON 并输出摘要（产品名/版本/引擎版本/start_scene/target/quality/窗口/字体/scripts/assets 数量）；
- `scripts-index.json`、`assets-index.json`、`runtime-index.json`：文件列表与大小（**不打包二进制内容**）；
- `build-config.json`（若存在）：脱敏副本；
- `logs/player.log`、`logs/startup-error.log`：经日志收集器脱敏后进入 `logs/player-package-logs/`。

## 5. 脱敏与敏感文件排除

### 5.1 文本脱敏（应用于所有收集的文本与生成的 JSON/REPORT）

| 类别 | 处理 |
|---|---|
| 家目录 | `<user>`（如 `C:\Users\alice`、`/home/alice`、`/Users/alice`、`%USERPROFILE%`） |
| 用户名/机器名 | `<user>` / `<computer>`（含 `DOMAIN\user` 形式） |
| 绝对路径 | `-ProjectRoot` → `<project_root>`，`-PlayerRoot` → `<player_root>`，仓库根 → `<hezhou_root>` |
| 密码/令牌/密钥 | `password=…`、`token=…`、`api_key=…`、`client_secret=…`、`Authorization` 等值 → `<REDACTED>` |
| Bearer/JWT/云密钥 | `Bearer …`、`ghp_…`、`sk-…`、`AKIA…`、私有密钥块 → `<REDACTED>` |

统计写入 `summary.json.redaction`（`path_replacements`、`secret_replacements` 等）。

### 5.2 敏感文件排除（文件名/路径匹配即不收集内容）

- 证书/私钥：`*.pfx`、`*.p12`、`*.cer`、`*.crt`、`*.key`、`*.pem`、`*.jks`、`*.keystore`、`*.snk`；
- 凭据/令牌：`token*`、`secret*`、`credential*`、`password*`、`auth*`、`.env*`、`npmrc`、`pypirc`、`netrc`、`git-credentials`；
- SSH/GPG/云配置：`id_rsa`、`id_ed25519`、`.ssh/`、`.gnupg/`、`.aws/`、`.azure/`；
- 崩溃转储：`*.dmp`、`*.dump`、`*.mdmp`（只记录元数据）；
- 构建目录：`.git/`、`node_modules/`、`target/`。

被排除项写入 `summary.json.redaction.excluded_files`（路径已脱敏、reason 可读），
方便用户和支持人员确认“没有带上敏感文件”。

## 6. 产物与大小限制

```text
<OutDir>/
├── hezhou-diagnostics-<timestamp>.zip     # 符号链接/文本均已脱敏；含 summary.json + REPORT.txt
├── summary.json                            # 机器可读摘要（同样脱敏）
└── REPORT.txt                              # 人可读摘要 + 警告 + 排除统计
```

zip 内布局：

```text
summary.json
REPORT.txt
system/system.json
logs/<source-label>/<relative-log-path>...
crashes/crash_report.txt | error-scan.txt | excluded-dumps.json...
project/project.json
project/build.settings.json
project/assets-index.json
project/cook_manifest.json (可选)
player/manifest.json
player/scripts-index.json
player/assets-index.json
player/runtime-index.json
player/build-config.json (可选)
extra/<path>... (来自 -IncludePaths，仅文本)
```

限制与行为：

- 单文件默认 ≤10 MB（文本超限保留尾部并 `truncated=true`），总内容默认 ≤50 MB；
- 每个日志目录 ≤300 个文件（按修改时间倒序），防止超大日志拖垮；
- zip 创建失败会以退出码 2 报错并打印原因（不会生成半成品 zip）；
- 输出目录不存在会自动创建；用户目录需可写。

## 7. 自检（fixture）与 CI

`-SelfTest` 在临时目录构造 fixture（日志含家目录/用户名/密码/token/Bearer、崩溃报告与 `.dmp`、
带证书/`.env` 的工程、Player manifest/日志/证书/二进制 runtime、额外 include 路径），然后：

- 生成诊断包并解压；
- 断言必需文件存在（日志、崩溃、工程、Player、extra）；
- 断言敏感/二进制文件（`token.txt`、`credentials.env`、`client.pfx`、`memory.dmp`、`id_rsa`、
  `Game.dll`、`mono.dll`）不在 zip 中；
- 断言 zip 内所有文本不包含家目录、用户名、fixture 密码/token/Bearer/private key；
- 断言 `summary.json` 标记 `self_test`、工程/Player 摘要正确；
- 用不存在的 `-ProjectRoot/-PlayerRoot/-LogDirs/-IncludePaths` 再跑一次，断言安全降级（不抛异常、仍生成 zip）。

CI `diagnostics-smoke` job（ubuntu-latest + windows-latest matrix）运行：

```bash
pwsh -File engine/scripts/diagnostics.ps1 -SelfTest -OutDir test-results/diagnostics/selftest-<os>
```

并上传 `test-results/diagnostics/**` artifact（**只含 fixture 假数据，不含真实用户数据**）。

> 说明：本次改动未 commit/push，GitHub Actions 未真实触发；本地已用与 CI 相同命令
> 在 Windows 上通过（`SELFTEST PASS`，zip 约 8.5 KB），并额外用同一套 fixture 跑了
> 一次普通模式（生成约 1.5 MB zip，验证真实采集路径）。ubuntu 分支依赖 PowerShell 7
> 与 `/proc`、`uname` 等标准工具，已在脚本中做平台分支与失败降级。

## 8. 用户报障流程（建议）

1. 复现问题后立即运行 `diagnostics.ps1`（越早越能保留日志/崩溃现场）；
2. 若问题在 Player：带上 `-PlayerRoot`；若在编辑器工程：带上 `-ProjectRoot`；
3. 打开 `REPORT.txt` 检查“警告/排除”是否异常；
4. 确认后把 zip 发给支持人员；如担心隐私，可先手动删除 `logs/` 中的业务日志再打包（脚本会重新脱敏，但不会判断业务敏感性）；
5. 支持人员用 `summary.json` 的 `git.commit`、系统信息、日志/崩溃摘要定位问题。

## 9. 已知限制

1. **脱敏是尽力而为**：只覆盖家目录/用户名/常见密钥模式，业务日志里的私有内容（账号 ID、内部 URL 等）
   不会被自动识别；**外发前必须人工检查**；
2. 崩溃转储默认不收集（隐私优先），需要内存级分析时让用户单独提供并走安全渠道；
3. 系统信息按平台可用命令采集：Windows 用 CIM，Linux 用 `/proc`/`uname`/可选 `lspci`，
   缺失命令只记录 unavailable；
4. 日志为采样收集（目录/数量/大小上限），不是完整归档；
5. PowerShell 脚本跨平台依赖 pwsh 7（`Compress-Archive`/`Expand-Archive`）；
6. 脱敏对二进制文件不适用——因此二进制内容默认不进入包（Player 只给索引）；
7. 本工具不上传、不联网、不自动发送；由用户决定是否外发。

## 10. 变更记录

| 日期 | 变更 |
|---|---|
| 2026-11-08 | 首版：`diagnostics.ps1`（日志/崩溃/系统/工程/Player 采集 + 脱敏 + zip + summary + self-test）、CI `diagnostics-smoke`、本文档 |
