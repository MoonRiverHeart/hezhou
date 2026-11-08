# Hezhou 1.0 发布门禁（release check）

> 状态：1.0 发布门禁自动化（2026-11）
> 入口：`engine/scripts/release_check.ps1` + `engine/scripts/fresh_start_smoke.ps1` + CI `release-check` job
> 关联：[1.0_RELEASE_CHECKLIST.md](1.0_RELEASE_CHECKLIST.md)、[GETTING_STARTED_1_0.md](GETTING_STARTED_1_0.md)、[PLATFORM_SUPPORT_1_0.md](PLATFORM_SUPPORT_1_0.md)、[API_STABILITY_1_0.md](API_STABILITY_1_0.md)

## 1. 定位

`release_check.ps1` 把 1.0 发布前**可自动化**的检查收敛为一条命令：

- 版本一致性（Cargo vs 安装包/deb/MSIX/RELEASE_NOTES）；
- 必需文档/脚本/模板/插件/包 registry/字体存在；
- FFI contract（Rust/C#/装配三方）；
- workspace 构建/检查、关键测试（core/player/net/package/plugin/physics）；
- `capability_demo` headless 能力组合；
- `perf_smoke` 性能门禁 + `soak` 稳定性冒烟；
- Player fixture 构建 + `--validate`（或校验外部 `-PlayerRoot`）；
- `fresh_start_smoke.ps1` 模板 → 工程 → Player dry-run；
- 输出 `release-report.json` + `REPORT.md`，失败非 0。

无法自动化的发布项（真实安装包/干净机器/30 分钟 soak/交互冒烟/Player host/签字等）会输出到
JSON `manual_items` 与报告末尾，**不得默认通过**。

## 2. 用法

```powershell
# 仓库根目录（Windows / Linux，需 pwsh 7）
# 快速预检（CI / 本地；debug 构建、10s soak）
pwsh -File engine/scripts/release_check.ps1 -Quick -Offline

# 完整检查（release benchmark + 60s soak；耗时更长）
pwsh -File engine/scripts/release_check.ps1

# 复用已有 target 产物（跳过 workspace 构建检查；Player 二进制缺失时该检查 skip）
pwsh -File engine/scripts/release_check.ps1 -Quick -Offline -SkipBuild

# 已有 Player 包（含 manifest.json）时跳过 fixture 构建，直接 --validate
pwsh -File engine/scripts/release_check.ps1 -Quick -Offline -PlayerRoot dist/player/windows
```

参数：

| 参数 | 说明 |
|---|---|
| `-Offline` | cargo `--offline`、子脚本 `-Offline`；CI 先 `cargo fetch` 预热缓存 |
| `-Quick` | debug 构建/测试、capability steps=60、perf debug quick、soak 10s（默认 60s release） |
| `-SkipBuild` | 跳过 workspace 构建；Player 构建若无产物则 skip |
| `-OutDir` | 输出目录，默认 `test-results/release-check` |
| `-PlayerRoot` | 外部 Player 包根目录；提供时直接 validate，不构建 fixture |

输出：

```text
test-results/release-check/
├── release-report.json       # 机器可读（checks/manual_items/warnings/reports）
├── REPORT.md                 # 人可读摘要（失败项/警告/人工项）
├── reports/                  # FFI/perf/soak/capability/player/fresh-start 日志与子报告
├── logs/player/player.log    # Player validate 日志
├── player-fixture/           # minimal fixture 与构建产物
└── fresh-start/              # 模板 → 工程 → Player dry-run 报告
```

退出码：`0` 通过（含 warning/skip）；`1` 有 fail；`2` 脚本/环境错误。

## 3. 自动检查项

| id | 检查 | 依据 |
|---|---|---|
| `version.workspace` | workspace 版本解析 | `engine/Cargo.toml` |
| `version.installers` | deb 默认版本、MSI `$(var.Version)`、MSIX 版本读取/模板覆盖 | installer 脚本 |
| `version.release_notes` | `RELEASE_NOTES_<version>.md` 且含版本号 | 根目录 |
| `required.files` | 必需 docs/scripts/templates/plugins/packages/font/installer | 清单见脚本 |
| `repo.gitignore_artifacts` | `dist/`、`installer/stage/`、`build-temp/`、`download/` 忽略 | `.gitignore` |
| `ffi.contract` | `scripts/check_ffi_contract.py` | 343 字段基线 |
| `workspace.build` | `cargo check --workspace`（Quick）/`cargo build --workspace` | workspace |
| `tests.key` | `cargo test -p hezhou-core -p hezhou-player -p hezhou-net -p hezhou-package -p hezhou-plugin -p hezhou-physics` | 关键 crate |
| `capability.demo` | `capability_demo --steps N --out report.json` | examples |
| `perf.quick` | `perf_baseline.ps1`（Quick=debug quick，否则 release） | 性能门禁 |
| `soak.quick` | `soak_test.ps1`（Quick=debug 10s，否则 release ci 60s） | 稳定性门禁 |
| `player.validate` | minimal fixture 真实构建 + `--validate`，或 `-PlayerRoot` 校验 | Player |
| `fresh_start.smoke` | `fresh_start_smoke.ps1 -Template all` | 模板/开发体验 |

