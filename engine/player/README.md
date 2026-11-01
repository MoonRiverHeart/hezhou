# hezhou-player

Hezhou 0.2 **Player 导出构建器 + 最小运行时**（Windows / Linux）。

本 crate 是 `engine/player`，对应 `docs/PROJECT_PLAN.md` §1.5 Player 导出与
§9 AI 并行工作流中的 Player 工作流。已完成三轮：

1. **格式契约 + dry-run**：产物目录、`manifest.json`、跨平台路径规则、双平台清单；
2. **真实产物构建**：按计划复制 exe/assets/scripts/runtime/manifest，staging 原子提交、
   失败清理、大小/校验和/权限检查；最小 fixture 产出的包可通过 `hezhou-player --validate`；
3. **asset cooker 集成**：直接读取 `hezhou-asset` 的 `cook_manifest.json`
   （`format_version=1`、`roots`、`entries`），按 `AssetId` 映射资产、复制
   `imported/` + `assets/` 输出并写入 Player manifest（可追踪 id/kind/locator/output）。

不绑定具体游戏内容；真实 cooker 产物可原样替换测试 fixture。

```
engine/player/
├── src/
│   ├── lib.rs        # 公共 API 导出
│   ├── config.rs     # PlayerBuildConfig（导出设置）与校验
│   ├── manifest.rs   # manifest.json 契约与校验（含 cook 资产字段）
│   ├── builder.rs    # 构建器：dry-run 计划 + 真实复制（预检/staging/校验/权限）
│   ├── cook_bridge.rs# hezhou-asset CookManifest → Player assets[]/文件映射
│   ├── paths.rs      # 逻辑路径规则（分隔符/大小写/保留名/权限位）
│   ├── platform.rs   # 目标平台差异（exe 命名/动态库查找/rpath/归档）
│   ├── logging.rs    # UTC 文件日志
│   ├── runtime.rs    # hezhou-player 启动流程
│   ├── main.rs       # hezhou-player 二进制入口（运行时）
│   └── bin/player_build.rs  # hezhou-player-build CLI（dry-run + 真实构建）
├── template/         # 构建配置 / cook 清单示例 JSON
└── tests/
    ├── fixtures/minimal/  # 最小 fixture（legacy assets[]，空场景 + 占位 DLL/runtime）
    ├── fixtures/cooked/   # hezhou-asset 真实 cook 输出布局 fixture
    └── *.rs               # 62 个测试（单元 + 集成）
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
4. 打开文件日志（`player.log`，UTC 时间戳、INFO/WARN/ERROR），记录启动摘要；
5. `--validate` 模式到此结束（CI 冒烟）；否则进入"运行时骨架"日志并正常退出。

参数：

```text
hezhou-player [--player-root <dir>] [--manifest <file>] [--log-dir <dir>]
              [--validate] [--version|-V] [--help|-h]
```

- **Windows 无终端**：`#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]`，
  发布版双击无控制台；所有诊断写文件。
- 日志默认路径：
  - Windows `%LOCALAPPDATA%\Hezhou\<product-slug>\logs\player.log`
  - Linux `$XDG_DATA_HOME/hezhou/<product-slug>/logs/player.log`
    （无 XDG 时 `$HOME/.local/share/hezhou/<product-slug>/logs`）
- 启动失败写 `startup-error.log`（包内 `logs/`，只读则回退系统临时目录）；
  退出码 `0` 成功、`2` 启动检查失败、未知参数/用法错误 `2`。
- 字体/脚本/场景缺失属于硬错误；`runtime/` 为空只告警（骨架阶段），
  正式发布必须内置运行时（PROJECT_PLAN §1.5 验收：干净机器不依赖 Mono）。

## 7. 测试

```bash
cd engine
cargo test -p hezhou-player     # 62 个测试
cargo build -p hezhou-player    # 构建两个二进制 + 库
```

覆盖：manifest 读写/严格字段/格式版本、配置校验、路径规范化与大小写冲突、
双平台 dry-run 清单与指纹稳定性、版本兼容矩阵、真实构建（复制/清单一致/大小与
校验和/失败清理/权限/跨平台拒绝）、fixture CLI 端到端（构建 → 复制出的 Player
`--validate`）、**hezhou-asset cook 集成**（manifest 解析、AssetId/kind/locator/output
映射、cook_manifest.json 入包、缺失输出、产物大小/哈希损坏、格式版本、启动场景/字体
定位、CLI 真实构建 + validate）、运行时 CLI、日志时间戳。

最小 fixture（`tests/fixtures/minimal/`）依赖零外部资产：空场景 3 字节、字体占位
12 字节、假 C# 程序集 21 字节、占位 runtime 库 19 字节，用于无真实 cook 数据时
验证"构建 → 打包目录 → 运行时启动校验"的完整链路。

真实 cook fixture（`tests/fixtures/cooked/`）模拟 `Cooker::cook_to_dir` 的
`cook_manifest.json + imported/<uuid>.json + assets/<uuid>.<ext>` 布局
（scene/font/texture/gltf 四个条目），用于验证 asset → Player 的字段与文件映射。

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
  **第一轮运行时尚未加载资产索引**（仅校验文件存在），但清单已随包提供
  `assets/cook_manifest.json`，且 Player manifest 内嵌 `asset_id/output/dependencies`，
  下一轮直接按 `AssetId` 加载 `imported/*.json` 即可，无需改构建契约。
- `imported/*.json` 内容当前不解析/不校验（只复制 + 校验文件存在/大小/哈希）。
- asset 依赖表/压缩标记等新增字段需先扩展 `ManifestAsset`（`assets[]` 严格反序列化）。
- legacy `{ "assets": [...] }` 清单仍支持（`--assets-dir` 提供资产根目录），
  作为无 cooker 时的过渡；字段映射为：`path`（相对资产根）、`kind`、
  `size_bytes`、`checksum`。
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
  下一轮在此 bin 内启用 `mono` feature 并链接引擎 crate，或由 examples 产出最终
  Player 可执行文件后由构建器复制。

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
   非 0 退出即失败；无真实 cooker 数据时先用 `tests/fixtures/cooked`（或
   `tests/fixtures/minimal`）做前置检查；
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
- [ ] Player 运行时接入窗口/Vulkan/Mono（当前仅启动校验骨架）；
- [ ] Player 运行时按 `AssetId` 加载 `assets/cook_manifest.json` / `imported/*.json`；
- [ ] Linux rpath 链接参数固化到构建脚本/CI（需要在 Linux job 注入 `RUSTFLAGS`）；
- [ ] 跨平台构建（在 Windows 上产出 Linux 包）——不计划支持，由 CI 双平台分别执行；
- [ ] 资产 pack/压缩格式（当前按 loose files 计划）；
- [ ] shader 资源打包规则；
- [ ] 窗口图标（Windows `.ico` / Linux `.png`）与 `window.icon` 字段；
- [ ] `--window-mode`/分辨率命令行覆盖（PROJECT_PLAN §1.5 启动参数）；
- [ ] 崩溃处理器与 `startup-error.log` 之外的结构化崩溃报告；
- [ ] 清单签名/校验（发布完整性）；
- [ ] 与 `dfx` 日志系统共用输出（当前自带最小文件日志）。
