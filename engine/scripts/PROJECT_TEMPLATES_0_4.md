# 0.4 项目模板与新建工程（task-49）

`engine/templates/` 提供项目模板，编辑器「工程」Tab 的「新建项目（模板）」可从模板创建工程并打开。

| 层 | 文件 |
|---|---|
| 模板 | `engine/templates/{empty-3d,action-sandbox}/`（`template.json` + 工程文件） |
| Rust 模板管理 | `engine/examples/src/editor/project_templates.rs`（新增） |
| FfiContext | `engine/scripting/src/ffi_context/{renderer.rs,mod.rs}`（tail，基线 **336**） |
| C# UI | `engine/scripts/{UI.cs,UI.Scene.cs,EditorScript*.cs,TestRunner.cs}` |
| 契约 | `engine/scripts/check_ffi_contract.py`（`EXPECTED_FIELD_COUNT = 336`） |

## 1. 模板格式

```text
engine/templates/<template-id>/
├── template.json
├── project.json          # {{PROJECT_NAME}} 等占位符
├── build.settings.json   # Player 导出设置
├── README.md
├── assets/README.md
├── assets/scenes/main.scene
└── scripts/README.md (+ 可选示例 .cs)
```

`template.json`：

```json
{
  "id": "empty-3d",
  "name": "Empty 3D",
  "version": "0.4.0",
  "description": "空 3D 工程模板…",
  "min_engine_version": "0.3.0",
  "files": ["project.json", "build.settings.json", "README.md", "assets/scenes/main.scene", "assets/README.md", "scripts/README.md"]
}
```

- `files` 是**文件白名单**（相对模板根，拒绝绝对路径/`..`）；为空时递归复制（不含 `template.json`）。
- 文本文件（json/md/txt/cs/scene/prefab/settings/meta）做占位符替换；其他文件原样复制。
- 占位符：`{{PROJECT_NAME}}`、`{{VERSION}}`（模板版本）、`{{TEMPLATE_ID}}`、`{{ENGINE_VERSION}}`。
- JSON 文件替换后必须可解析，否则创建失败并清理半成品。

内置模板：

| id | 内容 |
|---|---|
| `empty-3d` | 空工程：project.json + build.settings.json + assets/scenes/main.scene + scripts/README |
| `action-sandbox` | Cube（PBR 橙色材质 + 内置 `RotatingEntity` 脚本绑定）+ 方向光 + 场景 + `scripts/Spin.cs` 示例 |

## 2. FFI（新增 2，FfiContext 336）

| FFI | 签名 | 语义 |
|---|---|---|
| `template_list_json` | `fn(buf,len) -> usize` | 扫描模板返回 `{"count":N,"templates":[{id,name,version,description,min_engine_version,file_count}]}` |
| `template_create` | `fn(template_id, dest_dir, project_name) -> i32` | 0=成功；-1 参数/模板无效；-2 目标已存在且非空/不是目录；-3 IO/模板文件失败（已清理半成品） |

校验：

- 模板 id：`[A-Za-z0-9_-]{1,64}` 且必须存在；
- 项目名：非空、≤64 字符、禁止 `/ \ : * ? " < > |` 与控制字符、非 `.`/`..`；
- 目标目录：非空、拒绝 `..` 组件；不存在则创建；存在必须是**空目录**；
- 模板根定位：cwd/templates → cwd/../templates → `CARGO_MANIFEST_DIR/../templates`（开发）→ exe 目录候选（安装版）。

C# 包装：`UI.TemplateListJson()`、`UI.TemplateCreate(id,dest,name)`。

## 3. 编辑器用法

资产面板 →「工程」Tab → **新建项目（模板）**：

1. 对话框加载模板列表（来自 `template_list_json`）→ 模板下拉；
2. 填「项目名」（默认 MyGame）与「目标目录」（默认 `<cwd>/MyGame`，必须不存在或空）；
3. 点「创建」：
   - `template_create` 成功 → `Project.Load(dest)` → 清理脚本实例 → `SyncToScene` → 刷新层级/属性/撤销 → `AssetOpenProject(dest)` + `AssetScan` → 重新加载 Build Settings；
   - 失败按 rc 显示「模板/项目名/目录参数无效」「目标目录已存在且非空」「IO 失败」到 Console/状态栏，不崩溃。

## 4. 测试证据

- Rust 单元测试（`project_templates` 4 项，**4 passed**）：
  - 扫描内置模板（含 empty-3d/action-sandbox，白名单非空）
  - empty-3d 创建 + 占位符替换（project.json/build.settings.json 含项目名、无残留占位符、assets/scripts 占位文件存在）
  - action-sandbox 创建（Cube/方向光/pbr_material/Spin.cs/场景含项目名）
  - 非空目标拒绝、非法项目名拒绝、非法相对路径拒绝、不存在模板拒绝
- 编辑器启动自检：**PASS=106, FAIL=0**（task-48 的 102 + 模板 4 项），证据（`engine/logs/hezhou_2026-11-08.log` 行 3819-3826）：
  - `模板列表包含 empty-3d/action-sandbox (len=399)`
  - `创建模板工程+占位替换 (rc=0)`
  - `目标已存在非空被拒绝 (rc=-2)`
  - `加载模板工程 (loaded=True, name=TemplateSelfTest)`
  - 自检结束删除临时工程（logs/ 下无残留）
- FFI contract **336/336/336 OK**；`build_mono.ps1` exit 0（DLL 422,912 字节 / 413 KB）；
  `ci_local.ps1 -SkipBuild` 通过（需可写 TMP/TEMP 供 glfw-sys CMake/MSBuild）。
- 注：首次冒烟曾出现 3 个 PlayerBuild FAIL（`清单引擎版本 0.3.0 与运行时 0.2.0 不兼容`），
  原因是 `target/debug/hezhou-player.exe` 为版本升级前旧产物；`cargo build -p hezhou-player`
  重建后复跑 **PASS=106, FAIL=0**（非本任务代码问题）。

## 5. 限制 / 后续

1. 无模板版本迁移/升级；`min_engine_version` 仅记录未强制校验。
2. 无包管理器/依赖解析/远程模板；模板仅本地 `engine/templates`。
3. 工程 C# 脚本尚未加入 EditorScript.dll 编译流程（示例脚本仅供结构说明；场景绑定用已编译内置脚本）。
4. 安装版模板目录需随安装包分发（当前定位逻辑已含 exe 目录候选，但打包脚本未包含 templates）。
5. 无模板预览/缩略图；新建工程对话框为简单字段（无目录浏览按钮）。
