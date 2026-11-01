# hezhou-asset — 引擎级资产管线（0.2）

`hezhou-asset` 是 Hezhou 0.2 的资产底层能力 crate：**AssetId/UUID、`.meta`、资产数据库、
依赖图、glTF/GLB 导入、cook manifest**。它不绑定任何具体游戏，也不解析场景/编辑器状态。

上游：编辑器（导入、资产浏览）与 Player 构建器；下游：animation / render / audio / gameplay
通过本 crate 的公开数据契约消费导入结果。

## 目录

- [1. 能力与模块](#1-能力与模块)
- [2. 工程目录与逻辑路径规则](#2-工程目录与逻辑路径规则)
- [3. `.meta` 格式](#3-meta-格式)
- [4. 公开 API](#4-公开-api)
- [5. 导入数据契约（animation/render 集成）](#5-导入数据契约animationrender-集成)
- [6. Cook manifest 与 Player 集成契约](#6-cook-manifest-与-player-集成契约)
- [7. 跨平台与确定性](#7-跨平台与确定性)
- [8. Feature 与依赖/网络限制](#8-feature-与依赖网络限制)
- [9. 测试](#9-测试)
- [10. 0.2 已知限制与 TODO](#10-02-已知限制与-todo)

## 1. 能力与模块

| 模块 | 内容 |
|---|---|
| `id` | `AssetId`（UUID，稳定身份）、`AssetKind`、`AssetRef<K>` 类型化引用（MeshRef/TextureRef/…） |
| `path` | `LogicalPath`（工程相对、大小写不敏感、`/` 分隔）、`AssetLocator`（逻辑路径 + 子资产键） |
| `meta` | `.meta` JSON 读写：UUID、类型、导入设置、依赖、子资产表、前向兼容字段 |
| `database` | `AssetDatabase`：扫描、路径 ↔ ID、依赖图、glTF 导入注册 |
| `import` | 与第三方库解耦的导入数据契约；`import::gltf` 为真实 glTF/GLB 解析器（feature） |
| `animation_bridge` | （feature `animation-bridge`）把 `ImportedScene` 适配成 `hezhou-animation::gltf::GltfAsset` |
| `cook` | 依赖闭包 → 确定性 `cook_manifest.json` + Player 产物 |
| `math` | 导入契约用的轻量数学类型：Vec2/3/4、Quat、Mat4（列主序）、Trs、Aabb |

**引用稳定**：场景/Prefab/材质等一切引用只保存 `AssetId`（UUID），不保存路径。
资产或整个工程移动目录后，重新 `AssetDatabase::scan()` 即可恢复全部引用。

## 2. 工程目录与逻辑路径规则

```text
my_project/
├── assets/                     # 资产根（默认，可由 AssetDatabase::with_assets_root 配置）
│   ├── models/hero.gltf
│   ├── models/hero.gltf.meta
│   ├── textures/hero_albedo.png
│   └── textures/hero_albedo.png.meta
└── (工程文件 / 场景等由其它模块负责)
```

逻辑路径规则（Windows/Linux 一致）：

1. 相对**工程根目录**，如 `assets/models/hero.gltf`（不是相对 `assets/`）。
2. 分隔符统一 `/`；输入中的 `\` 自动转换。
3. **大小写不敏感**比较（相等/排序/Hash 基于 Unicode 小写键），原始大小写保留用于显示。
   避免 Windows 不区分大小写、Linux 区分大小写导致的“同一资产两个 ID”。
4. `.` 忽略，`..` 只在工程内回退，越界报错；拒绝绝对路径/盘符/UNC 与 Windows 非法字符
   `< > : " | ? * #`。
5. 子资产定位符：`assets/models/hero.gltf#mesh:0`（`#<kind>:<index>`）。

## 3. `.meta` 格式

每个资产文件旁有一个 `<文件名>.meta`（UTF-8 JSON，原子写：临时文件 + rename）。

```json
{
  "meta_version": 1,
  "uuid": "f6f9c1d2-1111-4222-8333-444455556666",
  "kind": "gltf",
  "import_settings": {
    "importer": "gltf",
    "settings": {
      "import_meshes": true,
      "import_materials": true,
      "import_textures": true,
      "import_skins": true,
      "import_animations": true,
      "scale": 1.0
    }
  },
  "dependencies": ["<被引用的其它资产 uuid>"],
  "sub_assets": {
    "animation:0": {
      "uuid": "<子资产 uuid>",
      "kind": "animation",
      "name": "Idle",
      "dependencies": ["<父文件 uuid>"]
    },
    "mesh:0": { "uuid": "…", "kind": "mesh", "name": "HeroMesh", "dependencies": ["…"] }
  }
}
```

- `uuid` 是资产身份的**唯一**来源；`.meta` 跟随文件移动，因此工程/目录移动不丢引用。
- 首次扫描时若缺少 `.meta`，按扩展名推断类型并生成 UUID v4 并写盘。
- 子资产 UUID：优先复用 `.meta` 里已有的 ID；新子资产由父 UUID 通过 **UUID v5**
  （`kind:index` 命名）确定性派生——同一份 glTF 在任意机器导入得到相同 ID。
- 未识别字段保存在内部 `extra` 中并在保存时原样写回（旧编辑器打开新工程不丢数据）。
- `meta_version` 高于当前支持版本时加载报错（`AssetError::UnsupportedMetaVersion`）。

## 4. 公开 API

```rust
// 打开工程：扫描 assets/，缺 meta 自动补齐
let mut db = AssetDatabase::open("my_project")?;

let path = LogicalPath::new("assets/models/hero.gltf")?;
let id: AssetId = db.id_of(&path).unwrap();               // 路径 -> ID
let again = db.id_of(&LogicalPath::new("ASSETS/Models/Hero.GLTF")?); // 大小写不敏感

// 类型化引用（校验实际类型）
let mesh_ref: MeshRef = db.typed_ref(mesh_id)?;

// 依赖图：直接依赖 / 拓扑序闭包（依赖在前）
let deps = db.dependencies(mesh_id)?;
let ordered = db.dependency_closure(&[mesh_id])?;         // 成环返回 DependencyCycle

// glTF 导入（注册 Mesh/Material/Texture/Skeleton/Animation 子资产）
let outcome: GltfImportOutcome = db.import_gltf(id)?;
let mesh = outcome.sub_asset(AssetKind::Mesh, 0).unwrap();
let scene: &ImportedScene = &outcome.scene;

// cook
let manifest = Cooker::new(&db).plan(&[mesh.id])?;        // 只含被引用资产
manifest.save(Path::new("out/cook_manifest.json"))?;
Cooker::new(&db).cook_to_dir(&manifest, "out")?;          // 写 manifest + imported/*.json + assets/*
```

## 5. 导入数据契约（animation/render 集成）

`ImportedScene`（`serde` 可序列化）是 asset → animation/render/player 的公开契约，**不含任何
`gltf` crate 类型**：

```rust
pub struct ImportedScene {
    pub format: ImportSourceFormat,           // Gltf / Glb
    pub generator: Option<String>,
    pub nodes: Vec<ImportedNode>,             // parent/children/mesh/skin/local_transform(Trs)
    pub meshes: Vec<ImportedMesh>,            // primitives: positions/normals/tangents/uvs/
    pub materials: Vec<ImportedMaterial>,     //   joints/weights/indices/material/skeleton/bounds
    pub textures: Vec<ImportedTexture>,       // sampler + Embedded(原始编码字节)/ExternalFile(uri)
    pub skeletons: Vec<ImportedSkeleton>,     // joints{node,parent,ibm} + root_joint
    pub animations: Vec<ImportedAnimation>,   // channels{target_node,target_joint,property,
                                              //   interpolation,times,values} + duration
    pub external_files: Vec<String>,          // 相对源文件的 buffer/image URI
    pub warnings: Vec<String>,
}
```

索引与数值约定（与 `hezhou-animation` 契约一致）：

- `ImportedNode.parent`、`ImportedAnimationChannel.target_node` 都是 **glTF 节点索引**；
- `ImportedSkeleton.joints` 顺序 = `skin.joints` 顺序 = 运行时 `JointIndex`；
- `ImportedJoint.parent` 是 **joint 索引**（不是节点索引）；
- 动画 `values` 扁平存储：Translation/Scale 每帧 3 个 f32，Rotation 每帧 4 个 f32（xyzw）；
  `times` 升序；CUBICSPLINE 每帧为 `in-tangent/value/out-tangent` 三段；
- 蒙皮：`ImportedPrimitive.joints: Vec<[u16;4]>`、`weights: Vec<[f32;4]>`。

### 与 hezhou-animation 的适配（feature：`animation-bridge`）

```toml
hezhou-asset = { path = "../asset", features = ["animation-bridge"] }
```

```rust
use hezhou_asset::animation_bridge::imported_scene_to_gltf_asset;

let bridge = imported_scene_to_gltf_asset(&outcome.scene); // GltfAssetBridge
let runtime = bridge.into_runtime()?;                      // Skeleton/Skin/Clip
```

桥接产出 `hezhou_animation::gltf::GltfAsset { skeleton, skins, animations }`：

- `GltfNode.parent` / `GltfChannel.target_node` 用 glTF 节点索引；`nodes` 表按节点索引填充；
- `GltfSkeleton.joint_nodes` 顺序 = `JointIndex`，`inverse_bind_matrices` 一一对应；
- 动画 `events` 目前为空（glTF 标准无事件；事件从扩展/命名约定解析是后续 TODO）。

animation crate 侧唯一入口仍是 `GltfAsset::into_runtime() -> AnimationAssetData`。

## 6. Cook manifest 与 Player 集成契约

`Cooker::plan(roots)` 从根资产计算依赖闭包，**只包含被引用资产**，输出确定性 manifest：

```json
{
  "format_version": 1,
  "roots": ["<根资产 uuid>"],
  "entries": [
    {
      "id": "<asset uuid>",
      "kind": "mesh",
      "locator": "assets/models/hero.gltf#mesh:0",
      "output": "imported/<父文件 uuid>.json",
      "dependencies": ["<父文件 uuid>", "<material uuid>", "<skeleton uuid>"],
      "source_size": 376,
      "source_hash": "0123456789abcdef"
    }
  ]
}
```

`cook_to_dir(manifest, out)` 的输出布局：

```text
<out>/
├── cook_manifest.json        # 本清单（Player 启动时读取并校验 format_version）
├── imported/<uuid>.json      # 一个 glTF 一份导入产物：ImportedScene JSON
│                             #   - 几何已烘焙（不依赖外部 .bin）
│                             #   - 纹理 ExternalFile.uri 已重写为 cooked 相对路径
└── assets/<uuid>.<ext>       # 其它资产（纹理/音频/二进制）原样拷贝
```

**Player 侧集成步骤（契约）**：

1. Player 启动读取 `cook_manifest.json`，校验 `format_version <= 1`；
2. 以 `AssetId` 为键建立索引（`id -> output`），**不要**依赖逻辑路径拼路径；
3. 资源按需加载：`imported/*.json` 反序列化为 `ImportedScene`（或 0.2 Player 定义的等价结构），
   `assets/*` 直接读字节（纹理/音频）；子树引用（材质→纹理、mesh→skeleton）用 manifest
   `dependencies` 校验完整性；
4. 若需要加载顺序，按 manifest `entries` 的依赖拓扑（`dependency_closure` 已保证依赖在前）
   或运行时按需索引；
5. `source_hash` / `source_size` 可用于资源校验、增量下载/热更新与包体审计；
6. Windows/Linux 使用同一份 manifest；路径均为 `/` 分隔的相对路径，由 Player 拼接自身根目录。

**与 player-owner 的接口（当前实现）**：

- 资产列表：`CookManifest.entries`（每个资产一条：`id/kind/locator/output/dependencies/source_size/source_hash`）。
- 只含被引用资产：`Cooker::plan` 的闭包语义；根资产由 Player 构建配置（启动场景/Prefab/
  gameplay 组件引用的 AssetId）传入。
- 值类型均实现 `Serialize/Deserialize`，可直接写入 Player 自己的 `manifest.json`（Player
  crate 已有格式），把 `CookManifest` 作为其资产段的嵌入数据即可。
- `CookOutput { output_dir, manifest_path, copied_files, imported_files, total_bytes }` 可用于
  构建日志与包体统计。
- 后续接入点：Player 侧把 0.2 的 `PlayerManifest` 与 `CookManifest` 合并、按 `AssetId` 加载
  `imported/*.json`、把 `assets/*` 映射到 Player 资源目录；这些工作由 Player 模块完成，
  asset crate 只保证格式与确定性。

## 7. 跨平台与确定性

- 逻辑路径在 Windows/Linux 得到同一套键（`\`→`/`、大小写不敏感）；扫描顺序按文件名排序，
  与文件系统返回顺序无关。
- `.meta` 依赖列表、子资产表序列化前排序去重；cook manifest 条目按
  `(逻辑路径, 子资产键, AssetId)` 排序，依赖按 ID 排序，`source_hash` 为内容 FNV-1a 64。
- 结果：同一工程 + 同一根资产，在 Windows/Linux、不同机器、重复执行下得到**逐字节一致**的
  `cook_manifest.json`（依赖闭包中资产内容不变的前提下）。
- 不写入时间戳/主机信息/绝对路径。

## 8. Feature 与依赖/网络限制

| feature | 默认 | 说明 |
|---|---|---|
| `gltf` | ✅ | 启用真实 glTF/GLB 解析器（依赖 crates.io `gltf = 1.4`，`default-features = false` + `import`/`names`）。关闭后 `import` 保留数据契约与同签名桩函数，数据库/cook 仍可编译，调用导入返回 `AssetError::ImporterDisabled("gltf")`。 |
| `animation-bridge` | ❌ | 启用 `hezhou_animation` 桥接模块；默认关闭避免 asset/animation 构建互相牵制。 |

其它依赖：`serde`/`serde_json`、`uuid`（v4/v5/serde）、`thiserror`、`base64`（data URI）、
`tempfile`（仅 dev）。**不依赖 `image`**：图片按原始编码字节保存，解码由渲染/纹理层负责。

- 网络限制：构建 `gltf` feature 需要一次从 crates.io 拉取依赖；离线环境用
  `cargo test -p hezhou-asset --no-default-features` 可完整编译（glTF 测试自动跳过）。
- 新增依赖会更新工作区 `engine/Cargo.lock`（asset 写作用域外的自动副作用）。

## 9. 测试

```bash
cd engine
cargo test -p hezhou-asset                                   # 默认（含 gltf）
cargo test -p hezhou-asset --features animation-bridge       # 含 animation 桥接
cargo test -p hezhou-asset --no-default-features             # 离线/无 gltf
```

覆盖：`.meta` 读写与版本/前向兼容、UUID v5 稳定性、路径规范化（分隔符/大小写/越界）、
数据库扫描与重新打开、工程移动、资产目录移动、类型化引用校验、依赖拓扑/成环、cook 闭包
（只含被引用资产）、cook manifest 确定性与 roundtrip、外部纹理 URI 重写、glTF/GLB 的
Mesh/Material/Texture/Skeleton/Animation 导入、导入设置开关、animation 桥接与运行时转换。

## 10. 0.2 已知限制与 TODO

- 不导入 FBX/OBJ/其他格式；glTF 不支持 Draco/meshopt/KTX2 等压缩扩展（产生 warning）。
- 只导入 `TEXCOORD_0`；顶点色 `COLOR_0` 产生 warning 后忽略；morph target 几何不导入。
- 稀疏 accessor 仅读基础值并 warning；CUBICSPLINE 数据保留但 animation 0.2 运行时不采样切线。
- 动画事件暂未解析（`GltfAnimation.events` 为空）。
- 子资产 ID 按 `kind:index` 复用：在 glTF 中间插入/删除子资产可能导致后续子资产 ID 漂移
  （0.2 可接受；0.3 可改为按内容哈希/名称追踪）。
- 不做纹理压缩/平台变体烘焙、不做分包/Addressables、不做资产热重载 watcher（编辑器侧
  重新 `scan()`/`import_gltf()` 即可）。
- 修改 workspace/core/render/editor/CI 不在本 crate 范围内；需要接口变更请通过项目负责人协调。
