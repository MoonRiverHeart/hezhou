# AI 意图测试与自动化点击测试

## 1. 分层

| 层 | 入口 | 目标 |
|----|------|------|
| UT | `cargo test -p hezhou-core -p hezhou-dfx -p hezhou-ui` | 纯 Rust 逻辑、解析、属性反射、场景 |
| FUZZ | `cargo +nightly fuzz run project_json` + `project_fuzz_smoke` | 任意 JSON 输入不 panic |
| 接口 | `python scripts/check_ffi_contract.py` | Rust FfiContext / C# FfiContext / 编辑器装配三方字段一致 |
| 集成 | `project_roundtrip`、`project_ffi_contract` | 工程保存/加载、FFI 往返 |
| E2E | `scripts/run_autotest.ps1` | 真实编辑器进程、UI 点击、截图、性能、错误采集 |
| 意图 | `HEZHOU_AUTOTEST_MODE=intent` | 用户意图级功能，不依赖具体 widget id |

## 2. 端到端自动点击

```powershell
cd engine
powershell -ExecutionPolicy Bypass -File scripts/run_autotest.ps1 `
  -Mode random -Steps 30 -Duration 10 -Seed 42 -ReportDir test-results/random
```

产物：

```text
test-results/random/
├── report.json       # 结构化报告（步骤/指标/错误）
├── summary.txt       # 人类可读摘要
├── ui_tree.txt       # 完整 UI 树
├── final.png         # 最终截图
└── DONE              # 给 Rust 主循环的退出信号
```

同时 `traces/trace_*.json` 保存 DFX trace，`screenshots/step_*.png` 保存定期截图。

### 指标

- `frames`、`avg_frame_ms`、`min_frame_ms`、`max_frame_ms`
- `max_frame_gap_ms`、`stalls_over_250ms`
- `telemetry.warnings/errors/fatal`
- 每步 `success`、`error`、`screenshot`、`path`、`x/y`
- 单帧 >3s 自动记录 `Engine Freeze` 失败步骤

### 模式

| Mode | 行为 |
|------|------|
| `sequential` | 按 UI 树顺序点击所有可交互控件 |
| `random` | 固定 seed 随机点击 |
| `stress` | 持续随机点击直到 duration |
| `intent` | 运行 `IntentTestRunner` 的意图用例 |

可交互类型过滤在 `UITestRunner.TraverseWidgetTree`，排除了 Panel/VStack 等纯容器。

## 3. 意图测试

当前用例（`engine/scripts/IntentTestRunner.cs`）：

- `intent.new_scene_defaults`
- `intent.create_select_delete_entity`
- `intent.running_paused_editing`

运行：

```powershell
$env:HEZHOU_AUTOTEST='1'
$env:HEZHOU_AUTOTEST_MODE='intent'
$env:HEZHOU_AUTOTEST_DIR='test-results/intent'
engine\target\release\mono_editor_demo.exe --auto-test
```

或者：

```powershell
cd engine
powershell -ExecutionPolicy Bypass -File scripts/run_autotest.ps1 -Mode intent -ReportDir test-results/intent
```

### AI 如何新增意图

1. 把用户目标写成一句可验证的意图，例如“选中一个 Cube 后删除，实体数恢复”。
2. 在 `IntentTestRunner.RunAll()` 增加一个 `RunCase("intent.xxx", Method)`。
3. 只通过 `EditorScript.Test*` 钩子操作编辑器，避免依赖易变的 widget id。
4. 运行 `run_autotest.ps1 -Mode intent`，检查 `report.json` 和 `summary.txt`。
5. 如果用例失败，先看 `errors[]`、`ui_tree.txt`、`final.png`、`traces/`，再决定是修产品还是修测试。

### 后续演进

- 将 `IntentTestRunner` 的硬编码 case 迁移到 JSON DSL（action/assert/wait），由 Python 生成并回放。
- 每个 P0 功能（撤销/重做、父子层级、多选、Console）必须新增至少一个意图 case。
- AI 生成的意图用例先进入 `test-results` 草稿，稳定后沉淀到 `IntentTestRunner`。

## 4. DFX 关联

- `TestTelemetry` 采集 C# `Warn/Error/Fatal`，写入 report。
- `Log.Error` 同时写入 DFX 文件日志和测试报告。
- `UI.DebugDumpTree()` 输出完整 UI 树，供失败定位。
- `UI.CaptureScreenshotToFile()` 采集步骤/最终截图。
- `ui_simulate_click_at` 走完整 TouchBegin → hit_test → gesture → TouchEnd 事件链。
- Rust 主循环检测 `DONE` 后退出并保存 trace。
