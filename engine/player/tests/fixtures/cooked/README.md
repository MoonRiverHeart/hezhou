# hezhou-asset 真实 cook 输出 fixture（Player 集成测试 / 手工冒烟）

模拟 `Cooker::cook_to_dir` 的产物布局（契约见 `engine/asset/README.md` §6）：

```text
cook_manifest.json
imported/<gltf uuid>.json
assets/<uuid>.<ext>
```

- `cook_manifest.json`：format_version=1、roots、entries（id/kind/locator/output/
  dependencies/source_size/source_hash）；
- `assets/*.scene|ttf|png`：原样拷贝类资产，`source_size`/`source_hash` 与文件一致
  （Player 构建会校验，用于发现 cook 产物损坏）；
- `imported/*.json`：glTF 导入产物（内容为占位 JSON，Player 第一轮只复制不解析）；
- `build-config.json`：start_scene/font 使用**工程相对 locator**（`scenes/...`、
  `fonts/...`），构建器会映射到 cook entries 的 `output`；
- `scripts/`、`runtime/`：占位程序集/运行时，供 CLI 端到端构建使用。

资产内容为占位字节，**不是**真实游戏内容；真实 cooker 输出可原样替换本目录。
