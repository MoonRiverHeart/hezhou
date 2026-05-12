# DFX 诊断框架设计

> 开发者体验框架 (Developer Framework for X)，包含日志、崩溃栈抓取、点位分析、性能监视器

---

## 1. 设计目标

### 1.1 问题背景

游戏引擎开发需要完善的诊断工具：
- **日志系统**: 追踪运行状态、调试问题
- **崩溃栈抓取**: 定位崩溃原因、自动生成报告
- **点位分析**: 性能热点分析、函数耗时追踪
- **性能监视器**: FPS、内存、CPU 实时监控

### 1.2 解决方案

构建统一的 DFX 系统 (`engine/dfx`)：
- 分级日志 + 多输出
- 自动崩溃栈抓取 + Signal Handler
- Trace Point + Counter 系统 (Chrome Trace Format)
- 性能 Snapshot + 统计分析

---

## 2. 模块架构

```
engine/dfx/
├── Cargo.toml
├── build.rs
├── src/
│   ├── lib.rs          ← DfxSystem + FFI 导出
│   ├── log_types.rs    ← 日志类型定义
│   ├── logger.rs       ← Logger 实现
│   ├── crash.rs        ← CrashHandler + StackTrace
│   ├── trace.rs        ← TraceAnalyzer + ScopedTrace
│   └── perf.rs         ← PerformanceMonitor
├── tests/
│   └── dfx_test.rs     ← 测试用例
```

---

## 3. 日志系统 (Logger)

### 3.1 日志级别

```rust
#[repr(C)]
pub enum LogLevel {
    Trace = 0,   // 详细追踪
    Debug = 1,   // 调试信息
    Info = 2,    // 一般信息
    Warn = 3,    // 警告
    Error = 4,   // 错误
    Fatal = 5,   // 致命错误
}
```

### 3.2 LogEntry 结构

```rust
#[repr(C)]
pub struct LogEntry {
    pub level: LogLevel,
    pub timestamp: u64,
    pub thread_id: u64,
    pub module: *const c_char,
    pub message: *const c_char,
    pub file: *const c_char,
    pub line: u32,
}
```

### 3.3 Logger 功能

| 功能 | 说明 |
|------|------|
| `set_level` | 设置最低日志级别 |
| `set_buffer_size` | 缓冲区大小 (环形) |
| `enable_file_output` | 输出到文件 |
| `enable_console_output` | 输出到控制台 |
| `register_callback` | 注册自定义回调 |
| `get_buffer` | 获取缓冲区日志 |
| `clear_buffer` | 清空缓冲区 |

### 3.4 日志格式

**控制台**:
```
[2026-05-12 15:30:45.123][INFO][RenderEngine] Initializing renderer
```

**文件**:
```
[2026-05-12 15:30:45.123][INFO][T:1234][RenderEngine][renderer.rs:45] Initializing renderer
```

---

## 4. 崩溃栈抓取 (CrashHandler)

### 4.1 CrashReport 结构

```rust
#[repr(C)]
pub struct CrashReport {
    pub crash_type: CrashType,
    pub timestamp: u64,
    pub message: *const c_char,
    pub frames: *mut StackFrame,
    pub frame_count: u32,
}

#[repr(C)]
pub struct StackFrame {
    pub address: u64,
    pub symbol: *const c_char,
    pub file: *const c_char,
    pub line: u32,
}
```

### 4.2 CrashType 类型

```rust
#[repr(C)]
pub enum CrashType {
    Panic = 0,        // Rust panic
    Segfault = 1,     // 段错误
    StackOverflow = 2,
    NullPointer = 3,
    OutOfMemory = 4,
    Custom = 5,
}
```

### 4.3 崩溃处理流程

1. **Panic Handler**: `std::panic::set_hook`
2. **Signal Handler** (Unix): `SIGSEGV`, `SIGBUS`, `SIGFPE`, `SIGILL`
3. **Backtrace**: 使用 `backtrace` crate
4. **生成报告**: `crash_report.txt`

### 4.4 崩溃报告示例

```
=== Crash Report ===
Time: 2026-05-12 15:30:45
Type: Segfault
Message: Access violation at 0x00000000

Stack Trace:
#0 main::render_frame @ src\renderer.rs:123 (0x140001234)
#1 Engine::run_frame @ src\engine.rs:45 (0x140003456)
#2 std::sys_common::backtrace::__rust_begin_short_backtrace @ :0 (0x140005678)
...
```

---

