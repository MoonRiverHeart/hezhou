# 0.2 per-entity PBR 材质：core / Project / Inspector / FFI（task-26）

本文档记录每实体 PBR 材质从渲染层接入 core、Project 持久化、Inspector 编辑与 FFI 的完整链路。
配套：`FFI_RENDER_0_2.md`（task-15 画质/PBR 通道）、`FFI_PREFAB_SCENE_PLAYER_0_2.md`（task-21/25）。

| 层 | 文件 |
|---|---|
| core 组件 | `engine/core/src/ecs/components/render.rs` → `PbrMaterial`、`RenderableComponent.pbr_material` |
| core Scene API | `engine/core/src/ecs/scene.rs` → `set_entity_pbr_material` / `get_entity_pbr_material` |
| Project 持久化 | `engine/core/src/project.rs` → `ProjectEntity.pbr_material`（serde default，旧工程兼容） |
| Prefab | `engine/core/src/prefab.rs` → `PrefabRenderable.pbr_material`（捕获/实例化保留） |
| core FFI | `engine/core/src/scene_ffi.rs` → `scene_set_entity_pbr_material_json` / `scene_get_entity_pbr_material_json` |
| 编辑器同步 | `engine/examples/src/editor/pbr_sync.rs`（主循环每帧 + `editor_sync_pbr_materials_now`） |
| FfiContext | `engine/scripting/src/ffi_context/{renderer.rs,mod.rs}`（tail 追加，基线 **316**） |
| C# UI | `engine/scripts/{UI.cs,UI.Scene.cs,EditorScript*.cs,TestRunner.cs}` |
| 契约 | `engine/scripts/check_ffi_contract.py`（`EXPECTED_FIELD_COUNT = 316`） |

## 1. core 数据模型

```rust
pub struct PbrMaterial {
    pub albedo: [f32; 4],              // RGBA 线性空间
    pub metallic: f32,                 // 0..1
    pub roughness: f32,                // 0..1
    pub normal_scale: f32,             // 0..8
    pub ao: f32,                       // 0..1
    pub emissive: [f32; 3],
    pub emissive_strength: f32,        // 0..64
    pub albedo_texture_path: Option<String>,
}
```

- `Default`：albedo=[1,1,1,1]、metallic=0、roughness=0.5、normal_scale=1、ao=1、
  emissive=[0,0,0]、emissive_strength=1、texture=None。
- `PbrMaterial::sanitized()`：NaN/∞ 回退默认值，数值 clamp（编辑器输入/旧数据保护）。
- 挂载：`RenderableComponent.pbr_material: Option<PbrMaterial>`；无渲染组件的实体设置失败。
- `PbrMaterial::to_json/from_json` 供 FFI/Inspector 使用（未知字段忽略、缺失字段取默认）。
- 限制（0.2）：**仅 albedo 单纹理槽**；metallic/roughness/normal/AO 纹理槽后续扩展。

## 2. Project 持久化

`ProjectEntity` 新增字段（`serde` 默认 + `skip_serializing_if`）：

```json
{
  "id": 1, "name": "PbrCube", "mesh_type": "Cube",
  "transform": { ... }, "material": null,
  "pbr_material": {
    "albedo": [0.8, 0.2, 0.1, 1.0],
    "metallic": 0.6, "roughness": 0.3,
    "normal_scale": 1.25, "ao": 0.9,
    "emissive": [0.15, 0.0, 0.3], "emissive_strength": 2.5,
    "albedo_texture_path": "assets/tex/rust.png"
  }
}
```

- 旧工程无 `pbr_material` 字段 → 反序列化为 `None`（兼容，不回归）。
- `Project::sync_from_scene` / `sync_to_scene` 双向映射；快照/撤销恢复同样保留。
- Prefab 捕获/实例化保留 PBR 材质（`PrefabRenderable.pbr_material`，旧 Prefab 兼容）。

## 3. FFI（新增 3，FfiContext 316）

| FFI | 签名 | 语义 |
|---|---|---|
| `scene_set_entity_pbr_material_json` | `fn(scene, entity_id, json) -> i32` | 0=成功；-1=无渲染组件/场景空；-2=JSON 解析失败；`null`/空串=清除 |
| `scene_get_entity_pbr_material_json` | `fn(scene, entity_id, buf, len) -> usize` | 写入 JSON（无材质写空串），返回字节数 |
| `editor_sync_pbr_materials_now` | `fn() -> i32` | 立即把场景材质同步到 renderer，返回 renderer 材质覆盖数 |

C# 包装：`UI.SceneSetEntityPbrMaterial(scene, id, json)`、`UI.SceneGetEntityPbrMaterial(scene, id)`、
`UI.EditorSyncPbrMaterialsNow()`。

渲染同步：`pbr_sync::sync_pbr_materials` 每帧遍历 `RenderableComponent.pbr_material`，
调用 `UIVulkanRenderer::set_entity_pbr_material_params` + `set_entity_pbr_texture`；
对已删除/已清除材质的实体调用 `remove_entity_pbr_material`；集合变化时日志
`[PBR] 每实体材质同步: N 个实体（renderer 覆盖 M）`。

## 4. Inspector 用法

属性面板 →「渲染」Tab（管线选择下方）新增 **PBR 材质（当前实体）** 区：

| 行 | 字段 |
|---|---|
| Albedo | R/G/B/A 4 个输入框 |
| M / R | metallic、roughness |
| N / AO | normal_scale、ao |
| Emissive | R/G/B |
| 强度 / 纹理 | emissive_strength、albedo_texture_path |
| 状态行 | 「PBR: 已设置/未设置/失败」+「清除材质」按钮 |

- 选择实体 → 自动读取 core 材质回填（无材质显示默认值）；修改任意输入框即时
  `SceneSetEntityPbrMaterial` + `EditorSyncPbrMaterialsNow`，renderer 下一帧使用；
- 「清除材质」→ 回退渲染层默认材质；
- 材质随 Project 保存/加载（`ProjectEntity.pbr_material`），撤销/重做快照同样保留。

## 5. 测试证据

- core：`cargo test -p hezhou-core --test pbr_material_roundtrip`（5 项：默认值/set-get-sanitize/
  快照 roundtrip/旧工程 JSON 兼容/Prefab 捕获实例化）；
- 全量 `cargo test -p hezhou-core`：无回归；
- 契约：316/316/316 OK；`build_mono.ps1` exit 0；`ci_local.ps1 -SkipBuild` OK；
- 编辑器启动自检：**PASS=77, FAIL=0**（task-25 的 71 + PBR 6 项），日志含：
  `PbrMaterial - 材质 JSON roundtrip ...`、`[PBR] 每实体材质同步: 1 个实体（renderer 覆盖 1）`、
  `清除材质后读取为空`、`[PBR] 每实体材质同步: 0 个实体（renderer 覆盖 0）`。

## 6. 已知限制 / 后续

1. 仅 albedo 纹理槽；metallic/roughness/normal/AO 纹理待渲染层接口扩展后接入。
2. 纹理路径仅记录字符串，未走 AssetDatabase 导入/cook（Player 侧需 asset 管线映射）。
3. Inspector 目前是固定行布局，无颜色拾取器/滑块；后续可用 Slider/颜色控件替换。
4. 多选批量编辑 PBR 材质未实现（当前按单实体编辑）。
5. PBR 覆盖仅在本帧渲染通道生效；legacy 通道忽略覆盖（符合 0.2 设计）。
