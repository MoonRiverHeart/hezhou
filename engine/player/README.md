# hezhou-player

Hezhou 0.2 **Player 导出构建器 + 最小运行时**（Windows / Linux）。

本 crate 是 `engine/player`，对应 `docs/PROJECT_PLAN.md` §1.5 Player 导出与
§9 AI 并行工作流中的 Player 工作流。已完成六轮：

1. **格式契约 + dry-run**：产物目录、`manifest.json`、跨平台路径规则、双平台清单；
2. **真实产物构建**：按计划复制 exe/assets/scripts/runtime/manifest，staging 原子提交、
   失败清理、大小/校验和/权限检查；最小 fixture 产出的包可通过 `hezhou-player --validate`；
3. **asset cooker 集成**：直接读取 `hezhou-asset` 的 `cook_manifest.json`
   （`format_version=1`、`roots`、`entries`），按 `AssetId` 映射资产、复制
   `imported/` + `assets/` 输出并写入 Player manifest（可追踪 id/kind/locator/output）；
4. **运行时资产索引 + host 接口**：运行时加载包内 `assets/cook_manifest.json` 建
   `AssetId → output/kind/locator` 索引并校验 output/start_scene/font；
   `--run`/`--host` 加载索引后调用 `PlayerHost` trait，默认 host 明确报告
   "等待 game host 接入"（退出码 3）；
5. **可选窗口渲染 host**（`feature = "window-host"`）：`WindowPlayerHost` 按清单创建
   GLFW 窗口 + `UIVulkanRenderer`，跑固定帧数（或窗口关闭）后返回 `Running`（退出码 0）；
6. **真实起始场景加载**：`scene_loader` 把 `manifest.start_scene` 对应的包内 Project JSON
   用 `hezhou_core::Project::restore_from_json` 恢复成 `Scene`（实体/层级/Transform），
   WindowPlayerHost 渲染该场景；缺失/坏 JSON/空场景回退内置 cube+light 并 WARN；
7. **cook 导入资产 bundle**：解析包内 `assets/imported/*.json`（`hezhou_asset::ImportedScene`），
   建立 `AssetId → mesh/material/texture/skeleton/animation` 查询，严格核对
   `cook_manifest`（kind/index/数量/scene 数据一致性），启动打印加载统计；错误安全
   （坏 JSON/不一致 → 可读 exit 2，未引用文件仅 WARN）。

不绑定具体游戏内容；真实 cooker 产物可原样替换测试 fixture。

```
engine/player/
├── src/
│   ├── lib.rs        # 公共 API 导出
│   ├── config.rs     # PlayerBuildConfig（导出设置）与校验
│   ├── manifest.rs   # manifest.json 契约与校验（含 cook 资产字段）
│   ├── asset_index.rs# 运行时资产索引（cook_manifest.json → AssetId 查询/校验）
│   ├── builder.rs    # 构建器：dry-run 计划 + 真实复制（预检/staging/校验/权限）
│   ├── cook_bridge.rs# hezhou-asset CookManifest → Player assets[]/文件映射
│   ├── imported_assets.rs# cook imported/*.json → ImportedScene / AssetId 查询（严格校验）
│   ├── scene_loader.rs# 起始场景：Project JSON → Scene（失败回退内置 cube+light）
│   ├── host.rs       # PlayerHost trait / HostOutcome / DefaultPlayerHost
│   ├── window_host.rs# （feature window-host）WindowPlayerHost：窗口+Vulkan 固定帧渲染
│   ├── paths.rs      # 逻辑路径规则（分隔符/大小写/保留名/权限位）
│   ├── platform.rs   # 目标平台差异（exe 命名/动态库查找/rpath/归档）
│   ├── logging.rs    # UTC 文件日志
│   ├── runtime.rs    # hezhou-player 启动流程（清单 → 布局 → 索引 → host）
│   ├── main.rs       # hezhou-player 二进制入口（运行时）
│   └── bin/player_build.rs  # hezhou-player-build CLI（dry-run + 真实构建）
├── template/         # 构建配置 / cook 清单示例 JSON
└── tests/
    ├── fixtures/minimal/  # 最小 fixture（legacy assets[]，空场景 + 占位 DLL/runtime）
    ├── fixtures/cooked/   # hezhou-asset 真实 cook 输出布局 fixture
    └── *.rs               # 默认 89 个测试；--features window-host 共 92 个（+1 ignored 真窗口 smoke）
```

> 硬性边界：本 crate 只改 `engine/player/**`。installer、CI、examples、editor
> 的集成改动由 Lead/对应负责人完成，本文第 8 节列出需要的集成点。

---

## 1. Player 产物目录格式

构建目标是"解压即运行"的目录（Windows 打 zip，Linux 打 tar.gz）：

```text
<PlayerRoot>/
├── hezhou-game.exe          # Windows 可执行文件（Linux 为 hezhou-game，无后缀）
├── manifest.json            # 运行时唯一入口契约（本 crate 定义）
├── assets/                  # 烘焙资产，逻辑路径全部相对此目录
│   ├── scenes/main.scene
│   ├── models/hero.glb
│   ├── textures/hero_albedo.png
│   ├── audio/bgm_theme.ogg
│   └── fonts/HarmonyOS_Sans_SC.ttf   # 必须随包，缺失 UI 文字空白
├── scripts/                 # 预编译 C# 程序集（mcs 产出，不能用 .NET 8 编译）
│   └── Game.dll
└── runtime/                 # 内置运行时，不依赖系统 Mono / 系统库
    ├── libmono.so (Linux) / mono-2.0-sgen.dll (Windows)
    ├── libvulkan.so.1 / vulkan-1.dll
    └── ...
```

约定：

