# Hezhou Editor 0.1.0

## Highlights

- 双主题 UI Design Tokens、SVG 图标、圆角控件与字体清晰度修复
- Dock v1/v2：布局持久化、Tab 拖拽重排、面板折叠、浮动面板、跨区域停靠
- Hierarchy / Inspector / Asset Browser / Console / Command Palette
- Shift 范围多选、拖拽 reparent、材质拖放与持久化
- Gizmo：平移/缩放/旋转、3D renderer 轴线段
- 资产缩略图：SVG 图标、程序化纹理、BMP 像素缩略图、PNG 完整解码与图集上传
- 引擎基础：ECS 系统调度、Lifetime、物理 AABB 碰撞/事件、几何射线
- 重力、CharacterController、变换关键帧动画、渐变天空盒
- 打包：Windows MSIX（自签名证书）+ MSI，Linux DEB（Python ar/tar），内置最小 Mono 运行时

## Packaging

- Windows: `dist/HezhouEditor-0.1.0.msix` + `dist/HezhouEditor.cer` + `dist/HezhouEditor-0.1.0.msi`
- Linux: `dist/hezhou-editor_0.1.0_amd64.deb`

CI 的 `package-windows` / `package-linux` 在 tag `v*` 或手动触发时构建，
`publish-release` 自动创建 GitHub Release 并上传 MSIX/MSI/DEB。

## Validation

- Visual E2E: 33/33
- Intent E2E: 26/26
- Random/Stress: passed
- FFI contract: 283/283/283
