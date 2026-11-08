# Hezhou API 稳定性与兼容规则（1.0 方向）

> 基线：仓库 **0.4.0 RC**（FFI 343 字段、编辑器启动自检 PASS=117）。1.0 尚未发布。
> 本文件定义「哪些 API/格式可以在 1.0 后放心依赖」，以及变更/弃用/ABI 追加规则。

## 1. 稳定性等级定义

| 等级 | 含义 | 变更规则 |
|---|---|---|
| **Stable（稳定）** | 1.0 后必须保持向后兼容 | 破坏性变更需 major 版本；弃用需公告 + 至少一个 minor 周期 |
| **Experimental（实验）** | 0.x 可调整；1.0 前可能变化 | 变更写入 release notes；不承诺跨版本兼容 |
| **Internal（内部）** | 编辑器实现细节，不供游戏代码依赖 | 可随时改；不保证 |

## 2. 稳定性清单

| API / 格式 | 入口 | 等级 | 说明 |
|---|---|---|---|
| C# `Hezhou.Engine` 外观 | `Hezhou.Engine.{Engine,Scene,Entity,Time,Audio,Animation,Debug}` | **Stable**（1.0 目标） | 自 0.3/0.4 起提供；游戏脚本应优先使用它，不直接依赖 UI |
| C# `UI.*` / `UI.Scene.*` 静态包装 | `engine/scripts/UI*.cs` | **Internal** | 编辑器内部 FFI 包装；字段/方法可随编辑器重构变化 |
| C# `EditorScript` / 编辑器内部类型 | `EditorScript*.cs` | **Internal** | 仅编辑器使用 |
| C# `Hezhou.PlayerGame` | `PlayerGame.Initialize(IntPtr)/Update(float)/Shutdown()` | **Experimental** | 位于 EditorScript.dll；Player context/程序集统一为 1.0 工作 |
| `FfiContext`（Rust↔C# 函数指针表） | `engine/scripting/src/ffi_context/` | **Stable-additive（ABI 追加）** | 只允许尾部追加字段；当前基线 **343**；见 §3 |
| `PlayerCSharpContext` | `engine/player/src/csharp_context.rs` | **Experimental（版本化追加）** | 当前 `PLAYER_CSHARP_CONTEXT_VERSION = 6`；v2..v6 仅尾部追加，C# 先读 `Version` |
| Project 工程格式 | `project.json`（`version` 字段） | **Stable（格式 1.0）** | 缺字段用 serde 默认值兼容旧工程（如 `pbr_material`/`light`/`parent_id`） |
| Prefab 格式 | `*.prefab.json`（`version` 1.0） | **Stable（格式 1.0）** | 未知字段忽略、缺失可选字段默认；0.2 起扩展 |
| AssetId / `.meta` / cook manifest | `hezhou-asset` | **Stable（format_version 1）** | AssetId 为 UUID 字符串；`.meta`/cook manifest `format_version=1` |
| Player manifest / PlayerBuildConfig | `hezhou-player` | **Experimental** | `manifest.json` 结构版本字段存在；1.0 前字段可能追加（尾部/可选） |
| `build.settings.json` | 编辑器 Build Settings | **Experimental** | 字段已知但 0.x 可调整；导出前请用编辑器校验 |
| 包清单 / lockfile | `hezhou-package` | **Experimental** | 0.4 首个版本；版本范围语义最小子集 |
| 插件清单 / enabled.json | `hezhou-plugin` | **Experimental** | `api_version: "0.4"`；命令 action 白名单 |
| 项目模板 / template.json | `hezhou-templates` | **Experimental** | `version: 0.4.0`；占位符/白名单 |
| QML/UI 数据格式、DFX trace JSON | 编辑器/DFX | **Internal** | 可随工具演进 |

> 结论：**1.0 游戏代码只应依赖 `Hezhou.Engine`（Stable）**；需要底层/新能力时用 Experimental API，并接受 0.x 变更。

## 3. FFI / ABI 追加规则（FfiContext）

