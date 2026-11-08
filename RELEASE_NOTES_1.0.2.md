# Hezhou 1.0.2 发布说明

> 状态：Release Candidate / 待 CI。基线：1.0.1。

## 修复

- Player 窗口 host 默认**运行到用户关闭窗口**，不再 120 帧（约 2 秒）自动退出。
  - `HEZHOU_PLAYER_WINDOW_FRAMES` 仍可限帧（CI 冒烟/自动化）。
  - `0` 表示无限。
- 第三人称 demo 现在可以直接双击运行并持续操作。

## 内容（沿用 1.0.1）

- `hezhou-third-person-player-windows.zip` / `hezhou-third-person-player-linux.tar.gz`
- WASD/方向键移动、Space 跳跃、物理胶囊、第三人称跟随相机
- 编辑器模板 `third-person`

## 验证

- Player tests（window-host/mono-host）通过
- 1.0.2 CI 全绿后发布

## 已知限制

- 无鼠标视角/相机平滑
- 方块角色无材质颜色
- 平台 Windows/Linux x64
