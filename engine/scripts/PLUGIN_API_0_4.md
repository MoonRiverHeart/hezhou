# 0.4 编辑器声明式插件 API（task-55）

`hezhou-plugin` crate + 编辑器「插件」Tab：本地 `plugin.json` 声明命令，启用后加入 Command Palette。
**不加载动态 Rust 代码 / 不执行任意命令**，命令 action 限定内置白名单。

| 层 | 文件 |
|---|---|
| crate | `engine/plugin/`（manifest/registry） |
| 示例插件 | `engine/plugins/{sample-tools,scene-tools}/plugin.json` |
| 编辑器 FFI | `engine/examples/src/editor/plugin_ffi.rs`（新增） |
| FfiContext | `engine/scripting/src/ffi_context/{renderer.rs,mod.rs}`（tail，基线 **343**） |
| C# UI | `engine/scripts/{UI.cs,UI.Scene.cs,EditorScript*.cs,TestRunner.cs}`（插件 Tab + 命令面板） |
| 契约 | `engine/scripts/check_ffi_contract.py`（`EXPECTED_FIELD_COUNT = 343`） |

## 1. 插件格式（`<project>/plugins/<id>/plugin.json`）

```json
{
  "id": "sample-tools",
  "name": "Sample Tools",
  "version": "0.1.0",
  "api_version": "0.4",
  "description": "示例插件",
  "author": "Hezhou",
  "enabled_by_default": true,
  "commands": [
    { "id": "toggle-theme", "title": "切换深浅主题", "shortcut": "", "action": "toggle_theme" },
    { "id": "console", "title": "打开 Console", "shortcut": "", "action": "console" }
  ],
  "menus": [ { "id": "menu", "title": "示例插件", "command": "console" } ],
  "assets": ["assets/readme.txt"],
  "scripts": []
}
```

校验（`PluginManifest::validate`）：
- `id`：小写字母/数字开头，允许 `. - _`，≤64；`version`：`x.y.z`；`api_version` 必须为 `0.4` 或 `0.4.*`；
- 命令 id 唯一合法、title 非空、**action 必须在白名单内**；菜单必须引用已有命令；
- `assets`/`scripts` 必须是安全相对路径（禁止绝对路径/`..`）。

## 2. Action 白名单（`ACTION_WHITELIST`）

`new_project` / `open_project` / `save` / `toggle_theme` / `console` / `command_palette` /
`profiler_tab` / `packages_tab` / `new_scene`

- C# 侧 `ExecutePluginAction` 映射到现有编辑器操作；白名单外 action 在扫描阶段即拒绝（不会加载）；
- `profiler_tab`/`packages_tab` 当前为面板定位提示（不强制切换 Tab，避免破坏 Dock 状态）；
- 不执行 shell/脚本/任意 DLL。

## 3. 发现与启用

- Registry 扫描 `<project>/plugins/*/plugin.json`（目录不存在 → 空）；重复 id/无效清单 → 可读错误；
- 启用状态文件：`<project>/plugins/enabled.json`（`{"enabled":["sample-tools"]}`）；
  - 文件不存在时用各插件 `enabled_by_default`；
  - `plugin_set_enabled` 写文件持久化；未知插件 → 失败；
- 命令只暴露**已启用**插件的命令（`enabled_commands`）。

示例插件：
| id | 默认 | 命令 |
|---|---|---|
| `sample-tools` | 启用 | `toggle_theme`、`console` |
| `scene-tools` | 禁用 | `save`、`profiler_tab` |

## 4. FFI（新增 3，FfiContext 343）

| FFI | 签名 | 语义 |
|---|---|---|
| `plugin_list_json` | `fn(project_root,buf,len) -> usize` | 插件列表 JSON（id/name/version/api_version/author/enabled/command_count/commands/root） |
| `plugin_set_enabled` | `fn(project_root,id,enabled) -> i32` | 0=成功；-1=未知插件/IO/扫描错误 |
| `plugin_commands_json` | `fn(project_root,buf,len) -> usize` | 仅启用插件命令 JSON（plugin_id/id/title/shortcut/action） |

C# 包装：`UI.PluginListJson(root)`、`UI.PluginSetEnabled(root,id,enabled)`、`UI.PluginCommandsJson(root)`。

## 5. 编辑器 UI

- 资产面板新增「插件」Tab：每插件一行 `启用/禁用  <name> <version> · <author>`；
  状态标签显示插件数/启用数/扫描错误；「刷新插件」按钮；命令数提示；
- 工程加载（`RefreshSceneTreeAfterLoad`）自动刷新插件状态；
- **Command Palette**（Ctrl+Shift+P）：内置命令后追加启用插件命令（`[插件] <title>  <shortcut>`），
  点击执行 `ExecutePluginAction`；未知 action 拒绝（Console 告警）；
- 失败只写 Console/状态栏，不崩溃。

## 6. 测试证据

- `cargo test -p hezhou-plugin`：**5 passed**（manifest/白名单校验、默认启用扫描、启用持久化+命令过滤、未知 action/坏路径拒绝、列表/命令 JSON）
- 编辑器启动自检：**PASS=117, FAIL=0**（task-52 的 112 + 插件 5 项），证据
  （`engine/logs/hezhou_2026-11-08.log` 行 5042-5065）：
  - 插件列表含 sample-tools/scene-tools；启用 scene-tools 并写 enabled.json
  - 启用命令 JSON 含 toggle_theme/console/profiler_tab
  - action 白名单执行（console=true，exec_shell=false 被拒绝）
  - 禁用后命令不再暴露；自检结束恢复/删除 enabled.json（无残留）
- 契约 **343/343/343 OK**；`build_mono.ps1` exit 0（DLL 436,736 字节 / 426.5 KB）；
  `ci_local.ps1 -SkipBuild` 通过（需可写 TMP/TEMP 供 glfw-sys CMake/MSBuild）。

## 7. 限制 / 后续

1. 仅**声明式**插件：无动态 Rust 代码/DLL/Hot Reload；不支持第三方程序集加载。
2. 无权限沙箱/签名/来源校验；插件只能引用内置 action 白名单（安全但能力有限）。
3. 无插件设置项/菜单树渲染（menus 仅校验与预留）；无插件市场/远程分发。
4. 启用状态按工程目录（`plugins/enabled.json`）；未接入 Project 文件与版本迁移。
5. `profiler_tab`/`packages_tab` 为面板提示而非强制切换；命令无参数/多级子菜单。
