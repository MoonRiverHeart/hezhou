# 编辑器材质纹理编辑（0.3 task-45）

为 PBR 材质提供 **albedo 纹理资产选择/清除/路径显示**，并把纹理路径保存到实体 PBR 材质
（Project/Prefab roundtrip 已由 task-26 支持；renderer 每帧同步由 `pbr_sync` 负责）。

| 层 | 文件 |
|---|---|
| 纹理列表 FFI | `engine/examples/src/editor/audio_asset_ffi.rs` → `asset_texture_list_json` |
| FfiContext | `engine/scripting/src/ffi_context/{audio_asset.rs,mod.rs}`（基线 **330**） |
| C# UI | `engine/scripts/{UI.cs,EditorScript*.cs,TestRunner.cs}`（资产面板「材质」Tab 扩展） |
| 契约 | `engine/scripts/check_ffi_contract.py`（`EXPECTED_FIELD_COUNT = 330`） |

## 1. 纹理来源（AssetDatabase）

- 新增 FFI `asset_texture_list_json(buf,len)`：遍历 AssetDatabase 中
  `AssetKind::Texture` 的**源资产**（跳过子资产），返回：

```json
{
  "count": 1,
  "textures": [
    { "path": "textures/wood.png", "asset_id": "uuid", "size_bytes": 12345, "format": "png" }
  ]
}
```

- 路径为相对 `assets/` 的逻辑路径；扩展名支持 png/jpg/jpeg/tga/bmp/hdr/ktx2/dds/webp
  （asset crate `AssetKind::from_extension` 规则）。
- 数据库未初始化时返回空列表；失败安全。

## 2. 材质 Tab 用法

资产面板 →「材质」Tab：

- 顶部原有 **材质库** primitive 材质按钮保持不变（不改变现有 material 赋值）；
- 新增 **PBR albedo 纹理（AssetDatabase）** 区：
  - **当前纹理** 标签：选中实体的 PBR `albedo_texture_path`（未设置显示“默认中性贴图”）；
  - **纹理列表**：每个 Texture 资产一个按钮 `🖼 <文件名> [<format> <size>B]`
    （本轮无缩略图渲染，🖼 为占位图标）；
  - 点击纹理按钮 → 把该路径写入**选中实体**的 PBR albedo：
    - 读取实体当前 PBR JSON（缺失字段用默认值），只替换 `albedo_texture_path`；
    - `UI.SceneSetEntityPbrMaterial` 写回 core → `EditorSyncPbrMaterialsNow` 立即同步 renderer；
    - 同时刷新 Inspector 渲染 Tab 的 PBR 材质行（若已构建）；
  - **清除 albedo 纹理**：把 `albedo_texture_path` 设为 `null`（材质其余字段保留）；
  - **刷新纹理列表**：重新扫描/导入后点此重建按钮列表；
  - 状态标签显示操作结果（含 renderer 覆盖数）与失败原因；未选实体时提示先选择。

## 3. 持久化与渲染同步

- `albedo_texture_path` 保存在 `RenderableComponent.pbr_material` → `ProjectEntity.pbr_material`
  （task-26，旧工程兼容）；Prefab 捕获/实例化同样保留；
- 编辑器主循环 `pbr_sync::sync_pbr_materials` 每帧把场景材质同步到
  `UIVulkanRenderer::set_entity_pbr_material_params` + `set_entity_pbr_texture`，
  因此赋值/清除后 renderer 覆盖表立即生效（legacy 通道不消费 PBR 覆盖，属既有设计）。

## 4. 测试证据

- 编辑器启动自检：**PASS=91, FAIL=0**（task-43 的 88 + 材质纹理 3 项），证据
  （`engine/logs/hezhou_2026-11-08.log`）：
  - `纹理列表包含临时 PNG（len=374）`（自检写入 `assets/__ffi_tex_selftest__.png` → `AssetScan` → 列表 JSON 含路径与 `"png"`）
  - `设置 albedo 纹理 roundtrip`（PBR JSON 写回含 `assets/__ffi_tex_selftest__.png`）
  - `清除 albedo 纹理后读取为空`
  - 自检结束删除临时纹理与 `.meta`、重新扫描并刷新列表（无仓库残留）。
- 契约 **330/330/330 OK**；`build_mono.ps1` exit 0（DLL 402,432 字节 / 393 KB）；
  `ci_local.ps1 -SkipBuild` 通过（需可写 TMP/TEMP 供 glfw-sys CMake/MSBuild）。

## 5. 限制 / 后续

1. 仅 **albedo 单纹理槽**；metallic/roughness/normal/AO 纹理槽待渲染层接口扩展。
2. 纹理路径未走 AssetDatabase 的导入/cook 引用（仅记录字符串路径）；Player cook 映射后续接入。
3. 无缩略图渲染（按钮前缀 🖼 占位）；后续可用 GridView + ImageRegion 显示小图。
4. 不提供采样器/压缩/尺寸等导入设置；不支持纹理拖放赋值。
5. 仅支持选中单实体；多选批量赋纹理未实现。
6. legacy 渲染通道忽略 PBR 覆盖（纹理仅在 PBR 质量档/PBR 通道下生效）。