- 包内路径**永远**用 `/` 记在清单里；落盘时按平台转换为 `\`（Windows）。
- 所有内容相对 `PlayerRoot`，可由用户整体移动位置（不写死绝对路径）。
- 目录名固定为 `assets/`、`scripts/`、`runtime/`、`logs/`（`logs/` 只是兜底，
  正常日志写用户数据目录，见第 6 节）。
- 可执行文件名 = `PlayerBuildConfig::executable_name + 平台后缀`。

### 1.1 hezhou-asset 真实 cook 输出映射

`--cook` 传 `hezhou-asset` 的 `cook_manifest.json` 时，cook 输出目录（清单所在目录）
原样映射到 Player 包 `assets/` 下，`output` 路径保持不变：

```text
<cook_out>/                        <PlayerRoot>/
├── cook_manifest.json      ->     └── assets/
├── imported/<uuid>.json    ->         ├── cook_manifest.json
└── assets/<uuid>.<ext>     ->         ├── imported/<uuid>.json
                                       └── assets/<uuid>.<ext>
```

因此 Player `manifest.json` 的 `assets[].path == cook entry.output`，运行时可以
直接以 `AssetId` 为键索引（`cook_manifest.json` 随包，供后续按需加载
`imported/*.json`）。legacy `{ "assets": [...] }` 清单仍支持（资产相对
`--assets-dir`），两者由 CLI 自动识别。

## 2. `manifest.json` 契约

运行时启动只读这一个文件；编辑器/构建器/CI 都不得自行发明字段。

```json
{
  "format_version": 1,
  "engine_version": "0.1.0",
  "product_name": "Capability Demo",
  "version": "0.2.0",
  "start_scene": "scenes/main.scene",
  "target": "windows",
  "quality": "high",
  "window": {
    "title": "Hezhou Capability Demo",
    "width": 1280,
    "height": 720,
    "mode": "windowed"
  },
  "paths": {
    "assets": "assets",
    "scripts": "scripts",
    "runtime": "runtime",
    "logs": "logs"
  },
  "font": "fonts/HarmonyOS_Sans_SC.ttf",
  "scripts": ["Game.dll"],
  "assets": [
    { "path": "scenes/main.scene", "kind": "scene", "size_bytes": 4096 },
    { "path": "fonts/HarmonyOS_Sans_SC.ttf", "kind": "font", "size_bytes": 8495076 }
  ],
  "built_by": "hezhou-player-builder 0.1.0"
}
```

| 字段 | 说明 | 校验规则 |
|---|---|---|
| `format_version` | 清单结构版本 | 必须等于运行时支持的 `1`，否则拒绝启动 |
| `engine_version` | 生成清单的引擎版本 | 必须 `major.minor` 与运行时一致（patch 可不同） |
| `product_name` | 产品显示名 | 非空、≤64 字符、无控制字符/路径分隔符/Windows 非法字符、非保留名 |
| `version` | 游戏版本 | 必须是 `major.minor.patch` |
| `start_scene` | 启动场景 | 相对 `assets/` 的逻辑路径，且必须出现在 `assets[]` 中 |
| `target` | `windows` / `linux` | 运行时必须与当前平台一致 |
| `quality` | `low` / `medium` / `high` / `ultra` | 枚举 |
| `window.title/width/height/mode` | 窗口 | 标题非空 ≤128；宽 320..=7680；高 240..=4320 |
| `paths.*` | 包内目录 | 逻辑相对路径；运行时实际解析用这些目录名 |
| `font` | 必检字体 | 若不为 `null`，必须在 `assets[]` 中且运行时可读到文件 |
| `scripts[]` | 预编译程序集 | 单段文件名、`.dll`、无大小写冲突 |
| `assets[]` | 烘焙资产 | 相对 `assets/` 的逻辑路径（真实 cook 时等于 `output`，可带 `assets/` 前缀）、无 `..`、无大小写冲突 |
| `assets[].kind` | 资产类型 | 开放字符串；真实 cook 时为 `AssetKind` 的 snake_case（scene/gltf/mesh/texture/audio/font/...） |
| `assets[].asset_id` | hezhou-asset `AssetId` | 可选；真实 cook 必填（UUID 字符串），运行时按 ID 索引 |
| `assets[].locator` | 源定位符 | 可选；工程相对逻辑路径 + 可选子资产键（`assets/...` 或 `...#mesh:0`） |
| `assets[].output` | cook 输出路径 | 可选；与 `path` 相同，保留用于对照 cook 契约 |
| `assets[].dependencies` | 直接依赖 AssetId | 可选；真实 cook 原样保留，用于完整性校验/加载顺序 |
| `assets[].source_hash/source_size` | 源文件信息 | 可选；真实 cook 的 FNV-1a 64 hex / 源字节数，用于 cook 产物校验 |
| `built_by` | 构建器标识 | 仅用于复现构建，可为空 |

> 决策：清单**严格**反序列化（`deny_unknown_fields`）。字段演进必须提升
> `format_version`，避免"看似兼容"的静默错误。cook 清单（输入）则允许额外字段。

## 3. 构建配置 `PlayerBuildConfig`

导出设置（编辑器"导出游戏"面板/CI 生成 JSON，构建器消费）：

```json
{
  "product_name": "Capability Demo",
  "executable_name": "capability-demo",
  "start_scene": "scenes/main.scene",
  "version": "0.2.0",
  "target": "windows",
  "quality": "high",
  "window": { "title": "Hezhou Capability Demo", "width": 1280, "height": 720, "mode": "windowed" },
  "log_dir": "logs",
  "required_font": "fonts/HarmonyOS_Sans_SC.ttf"
}
```

- `executable_name` 仅允许 ASCII 字母数字、`-`、`_`；留空/由 `PlayerBuildConfig::new`
  从产品名派生（中文等非 ASCII 折叠，空则回退 `hezhou-game`；命中 Windows 保留名
  自动追加 `-player`）。
- `required_font` 默认 `fonts/HarmonyOS_Sans_SC.ttf`；设为 `null` 可关闭字体检查
  （会产出 `font_not_configured` 警告，不建议发布版使用）。
- 校验一次性收集全部问题（`ConfigErrors`），构建日志能一次列全。
- 示例见 `template/build-config.example.json`。

## 4. 构建器

构建器分两阶段：`plan_*`（dry-run，纯元数据）与 `build`（真实复制到产物目录）。

### 4.1 dry-run 计划

```rust
use hezhou_player::{
    CookManifest, ManifestAsset, PlayerBuildConfig, PlayerBuildInput, PlayerBuilder,
    RuntimePayload, ScriptAssembly, TargetPlatform,
};

let config = PlayerBuildConfig::new("Demo", "1.0.0", "scenes/main.scene", TargetPlatform::Windows);
let cook = CookManifest::new(vec![
    ManifestAsset::new("scenes/main.scene", "scene"),
    ManifestAsset::new("fonts/HarmonyOS_Sans_SC.ttf", "font"),
]);
let input = PlayerBuildInput {
    config,
    cook,
    scripts: ScriptAssembly::from_dir("engine/scripts/bin/Mono")?, // 预编译 C# DLL
    runtime: RuntimePayload::from_dir("installer/stage/windows/runtime")?,
};
let builder = PlayerBuilder::new(input);
let plans = builder.plan_both_targets()?;               // Windows + Linux
let windows = builder.plan_for(TargetPlatform::Windows)?;
println!("{}", windows.report());                       // dry-run 文本报告
windows.write_plan("dist/player-plan/windows")?;        // 只写计划元数据，不拷二进制
```

`PlayerBuildPlan` 包含：

- `files: Vec<PlannedFile>`：逻辑路径、分类（executable/asset/script/runtime/manifest）、
  字节数、`fnv1a64` 校验、可执行标记、Unix 权限位（Linux 0o755/0o644，Windows `null`）；
- `manifest: PlayerManifest`：将写入产物根目录的清单；
- `warnings`：`runtime_payload_empty`、`scripts_empty`、`font_not_configured`、
  `linux_rpath_required`、`executable_produced_by_cargo`；
- `fingerprint`：整个清单的 `fnv1a64` 指纹，输入相同则完全相同（构建可重复）。

CLI dry-run：

```bash
cd engine
cargo run -p hezhou-player --bin hezhou-player-build -- \
  --config player/template/build-config.example.json \
  --cook player/template/cook-manifest.example.json \
  --runtime-dir ../installer/stage/windows/runtime \
  --plan-out ../dist/player-plan \
  --target both
```

`--plan-out`（别名 `--out`）写出 `<out>/windows/`、`<out>/linux/` 下的 `manifest.json`、
`plan.json`、`file-list.txt`。**dry-run 不复制 exe/assets/scripts/runtime**。

错误示例：启动场景不在 cook 清单、缺少必检字体、路径仅大小写不同、脚本名逃逸
`scripts/`、窗口尺寸越界、目标平台不受支持 —— 全部在计划阶段失败，不产生半成品。

### 4.2 真实产物构建

```rust
use hezhou_player::{BuildSources, PlayerBuilder, TargetPlatform};

let builder = PlayerBuilder::new(input); // 与 dry-run 相同的输入
let sources = BuildSources::new(
    "engine/target/debug/hezhou-player.exe",  // cargo 产出的 Player 可执行文件
    "cook/assets",                            // cook 资产根目录
);
let output = builder.build(TargetPlatform::host(), &sources, "dist/player/windows")?;
println!("{}", output);                       // 已复制文件：大小/校验/权限
```

行为保证：

- **只构建当前平台**（`TargetPlatform::host()`）；双平台由 CI 在 Windows/Linux 分别执行；
- 写盘前完整预检：源文件存在、大小与 cook/计划声明一致、`fnv1a64:` 校验和一致；
- 先写入同父目录的 `.<name>.staging-<pid>-<n>`，成功后原子替换输出目录；
  失败自动清理 staging，**不破坏已有产物、不留半成品**；
- 复制后逐文件校验大小/校验和，并核对文件集合与计划完全一致；
- Linux 按计划设置权限：exe `0755`、数据 `0644`；Windows 无 Unix 权限位；
- 产物 `manifest.json` 与 dry-run 计划同源生成，可直接被 `hezhou-player --validate` 读取。

CLI 真实构建（minimal fixture，见 `tests/fixtures/minimal/`）：

```bash
cd engine
cargo build -p hezhou-player

cargo run -p hezhou-player --bin hezhou-player-build -- \
  --config player/tests/fixtures/minimal/build-config.json \
  --cook player/tests/fixtures/minimal/cook-manifest.json \
  --assets-dir player/tests/fixtures/minimal/assets \
  --scripts-dir player/tests/fixtures/minimal/scripts \
  --runtime-dir player/tests/fixtures/minimal/runtime \
  --exe target/debug/hezhou-player.exe \
  --build-out ../dist/player-fixture

# 产物：../dist/player-fixture/<当前平台>/minimal-fixture[.exe]
#       + assets/ scripts/ runtime/ manifest.json

# 端到端冒烟（CI 用）
../dist/player-fixture/windows/minimal-fixture.exe --validate \
  --manifest ../dist/player-fixture/windows/manifest.json \
  --log-dir ../dist/player-fixture-logs
```

hezhou-asset 真实 cook 产物（推荐，见 `tests/fixtures/cooked/`；`--cook` 指向
`cook_manifest.json`，资产来源自动取清单所在目录，不需要 `--assets-dir`）：

```bash
cargo run -p hezhou-player --bin hezhou-player-build -- \
  --config player/tests/fixtures/cooked/build-config.json \
  --cook player/tests/fixtures/cooked/cook_manifest.json \
  --scripts-dir player/tests/fixtures/cooked/scripts \
  --runtime-dir player/tests/fixtures/cooked/runtime \
  --exe target/debug/hezhou-player.exe \
  --build-out ../dist/player-cooked

# 产物：<PlayerRoot>/assets/{cook_manifest.json, imported/*.json, assets/*}
```

要点：

- `--exe` 真实构建必需；`--assets-dir` 仅 legacy `assets[]` 清单需要；
- `--scripts-dir` / `--runtime-dir` 与 dry-run 相同；
- `--target both`（默认）在真实构建时只构建当前平台并提示；双平台由 CI 分别执行；
- 失败退出码 `2`，输出可读错误（缺源文件/大小不符/校验和不符/输出路径非法等）。

## 5. 跨平台规则（已实现）

| 问题 | 规则 |
|---|---|
| 路径分隔符 | 清单/计划内固定 `/`；写盘/运行时按平台转 `\`（Windows） |
| 绝对路径/`..` | 一律拒绝，资产不能逃逸包目录 |
| 大小写 | 所有产物路径做大小写不敏感冲突检测（含资产↔脚本↔运行时↔清单），Windows 上不会互相覆盖 |
| Windows 非法字符 | `: < > " \| ? *`、控制字符、`.`/空格结尾、保留设备名（CON/PRN/AUX/NUL/COM1-9/LPT1-9）全部拒绝 |
| 可执行权限 | Linux 真实构建按计划 chmod：exe=0o755、数据文件=0o644；打包 tar.gz 时保留 |
| 构建平台 | 真实构建只允许当前平台；`plan_for`/`plan_both_targets` 仍可预览双平台清单 |
| Linux 动态库查找 | 链接期 `-Wl,-rpath,$ORIGIN/runtime`（CI 注入）；运行时把 `runtime/` 追加到 `LD_LIBRARY_PATH`（仅影响 `dlopen`） |
| Windows DLL 查找 | 运行时把 `runtime/` 前置到 `PATH`；exe 同目录优先 |
| 字体 | 必须随包（legacy 相对 `assets/fonts/...`；真实 cook 为对应 `output`），构建期检查存在，运行期检查可读 |
| 日志目录 | 安装/解压目录可能只读 → 优先用户数据目录，其次包内 `logs/` 兜底 |
| 换行符 | 清单 JSON 统一 `\n`；不写平台相关文本 |

## 6. `hezhou-player` 最小运行时

启动流程（`runtime::run`）：

1. 定位包根目录（优先级：`--player-root` > `manifest.json` 所在目录 > 可执行文件目录）；
2. 读取 `manifest.json`，校验结构版本 / 字段 / 目标平台 / 引擎版本（major.minor）；
3. 解析并检查 `assets/`、`scripts/`、`runtime/`、启动场景、字体、每个脚本 DLL；
4. 打开文件日志（`player.log`，UTC 时间戳、INFO/WARN/ERROR）；
5. **加载资产索引**：读 `assets/cook_manifest.json`，建 `AssetId → output/kind/locator`
   索引，校验所有 `output` 存在、start_scene/font 可在索引中解析（见 §6.1）；
6. 按模式结束：
   - 默认：记录"运行时骨架启动成功"并以 0 退出；
   - `--validate`：启动检查完成（含资产索引）并以 0 退出；
   - `--run` / `--host <name>`：加载索引后调用 `PlayerHost`（见 §6.4），本轮默认 host
     返回 `Pending`，退出码 **3**（不是错误，表示 game host 尚未接入）。

参数：

```text
hezhou-player [--player-root <dir>] [--manifest <file>] [--log-dir <dir>]
              [--validate] [--run] [--host <name>] [--version|-V] [--help|-h]
```

- **Windows 无终端**：`#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]`，
  发布版双击无控制台；所有诊断写文件。
- 日志默认路径：
  - Windows `%LOCALAPPDATA%\Hezhou\<product-slug>\logs\player.log`
  - Linux `$XDG_DATA_HOME/hezhou/<product-slug>/logs/player.log`
    （无 XDG 时 `$HOME/.local/share/hezhou/<product-slug>/logs`）
- 启动失败写 `startup-error.log`（包内 `logs/`，只读则回退系统临时目录）。
- 退出码：`0` 启动检查/host 运行成功；`2` 启动检查失败（清单/布局/资产索引，详见
  `startup-error.log`）；`3` host pending（`--run` 时 game host 尚未接入）。
- 字体/脚本/场景/资产索引 output 缺失属于硬错误；`runtime/` 为空只告警（骨架阶段），
  正式发布必须内置运行时（PROJECT_PLAN §1.5 验收：干净机器不依赖 Mono）。

### 6.1 资产索引 `PlayerAssetIndex`

```rust
use hezhou_player::{PlayerAssetIndex, PlayerManifest, AssetKind, AssetId};

let manifest = PlayerManifest::load(&root.join("manifest.json"))?;
let index = PlayerAssetIndex::load(&root, &manifest)?;

index.entry(asset_id);                        // 按 AssetId
index.by_kind(AssetKind::Scene);              // 按 kind（按 output 排序）
index.entry_by_locator("assets/models/hero.gltf#mesh:0"); // 按 locator
index.entry_by_output("assets/<uuid>.png");   // 按包内 output（大小写不敏感）
index.start_scene();                          // 启动场景条目
index.font();                                 // 必检字体条目
index.len();                                  // 条目数
```

加载与校验规则：

- 复用 `hezhou_asset::cook::CookManifest`（不另写 schema）；`assets/cook_manifest.json`
  不存在时回退按 Player manifest 的 `asset_id` 建索引；两者都没有 asset_id（legacy 包）
  得到空索引并告警，不阻塞 `--validate`。
- 校验 `format_version`、entries 非空、`output` 合法/无重复、**每个 output 文件存在**；
  并核对 Player manifest `assets[]` 与 cook 索引一一对应（AssetId/output/kind），
  防止清单过期或被篡改。
- `start_scene`/`font` 用 Player manifest 中的 output 路径匹配索引条目；匹配不到返回
  可读错误 `MissingStartScene`/`MissingFont`。
- 坏 JSON / 读失败返回 `AssetIndexError::CookManifest`（含路径 + serde 错误），
  运行时转为退出码 2 并写 `startup-error.log`。

### 6.2 起始场景加载（`scene_loader`）

运行时在资产索引加载后、调用 host 前执行：

1. **路径解析**：索引 `start_scene()` 条目 → `<PlayerRoot>/assets/<output>`；
   legacy 包（无 cook 索引）退回 `manifest.start_scene` 相对 `assets/` 的路径；
2. **恢复**：读取 JSON → `hezhou_core::Project::restore_from_json`（替换语义，保留实体 ID/
   层级/Transform/组件/方向光）；
3. **安全回退**：文件缺失、JSON 非法、Project 格式不兼容、恢复 panic、或合法但 0 实体时，
   使用内置测试场景（cube + directional light）并把可读原因写 WARN，**不 panic、不阻塞启动**；
4. **结果**：`LoadedStartScene { scene, source, warnings }`，
   `StartSceneSource::{Project { path, entities } | BuiltinFallback { reason }}`；
   运行时摘要写入 `RuntimeReport.start_scene`，host 通过
   `PlayerHostContext.start_scene: &mut Scene` 拿到场景；
   `WindowPlayerHost` 直接渲染该场景（不再无条件自建 cube）。
5. `--validate` 也会加载起始场景（只是不渲染），启动日志会记录实体数/回退原因。

```rust
pub fn load_start_scene(
    root: &Path,
    manifest: &PlayerManifest,
    index: &PlayerAssetIndex,
) -> LoadedStartScene; // 永远成功；失败时 source=BuiltinFallback + warnings

pub fn builtin_test_scene() -> Scene; // cube + directional light
```

已知限制：只加载**工程 Project JSON**（编辑器工程/快照格式）；cook `imported/*.json`
资产内容、C# `Initialize()`、Gameplay/Animation 等运行时数据仍在后续 TODO。

### 6.3 cook 导入资产 bundle（`imported_assets`）

运行时在资产索引之后、起始场景之前解析包内 `assets/imported/*.json`
（`hezhou_asset::ImportedScene`），建立 **`AssetId → 元素`** 查询：

```rust
let bundle = ImportedAssetBundle::load(&layout.assets_dir, &index)?;

bundle.scene(gltf_asset_id);        // 整个 ImportedScene（每份 glTF 一个）
bundle.mesh(asset_id);              // ImportedMesh（顶点/索引/蒙皮/材质索引）
bundle.material(asset_id);          // ImportedMaterial（PBR 因子 + 纹理引用）
bundle.texture(asset_id);           // ImportedTexture
bundle.skeleton(asset_id);          // ImportedSkeleton（joints/IBM）
bundle.animation(asset_id);         // ImportedAnimation（通道/关键帧）
bundle.scene_of(sub_asset_id);      // 所属 glTF 场景 AssetId
bundle.stats();                     // scenes/meshes/materials/textures/skeletons/animations/mapped
```

**映射规则**：`gltf` entry 的 `output = imported/<uuid>.json` 解析为 `ImportedScene`；
子资产 entry 的 `locator` 子资产键（`#mesh:0`/`#material:0`/`#skeleton:0`/`#animation:0`/
`#texture:0`）定位数组下标，`AssetId` 来自 cook manifest。

**严格校验（失败 → 可读错误，启动/`--validate` exit 2 + startup-error.log）**：

- `imported/*.json` 必须存在、JSON 可解析、通过 `hezhou_asset::validate_imported_scene`；
- 子资产必须有同 output 的 glTF 父 entry；子资产键 kind/index 与 entry.kind 一致且不越界；
- 每份 `ImportedScene` 的数组长度必须等于 cook manifest 中对应子资产 entry 数量
  （防止 cook 产物与 manifest 过期/不一致）。

未被 manifest 引用的 `imported/*.json` 只产生 WARN 并忽略；无 gltf/imported 条目的
legacy/最小包得到空 bundle（不报错）。

**C# 衔接契约（下一轮 scripting，`Initialize()`）**：

- 每个子资产以 cook `AssetId`（UUID）为稳定句柄，C# 侧不需要路径/ID 重建；
- host 通过 [`PlayerHostContext.imported_assets`] 拿到只读 bundle；下一轮在
  `scripting` 侧暴露 C# 可见句柄（建议 FFI：`player_get_mesh(id)` /
  `player_get_material(id)` / `player_get_skeleton(id)` / `player_get_animation(id)`，
  返回只读数据指针/长度或由 host 拷贝为托管对象）；
- `scene_of(id)` 用于按 glTF 场景管理生命周期（一次反序列化，子资产共享）；
- C# 侧字段映射：`ImportedMesh`（primitives/positions/normals/indices/joints/weights/
  material/skeleton）、`ImportedMaterial`（base_color_factor/metallic/roughness/纹理索引）、
  `ImportedSkeleton`（joints[node,parent,IBM]/root_joint）、
  `ImportedAnimation`（channels/property/times/values/duration）。

### 6.4 `PlayerHost` 接口（game host 接入契约）

提供接口与两个实现：无窗口的 `DefaultPlayerHost`（`Pending`，退出码 3）与可选的
`WindowPlayerHost`（`feature = "window-host"`，真实窗口/帧循环，`Running`，退出码 0）。

```rust
pub trait PlayerHost {
    fn name(&self) -> &'static str;
    fn run(&mut self, context: &mut PlayerHostContext<'_>) -> Result<HostOutcome, HostError>;
}

pub struct PlayerHostContext<'a> {
    pub root: &'a Path,                    // Player 包根
    pub assets_dir: &'a Path,              // <root>/assets
    pub manifest: &'a PlayerManifest,      // 已校验清单（窗口/quality/scripts 等）
    pub index: &'a PlayerAssetIndex,       // 已加载资产索引（AssetId → output/kind/locator）
    pub log: &'a mut dyn HostLog,          // 写 player.log（HostLog trait）
}

pub enum HostOutcome {
    Running { frames: u64 },               // 已进入游戏循环并正常结束 → 退出码 0
    Pending { reason: String },            // host 尚未接入 → 退出码 3
}
```

- 选择：`resolve_host("default" | "window" | "headless" | "null")`
  - 未编译 `window-host`：`default`/`headless`/`null` → `DefaultPlayerHost`（Pending）；
    `window` → 可读错误（提示开启 feature）；
  - 编译 `window-host`：`default`/`window` → `WindowPlayerHost`；`headless`/`null` →
    `DefaultPlayerHost`（CI/无 GPU 环境强制 pending）。
- `DefaultPlayerHost`：加载索引后记录资产数/启动场景/字体/窗口/quality，
  明确输出"等待 game host 接入"（`WARN`），返回 `HostOutcome::Pending`（退出码 3）。
- 下一轮 game host 接入顺序（实现 `PlayerHost` 即可，无需改运行时）：
  `platform(window/input)` → `rhi`/`rhi-vulkan` → `render`/`render-pipeline`
  → `scripting`（加载 `scripts/*.dll` 并调用 C# 静态 `Initialize()`）
  → `gameplay`/`animation`/`physics`/`audio`（按 AssetId 从索引加载
  `assets/cook_manifest.json` / `imported/*.json` / 纹理音频字节）→ 帧循环。
- CI：默认 feature 下 `--run` 应视为"host pending = 退出码 3"；编译 `window-host` 后
  `--run` 会真实开窗（CI 无显示环境请用 `--host headless` 或 `--validate`）。

### 6.5 可选窗口渲染 host（`feature = "window-host"`）

默认构建**不编译** `hezhou-core`/`hezhou-rhi-vulkan`/`hezhou-platform` 依赖，二进制与
依赖面保持最小；需要真实窗口渲染时显式开启 feature：

```bash
# 编译（默认不发版；`hezhou-platform` 作为窗口/输入契约依赖，GLFW 窗口由
# rhi-vulkan 内部的 UIVulkanRenderer 创建，Player 不重复初始化 GLFW）
cargo build -p hezhou-player --features window-host --release
# 注意：glfw 以 src-build 编译，release 构建需要 CMake/Ninja + VS/MSVC 环境；
# 若 shell 未初始化 VS 环境，CMake/MSBuild 会失败（本机 debug feature 构建/运行已验证，
# release 需在 CI 或 Developer PowerShell 中构建）。

# 真实窗口 host（默认固定 120 帧；可用环境变量覆盖帧数）
HEZHOU_PLAYER_WINDOW_FRAMES=30 \
  hezhou-player --run --host window --manifest <pkg>/manifest.json --log-dir <logs>

# 无窗口 pending（CI/无 GPU）
hezhou-player --run --host headless --manifest <pkg>/manifest.json
```

`WindowPlayerHost` 行为：

- 按 `manifest.window`（title/width/height）创建 GLFW 窗口 + `UIVulkanRenderer`
  （Vulkan device/swapchain/render pass/pipeline/FXAA）；
- 使用运行时加载的起始场景（§6.2：`manifest.start_scene` 的 Project JSON → `Scene`；
  失败时 loader 已回退内置 cube+light），host 不再无条件自建 cube；
  `set_game_state(1)`（Running）+ 固定相机 + 天空盒；
- 跑固定帧数（`HEZHOU_PLAYER_WINDOW_FRAMES`，默认 120，上限 100000）或用户关闭窗口；
- 每 30 帧/结束时写 `player.log`（窗口尺寸、起始场景实体数/来源、帧数、耗时），返回
  `HostOutcome::Running { frames }` → 进程退出码 0；
- 窗口/Vulkan/GPU 不可用、渲染 panic 都转为可读 `HostError`（运行时退出码 2 +
  `startup-error.log`），不 panic、不静默崩溃。

**本机实测（2026-11-01，Windows + Intel Arc B370，debug feature）**：

```text
WindowPlayerHost：创建窗口 960x540 mode=windowed title="Hezhou Cooked Fixture"
使用起始场景：3 个实体（来源：Project JSON .../assets/assets/1111...scene）
窗口与 Vulkan 渲染器创建成功：960x540，quality=low，frame_limit=30
窗口渲染进度：30/30 帧，658.4 ms
窗口 host 结束：30 帧，674.7 ms（达到固定帧数）
PlayerHost `window` 返回退出码 0
```

起始场景 3 个实体来自 cooked fixture 的真实 Project JSON（CubeA + 子 Cube + DirectionalLight），
不再是内置测试 cube。

**已知限制**：

- 本 host 结束时不调用 `UIVulkanRenderer::cleanup()`：当前该函数在"仅渲染起始场景"
  路径下会触发 Windows 堆损坏（`0xC0000374`）；窗口 host 是一次性运行、返回后进程
  即将退出，由操作系统回收 Vulkan/GLFW 资源。上游修复后应恢复显式 cleanup。
- Esc 退出尚未接线：`process_events` 把按键交给 UI input handler，未暴露给 host；
  当前用固定帧数或窗口关闭（`draw_frame` 返回 false）退出（见 TODO）。
- 场景内资产加载/C#：Project JSON 只恢复实体/组件；cook `imported/*.json` 内容、
  C# `Initialize()`、Gameplay/Animation/Physics 运行时数据仍留待后续 game host。

## 7. 测试

```bash
cd engine
cargo test -p hezhou-player                        # 默认 89 个测试
cargo build -p hezhou-player                       # 构建两个二进制 + 库
cargo test -p hezhou-player --features window-host # 92 个（含 window-host 单元/契约测试）
# 真实窗口 smoke：需显示环境 + Vulkan GPU，CI 跳过；本机已实测通过（30 帧，exit 0）
cargo test -p hezhou-player --features window-host --test window_host -- --ignored --nocapture
```

覆盖：manifest 读写/严格字段/格式版本、配置校验、路径规范化与大小写冲突、
双平台 dry-run 清单与指纹稳定性、版本兼容矩阵、真实构建（复制/清单一致/大小与
校验和/失败清理/权限/跨平台拒绝）、fixture CLI 端到端（构建 → 复制出的 Player
`--validate`）、**hezhou-asset cook 集成**（manifest 解析、AssetId/kind/locator/output
映射、cook_manifest.json 入包、缺失输出、产物大小/哈希损坏、格式版本、启动场景/字体
定位、CLI 真实构建 + validate）、**运行时资产索引**（按 AssetId/kind/locator/output
查询、start_scene/font 解析、缺失 output、坏 JSON、manifest/索引不一致、legacy 空索引、
`--run`/`--host` 退出码 3、未知 host、`--validate` 不回归）、**起始场景加载**
（`hezhou_core::Project::restore_from_json`：实体/层级/Transform/方向光恢复、
缺失文件/坏 JSON/空场景安全回退内置 cube+light、与 `PlayerAssetIndex.start_scene`
output 对接、runtime 摘要/日志）、**cook imported 资产 bundle**
（`ImportedScene` 解析、AssetId→mesh/material/texture/skeleton/animation 查询、
kind/index/数量严格校验、坏 JSON/越界/数量不匹配错误、未引用文件 WARN、runtime 统计）、
**window-host feature**（默认 host 选择、frame limit 解析/clamp、真实窗口固定帧渲染加载
真实场景 + `Running`/退出码 0；真实窗口测试 `#[ignore]`，本机手工已验证）、日志时间戳。

最小 fixture（`tests/fixtures/minimal/`）依赖零外部资产：空场景 3 字节、字体占位
12 字节、假 C# 程序集 21 字节、占位 runtime 库 19 字节，用于无真实 cook 数据时
验证"构建 → 打包目录 → 运行时启动校验"的完整链路（场景非 Project JSON，会走回退）。

真实 cook fixture（`tests/fixtures/cooked/`）模拟 `Cooker::cook_to_dir` 的
`cook_manifest.json + imported/<uuid>.json + assets/<uuid>.<ext>` 布局
（scene/font/texture/gltf 四个条目）。其中 scene 是真实 Project JSON（1488 字节、
3 个实体：CubeA + 子 Cube + DirectionalLight），用于端到端验证
"cook → Player 包 → 起始场景恢复 → 窗口渲染"。

## 8. 与其它模块 / CI / installer 的集成点

### asset（hezhou-asset cook manifest）— 已接入

契约来源：`engine/asset/README.md` §6（`format_version = 1`）。Player 直接复用
`hezhou_asset::cook::CookManifest` 类型（`engine/player/Cargo.toml` 依赖
`hezhou-asset`，关闭默认 `gltf` feature），不另写不兼容 schema。

**字段映射（cook entry → Player manifest `assets[]`）**

| cook entry | Player `assets[]` | 说明 |
|---|---|---|
| `id`（UUID） | `asset_id` | 稳定身份；运行时应以此为键索引 |
| `kind` | `kind` | `AssetKind` 的 snake_case（scene/gltf/mesh/texture/audio/font/...） |
| `locator` | `locator` | 源定位符（工程相对逻辑路径 + 可选子资产键） |
| `output` | `path` / `output` | Player 包内相对 `assets/` 的路径，原样保留 |
| `dependencies` | `dependencies` | 直接依赖 AssetId，用于完整性校验/加载顺序 |
| `source_size` | `source_size` | 源文件字节数（cook 原样拷贝资产还用于校验产物） |
| `source_hash` | `source_hash` | 源文件 FNV-1a 64 hex（同上） |
| cook 输出文件实际大小/哈希 | `size_bytes` / `checksum` | 复制前计算并写入 Player manifest，构建时复核 |

**文件映射**：`cook_manifest.json`、`imported/*.json`、`assets/*` 原样复制到
`<PlayerRoot>/assets/` 下（见 §1.1），路径不变，运行时可用
`manifest.paths.assets + assets[].path` 定位。

**构建校验**（失败在写盘前报错、不留半成品）：

- `format_version > 1`、entries 为空、`output` 非法/与 `cook_manifest.json` 撞名 →
  拒绝；
- 每个 `output` 文件必须存在；
- 对非 glTF/子资产条目（cook 原样拷贝语义）：产物大小/哈希必须等于
  `source_size`/`source_hash`，可发现 cook 产物损坏或过期；
- `imported/*.json` 是重新生成的 JSON，不按源文件大小/哈希校验（记录实际值）；
- `start_scene` / `required_font` 用**工程相对逻辑路径**在 entries 中按 `locator`
  匹配（`scenes/main.scene` 或 `assets/scenes/main.scene` 均可），匹配不到报
  `MissingStartScene` / `MissingFont`；匹配到的条目 `output` 写入 Player manifest。

**契约差异与限制（相对 asset README §6）**

- asset §6 建议启动时读 `cook_manifest.json` 建立 `id -> output` 索引；Player
  **运行时已实现**（`PlayerAssetIndex`，见 §6.1），并校验 output 存在、
  start_scene/font 可解析、Player manifest 与索引一致。
- `imported/*.json` 内容当前仍不解析/不校验（构建时只复制 + 校验文件存在/大小/哈希；
  运行时由后续 game host 按 AssetId 加载并反序列化为 `ImportedScene` 等价结构）。
- asset 依赖表/压缩标记等新增字段需先扩展 `ManifestAsset`（`assets[]` 严格反序列化）。
- legacy `{ "assets": [...] }` 清单仍支持（`--assets-dir` 提供资产根目录），
  作为无 cooker 时的过渡；字段映射为：`path`（相对资产根）、`kind`、
  `size_bytes`、`checksum`；运行时 legacy 包得到空资产索引（告警但不阻塞 `--validate`）。
- 真实 cooker 接入后只需把 CLI `--cook` 指向 `<cook_out>/cook_manifest.json`，
  无需改 Player 构建代码。

### audio

- 音频资源以 `kind: "audio"` 进入 cook 清单并落在 `assets/`；构建器不解析音频格式；
- 运行时初始化 `hezhou-audio` 后，通过清单路径解析音频（相对 `assets/`），
  不使用系统绝对路径；
- 无音频时的 `assets/` 允许为空目录（构建器按资产条目生成清单）。

### render

- `quality`（low/medium/high/ultra）与 `window`（宽高/模式）写入 manifest，
  由 Player 渲染初始化读取；构建器不做画质决策；
- 着色器 `.spv` 当前手工编译入库；Player 包需要把 Player 用到的 `.spv` 作为
  `runtime/` 或 `assets/` 条目纳入清单（后续定 shader pack 格式）；
- Linux 需要 `$ORIGIN/runtime` rpath（见第 5 节），由 CI 链接参数注入。

### gameplay / scripting

- `scripts[]` 是预编译 C# 程序集（`mcs -target:library`，禁止 .NET 8 编译产物）；
- C# 无 `Main()`，入口为静态 `Initialize()`，由 Rust 运行时调用；
- 本轮 Player crate 故意**不依赖** core/render/scripting（保持构建器可独立编译）；
  下一轮实现 `PlayerHost`（见 §6.4）时再启用 `mono` feature 并链接引擎 crate，
  在 host 内加载 `manifest.scripts[]`、调用 C# 静态 `Initialize()`、按 AssetId
  从资产索引加载 gameplay/animation/physics/audio 数据。

### CI（需要 Lead 新增两个 job：`player-build-windows` / `player-build-linux`）

建议流程（本 crate 已提供命令，不需要改 CI 文件本身）：

1. `cargo build -p hezhou-player`（产出 `hezhou-player` 运行时二进制）；
2. 用 asset `Cooker` 产出 `<cook_out>/cook_manifest.json` + `imported/` + `assets/`，
   并生成 `build-config.json`（start_scene/font 为工程相对 locator）；
3. dry-run 审查（可选）：`hezhou-player-build --config ... --cook <cook_out>/cook_manifest.json --plan-out dist/player-plan`；
4. 真实构建：
   `hezhou-player-build --config ... --cook <cook_out>/cook_manifest.json --scripts-dir ... --runtime-dir ... --exe target/release/hezhou-player[.exe] --build-out dist/player`
   （真实 cook 清单不需要 `--assets-dir`）Linux 链接时注入
   `RUSTFLAGS="-C link-arg=-Wl,-rpath,$ORIGIN/runtime"`；
5. 打包：Windows `zip` / Linux `tar.gz`，tar 需保留 staging 时写入的 `unix_mode`
   （exe 0755，数据 0644，可用 `tar --mode` 或 `chmod` 后打包）；
6. 冒烟：运行 `<dist>/player/<target>/<exe> --validate --manifest ... --log-dir <tmp>`，
   非 0 退出即失败（会加载资产索引）；无真实 cooker 数据时先用 `tests/fixtures/cooked`
   （或 `tests/fixtures/minimal`）做前置检查。`--run` 当前默认 host 返回 3
   （host pending），CI 冒烟请用 `--validate`，等 game host 接入后再把 `--run` 纳入门禁；
7. 可选 `cross-platform-assets`：同一 cook 清单在双平台各跑一次 dry-run 对比指纹；
8. Release 上传 Player zip/tar.gz（与 Editor 安装包并列）。

### installer / 发布

- Player 产物与编辑器安装包**分离**：Editor 继续 MSI/MSIX/DEB；Player 作为
  Release 附件（Windows zip、Linux tar.gz），由 `publish-release` 上传；
- 本 crate 不改 `installer/**`；installer 若需要复用运行时目录，直接指
  `--runtime-dir <installer stage runtime>` 即可（只读输入）。

### editor

- "导出游戏"按钮/菜单后续在 editor 侧生成 `build-config.json` 并调用
  `hezhou-player-build`（或同 workspace 下直接依赖 `hezhou-player` 库）；
- 编辑器工程 ↔ `PlayerBuildConfig` 的字段映射由 editor 负责人实现；
  本 crate 不反向依赖 editor。

## 9. 本轮未做 / TODO

- [x] 真实复制 exe/assets/scripts/runtime（预检 + staging 原子提交 + 失败清理 + 权限）；
- [x] 真实 asset cooker 输出对接（`cook_manifest.json` + `imported/` + `assets/`、
  AssetId/kind/locator/output 映射、cook 产物大小/哈希校验）；
- [x] 运行时加载 `assets/cook_manifest.json`，建 `AssetId → output/kind/locator` 索引，
  校验 output/start_scene/font（`PlayerAssetIndex`）；
- [x] `PlayerHost` trait + 默认 host + `--run`/`--host`（明确 host pending，退出码 3）；
- [x] 可选窗口渲染 host（feature `window-host`）：GLFW 窗口 + `UIVulkanRenderer` +
  固定帧循环（`Running`，退出码 0）；
- [x] 起始场景加载：`manifest.start_scene` 的 Project JSON → `Scene`（实体/层级/
  Transform/方向光），缺失/坏 JSON/空场景回退内置 cube+light 并 WARN；
- [x] cook 导入资产 bundle：解析 `assets/imported/*.json`（`ImportedScene`），
  AssetId→mesh/material/texture/skeleton/animation 查询，严格核对 cook manifest，
  启动统计 + 错误安全；
- [ ] C# 脚本 `Initialize()`（scripting）与 host 消费 bundle（mesh/material/animation
  接入 render/animation/gameplay）；
- [ ] 纹理/音频原始字节加载（`assets/*`，非 imported JSON）；
- [ ] WindowPlayerHost 的 Esc/输入接线与显式 cleanup（上游修复后）；
- [ ] Mono 加载 + C# 静态 `Initialize()`（scripting）；
- [ ] Esc 退出接线（`UIVulkanRenderer` 未把 ESC 暴露给 host；当前用固定帧数/窗口关闭）；
- [ ] 上游修复 `UIVulkanRenderer::cleanup()` 堆损坏后恢复窗口 host 显式清理；
- [ ] `window-host` release 构建/打包接入 CI（编译冒烟；真实开窗需显示环境，本地/自托管）；
- [ ] Linux rpath 链接参数固化到构建脚本/CI（需要在 Linux job 注入 `RUSTFLAGS`）；
- [ ] 跨平台构建（在 Windows 上产出 Linux 包）——不计划支持，由 CI 双平台分别执行；
- [ ] 资产 pack/压缩格式（当前按 loose files 计划）；
- [ ] shader 资源打包规则；
- [ ] 窗口图标（Windows `.ico` / Linux `.png`）与 `window.icon` 字段；
- [ ] `--window-mode`/分辨率命令行覆盖（PROJECT_PLAN §1.5 启动参数）；
- [ ] 崩溃处理器与 `startup-error.log` 之外的结构化崩溃报告；
- [ ] 清单签名/校验（发布完整性）；
- [ ] 与 `dfx` 日志系统共用输出（当前自带最小文件日志）。
