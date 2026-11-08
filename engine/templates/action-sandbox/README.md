# {{PROJECT_NAME}}

由 Hezhou `action-sandbox` 模板创建（engine {{ENGINE_VERSION}}，模板版本 0.4.0）。

## 内容

- 一个 `Cube` 实体（PBR 材质：橙色金属感）并绑定内置 `RotatingEntity` 脚本
- 一个 `Sun` 方向光
- `build.settings.json`：Player 导出配置（启动场景 `scenes/main.scene`）
- `scripts/Spin.cs`：示例工程脚本骨架（未加入 EditorScript 编译，见 scripts/README）

## 下一步

1. 编辑器「工程 → 打开工程」选择此目录，或「新建项目」时用 action-sandbox 模板。
2. 运行/暂停预览，观察 Cube 旋转（RotatingEntity 已随编辑器编译）。
3. 在「Build」Tab 调整产品名/窗口/质量后导出 Player。