## 5. 点位分析系统 (TraceAnalyzer)

### 5.1 TracePoint 结构

```rust
#[repr(C)]
pub struct TracePoint {
    pub name: *const c_char,
    pub category: *const c_char,
    pub start_time: u64,
    pub end_time: u64,
    pub duration_ns: u64,
    pub thread_id: u64,
}
```

### 5.2 CounterPoint 结构

```rust
#[repr(C)]
pub struct CounterPoint {
    pub name: *const c_char,
    pub category: *const c_char,
    pub value: i64,
    pub timestamp: u64,
}
```

### 5.3 使用方式

**手动追踪**:
```rust
analyzer.begin_point("render_frame", "render");
// ... 执行代码 ...
analyzer.end_point("render_frame", "render");
```

**自动追踪 (Scoped)**:
```rust
let trace = ScopedTrace::new(analyzer, "update_entities", "ecs");
// ... 执行代码 ...
// Drop 时自动 end_point
```

**计数器**:
```rust
analyzer.set_counter("fps", "performance", 60);
analyzer.increment_counter("draw_calls", "render");
```

### 5.4 Chrome Trace Format 导出

```json
{
  "traceEvents": [
    {"name":"render_frame","cat":"render","ph":"X","ts":12345,"dur":5000,"tid":1},
    {"name":"fps","cat":"performance","ph":"C","ts":12345,"value":60}
  ]
}
```

可直接在 Chrome `chrome://tracing` 中可视化分析。

---

## 6. 性能监视器 (PerformanceMonitor)

### 6.1 PerformanceSnapshot 结构

```rust
#[repr(C)]
pub struct PerformanceSnapshot {
    pub timestamp: u64,
    pub fps: f32,
    pub frame_time_ms: f32,
    pub cpu_usage_percent: f32,
    pub memory_used_mb: f32,
    pub memory_available_mb: f32,
    pub draw_calls: u32,
    pub triangle_count: u32,
}
```

### 6.2 监控指标

| 指标 | 说明 | 来源 |
|------|------|------|
| FPS | 每秒帧数 | 计数统计 |
| Frame Time | 帧耗时 (ms) | begin/end_frame |
| CPU Usage | CPU 使用率 (%) | GetSystemTimes (Win) |
| Memory Used | 已用内存 (MB) | GetProcessMemoryInfo (Win) |

### 6.3 使用方式

```rust
monitor.enable();

loop {
    monitor.begin_frame();
    engine.run_frame(delta);
    monitor.end_frame();
    
    if monitor.get_fps() < 30.0 {
        logger.warn("Performance", "Low FPS detected");
    }
}
```

### 6.4 统计分析

```rust
let avg_fps = monitor.get_average_fps();
let avg_frame_time = monitor.get_average_frame_time();
let avg_memory = monitor.get_average_memory();
```

---

## 7. DfxSystem 统一入口

### 7.1 结构

```rust
pub struct DfxSystem {
    logger: Arc<Mutex<Logger>>,
    crash_handler: Arc<Mutex<CrashHandler>>,
    trace_analyzer: Arc<Mutex<TraceAnalyzer>>,
    perf_monitor: Arc<Mutex<PerformanceMonitor>>,
}
```

### 7.2 方法

| 方法 | 说明 |
|------|------|
| `new` | 创建 DFX 系统 |
| `enable_all` | 启用所有子系统 |
| `disable_all` | 禁用所有子系统 |
| `get_logger` | 获取 Logger |
| `get_crash_handler` | 获取 CrashHandler |
| `get_trace_analyzer` | 获取 TraceAnalyzer |
| `get_perf_monitor` | 获取 PerformanceMonitor |

---

## 8. FFI 导出接口

