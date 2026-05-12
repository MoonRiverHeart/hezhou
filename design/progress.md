# 引擎开发进度

> 记录各模块完成状态和测试结果

---

## Phase 1 MVP - ✅ 完成

### 模块状态

| 模块 | 路径 | 状态 | 测试 |
|------|------|------|------|
| **scripting** | `engine/scripting` | ✅ | 15 测试通过 |
| **core** | `engine/core` | ✅ | 编译通过 |
| **render** | `engine/render` | ✅ | 编译通过 |
| **harmony** | `engine/harmony` | ✅ | 编译通过 |
| **platform** | `engine/platform` | ✅ | GLFW Demo 运行成功 |
| **dfx** | `engine/dfx` | ✅ | 26 测试通过 |
| **examples** | `engine/examples` | ✅ | 2 Demo 运行成功 |

---

## 已实现功能

### 1. 脚本系统 (Scripting)

**文件**: `engine/scripting/src/lib.rs`

**FFI 导出**:
- `scripting_init()` - 初始化脚本管理器
- `scripting_shutdown()` - 关闭脚本管理器
- `scripting_register_sync_callback()` - 注册同步回调
- `scripting_register_async_callback()` - 注册异步回调
- `scripting_register_task_callback()` - 注册任务回调
- `scripting_trigger_sync()` - 触发同步回调
- `scripting_notify_completion()` - 通知完成
- `scripting_notify_progress()` - 通知进度

**特性**:
- ScriptValue 固定结构体 (避免 GC)
- Closure 支持 (GCHandle context)
- Result<T> 通过 error_flag 返回
- Sync/Async/Task 三种回调类型

**测试**:
- `registry_test.rs` - 6 测试
- `ffi_test.rs` - 5 测试
- `standalone_test.rs` - 4 测试

---

### 2. 核心引擎 (Core)

**文件**: `engine/core/src/ffi.rs`

**FFI 导出**:
- 引擎生命周期: `engine_create/destroy/start/stop/run_frame`
- 时间: `engine_get_time/delta_time/frame_count`
- World: `engine_get_world`
- EventBus: `engine_get_event_bus`
- ECS 实体: `ecs_create_entity/destroy_entity/entity_exists`
- ECS 层级: `ecs_set/get_entity_parent/get_entity_children`
- TransformComponent: `ecs_add/get/set_transform_*`
- NameComponent: `ecs_add/get_name_component`
- TagComponent: `ecs_add/get_tag_component`
- 事件: `event_bus_subscribe/publish/dispatch`

**子模块**:
- `math/` - Vec2/Vec3/Vec4, Quaternion, Mat4, Transform
- `ecs/` - Entity, Component, World, System
- `event/` - EventType, Event, EventBus
- `time_loop/` - Time, MainLoop

---

### 3. 渲染引擎 (Render)

**文件**: `engine/render/src/ffi.rs`

**FFI 导出**:
- `render_engine_create/destroy`
- `render_init_surface/resize/begin_frame/end_frame`
- `render_set_clear_color`
- `render_create_camera`
- `renderer_create/destroy`
- `mesh_create_triangle/quad`
- `texture_create`
- `material_create`

**子模块**:
- `surface.rs` - RenderSurface (OH_NativeWindow)
- `camera.rs` - Camera (position, rotation, fov, matrices)
- `renderer.rs` - Renderer
- `mesh.rs` - Mesh, Vertex
- `material.rs` - Material
- `texture.rs` - Texture, TextureFormat
- `color.rs` - Color

---

### 4. HarmonyOS 对接层

**文件**: `engine/harmony/src/lib.rs`

**FFI 导出**:
- `harmony_engine_init/shutdown`
- `harmony_get_window_context`
- `harmony_get_event_bus`
- `harmony_register_event_callback`

**事件类型**:
- `TouchEvent` - action, x, y, pointer_id
- `KeyEvent` - action, keycode, modifiers
- `SizeEvent` - width, height
- `LifecycleEvent` - Create/Start/Resume/Pause/Stop/Destroy

**Native 代码**:
- `native/hezhou_native.cpp` - NAPI 模块
- `native/include/hezhou_native.h` - C 头文件

---

### 5. 平台抽象层 (Platform)

**文件**: `engine/platform/src/lib.rs`

**FFI 导出**:
- `platform_manager_create/destroy`
- `platform_init_glfw`
- `platform_init_harmony`
- `platform_create_window`
- `platform_poll_events`
- `platform_is_running`
- `platform_get_time`
- `platform_get_window_handle`