`FfiContext` 是 `#[repr(C)]` 扁平的函数指针表，C# `UI.cs` 用相同顺序的 `IntPtr` 字段镜像。规则：

1. **只允许尾部追加**新字段，禁止删除/重排/改类型；旧字段偏移必须保持。
2. Rust 与 C# 必须**同一顺序**同步追加（`scripts/UI.cs` 的 `FfiContext` 结构体）。
3. 新增 FFI 同时更新：
   - `scripting/src/ffi_context/*` 类型别名 + `mod.rs` 字段；
   - `examples/src/editor/mod.rs` 装配 + `mono_ui_thunk_demo.rs` stub；
   - `scripts/UI.cs` delegate + IntPtr 字段 + C# 包装；
   - `scripts/check_ffi_contract.py` 的 `EXPECTED_FIELD_COUNT`。
4. CI/本地强制校验：

```bash
cd engine
python scripts/check_ffi_contract.py    # Rust / C# / Editor assembly 三方字段数一致
pwsh -File scripts/build_mono.ps1
```

5. 运行时兼容：C# 侧通过 `Marshal.PtrToStructure` 读取同一内存布局；尾追加对旧 C# 程序集安全（旧代码不读新字段）。
6. `PlayerCSharpContext` 采用自己的 `Version` 字段（当前 6）：新增能力先升版本、尾部追加，C# 读取 `Version` 决定是否调用新函数指针。

## 4. 版本规则

- 引擎/编辑器/Player 共用 workspace 版本（`engine/Cargo.toml` `[workspace.package].version`，当前 **0.4.0**）；
- 发布 tag：`v<major>.<minor>.<patch>`（如 `v0.3.0`）；CI 在 tag 时打包并发布；
- **1.0 前（0.x）**：允许破坏性变更，但必须在 `RELEASE_NOTES_<version>.md` 明确列出「Breaking」与迁移方式；
- **1.0 起**：遵循 SemVer——major 升才可破坏 Stable API/格式；minor 只加功能/可选字段；patch 只修 bug；
- 格式文件（Project/Prefab/Asset/meta/Player manifest）都有显式版本字段；读取方对更高「主版本」应拒绝并给出可读错误（前向兼容靠次要版本/可选字段）。

## 5. 弃用流程（1.0 起）

1. 在 Release Notes 标注「Deprecated」及替代方案；
2. 保留至少一个 minor 周期的运行时/编译期警告（C# `[Obsolete]`、Rust `#[deprecated]`、JSON `warnings`）；
3. 下一个 major 才移除；移除前在迁移文档给出替代代码片段；
4. 格式字段弃用：停止写入但继续读取（旧工程可加载），并在 release notes 说明。

## 6. 格式/契约检查清单（提交前）

- [ ] 新增 FFI：尾追加 + C# 镜像 + contract 计数 + editor/mono stub 同步；
- [ ] 修改 Project/Prefab/Asset/meta/Player 格式：更新 `format_version`（主版本才拒绝旧文件）并加 roundtrip 测试；
- [ ] 修改 `Hezhou.Engine`：保持旧签名/语义；必须变更则走弃用流程；
- [ ] 更新文档：`docs/GETTING_STARTED_1_0.md`、`docs/API_STABILITY_1_0.md`、相关 `engine/scripts/FFI_*` 说明；
- [ ] 运行 `check_ffi_contract.py`、`build_mono.ps1`、`ci_local.ps1 -SkipBuild`、编辑器启动自检。

## 7. 已知不稳定点（1.0 前）

1. Player `--run/--host` 游戏 host 未完成（默认 pending，退出码 3），`PlayerCSharpContext` 仍可能扩展；
2. 网络（`hezhou-net`）无加密/预测/AOI，API 标 Experimental；
3. 插件/包管理器/模板为 0.4 首批，manifest 字段/语义可能调整；
4. `Hezhou.Engine` 位于 `EditorScript.dll`（非独立程序集），Player 独立 context 统一在 1.0 工作项；
5. `UI.*` 内部 API 随编辑器重构变化，不提供兼容承诺。
