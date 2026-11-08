# Hezhou 1.0.1 发布说明

> 状态：Release Candidate / 待 CI。基线：1.0.0。

## 修复与新增

- **Capability Player 改为真实第三人称 demo**：
  - 可见地面、方块角色、天空盒/方向光
  - WASD/方向键移动、Space 跳跃、物理胶囊
  - 第三人称跟随相机（ABI v7）
  - 角色四肢摆动、移动转向
- 新增编辑器项目模板 **third-person**：
  - Ground、Player 占位、CameraAnchor、障碍/台阶、Sun
  - Build Settings、README
- 新增 renderer 键盘持续按键状态 API
- 新增 Player ABI v7：input axis/jump、camera follow/clear

## 验证

- 本机第三人称 Player 30 帧：角色 +3m、jump frame 20 离地、相机跟随 exit 0
- Player tests：默认 132 / window-host 135 / mono-host 144
- renderer key_state 60 tests
- 模板 JSON/core 加载/扫描 PASS

## 使用

下载 `hezhou-third-person-player-windows.zip` / tar.gz，解压运行：

```powershell
.\third-person-demo.exe --run --host window --manifest .\manifest.json --log-dir .\logs
```

WASD/方向键移动，Space 跳跃；关闭窗口退出。窗口默认 120 帧，可设置 `HEZHOU_PLAYER_WINDOW_FRAMES=100000` 保持更久。

## 已知限制

- 无鼠标视角/相机平滑
- 方块角色无材质颜色
- 工程脚本编译未接入模板（PlayerGame 生成角色）
- 平台仍为 Windows/Linux x64
