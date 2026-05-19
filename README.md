# Hezhou Engine Demo Programs

## 概述

Hezhou是一个跨平台游戏引擎，支持Rust原生代码和C#脚本（通过Mono JIT或NativeAOT）。以下是所有演示程序及其功能说明。

## Demo程序列表

### 基础演示

#### `engine_demo`
基础引擎功能演示，展示核心引擎架构和初始化流程。

#### `glfw_demo`
GLFW窗口系统演示，测试窗口创建、事件处理和输入系统。

#### `triangle_demo`
基础三角形渲染演示，展示RHI抽象层的基本使用。

### Vulkan渲染演示

#### `vulkan_triangle_demo`
Vulkan原生渲染三角形，展示底层Vulkan API的使用。

#### `vulkan_render_demo`
完整的Vulkan渲染管线演示，包括：
- Swapchain创建和管理
- 渲染通道配置
- 管线状态对象
- 缓冲区管理

#### `ui_vulkan_demo`
UI系统与Vulkan渲染集成演示，展示：
- UI布局系统
- 文本渲染
- Vulkan后端渲染

### Mono JIT脚本演示

#### `mono_rotation_demo`
**控制台模式**，测试Mono JIT脚本执行：
- 无渲染窗口，纯控制台输出
- C#脚本计算旋转角度
- 测试Mono运行时初始化

运行命令：
```bash
cd engine
cargo run --bin mono_rotation_demo --features mono
```

#### `mono_triangle_demo`
**Vulkan渲染 + Mono JIT**，展示：
- 旋转三角形渲染
- C#脚本驱动旋转逻辑
- **支持热重载**：按 `R` 键重新编译C#脚本并立即生效

运行命令：
```bash
cd engine
cargo run --bin mono_triangle_demo --features mono
```

热重载测试：
1. 运行demo
2. 修改 `engine/scripts/RotationScript.cs` 中的 `_rotationSpeed`
3. 在窗口中按 `R` 键
4. 观察旋转速度立即改变

#### `mono_hot_reload_test`
**自动化热重载测试**，验证：
- C#脚本修改后重新编译
- Mono assembly unload/reload
- 静态变量重新初始化
- 修改效果自动验证

运行命令：
```bash
cd engine
cargo run --bin mono_hot_reload_test --features mono
```

#### `mono_ui_demo`
**UI系统 + Mono JIT**，展示：
- UI组件创建（Button、Label、Panel等）
- C#脚本驱动UI逻辑
- 基础交互功能

运行命令：
```bash
cd engine
cargo run --bin mono_ui_demo --features mono
```

#### `mono_ui_thunk_demo`
**UI系统 + Thunk回调机制**，展示：
- VStack/HStack布局容器
- C#创建widgets，Rust计算布局
- **Thunk回调**：C#按钮点击回调通过Thunk机制传递到Rust
- 窗口resize触发布局更新

运行命令：
```bash
cd engine
cargo run --bin mono_ui_thunk_demo --features mono
```

#### `mono_editor_demo`
**游戏编辑器演示**，完整编辑器UI：
- **窗口尺寸**：1280x720
- **布局结构**：
  - 顶部工具栏（40px）：新建/打开/保存/运行/编辑器按钮
  - 左侧项目树（250px）：项目结构导航
  - 左下资产管理（200px）：资源列表
  - 中央预览区：游戏预览窗口
  - 右侧属性面板（250px）：属性编辑器
  - 底部状态栏（30px）：FPS显示
- **动态布局**：窗口resize自动重新计算布局
- **Trace记录**：运行时记录性能trace到 `traces/trace_latest.json`

运行命令：
```bash
cd engine
cargo run --bin mono_editor_demo --features mono --release
```

### NativeAOT演示

#### `rotation_demo`
**NativeAOT高性能版本**，展示：
- 预编译C#为原生代码
- 直接函数指针调用（无JIT开销）
- 最高执行性能

运行命令：
```bash
cd engine
cargo run --bin rotation_demo --features native-aot
```

### 性能分析工具

#### `trace_viewer_demo`
**Trace泳道图查看器**，用于分析性能：
- 加载 `traces/trace_latest.json` 文件
- 显示Frame和DrawFrame的泳道图
- 时间轴可视化
- 帮助定位性能瓶颈

运行命令：
```bash
cd engine
cargo run --bin trace_viewer_demo --release
```

## 运行要求

### Mono JIT版本
需要安装Mono SDK：https://www.mono-project.com/download/stable/
- Mono编译器 `mcs` 用于编译C#脚本
- Mono运行时加载DLL

### NativeAOT版本
需要安装.NET 8 SDK：https://dotnet.microsoft.com/download
- 使用 `dotnet publish` 预编译为原生DLL

### Vulkan
需要Vulkan驱动支持，大多数现代GPU都支持。

## DFX日志系统

所有demo使用统一的DFX日志系统：
- **日志格式**：`[时间][级别][线程ID][模块] 消息`
- **日志文件**：`logs/hezhou_YYYY-MM-DD.log`
- **日志级别**：Trace(0), Debug(1), Info(2), Warn(3), Error(4), Fatal(5)

### 运行目录
必须在 `engine/` 目录下运行，否则日志路径不正确。

## 项目结构

```
hezhou/
├── engine/
│   ├── core/           # 核心引擎
│   ├── platform/       # 平台抽象层（GLFW）
│   ├── rhi/            # 渲染硬件接口抽象
│   ├── rhi-vulkan/     # Vulkan实现
│   ├── ui/             # UI系统
│   ├── scripting/      # 脚本系统（Mono/NativeAOT）
│   ├── dfx/            # DFX日志和诊断系统
│   ├── geometry/       # 几何数学库
│   ├── scripts/        # C#脚本源码
│   │   ├── RotationScript.cs
│   │   ├── EditorScript.cs
│   │   ├── UI.cs
│   │   └── DFX.cs
│   ├── examples/       # Demo程序
│   │   └── src/
│   │       ├── mono_editor_demo.rs
│   │       ├── trace_viewer_demo.rs
│   │       └── ... (其他demo)
│   ├── logs/           # 日志输出（运行时生成）
│   └── traces/         # Trace文件（运行时生成）
└── README.md
```

## 常见问题

### Mono/.NET 8不兼容
- .NET 8编译的DLL在Mono中加载时方法数为0
- **解决方案**：使用Mono编译器 `mcs` 编译Mono版本

### wrapped_mono方法查找问题
- `mono_class_get_method_from_name` 可能返回null
- **解决方案**：遍历方法列表匹配名称

### 程序卡死
- Logger文件输出使用 `unwrap()` 可能导致panic
- 已修复为错误处理模式，使用append + flush防止丢失日志

### 热重载流程
1. 用户修改C#脚本
2. 在demo窗口按 `R` 键
3. Rust调用 `build_mono.ps1` 重新编译
4. Rust调用 `executor.reload()` 卸载旧assembly，加载新的
5. Rust调用 `ResetAll()` 重新初始化静态变量
6. 新配置立即生效