## 4. 未自动化项（manual_items）

脚本从 `docs/1.0_RELEASE_CHECKLIST.md` 抽取仍需人工的项，至少包括：

1. 1.0.0 版本 bump + `RELEASE_NOTES_1.0.0.md`（当前仓库仍为 0.4.x RC 基线）；
2. MSI/MSIX/DEB 真实打包、签名、干净机器安装/启动/卸载；
3. 推送 tag 后 CI 全链路（rust-tests/csharp/perf/soak/package/publish）全绿；
4. Player 完整游戏 host（`--host default`）与 `PlayerGame`/`PlayerCSharpContext` 端到端（**1.0 阻断项**）；
5. 1.0 构建上的 30 分钟 soak（1800s）复核；
6. 编辑器 30+ 分钟交互/导出冒烟（FPS/帧时间/内存）；
7. `diagnostics.ps1` 普通模式 Windows/Linux 双平台人工验证；
8. README 文档入口/交叉链接/FAQ/已知限制评审；
9. 发版签字（引擎/Player/测试/文档/发布负责人）；
10. Release 资产上传、回滚预案、发布后 24h 观察。

> 这些项不会被脚本标记为 pass；JSON 中 `manual_items` 带 `checklist` 章节引用与原因。

## 5. fresh-start 冒烟

`fresh_start_smoke.ps1` 在 `test-results/fresh-start-smoke/<template>/` 下：

1. 复制 `engine/templates/{empty-3d,action-sandbox}` 到临时工程；
2. 替换 `{{PROJECT_NAME}}` / `{{VERSION}}` / `{{ENGINE_VERSION}}`；
3. 校验 `template.json` 文件清单、`project.json`、`build.settings.json`（窗口/场景/targets/可执行名）；
4. 由 `build.settings.json` 生成 Player `build-config.json` + legacy `cook-manifest.json`；
5. 调用 `hezhou-player-build --plan-out --target both` 做 **Player dry-run**（不写产物二进制），
   校验 windows/linux plan 生成。

```powershell
pwsh -File engine/scripts/fresh_start_smoke.ps1 -Template all -Offline
pwsh -File engine/scripts/fresh_start_smoke.ps1 -Template empty-3d -SkipBuild
```

输出 `smoke-report.json` + `REPORT.txt`；退出码 0/1/2。该检查**不依赖 GUI**。

## 6. CI `release-check` job

`.github/workflows/ci.yml` 新增独立 job：

- 触发：`workflow_dispatch` 或 tag `v*`（与 package/publish 一致，但**不作为它们的依赖**）；
- `ubuntu-latest`，安装 Rust、Mono、CMake/Ninja、Vulkan/GLFW/X11/Wayland/ALSA 依赖；
- `cargo fetch` 预热缓存后运行：

```bash
pwsh -File engine/scripts/release_check.ps1 -Quick -Offline -OutDir test-results/release-check
```

- 上传 `test-results/release-check/**` artifact；timeout 20 分钟；
- 现有 `package-*`/`publish-release` 依赖不变，本 job 可单独失败/重跑。

## 7. 失败处理

1. 看 `REPORT.md` 的「失败项」与对应 `reports/*.txt` 日志；
2. 版本/文件类：修正版本号、补文档/模板/registry 或安装包脚本；
3. FFI：按 `docs/API_STABILITY_1_0.md` 同步 Rust/C#/装配字段并更新基线；
4. 构建/测试：先本地复现 `cargo check`/`cargo test`；
5. perf/soak：查看子报告 `reports/perf-report.json`、`reports/soak-report.json`，区分环境噪声与真实回归；
6. Player/fresh-start：看 `logs/player/player.log`、`fresh-start/*/player/plan`；
7. `skip` 项（`-SkipBuild`/平台不支持）必须在最终发布记录中说明原因，不能当作通过。

## 8. 1.0 放行标准

- **必须全绿**：`release_check.ps1` 无 fail；CI `release-check` 全绿；tag CI 的
  rust-tests/csharp-build/perf-baseline/soak-quick/diagnostics-smoke/player-build/
  package-windows/package-linux/package-player/publish-release 全绿；
- **人工项全部完成并签字**：30 分钟 soak、编辑器交互、干净机器安装、Player full host、
  发版签字、回滚预案；
- 版本/发布说明/安装包版本一致；1.0 阻断项（Player host、30min soak、双平台安装包）关闭；
- 已知非阻断限制在 `RELEASE_NOTES_1.0.0.md` 与文档中明确；
- 任一自动检查 fail 或 1.0 阻断项未关闭 → **不许发布**。

## 9. 维护

- 新增 docs/模板/脚本/插件/包样本时，同步更新 `release_check.ps1` 的 `requiredFiles`；
- 新增自动化可行的人工项时，从 `manualItems` 移除并加对应 check；
- 报告格式 `format_version=1`；字段变更需同步 docs 与 CI artifact 消费方。