**Platform Trait**:
- `init/shutdown`
- `create_window/destroy_window`
- `poll_events/wait_events`
- `get_time/sleep`
- `is_running/request_quit`

**后端**:
- `GLFWPlatform` - Windows/Linux/macOS
- `HarmonyPlatform` - HarmonyOS

---

### 6. DFX 诊断框架

**文件**: `engine/dfx/src/lib.rs`

**FFI 导出**:
- `dfx_create/destroy` - 系统生命周期
- `dfx_enable_all/disable_all` - 启用/禁用子系统
- `dfx_set_log_level/log` - 日志系统
- `dfx_enable_crash_handler` - 崩溃栈抓取
- `dfx_capture_stack_trace` - 手动栈抓取
- `dfx_enable_trace/trace_begin/trace_end` - 点位分析
- `dfx_save_trace` - 导出 Chrome Trace Format
- `dfx_enable_perf_monitor/perf_begin_frame/perf_end_frame` - 性能监视
- `dfx_get_fps/get_frame_count/get_perf_snapshot` - 性能数据获取

**子模块**:
- `log_types.rs` - LogLevel, LogEntry 定义
- `logger.rs` - Logger (分级、缓冲、多输出)
- `crash.rs` - CrashHandler (Panic Hook, Signal Handler, Backtrace)
- `trace.rs` - TraceAnalyzer (TracePoint, CounterPoint, ScopedTrace)
- `perf.rs` - PerformanceMonitor (FPS, Memory, CPU)

**特性**:
- 日志分级: Trace/Debug/Info/Warn/Error/Fatal
- 日志输出: 控制台、文件、缓冲区、回调
- 崩溃栈抓取: 自动 Panic Hook + Unix Signal Handler
- 点位分析: Trace Point + Counter 系统 (Chrome Trace Format 导出)
- 性能监视: FPS、Frame Time、Memory Used、CPU Usage
- 调用栈抓取: 手动调用 `dfx_capture_stack_trace()`

**测试**:
- `dfx_test.rs` - 26 测试通过

---

## 测试结果

```
scripting:  15 测试通过 ✅
dfx:        26 测试通过 ✅
core:       编译通过 ✅
render:     编译通过 ✅
harmony:    编译通过 ✅
platform:   编译通过 ✅
examples:   2 Demo 运行成功 ✅
```

### 运行测试
```bash
cd engine
cargo test
```

### 运行 Demo
```bash
# 基础引擎 Demo
cargo run -p hezhou-examples --bin engine_demo

# GLFW 平台 Demo
cargo run -p hezhou-examples --bin glfw_demo

# 脚本系统集成测试
cargo run --example integration_test -p hezhou-scripting
```

### 构建 DLL
```bash
cargo build --release -p hezhou-scripting
cargo build --release -p hezhou-harmony
```

---

## Demo 输出示例

### engine_demo
```
=== Hezhou Game Engine Demo ===

[1] 创建引擎实例... Engine ptr: 0x160b8292d50
[2] 启动引擎... Engine running: true
[3] 创建实体... Entity 1/2/3
[4] 添加 TransformComponent...
[5] 运行帧循环... Frame 1-5
[6] 测试脚本系统... 10 * 5 = 50
=== Demo Complete ===
```

### glfw_demo
```
=== Hezhou Engine - GLFW Platform Demo ===

[1] 初始化 GLFW 平台... GLFW 初始化成功!
[2] 创建窗口... type=GLFW, 800x600
[3] 初始化引擎... Engine started
[4] 初始化脚本系统...
[5] 主循环... 100 frames
=== Demo Complete ===
```

---

## 下一步 (Phase 2)

### 待实现模块

| 模块 | 功能 | 预计时间 |
|------|------|----------|
| **physics** | 碰撞检测、刚体、射线 | 1 周 |
| **audio** | 播放、混音、空间音频 | 1 周 |
| **asset** | 加载、VFS、打包 | 1 周 |
| **scene** | 场景管理、层级 | 1 周 |
| **C# 绑定** | Engine API 封装 | 1 周 |

---

## 关键技术决策

| 决策 | 原因 |
|------|------|
| ScriptValue 固定结构体 | 避免 GC 压力 |
| Context 参数 (usize) | 支持 Closure |
| error_flag 返回错误 | FFI 无异常 |
| Platform trait | 统一多平台接口 |
| Feature flags | 编译时选择后端 |
| crate-type rlib + cdylib | 同时支持 Rust 内部调用和 FFI |

---

*文档版本: 1.0 | 创建时间: 2026-05-12*