```rust
// 系统生命周期
#[unsafe(no_mangle)]
pub extern "C" fn dfx_create() -> *mut DfxSystem;

#[unsafe(no_mangle)]
pub extern "C" fn dfx_destroy(system: *mut DfxSystem);

// 日志系统
#[unsafe(no_mangle)]
pub extern "C" fn dfx_set_log_level(system: *mut DfxSystem, level: u8);

#[unsafe(no_mangle)]
pub extern "C" fn dfx_log(system, level, module, message, file, line);

#[unsafe(no_mangle)]
pub extern "C" fn dfx_get_log_buffer_count(system) -> u32;

#[unsafe(no_mangle)]
pub extern "C" fn dfx_clear_log_buffer(system);

// 崩溃栈抓取
#[unsafe(no_mangle)]
pub extern "C" fn dfx_enable_crash_handler(system);

#[unsafe(no_mangle)]
pub extern "C" fn dfx_capture_stack_trace() -> *mut StackFrame;

#[unsafe(no_mangle)]
pub extern "C" fn dfx_free_stack_trace(frames, count);

// 点位分析
#[unsafe(no_mangle)]
pub extern "C" fn dfx_enable_trace(system);

#[unsafe(no_mangle)]
pub extern "C" fn dfx_trace_begin(system, name, category);

#[unsafe(no_mangle)]
pub extern "C" fn dfx_trace_end(system, name, category);

#[unsafe(no_mangle)]
pub extern "C" fn dfx_save_trace(system, path) -> i32;

#[unsafe(no_mangle)]
pub extern "C" fn dfx_clear_trace(system);

// 性能监视
#[unsafe(no_mangle)]
pub extern "C" fn dfx_enable_perf_monitor(system);

#[unsafe(no_mangle)]
pub extern "C" fn dfx_perf_begin_frame(system);

#[unsafe(no_mangle)]
pub extern "C" fn dfx_perf_end_frame(system);

#[unsafe(no_mangle)]
pub extern "C" fn dfx_get_fps(system) -> f32;

#[unsafe(no_mangle)]
pub extern "C" fn dfx_get_frame_count(system) -> u64;

#[unsafe(no_mangle)]
pub extern "C" fn dfx_get_perf_snapshot(system) -> PerformanceSnapshot;

#[unsafe(no_mangle)]
pub extern "C" fn dfx_clear_perf(system);
```

---

## 9. Cargo.toml 配置

```toml
[package]
name = "hezhou-dfx"
version = "0.1.0"
edition = "2021"

[lib]
name = "hezhou_dfx"
crate-type = ["cdylib", "rlib"]

[dependencies]
hezhou-scripting = { path = "../scripting" }
parking_lot = "0.12"
chrono = "0.4"
backtrace = "0.3"

[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["processthreadsapi", "psapi", "memoryapi"] }

[target.'cfg(unix)'.dependencies]
libc = "0.2"

[[test]]
name = "dfx_test"
path = "tests/dfx_test.rs"
```

---

## 10. 测试用例清单

### 10.1 日志系统测试

| 测试 | 说明 |
|------|------|
| `test_logger_levels` | 日志级别设置与获取 |
| `test_log_output` | 各级别日志输出 |
| `test_log_filtering` | 日志过滤验证 |
| `test_log_buffer_size` | 缓冲区大小限制 |
| `test_log_clear_buffer` | 清空缓冲区 |
| `test_log_level_conversion` | u8 转 LogLevel |
| `test_log_level_string` | 字符串转换 |

### 10.2 点位分析测试

| 测试 | 说明 |
|------|------|
| `test_trace_analyzer_enable_disable` | 启用/禁用 |
| `test_trace_points` | Trace Point 追踪 |
| `test_trace_counters` | Counter 设置 |
| `test_trace_counter_increment` | Counter 递增 |
| `test_trace_clear` | 清空追踪数据 |
| `test_trace_export_json` | JSON 导出 |

### 10.3 性能监视测试

| 测试 | 说明 |
|------|------|
| `test_perf_monitor_enable_disable` | 启用/禁用 |
| `test_perf_monitor_frames` | 帧计数与 FPS |
| `test_perf_monitor_snapshots` | Snapshot 记录 |
| `test_perf_monitor_averages` | 统计分析 |
| `test_perf_monitor_clear` | 清空数据 |

### 10.4 DFX 系统测试

| 测试 | 说明 |
|------|------|
| `test_dfx_create_destroy` | 系统创建/销毁 |
| `test_dfx_system_enable_all` | 全部启用 |
| `test_dfx_system_disable_all` | 全部禁用 |
| `test_dfx_ffi_create` | FFI 创建 |
| `test_dfx_ffi_log` | FFI 日志 |
| `test_dfx_ffi_trace` | FFI 追踪 |
| `test_dfx_ffi_perf` | FFI 性能 |
| `test_stack_trace_capture` | 栈抓取 |

---

## 11. 构建过程记录

### 11.1 文件创建顺序

1. `Cargo.toml` - 包配置
2. `log_types.rs` - 日志类型
3. `logger.rs` - Logger 实现
4. `crash.rs` - CrashHandler 实现
5. `trace.rs` - TraceAnalyzer 实现
6. `perf.rs` - PerformanceMonitor 实现
7. `lib.rs` - DfxSystem + FFI
8. `dfx_test.rs` - 测试用例

