# 编辑器 Build Settings 与 Player 导出配置（0.3 task-41）

项目级构建设置 `build.settings.json`：编辑器 UI 读写/校验，Player 导出（dry-run/build）优先使用。
配套：`FFI_PREFAF_SCENE_PLAYER_0_2.md`（导出链路）、`FFI_PROFILER_0_2.md`（Profiler）。

| 层 | 文件 |
|---|---|
| 设置模型/校验/FFI | `engine/examples/src/editor/build_settings.rs`（新增） |
| 导出集成 | `engine/examples/src/editor/prefab_scene_player_ffi.rs`（request 缺失字段从设置取） |
| FfiContext | `engine/scripting/src/ffi_context/{renderer.rs,mod.rs}`（tail，基线 **322**） |
| C# UI | `engine/scripts/{UI.cs,UI.Scene.cs,EditorScript*.cs,TestRunner.cs}`（资产面板「Build」Tab） |
| 契约 | `engine/scripts/check_ffi_contract.py`（`EXPECTED_FIELD_COUNT = 322`） |

## 1. 文件格式（`<project_root>/build.settings.json`）

```json
{
  "product_name": "My Game",
  "executable_name": "my-game",
  "start_scene": "scenes/main.scene",
  "quality": "high",
  "window": { "width": 1280, "height": 720, "mode": "windowed" },
  "targets": ["windows", "linux"],
  "version": "1.2.3",
  "log_dir": "logs",
  "required_font": "fonts/HarmonyOS_Sans_SC.ttf"
}
```

- 所有字段 `serde(default)`；缺字段/旧文件自动补默认值，未知字段忽略。
- `quality`：`low`/`medium`/`high`/`ultra`；`window.mode`：`windowed`/`borderless_fullscreen`/`fullscreen`；
  `targets` 仅 `windows`/`linux`。
- `start_scene`/`required_font` 相对 `assets/`；`required_font` 为 `null` 表示不检查字体。
- **缺失文件时自动推断**：`project_name` 优先读 `project.json` 的 `name`；`start_scene` 优先第一个
  `.scene`/`.hzscene` 资产，否则第一个文件资产（告警），`assets/` 为空时用
  `scenes/main.scene.json`（导出 dry-run 会生成合成条目）。

## 2. 校验规则（错误=拒绝保存）

- `product_name` 非空且 ≤64 字符；
- `executable_name` 1~64 个 ASCII 字母数字/`-`/`_`；
- `version` 为 `major.minor.patch`（允许 `-`/`+` 后缀）；
- `quality` 在支持列表内；
- `window.width` 320..=7680、`window.height` 240..=4320、`window.mode` 合法；
- `targets` 非空且只含 `windows`/`linux`；
- `start_scene` 非空、安全相对路径，且 **`assets/<start_scene>` 文件存在**；
- `log_dir` 非空安全相对路径；`required_font` 若给出须为安全相对路径（存在性由导出时告警处理）。

## 3. FFI（新增 3，FfiContext 322）

| FFI | 签名 | 语义 |
|---|---|---|
| `build_settings_snapshot_json` | `fn(buf, len) -> usize` | 读取当前工程设置（缺失时推断默认值），返回字节数 |
| `build_settings_save_json` | `fn(json) -> i32` | 0=成功，-1=解析失败，-2=校验失败，-3=写盘失败 |
| `build_settings_validate_json` | `fn(json, errors_buf, errors_len) -> i32` | 0=通过，-1=解析失败，-2=校验失败；错误文本写入 buffers |

C# 包装：`UI.BuildSettingsSnapshotJson()`、`UI.BuildSettingsSaveJson(json)`、
`UI.BuildSettingsValidateJson(json, out errors)`。

## 4. 编辑器 UI（资产面板 →「Build」Tab）

字段（InputField）：产品名、可执行名、版本、启动场景、质量档、窗口宽/高/模式、目标平台
（逗号分隔）、日志目录、必检字体。

- **重新读取**：从 `build.settings.json` 重新加载面板（启动时自动读取；工程/场景加载后
  通过 `RefreshSceneTreeAfterLoad` 自动重新读取）；
- **保存设置**：先校验，通过后写 `build.settings.json`；失败只写 Console/状态栏；
- **校验**：显示通过/失败原因；
- 状态栏：文件路径 + 最近一次读取/校验/保存结果。

## 5. Player 导出使用设置

- C# 导出请求的产品名/版本/启动场景/必检字体来自 Build Settings 面板字段；未填字段由 Rust
  侧从 `build.settings.json` 补齐（request 显式字段优先）。
- 输出目录留空时默认 `<project_root>/exports/<product_name>/<platform>/`（dry-run 与真实构建共用）。
- `quality/window/executable_name/log_dir` 映射到 `hezhou_player::PlayerBuildConfig`
  （`with_quality`/`with_window`/`with_executable_name`/`log_dir`）。
- 真实构建前校验 `targets` 是否包含当前平台，不包含则返回错误（只日志/摘要）。
- 启动场景/字体缺失时沿用既有降级：字体缺失关闭必检并告警；场景由 dry-run 合成条目兜底。

## 6. 测试证据

- Rust 单元测试（`build_settings` 4 项）：JSON roundtrip、默认推断有效、坏值校验拒绝、JSON 解析失败；
  `cargo test -p hezhou-examples --bin mono_editor_demo --features mono build_settings` → **4 passed**。
- 编辑器启动自检：**PASS=84, FAIL=0**（task-39 的 80 + Build Settings 4 项）：
  - `Build Settings 快照包含必要字段`
  - `校验有效设置返回 0`
  - `保存 build.settings.json 成功`（自检结束已恢复/删除，不污染仓库）
  - `Player dry-run 使用 Build Settings (start_scene=icons/check.svg)`（产品/版本/启动场景来自设置）
- 契约 **322/322/322 OK**；`build_mono.ps1` exit 0 部署 DLL；
  `ci_local.ps1 -SkipBuild`（若 physics 等并发改动完成则全绿）。

## 7. 限制 / 后续

1. 只支持单一项目配置，未做多 profile / 命令行覆盖（后续可加 `--settings <file>`）。
2. `targets` 目前只用于校验/提示，跨平台真实构建仍需在对应平台执行（PlayerBuilder 限制）。
3. Build Settings 与 Project 文件分离；加载工程目录变化时以当前工作目录为准（编辑器暂用 cwd）。
4. `required_font` 存在性未作为保存错误（构建时告警），避免新工程必须已有字体资产。
5. 未把窗口标题独立于 product_name；未支持自定义图标/启动参数。
