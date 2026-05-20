# Hezhou Editor - Release v0.1.0

## 系统要求

**已内置 Mono SDK**，无需额外安装依赖！

直接运行即可。

## 运行方法

### 方式1: 双击exe
直接双击 `hezhou-editor.exe`

### 方式2: 使用启动脚本
双击 `start.bat` (会显示启动信息)

## 功能说明

### 工具栏按钮
- **新建**: 创建新场景/脚本
- **打开**: 打开场景/项目
- **保存**: 保存场景
- **运行**: 切换游戏运行模式 (立方体开始旋转)
- **编辑器**: 切换脚本编辑器视图

### 游戏预览控制
- **运行模式**: 点击"运行"按钮进入
  - PreviewWindow边框变为蓝色
  - WASD键移动摄像机
  - 鼠标拖拽旋转摄像机
  - ESC键退出运行模式 (回到编辑状态)
  
- **编辑模式**: 初始状态
  - PreviewWindow边框为橙色
  - 点击立方体选中Entity
  - 右侧属性面板显示Entity信息
  - ESC键取消选中

### 状态栏
- FPS显示
- 当前状态: 就绪/运行中
- 项目名称

## 技术信息

- 渲染引擎: Vulkan
- UI系统: 自研 Rust UI框架
- 脚本系统: Mono JIT (C#)
- 字体: 内嵌Roboto

## 文件结构

```
hezhou-editor/                   (~18 MB 解压后, ~7 MB 压缩包)
├── hezhou-editor.exe            # 主程序 (10 MB)
├── mono-2.0-sgen.dll            # Mono核心 (5.8 MB)
├── libmono-btls-shared.dll      # Mono TLS支持 (0.8 MB)
├── MonoPosixHelper.dll          # Mono POSIX辅助 (0.15 MB)
├── etc/                         # Mono配置 (1 MB)
│   └── mono/
│       ├── config
│       ├── 2.0/
│       ├── 4.0/
│       └── 4.5/
├── scripts/
│   └── bin/
│       └── Mono/
│           └── EditorScript/
│               └── EditorScript.dll  # C#编辑器脚本
├── start.bat                    # 启动脚本
└── README.md                    # 本说明文件
```

**已内置所有依赖**，无需安装Mono SDK！

## 已知问题

见 `engine/FIXES.md` (需要编译查看)

## 更新日志

### 2026-05-20
- ✅ 修复 outline winding order (旋转后outline消失)
- ✅ 修复 ESC键处理 (Running模式ESC切换到Editing)
- ✅ 添加 depth testing (立方体正确遮挡)