# 0.4 最小本地包管理器（task-52）

`hezhou-package` crate + 编辑器「包」Tab：本地目录 registry → 依赖解析 → 安装到工程 `packages/` → 写 `lock.json`。

| 层 | 文件 |
|---|---|
| crate | `engine/package/`（manifest/registry/resolver/installer/lockfile） |
| 示例 registry | `engine/packages/registry/{hezhou-hello,hezhou-mathx,hezhou-gameplay-kit}` |
| 编辑器 FFI | `engine/examples/src/editor/package_ffi.rs`（新增） |
| FfiContext | `engine/scripting/src/ffi_context/{renderer.rs,mod.rs}`（tail，基线 **340**） |
| C# UI | `engine/scripts/{UI.cs,UI.Scene.cs,EditorScript*.cs,TestRunner.cs}`（资产面板「包」Tab） |
| 契约 | `engine/scripts/check_ffi_contract.py`（`EXPECTED_FIELD_COUNT = 340`） |

## 1. 包格式（`package.json`）

```json
{
  "name": "hezhou-gameplay-kit",
  "version": "0.1.0",
  "display_name": "Gameplay Kit",
  "description": "示例玩法包：依赖 hezhou-mathx",
  "dependencies": { "hezhou-mathx": "^0.1.0" },
  "assets": ["assets/kit.json"],
  "scripts": ["scripts/Kit.cs"],
  "entry": "Kit",
  "files": []
}
```

- `name`：小写字母/数字开头，允许 `. - _`，≤64；`version`：`major.minor.patch`；
- `dependencies`：name → 版本范围；支持 `*`、精确 `1.2.3`、`~1.2.3`、`^1.2.3`、`1.*`、`1.2.*`
  （不支持完整 npm 语义/预发布/多范围组合）；
- `assets`/`scripts`/`entry`/`files`：安全相对路径（禁止绝对路径与 `..`）；
- `files` 非空时为**复制白名单**（`package.json` 总会复制），为空复制整包。

## 2. Registry 布局

```text
engine/packages/registry/
├── hezhou-hello/package.json
├── hezhou-mathx/package.json
└── hezhou-gameplay-kit/package.json   # dependencies: hezhou-mathx ^0.1.0
```

扫描规则：`<registry>/<任意子目录>/package.json`；同名同版本重复 → 报错；同名多版本允许（选最高匹配）。
仅本地目录，不联网。

## 3. 依赖解析与安装

- `resolve_closure(registry, roots)`：DFS 选择最高匹配版本、依赖优先拓扑序；
  同一包只选一个版本，后续请求不满足 → **版本冲突**；检测**循环依赖**与**缺失/版本不匹配**；
- 安装：`<project>/packages/<name>/`（先复制到临时目录再原子替换）；
  `<project>/packages/lock.json` 记录 `name/version/hash(fnv1a64)/dependencies/path`；
  - 重复安装同版本 → 幂等（`noop:true`，不复制）；
  - 已安装不同版本 / 目标目录存在但 lock 无记录 → 拒绝；
  - 包名/路径校验防越界；
- 卸载：被其他已安装包依赖时拒绝；先检查后删除目录并更新 lock。

## 4. FFI（新增 4，FfiContext 340）

| FFI | 签名 | 语义 |
|---|---|---|
| `package_registry_list_json` | `fn(registry_dir, buf, len) -> usize` | JSON：root/count/packages[]/installed[]（dir 空=默认 `packages/registry`） |
| `package_install_json` | `fn(project_root, registry_dir, name, buf, len) -> i32` | 0=成功；-1=失败（JSON 含 error）；成功 JSON 含 installed/skipped/noop/lock_path |
| `package_installed_json` | `fn(project_root, buf, len) -> usize` | lock.json 内容（不存在返回空记录） |
| `package_uninstall` | `fn(project_root, name) -> i32` | 0=成功；-1=失败（未安装/被依赖/IO） |

C# 包装：`UI.PackageRegistryListJson(dir)`、`UI.PackageInstallJson(root,dir,name,out report)`、
`UI.PackageInstalledJson(root)`、`UI.PackageUninstall(root,name)`。

## 5. 编辑器「包」Tab

资产面板 →「包」：
- 「可安装（本地 registry）」：每个包一个 `安装  <display_name> (<name>@<version>)` 按钮；
- 「已安装（packages/lock.json）」：每个已安装包一个 `卸载  …` 按钮；
- 状态标签：registry 包数/已安装数/registry 错误；lock 路径 + 已安装数；
- 「刷新包列表」按钮；工程加载（`RefreshSceneTreeAfterLoad`）时自动刷新已安装列表；
- 安装/卸载失败只写 Console/状态栏（版本冲突/被依赖等），不崩溃。

## 6. 测试证据

- `cargo test -p hezhou-package`：**单元 2 + 集成 4 = 6 passed**
  - manifest 解析/校验、版本范围匹配
  - registry 扫描 + semver 选择（精确/tilde/latest）
  - 依赖拓扑序（c→b→a）、循环检测、缺失依赖
  - 安装依赖闭包 + lockfile（hash/dependencies/path）、重复安装幂等、依赖保护卸载、版本冲突/未知目录/非法名拒绝
- 编辑器启动自检：**PASS=112, FAIL=0**（task-49 的 106 + 包管理器 6 项），证据
  （`engine/logs/hezhou_2026-11-08.log` 行 4382-4393）：
  - registry 列表含 3 个示例包（len=574）
  - 安装 hezhou-gameplay-kit + 依赖 hezhou-mathx + lock.json
  - lockfile 含 name/version/hash/dependencies
  - 重复安装幂等（noop）；被依赖的 hezhou-mathx 拒绝卸载；卸载依赖方后成功
  - 自检临时工程已清理
- 契约 **340/340/340 OK**；`build_mono.ps1` exit 0（DLL 429,568 字节 / 419.5 KB）；
  `ci_local.ps1 -SkipBuild` 通过（需可写 TMP/TEMP 供 glfw-sys CMake/MSBuild）。

## 7. 限制 / 后续

1. 仅本地目录 registry；**无联网下载**、无发布/上传、无签名/校验和源验证。
2. 版本范围只支持最小子集（无 `>=`/`||`/预发布）；不做版本迁移/依赖覆盖（overrides）。
3. 无包内容合并到工程资产/脚本流程（安装仅复制到 `packages/`；工程 C# 脚本编译流程 0.4 未接入）。
4. 无并行下载/缓存/registry 索引文件；registry 每次全量扫描。
5. lockfile 无完整性签名；hash 为 fnv1a64 内容指纹（本地确定性，不防篡改）。
6. 编辑器卸载为逐个操作（无批量/依赖自动清理）；`hezhou-gameplay-kit` 依赖未安装时无法卸载。
