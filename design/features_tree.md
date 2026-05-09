# 游戏引擎特性树 (Game Engine Feature Tree)

> 本文档以树形结构展示游戏引擎的完整特性层级，涵盖核心引擎、渲染、物理、音频、脚本、资源管理、场景管理、UI、网络、AI等模块，同时详细规划开发者工具链。

---

## 目录

| 模块 | 说明 |
|------|------|
| [1. 核心引擎](#1-核心引擎) | 运行时基础、ECS、事件系统 |
| [2. 渲染引擎](#2-渲染引擎) | 图形API、渲染管线、光照、材质、后处理、粒子 |
| [3. 物理引擎](#3-物理引擎) | 碰撞检测、刚体、布料、流体 |
| [4. 音频引擎](#4-音频引擎) | 播放、混音、音效、资源管理 |
| [5. 输入系统](#5-输入系统) | 设备支持、输入映射、处理 |
| [6. 脚本系统](#6-脚本系统) | 语言支持、运行时、引擎绑定 |
| [7. 资源管理](#7-资源管理) | 加载、序列化、打包、虚拟文件系统 |
| [8. 场景管理](#8-场景管理) | 加载、空间划分、遮挡剔除、LOD |
| [9. UI系统](#9-ui系统) | 渲染、布局、交互、动画 |
| [10. 网络系统](#10-网络系统) | 传输、同步、架构、优化 |
| [11. AI系统](#11-ai系统) | 寻路、行为树、状态机、感知 |
| [12. 开发者工具](#12-开发者工具) | 编辑器、调试、性能分析、资源工具链、构建部署 |
| [13. 平台支持](#13-平台支持) | 桌面、移动、Web、主机 |
| [14. 扩展与集成](#14-扩展与集成) | 第三方SDK、中间件、云服务 |

---

## 1. 核心引擎

```
核心引擎 (Core Engine)
├── 1.1 运行时基础 (Runtime Foundation)
│   ├── 主循环 (Main Loop)
│   │   ├── 固定时间步长更新
│   │   ├── 可变帧率渲染
│   │   └── 帧同步控制 (VSync / Adaptive)
│   ├── 内存管理 (Memory Management)
│   │   ├── 自定义分配器
│   │   │   ├── 池分配器 (Pool Allocator)
│   │   │   ├── 栈分配器 (Stack Allocator)
│   │   │   └── 帧分配器 (Frame Allocator)
│   │   ├── 垃圾回收 (Garbage Collection) [可选]
│   │   └── 内存追踪与泄漏检测
│   ├── 多线程架构 (Threading)
│   │   ├── 任务系统 (Job System)
│   │   ├── 工作线程池 (Worker Thread Pool)
│   │   └── 无锁队列 (Lock-free Queue)
│   └── 数学库 (Math Library)
│       ├── 向量 (Vec2 / Vec3 / Vec4)
│       ├── 矩阵 (Mat3 / Mat4)
│       ├── 四元数 (Quaternion)
│       ├── 碰撞检测原语
│       └── 插值函数 (Lerp / Slerp / SmoothStep)
│
├── 1.2 实体组件系统 (ECS)
│   ├── 实体管理 (Entity Manager)
│   │   ├── 实体创建与销毁
│   │   ├── 层级关系 (Parent-Child)
│   │   └── 标签与分组 (Tags & Groups)
│   ├── 组件系统 (Component System)
│   │   ├── 组件注册与反射
│   │   ├── 组件序列化
│   │   └── 组件依赖关系
│   └── 系统调度 (System Scheduler)
│       ├── 系统执行顺序
│       ├── 并行系统执行
│       └── 条件系统激活
│
└── 1.3 事件系统 (Event System)
    ├── 事件总线 (Event Bus)
    ├── 事件过滤与优先级
    └── 延迟事件队列
```

---

## 2. 渲染引擎

```
渲染引擎 (Rendering Engine)
├── 2.1 图形API抽象 (Graphics API Abstraction)
│   ├── Vulkan
│   ├── DirectX 12
│   ├── Metal
│   ├── OpenGL
│   ├── 自动回退机制
│   └── 渲染设备能力检测
│
├── 2.2 渲染管线 (Render Pipeline)
│   ├── 前向渲染 (Forward Rendering)
│   ├── 延迟渲染 (Deferred Rendering)
│   │   ├── G-Buffer
│   │   └── 延迟着色 (Deferred Shading)
│   ├── 可编程渲染管线 (Scriptable Render Pipeline)
│   └── 多Pass渲染支持
│
├── 2.3 光照系统 (Lighting)
│   ├── 基础光源
│   │   ├── 方向光 (Directional Light)
│   │   ├── 点光源 (Point Light)
│   │   └── 聚光灯 (Spot Light)
│   ├── 实时全局光照 (Real-time GI)
│   ├── 烘焙光照贴图 (Baked Lightmap)
│   └── 光照探针 (Light Probes)
│
├── 2.4 材质系统 (Material System)
│   ├── PBR材质工作流
│   │   ├── 金属度/粗糙度 (Metallic/Roughness)
│   │   └── 镜面度/光泽度 (Specular/Glossiness)
│   ├── 着色器编译与管理
│   │   ├── HLSL / GLSL / MSL 支持
│   │   ├── 运行时着色器编译
│   │   └── 着色器变体管理
│   └── 材质实例化
│
├── 2.5 后处理 (Post-Processing)
│   ├── Bloom / HDR
│   ├── 色调映射 (Tone Mapping)
│   │   ├── ACES
│   │   ├── Reinhard
│   │   └── Filmic
│   ├── 景深 (DOF)
│   ├── 运动模糊 (Motion Blur)
│   ├── 抗锯齿
│   │   ├── TAA
│   │   ├── FXAA
│   │   └── MSAA
│   └── 体积光/雾 (Volumetric Light/Fog)
│
├── 2.6 粒子系统 (Particle System)
│   ├── GPU粒子 (Compute Shader)
│   ├── 粒子发射器
│   │   ├── 点发射器
│   │   ├── 锥发射器
│   │   ├── 盒发射器
│   │   └── 网格发射器
│   ├── 粒子碰撞
│   └── 粒子力场
│       ├── 重力
│       ├── 风力
│       └── 湍流
│
├── 2.7 动画渲染 (Animation Rendering)
│   ├── 骨骼动画 (Skeleton Animation)
│   ├── 蒙皮计算 (Skinning)
│   │   ├── CPU蒙皮
│   │   └── GPU蒙皮
│   ├── 动画混合 (Animation Blending)
│   │   ├── 叠加混合
│   │   ├── 过渡混合
│   │   └── 层级混合
│   └── 形态键 (Morph Targets)
│
└── 2.8 地形与植被 (Terrain & Vegetation)
    ├── 高度图地形
    ├── 地形LOD
    ├── 植被实例化渲染
    └── 草地渲染
```

---

## 3. 物理引擎

```
物理引擎 (Physics Engine)
├── 3.1 碰撞检测 (Collision Detection)
│   ├── 碰撞体类型
│   │   ├── 盒体 (Box)
│   │   ├── 球体 (Sphere)
│   │   ├── 胶囊体 (Capsule)
│   │   ├── 凸包 (Convex Hull)
│   │   ├── 网格碰撞体 (Mesh Collider)
│   │   └── 复合碰撞体 (Compound)
│   ├── 碰撞层与过滤
│   └── 射线检测
│       ├── 射线 (Raycast)
│       ├── 盒扫 (BoxCast)
│       └── 球扫 (SphereCast)
│
├── 3.2 刚体动力学 (Rigid Body Dynamics)
│   ├── 质量与惯性张量
│   ├── 力与扭矩
│   │   ├── 施加力 (Force)
│   │   ├── 冲量 (Impulse)
│   │   └── 扭矩 (Torque)
│   ├── 约束与关节
│   │   ├── 铰链关节 (Hinge)
│   │   ├── 滑动关节 (Slider)
│   │   ├── 弹簧关节 (Spring)
│   │   └── 固定关节 (Fixed)
│   └── 睡眠优化 (Sleep)
│
├── 3.3 物理材质 (Physics Material)
│   ├── 摩擦力 (Friction)
│   └── 弹性系数 (Bounciness)
│
├── 3.4 触发器 (Triggers)
│   ├── 区域检测
│   └── 重叠事件
│
├── 3.5 布料与软体 (Cloth & Soft Body)
│   ├── 布料模拟
│   └── 软体变形
│
└── 3.6 流体模拟 (Fluid Simulation)
    ├── 粒子流体
    └── 水面模拟
```

---

## 4. 音频引擎

```
音频引擎 (Audio Engine)
├── 4.1 音频播放 (Audio Playback)
│   ├── 2D音频
│   ├── 3D音频
│   │   ├── 空间定位
│   │   └── 距离衰减
│   ├── 空间音频 (Spatial Audio)
│   │   ├── HRTF
│   │   └── 环绕声
│   └── 多声道支持
│       ├── 立体声
│       ├── 5.1
│       └── 7.1
│
├── 4.2 音频混音 (Audio Mixing)
│   ├── 混音器总线
│   │   ├── 分层混音
│   │   └── 总线控制
│   ├── 音量/音调控制
│   ├── 淡入淡出
│   └── 音频效果 (Audio Effects)
│       ├── 混响 (Reverb)
│       ├── 均衡器 (EQ)
│       ├── 压缩器 (Compressor)
│       └── 滤波器 (Filter)
│
└── 4.3 音频资源管理 (Audio Resource Management)
    ├── 流式加载
    ├── 音频压缩格式
    │   ├── OGG
    │   ├── MP3
    │   └── ADPCM
    └── 内存池管理
```

---

## 5. 输入系统

```
输入系统 (Input System)
├── 5.1 设备支持 (Device Support)
│   ├── 键盘/鼠标
│   ├── 游戏手柄
│   │   ├── XInput
│   │   ├── DirectInput
│   │   └── Generic
│   ├── 触摸屏
│   │   ├── 多点触控
│   │   └── 手势识别
│   └── VR控制器
│       ├── 6DoF追踪
│       └── 扳机/按键
│
├── 5.2 输入映射 (Input Mapping)
│   ├── 动作映射 (Action Mapping)
│   ├── 轴映射 (Axis Mapping)
│   ├── 输入组合键
│   └── 输入重绑定 (Rebinding)
│
└── 5.3 输入处理 (Input Processing)
    ├── 输入缓冲
    ├── 输入消抖
    └── 输入回放 (用于调试/录像)
```

---

## 6. 脚本系统

```
脚本系统 (Scripting System)
├── 6.1 脚本语言支持 (Language Support)
│   ├── C#
│   ├── Lua
│   ├── Python
│   ├── JavaScript
│   ├── 原生C++ API
│   └── 热重载支持
│
├── 6.2 脚本运行时 (Script Runtime)
│   ├── 脚本编译与执行
│   │   ├── JIT编译
│   │   └── AOT编译
│   ├── 脚本调试接口
│   └── 脚本沙箱
│
└── 6.3 引擎绑定 (Engine Bindings)
    ├── 自动绑定生成
    ├── 反射集成
    └── 跨语言调用
```

---

## 7. 资源管理

```
资源管理 (Asset Management)
├── 7.1 资源加载 (Asset Loading)
│   ├── 同步加载
│   ├── 异步加载
│   ├── 依赖管理
│   └── 引用计数
│
├── 7.2 资源序列化 (Asset Serialization)
│   ├── 二进制格式
│   ├── 文本格式 (JSON/YAML)
│   ├── 版本兼容
│   └── 增量更新
│
├── 7.3 资源打包 (Asset Bundling)
│   ├── 资源分组
│   ├── 压缩与加密
│   │   ├── LZ4/ZSTD压缩
│   │   └── AES加密
│   └── 热更新支持
│
└── 7.4 虚拟文件系统 (Virtual File System)
    ├── 多源挂载
    │   ├── 本地文件
    │   ├── 网络文件
    │   ├── 内存文件
    │   └── 打包文件
    ├── 路径映射
    └── 缓存管理
        ├── LRU
        └── LFU
```

---

## 8. 场景管理

```
场景管理 (Scene Management)
├── 8.1 场景加载 (Scene Loading)
│   ├── 场景切换
│   ├── 异步场景加载
│   └── 流式加载 (开放世界)
│
├── 8.2 空间划分 (Spatial Partitioning)
│   ├── 四叉树 (QuadTree) [2D]
│   ├── 八叉树 (Octree) [3D]
│   ├── BVH (层次包围盒)
│   └── 网格划分 (Grid)
│
├── 8.3 遮挡剔除 (Occlusion Culling)
│   ├── 硬件遮挡查询 (GPU)
│   ├── 软件遮挡剔除 (CPU)
│   └── 预计算遮挡数据
│
└── 8.4 LOD系统 (Level of Detail)
    ├── 网格LOD
    ├── 动画LOD
    └── 自动LOD生成
```

---

## 9. UI系统

```
UI系统 (UI System)
├── 9.1 UI渲染 (UI Rendering)
│   ├── 2D UI (屏幕空间)
│   ├── 3D UI (世界空间)
│   ├── 矢量图形
│   └── 富文本
│
├── 9.2 UI布局 (UI Layout)
│   ├── 锚点与对齐
│   ├── 自动布局
│   │   ├── Flex布局
│   │   └── Grid布局
│   └── 响应式设计
│
├── 9.3 UI交互 (UI Interaction)
│   ├── 事件冒泡
│   ├── 焦点管理
│   └── 拖拽系统
│
└── 9.4 UI动画 (UI Animation)
    ├── 过渡动画
    ├── 状态机动画
    └── 曲线动画 (缓动)
```

---

## 10. 网络系统

```
网络系统 (Networking)
├── 10.1 网络传输 (Network Transport)
│   ├── TCP
│   ├── UDP
│   ├── WebSocket
│   └── 可靠UDP (自定义)
│
├── 10.2 网络同步 (Network Synchronization)
│   ├── 状态同步 (服务器权威)
│   ├── 帧同步 (确定性模拟)
│   ├── 客户端预测
│   └── 服务器调和 (Reconciliation)
│
├── 10.3 网络架构 (Network Architecture)
│   ├── 客户端-服务器 (C/S)
│   ├── P2P
│   └── 房间系统
│       ├── 匹配
│       └── 房间管理
│
└── 10.4 网络优化 (Network Optimization)
    ├── 数据压缩
    │   ├── 增量压缩
    │   └── 位打包
    ├── 带宽管理
    │   ├── 优先级队列
    │   └── 限流
    └── 延迟补偿
        ├── 回滚 (Rollback)
        └── 插值 (Interpolation)
```

---

## 11. AI系统

```
AI系统 (AI System)
├── 11.1 寻路 (Pathfinding)
│   ├── A*算法
│   ├── 导航网格 (NavMesh)
│   └── 动态避障
│       └── RVO (Reciprocal Velocity Obstacles)
│
├── 11.2 行为树 (Behavior Tree)
│   ├── 行为节点 (Action)
│   ├── 装饰器节点 (Decorator)
│   │   ├── 条件判断
│   │   ├── 循环
│   │   └── 取反
│   └── 组合节点 (Composite)
│       ├── Sequence
│       ├── Selector
│       └── Parallel
│
├── 11.3 状态机 (State Machine)
│   ├── 有限状态机 (FSM)
│   ├── 分层状态机 (HSM)
│   └── 模糊状态机
│
└── 11.4 感知系统 (Perception System)
    ├── 视觉感知
    │   ├── 视野锥
    │   └── 视线检测
    ├── 听觉感知
    │   ├── 声音传播
    │   └── 听力范围
    └── 记忆系统
        └── 感知信息存储与衰减
```

---

## 12. 开发者工具

```
开发者工具 (Developer Tools)
│
├── 12.1 游戏编辑器 (Game Editor)
│   │
│   ├── 12.1.1 场景编辑器 (Scene Editor)
│   │   ├── 可视化场景编辑
│   │   ├── 实体选择与变换
│   │   ├── 多视图支持
│   │   │   ├── 透视视图
│   │   │   ├── 正交视图
│   │   │   ├── 顶视图
│   │   │   ├── 前视图
│   │   │   └── 侧视图
│   │   ├── 网格捕捉与对齐
│   │   └── 撤销/重做系统
│   │
│   ├── 12.1.2 材质编辑器 (Material Editor)
│   │   ├── 节点式材质编辑
│   │   ├── 实时预览
│   │   ├── 材质参数暴露
│   │   └── 材质模板库
│   │
│   ├── 12.1.3 动画编辑器 (Animation Editor)
│   │   ├── 时间轴编辑
│   │   ├── 关键帧编辑
│   │   ├── 动画曲线编辑
│   │   ├── 动画状态机编辑
│   │   └── 骨骼蒙皮预览
│   │
│   ├── 12.1.4 UI编辑器 (UI Editor)
│   │   ├── 所见即所得编辑
│   │   ├── 组件拖拽
│   │   ├── 样式预览
│   │   └── 响应式预览
│   │
│   └── 12.1.5 地形编辑器 (Terrain Editor)
│       ├── 高度绘制
│       ├── 纹理绘制
│       ├── 植被放置
│       └── 地形雕刻
│
├── 12.2 脚本开发工具 (Scripting Tools)
│   │
│   ├── 12.2.1 代码编辑器集成
│   │   ├── VS Code 插件
│   │   ├── Visual Studio 插件
│   │   ├── JetBrains Rider 插件
│   │   └── 功能
│   │       ├── 智能代码补全
│   │       ├── 语法高亮
│   │       └── 代码导航
│   │
│   └── 12.2.2 脚本调试器 (Script Debugger)
│       ├── 断点调试
│       │   ├── 条件断点
│       │   └── 日志断点
│       ├── 变量监视
│       ├── 调用栈查看
│       └── 热重载
│
├── 12.3 调试工具 (Debugging Tools)
│   │
│   ├── 12.3.1 运行时调试 (Runtime Debugging)
│   │   ├── 控制台 (Console)
│   │   │   ├── 命令注册与执行
│   │   │   ├── 日志过滤
│   │   │   └── 命令历史
│   │   └── 调试覆盖层 (Debug Overlay)
│   │       ├── FPS显示
│   │       ├── 内存使用
│   │       ├── 绘制调用 (DrawCall)
│   │       └── 实体信息
│   │
│   ├── 12.3.2 可视化调试 (Visual Debugging)
│   │   ├── 碰撞体可视化
│   │   ├── 物理射线可视化
│   │   ├── 导航网格可视化
│   │   ├── 光照探针可视化
│   │   └── AI路径可视化
│   │
│   └── 12.3.3 日志系统 (Logging System)
│       ├── 多级别日志
│       │   ├── Debug (灰色)
│       │   ├── Info (白色)
│       │   ├── Warning (黄色)
│       │   ├── Error (红色)
│       │   └── Fatal (红色+弹窗)
│       ├── 日志分类过滤
│       ├── 日志输出
│       │   ├── 文件
│       │   ├── 控制台
│       │   └── 远程服务器
│       └── 结构化日志 (JSON)
│
├── 12.4 性能分析工具 (Profiling Tools)
│   │
│   ├── 12.4.1 CPU性能分析 (CPU Profiler)
│   │   ├── 函数调用追踪
│   │   ├── 时间线视图
│   │   ├── 热点分析
│   │   └── 线程活动视图
│   │
│   ├── 12.4.2 GPU性能分析 (GPU Profiler)
│   │   ├── 渲染Pass分析
│   │   ├── 着色器性能
│   │   ├── 显存使用
│   │   └── 绘制调用分析
│   │
│   ├── 12.4.3 内存分析 (Memory Profiler)
│   │   ├── 内存分配追踪
│   │   ├── 内存泄漏检测
│   │   ├── 内存快照对比
│   │   └── 对象引用图
│   │
│   └── 12.4.4 网络分析 (Network Profiler)
│       ├── 带宽监控
│       ├── 延迟监控
│       ├── 数据包分析
│       └── 同步状态监控
│
├── 12.5 资源工具链 (Asset Pipeline Tools)
│   │
│   ├── 12.5.1 资源导入器 (Asset Importer)
│   │   ├── 模型导入
│   │   │   ├── FBX
│   │   │   ├── OBJ
│   │   │   └── glTF
│   │   ├── 纹理导入
│   │   │   ├── PNG
│   │   │   ├── JPG
│   │   │   ├── TGA
│   │   │   └── PSD
│   │   ├── 音频导入
│   │   │   ├── WAV
│   │   │   ├── MP3
│   │   │   └── OGG
│   │   ├── 自动格式转换
│   │   └── 导入设置预设
│   │
│   ├── 12.5.2 资源浏览器 (Asset Browser)
│   │   ├── 资源预览
│   │   ├── 资源搜索与过滤
│   │   ├── 资源依赖查看
│   │   └── 批量操作
│   │
│   └── 12.5.3 资源打包工具 (Asset Packaging Tool)
│       ├── 资源分组配置
│       ├── 压缩设置
│       ├── 增量打包
│       └── 热更新包生成
│
├── 12.6 构建与部署工具 (Build & Deployment Tools)
│   │
│   ├── 12.6.1 构建系统 (Build System)
│   │   ├── 多平台构建
│   │   │   ├── Windows (x64, ARM64)
│   │   │   ├── macOS (Intel, Apple Silicon)
│   │   │   ├── Linux (x64, ARM64)
│   │   │   ├── iOS (ARM64)
│   │   │   ├── Android (ARM64, x86_64)
│   │   │   ├── Web (WebAssembly)
│   │   │   └── 主机平台 (PS/Xbox/Switch)
│   │   ├── 构建配置管理
│   │   │   ├── Debug
│   │   │   ├── Release
│   │   │   └── Development
│   │   └── 自动化构建脚本 (CI/CD)
│   │
│   └── 12.6.2 部署工具 (Deployment Tool)
│       ├── 一键部署
│       ├── 远程调试
│       └── 日志收集
│
├── 12.7 版本控制集成 (Version Control Integration)
│   │
│   ├── 12.7.1 VCS支持
│   │   ├── Git 集成
│   │   ├── Perforce 集成
│   │   └── SVN 集成
│   │
│   └── 12.7.2 协作功能
│       ├── 场景锁定
│       ├── 变更合并
│       └── 协作编辑 (可选)
│
├── 12.8 测试工具 (Testing Tools)
│   │
│   ├── 12.8.1 单元测试框架 (Unit Testing)
│   │   ├── 测试用例编写
│   │   ├── 测试运行器
│   │   └── 测试覆盖率报告
│   │
│   ├── 12.8.2 自动化测试 (Automated Testing)
│   │   ├── 输入录制与回放
│   │   ├── 场景自动化测试
│   │   └── 性能回归测试
│   │
│   └── 12.8.3 截图对比测试 (Screenshot Comparison)
│       ├── 渲染回归测试
│       └── 差异可视化
│
├── 12.9 插件系统 (Plugin System)
│   │
│   ├── 12.9.1 编辑器插件 (Editor Plugins)
│   │   ├── 插件API
│   │   ├── 自定义面板
│   │   ├── 菜单扩展
│   │   └── 快捷键自定义
│   │
│   └── 12.9.2 运行时插件 (Runtime Plugins)
│       ├── 动态库加载
│       ├── 插件生命周期
│       └── 插件依赖管理
│
└── 12.10 文档与帮助 (Documentation & Help)
    ├── 12.10.1 内置文档
    │   ├── API文档浏览器
    │   ├── 教程与指南
    │   └── 上下文敏感帮助
    │
    └── 12.10.2 代码文档生成
        ├── 注释解析
        ├── 文档生成
        └── 文档预览
```

---

## 13. 平台支持

```
平台支持 (Platform Support)
├── 13.1 桌面平台 (Desktop)
│   ├── Windows (7/8/10/11)
│   ├── macOS (10.15+)
│   └── Linux (Ubuntu/Fedora/Arch)
│
├── 13.2 移动平台 (Mobile)
│   ├── iOS (12+)
│   └── Android (8.0+)
│
├── 13.3 Web平台 (Web)
│   ├── WebGL
│   ├── WebGPU
│   └── WebAssembly
│
└── 13.4 主机平台 (Console) [可选]
    ├── PlayStation
    ├── Xbox
    └── Nintendo Switch
```

---

## 14. 扩展与集成

```
扩展与集成 (Extensions & Integrations)
├── 14.1 第三方SDK集成 (Third-party SDK)
│   ├── 广告SDK (AdMob, Unity Ads)
│   ├── 分析SDK (Firebase, GameAnalytics)
│   ├── 社交SDK (Steam, Epic, PSN)
│   └── 支付SDK (IAP, Steam Microtransactions)
│
├── 14.2 中间件支持 (Middleware)
│   ├── 物理引擎替换
│   │   ├── PhysX
│   │   ├── Havok
│   │   └── Box2D
│   ├── 音频中间件
│   │   ├── FMOD
│   │   └── Wwise
│   └── 动画中间件
│       ├── Spine
│       └── Live2D
│
└── 14.3 云服务集成 (Cloud Services)
    ├── 多人游戏服务
    ├── 云存档
    ├── 排行榜
    └── 成就系统
```

---

## 附录：优先级说明

| 优先级 | 含义 | 阶段 |
|:------:|------|------|
| **P0** | 核心必备，MVP阶段必须实现 | Phase 1 |
| **P1** | 重要功能，正式版需要 | Phase 2 |
| **P2** | 增强功能，提升体验 | Phase 3 |
| **P3** | 锦上添花，可选实现 | Phase 4 |

---

*文档版本: 1.0 | 最后更新: 2026-05-09*