### 11.2 依赖关系

```
dfx
├── hezhou-scripting (ScriptValue 集成)
├── parking_lot (Mutex)
├── chrono (时间格式化)
├── backtrace (栈抓取)
├── winapi (Windows API)
│   ├── processthreadsapi (CPU)
│   └── psapi (Memory)
└── libc (Unix Signal)
```

### 11.3 编译命令

```bash
cd engine
cargo build -p hezhou-dfx
cargo test -p hezhou-dfx
```

### 11.4 测试运行

```bash
cargo test -p hezhou-dfx --lib
```

预期输出:
```
running 30 tests
test test_dfx_create_destroy ... ok
test test_logger_levels ... ok
test test_log_output ... ok
...
test result: ok. 30 passed; 0 failed
```

---

## 12. C# 端绑定示例

```csharp
// Dfx.cs
public static class Dfx
{
    private static IntPtr _system;
    
    public static void Initialize() {
        _system = NativeMethods.dfx_create();
        NativeMethods.dfx_enable_all(_system);
    }
    
    public static void Shutdown() {
        NativeMethods.dfx_destroy(_system);
    }
    
    public static void Log(LogLevel level, string module, string message) {
        var modulePtr = CString.FromString(module);
        var messagePtr = CString.FromString(message);
        var filePtr = CString.FromString("");
        
        NativeMethods.dfx_log(_system, (byte)level, modulePtr, messagePtr, filePtr, 0);
    }
    
    public static void BeginFrame() {
        NativeMethods.dfx_perf_begin_frame(_system);
    }
    
    public static void EndFrame() {
        NativeMethods.dfx_perf_end_frame(_system);
    }
    
    public static float FPS => NativeMethods.dfx_get_fps(_system);
    
    public static void SaveTrace(string path) {
        var pathPtr = CString.FromString(path);
        NativeMethods.dfx_save_trace(_system, pathPtr);
    }
}

public enum LogLevel : byte {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Fatal = 5,
}
```

---

## 13. 集成示例

### 13.1 主循环集成

```rust
fn main() {
    let dfx = DfxSystem::new();
    dfx.enable_all();
    
    dfx.get_logger().lock().enable_file_output("game.log");
    dfx.get_crash_handler().lock().set_crash_file("crash_report.txt");
    
    let engine = engine_create();
    engine_start(engine);
    
    dfx.get_logger().lock().info("Main", "Engine started", "main.rs", 10);
    
    loop {
        dfx.get_perf_monitor().lock().begin_frame();
        
        let trace = ScopedTrace::new(
            dfx.get_trace_analyzer(),
            "run_frame",
            "engine"
        );
        
        engine_run_frame(engine, 0.016);
        
        dfx.get_perf_monitor().lock().end_frame();
        
        let fps = dfx.get_perf_monitor().lock().get_fps();
        if fps < 30.0 {
            dfx.get_logger().lock().warn("Performance", "Low FPS detected", "main.rs", 25);
        }
        
        if !platform_is_running(manager) {
            break;
        }
    }
    
    dfx.get_trace_analyzer().lock().save_to_file("trace.json");
    dfx.get_logger().lock().info("Main", "Engine shutdown", "main.rs", 30);
    
    engine_destroy(engine);
    dfx_destroy(Box::into_raw(Box::new(dfx)));
}
```

---

## 14. 性能数据示例

```
=== Performance Snapshot ===
Timestamp: 1715512845123
FPS: 60.0
Frame Time: 16.67 ms
CPU Usage: 25.5%
Memory Used: 128.5 MB
Draw Calls: 150
Triangles: 50000
```

---

## 15. 后续扩展

### 15.1 远程调试

- WebSocket 实时日志推送
- 性能数据网络上报
- 崩溃报告自动上传

### 15.2 可视化工具

- 性能曲线图 (FPS/Memory)
- Trace 热力图
- 崩溃栈可视化

### 15.3 诊断命令

```rust
// 运行时调整
dfx_set_log_level(system, 0);  // 开启全量日志
dfx_enable_trace(system);      // 开启追踪

// 性能诊断
let snapshot = dfx_get_perf_snapshot(system);
println!("FPS: {}, Memory: {} MB", snapshot.fps, snapshot.memory_used_mb);
```

---

*文档版本: 1.0 | 创建时间: 2026-05-12*