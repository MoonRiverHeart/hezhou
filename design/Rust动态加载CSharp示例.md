# Rust 动态调用 C# 示例

## 优先级排序

| 方式 | 性能 | 复杂度 | 推荐场景 |
|------|------|--------|----------|
| **1. Thunk 函数指针** | 最高 | 低 | **高频调用、每帧逻辑** |
| 2. NativeAOT DLL | 高 | 中 | 预编译插件、模块化加载 |
| 3. Mono JIT | 低 | 高 | 热重载、开发期调试 |

---

## 实际实现总结

### 已完成：Mono JIT 热更新系统（开发模式）

#### 核心架构

```
┌─────────────────────────────────────────────────────────────────────┐
│  Rust MonoExecutor                                                  │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  ScriptManager                                               │   │
│  │    ├── Domain (mono jit::init())                            │   │
│  │    ├── Assemblies HashMap<String, Assembly>                 │   │
│  │    └── execute(assembly, ns, class, method, args)           │   │
│  │                                                              │   │
│  │  热更新流程:                                                  │   │
│  │    1. recompile_mono_dll() → mcs 编译新 DLL                   │   │
│  │       └── 生成带时间戳的 Assembly 名称: RotationScript_12345.dll│   │
│  │    2. executor.load(new_dll_path) → 加载新 Assembly          │   │
│  │    3. call("ResetAll") → 重置 C# 静态实例                     │   │
│  │    4. 新常量值生效                                            │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              ↓ mono_runtime_invoke                  │
├─────────────────────────────────────────────────────────────────────┤
│  C# RotationController                                              │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  private const float DefaultRotationSpeed = 90.0f;          │   │
│  │  private static RotationController _instance;               │   │
│  │                                                              │   │
│  │  ResetAll():                                                 │   │
│  │    _instance = null;                                         │   │
│  │    GetInstance(); // 重新创建实例，读取新常量                   │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

#### 关键技术点

**1. Mono Assembly 缓存问题**

问题：`mono_domain_assembly_open` 会缓存已加载的 Assembly，即使文件更新也返回旧版本。

**解决方案**：每次编译生成不同名称的 Assembly
- 编译脚本使用时间戳命名：`RotationScript_{timestamp}.dll`
- Mono 从 DLL 文件名推导 Assembly 名称
- 每次热更新加载新 Assembly，绕过缓存

**2. C# 静态变量重新初始化**

问题：Mono 不重新初始化静态字段（`_instance` 保持旧值）。

**解决方案**：实例模式 + `ResetAll()` 方法
```csharp
public static void ResetAll() {
    _instance = null;           // 清除旧实例
    GetInstance();              // 创建新实例，读取新常量
}
```

**3. wrapped_mono Method 查找问题**

问题：`Method::get_from_name` 返回 null。

**解决方案**：迭代方法列表 + 名称匹配
```rust
let mut iter = std::ptr::null_mut();
loop {
    let method_ptr = mono_class_get_methods(class.get_ptr(), &mut iter);
    if method_ptr.is_null() { break; }
    let name = mono_method_get_name(method_ptr);
    if name == method_name {
        found_method_ptr = method_ptr;
        break;
    }
}
```

#### 热更新测试结果

```
[修改 RotationScript.cs 第 13 行: DefaultRotationSpeed = 900.0f]

Frame 600: angle=188.9°, speed=90°/s
[HotReload] R pressed, recompiling...
[Info] Compiling: RotationScript.cs
[Success] RotationScript_1747000000.dll compiled
AssemblyName: RotationScript_1747000000

[HotReload] Compile success! New DLL: .../RotationScript_1747000000.dll
[HotReload] Assembly loaded, calling ResetAll...
[C#] ResetAll complete, speed=900°/s
[HotReload] Speed after reload: 900°/s

Frame 840: angle=113.9°, speed=900°/s  ← 新速度生效！
```

#### 文件结构

```
engine/
├ scripting/src/
│   ├── script_manager.rs      ← Mono Assembly 加载/调用
│   ├── mono_executor.rs       ← MonoExecutor 封装
│   └── value_bridge.rs        ← ScriptValue 桥接
│
├ scripts/
│   ├── RotationScript.cs      ← C# 脚本（单文件 + 条件编译）
│   ├── build_mono.ps1         ← Mono 编译脚本（mcs + 时间戳命名）
│   └── bin/Mono/Release/net8.0/
│       └── RotationScript_{timestamp}.dll  ← 编译产物
│
├ rhi-vulkan/src/
│   └── mono_rotation_renderer.rs  ← Vulkan + Mono 渲染器
│       └── recompile_mono_dll()   ← 热更新触发
│       └── load(new_dll_path)     ← 加载新 Assembly
│
└ examples/src/
    └── mono_triangle_demo.rs      ← Demo 入口（按 R 热更新）
```

#### 优缺点

| 维度 | Mono JIT 热更新 | 评分 |
|------|-----------------|------|
| **热重载** | 支持（实时生效） | ⭐⭐⭐⭐⭐ |
| **开发体验** | 极佳（无需重启） | ⭐⭐⭐⭐⭐ |
| **性能** | 低（反射调用 ~100μs） | ⭐⭐ |
| **内存占用** | 较大（Mono 运行时） | ⭐⭐ |
| **跨平台** | 需配置 Mono SDK | ⭐⭐⭐ |
| **部署复杂度** | 高（依赖 Mono 环境） | ⭐⭐ |

---

### 待实现：NativeAOT 静态编译（发布模式）

#### 架构设计

```
┌─────────────────────────────────────────────────────────────────────┐
│  Rust Renderer                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  libloading::Library                                         │   │
│  │    ├── load("RotationScript.NativeAOT.dll")                 │   │
│  │    ├── get("csharp_update_rotation")                        │   │
│  │    └── 函数指针调用：extern "C" fn(f32) -> f32              │   │
│  │                                                              │   │
│  │  性能：~20ns 调用开销（直接函数指针）                          │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              ↓ 直接函数指针调用                      │
├─────────────────────────────────────────────────────────────────────┤
│  C# NativeAOT DLL                                                   │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  [UnmanagedCallersOnly(EntryPoint = "csharp_update_rotation")]│   │
│  │  public static float UpdateRotation(float deltaTime)         │   │
│  │                                                              │   │
│  │  编译为原生机器码（无 CLR 运行时）                             │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

#### 条件编译方案

**单 C# 源文件**：`RotationScript.cs`

```csharp
#if MONO
// Mono JIT 版本（开发模式）
public class RotationController {
    private const float DefaultRotationSpeed = 90.0f;
    private static RotationController _instance;
    
    public static float UpdateRotation(float deltaTime) {
        return GetInstance()._UpdateRotation(deltaTime);
    }
    
    public static void ResetAll() {
        _instance = null;
        GetInstance();
    }
}
#endif

#if NATIVEAOT
// NativeAOT 版本（发布模式）
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;

public static class RotationController {
    private static float _rotationSpeed = 90.0f;
    
    [UnmanagedCallersOnly(EntryPoint = "csharp_update_rotation", CallConvs = new[] { typeof(CallConvCdecl) })]
    public static float UpdateRotation(float deltaTime) {
        return _rotationSpeed * deltaTime;
    }
    
    [UnmanagedCallersOnly(EntryPoint = "csharp_set_rotation_speed", CallConvs = new[] { typeof(CallConvCdecl) })]
    public static void SetRotationSpeed(float speed) {
        _rotationSpeed = speed;
    }
}
#endif
```

**编译脚本分离**：
- `build_mono.ps1`：`mcs -define:MONO RotationScript.cs`
- `build_nativeaot.ps1`：`dotnet publish -p:PublishAot=true -define:NATIVEAOT`

#### 优缺点

| 维度 | NativeAOT 静态编译 | 评分 |
|------|-------------------|------|
| **性能** | 最高（~20ns 调用） | ⭐⭐⭐⭐⭐ |
| **内存占用** | 最小（无 CLR 运行时） | ⭐⭐⭐⭐⭐ |
| **跨平台** | 完全一致（原生 DLL） | ⭐⭐⭐⭐⭐ |
| **部署复杂度** | 低（单一 DLL） | ⭐⭐⭐⭐⭐ |
| **热重载** | 不支持（需重启） | ⭐ |
| **开发体验** | 差（每次修改需重新编译） | ⭐⭐ |

---

### 两种模式对比

| 特性 | Mono JIT (开发) | NativeAOT (发布) |
|------|-----------------|------------------|
| **编译器** | `mcs` (Mono SDK) | `dotnet publish` |
| **DLL 类型** | 标准 .NET DLL | 原生机器码 DLL |
| **加载方式** | Mono 反射加载 | `libloading` 动态加载 |
| **调用方式** | `mono_runtime_invoke` | 函数指针直接调用 |
| **调用开销** | ~100μs (反射) | ~20ns (函数指针) |
| **热重载** | ✅ 支持（按 R 键） | ❌ 不支持 |
| **内存占用** | 较大（Mono 运行时） | 最小（无运行时） |
| **部署依赖** | Mono SDK | 无额外依赖 |
| **适用场景** | 开发期快速迭代 | 生产环境高性能 |

---

### 最佳实践

**开发阶段**：
1. 使用 Mono JIT 模式
2. 运行 `mono_triangle_demo`（按 R 热更新）
3. 修改 `RotationScript.cs` 常量值
4. 立即验证效果，无需重启

**发布阶段**：
1. 使用 NativeAOT 模式编译
2. 集成到生产环境渲染器
3. 享受最高性能和最小内存占用

**切换方式**：
```bash
# 开发模式（Mono JIT）
cargo run --bin mono_triangle_demo --features mono

# 发布模式（NativeAOT）
cargo run --bin rotation_demo --features native-aot
```

---

## 方式一：Thunk 函数指针（第一优先级）

### 概述

通过 CLR 自动生成的 Thunk（跳板函数），Rust 直接调用 C# 函数指针，无需反射、无需封送、最高性能。

### 1. 架构图

```
┌─────────────────────────────────────────────────────────────────┐
│ C# 程序                                                         │
│                                                                 │
│  static float rotation_speed = 90.0f;                          │
│  static float current_angle = 0.0f;                            │
│                                                                 │
│  [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]│
│  public static float CalculateRotation(float deltaTime) {       │
│      float increment = rotation_speed * deltaTime;              │
│      current_angle += increment;                                │
│      return increment;                                          │
│  }                                                              │
│                                                                 │
│  public static unsafe delegate* unmanaged[Cdecl]<float, float>  │
│      CalculateRotationPtr = &CalculateRotation;                 │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ CLR 生成 Thunk（跳板函数）                                  ││
│  │                                                             ││
│  │ Thunk 指向 → CalculateRotation(value)                      ││
│ │   ├─ 保存托管上下文                                         ││
│  │   ├─ 切换到 CLR 运行时                                      ││
│  │   ├─ 执行 C# 方法                                          ││
│  │   └─ 返回结果                                              ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ register_rotation_callback(thunk_ptr)
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ Rust 程序                                                       │
│                                                                 │
│  type RotationCallback = extern "C" fn(f32) -> f32;            │
│                                                                 │
│  static ROTATION_CALLBACK: Mutex<Option<RotationCallback>>     │
│                                                                 │
│  register_rotation_callback(thunk_ptr) {                        │
│      *ROTATION_CALLBACK.lock() = Some(thunk_ptr);               │
│  }                                                              │
│                                                                 │
│  trigger_rotation_callback(delta_time) -> f32 {                │
│      let cb = ROTATION_CALLBACK.lock();                         │
│      cb.unwrap()(delta_time)  // 直接调用函数指针               │
│  }                                                              │
│                                                                 │
│  调用路径：                                                      │
│  trigger_rotation_callback(0.016)                               │
│    → thunk(0.016)                                               │
│    → CLR 上下文切换                                              │
│    → CalculateRotation(0.016)                                   │
│    → 访问静态字段 rotation_speed                                 │
│    → 计算并返回 1.44                                            │
│    → Rust 接收返回值                                             │
└─────────────────────────────────────────────────────────────────┘
```

### 2. C# 端实现

```csharp
// RotationScript.cs (NativeAOT 编译)
using System;
using System.Runtime.InteropServices;

namespace HezhouScripts
{
    public static class RotationController
    {
        private static float _rotationSpeed = 90.0f;
        private static float _currentAngle = 0.0f;
        
        public static unsafe delegate* unmanaged[Cdecl]<float, float> CalculateRotationPtr;
        
        public static unsafe void Initialize()
        {
            CalculateRotationPtr = &CalculateRotation;
            NativeMethods.register_rotation_callback(CalculateRotationPtr);
        }
        
        [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
        public static float CalculateRotation(float deltaTime)
        {
            float angleIncrement = _rotationSpeed * deltaTime;
            _currentAngle += angleIncrement;
            
            if (_currentAngle >= 360.0f)
            {
                _currentAngle -= 360.0f;
            }
            
            return angleIncrement;
        }
        
        [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
        public static void SetRotationSpeed(float speed)
        {
            _rotationSpeed = speed;
        }
        
        [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
        public static float GetRotationSpeed()
        {
            return _rotationSpeed;
        }
        
        [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
        public static void ResetRotation()
        {
            _currentAngle = 0.0f;
        }
    }
    
    internal static class NativeMethods
    {
        [DllImport("hezhou_scripting", CallingConvention = CallingConvention.Cdecl)]
        public static extern unsafe void register_rotation_callback(
            delegate* unmanaged[Cdecl]<float, float> callback
        );
        
        [DllImport("hezhou_scripting", CallingConvention = CallingConvention.Cdecl)]
        public static extern unsafe float trigger_rotation_callback(float deltaTime);
    }
}
```

### 3. Rust 端实现

```rust
// scripting/src/lib.rs

use parking_lot::Mutex;
use std::sync::LazyLock;

type RotationCallback = extern "C" fn(f32) -> f32;

static ROTATION_CALLBACK: LazyLock<Mutex<Option<RotationCallback>>> = 
    LazyLock::new(|| Mutex::new(None));

#[unsafe(no_mangle)]
pub extern "C" fn register_rotation_callback(callback: RotationCallback) {
    let mut cb = ROTATION_CALLBACK.lock();
    *cb = Some(callback);
    println!("[Rust] Registered rotation callback: {:?}", callback);
}

#[unsafe(no_mangle)]
pub extern "C" fn trigger_rotation_callback(delta_time: f32) -> f32 {
    let cb = ROTATION_CALLBACK.lock();
    if let Some(callback) = *cb {
        let result = callback(delta_time);
        println!("[Rust] Called C# callback: dt={} -> increment={}", delta_time, result);
        result
    } else {
        println!("[Rust] No callback registered, using default");
        90.0 * delta_time
    }
}
```

### 4. 完整调用流程

```
时间线：

T0: C# Main 启动
    ├── CLR 初始化
    ├── 静态字段初始化: rotation_speed = 90.0f
    ├── 获取函数指针: CalculateRotationPtr = &CalculateRotation
    └── 调用 Rust FFI: register_rotation_callback(thunk_ptr)
    
T1: Rust 接收并保存
    ├── ROTATION_CALLBACK = Some(thunk_ptr)
    └── thunk_ptr 指向 CLR 生成的跳板函数
    
T2: 每帧渲染循环
    ├── Rust 计算 delta_time = 0.016s
    ├── 调用 trigger_rotation_callback(0.016)
    ├── 通过函数指针调用 thunk(0.016)
    ├── CLR Thunk:
    │   ├─ 保存当前上下文
    │   ├─ 切换到 CLR 运行时
    │   ├─ 调用 CalculateRotation(0.016)
    │   │   └─ 访问静态字段 rotation_speed (90.0f)
    │   │   ─ 计算 increment = 90 * 0.016 = 1.44
    │   │   ─ 更新 current_angle += 1.44
    │   │   ─ 返回 1.44
    │   └─ 切换回非托管上下文
    ├── Rust 接收返回值: 1.44
    └── 更新渲染角度
    
T3: 修改速度
    ├── C# Main 调用 SetRotationSpeed(180.0f)
    ├── rotation_speed 更新为 180.0f
    └── 下次调用返回 increment = 180 * 0.016 = 2.88
```

### 5. 性能对比

| 方式 | 调用开销 | GC 压力 | 适用场景 |
|------|----------|---------|----------|
| Thunk 函数指针 | ~10ns | 无 | **每帧调用、高频逻辑** |
| Mono 反射 invoke | ~100μs | 有 | 偶尔调用、调试期 |
| NativeAOT 导出 | ~20ns | 无 | 预编译插件 |

### 6. 关键技术点

#### 6.1 函数指针语法

**C# 9.0 函数指针**：
```csharp
delegate* unmanaged[Cdecl]<float, float>  // C# 函数指针类型
&CalculateRotation                          // 获取方法地址
```

**Rust extern "C"**：
```rust
extern "C" fn(f32) -> f32                  // Rust 函数指针类型
```

#### 6.2 调用约定

| 约定 | C# | Rust |
|------|-----|-------|
| Cdecl | `[CallConvCdecl]` | `extern "C"` |
| Stdcall | `[CallConvStdcall]` | `extern "stdcall"` |

#### 6.3 静态字段捕获

```csharp
// 通过静态字段模拟闭包
private static float _rotationSpeed = 90.0f;  // 捕获的状态
private static float _currentAngle = 0.0f;    // 状态更新
```

**原理**：
- 静态字段在 CLR 中全局可见
- Thunk 调用时自动访问静态字段
- 无需额外上下文传递

#### 6.4 Thunk 自动生成

```
CLR 编译时生成 Thunk:

CalculateRotation 方法
    ↓
CLR JIT 编译为机器码
    ↓
生成 Thunk 入口点:
    - push rbp
    - mov rbp, rsp
    - mov edi, [rsp+8]       ; 参数 deltaTime
    - call CalculateRotation_body
    - mov eax, result        ; 返回值
    - pop rbp
    - ret
    ↓
Thunk 地址 = 0x7FF1234000
    ↓
register_rotation_callback(0x7FF1234000)
    ↓
Rust 保存: ROTATION_CALLBACK = Some(0x7FF1234000)
```

### 7. 实际运行示例

```
=== Rust -> C# Thunk 调用 Demo ===

[架构说明]
  C# 端:
    - 定义静态字段 rotation_speed = 90°/s
    - 定义函数指针 CalculateRotationPtr
    - [UnmanagedCallersOnly] 标记方法可被非托管代码调用
    - CLR 自动生成 Thunk (跳板函数)
  Rust 端:
    - ROTATION_CALLBACK: Mutex<Option<extern "C" fn(f32) -> f32>>
    - register_rotation_callback(thunk_ptr) 保存函数指针
    - trigger_rotation_callback(dt) -> thunk(dt) -> C# CalculateRotation
  调用路径:
    Rust -> Thunk -> CLR 上下文切换 -> C# CalculateRotation -> 返回值

[1] 创建 Vulkan Renderer...
    [Rust] Registered rotation callback from C#: 0x7FF1234567
    Renderer 初始化成功!

[2] 运行渲染循环...
    每帧调用 C# thunk 计算旋转角度
    旋转速度: 90 度/秒

    [Mock C#] Thunk called: dt=0.0164s -> increment=1.47°
[Rust] Called C# rotation callback: dt=0.0164s -> increment=1.47°
    Frame 60: angle = 101.9°
    
    [Mock C#] Thunk called: dt=0.0167s -> increment=1.50°
[Rust] Called C# rotation callback: dt=0.0167s -> increment=1.50°
    Frame 120: angle = 191.2°

=== Demo Complete ===
```

### 8. 与 Mono 方式对比

| 特性 | Thunk 方式 | Mono 方式 |
|------|-----------|-----------|
| 启动时间 | 快（NativeAOT） |慢（JIT预热） |
| 调用性能 | 10ns | 100μs |
| 热重载 | 不支持 | 支持 |
| 内存占用 | 小 | 大 |
| 开发体验 | 需重新编译 | 即时生效 |
| **推荐** | **生产环境** | **开发期** |

---

## 方式二：NativeAOT DLL 加载（第二优先级）

### 概述

C# 通过 NativeAOT 编译为原生 DLL，Rust 使用动态加载器调用导出函数。

### 1. C# 项目配置

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <PublishAot>true</PublishAot>
    <NativeLib>Shared</NativeLib>
    <AllowUnsafeBlocks>true</AllowUnsafeBlocks>
  </PropertyGroup>
</Project>
```

### 2. 导出函数

```csharp
[UnmanagedCallersOnly(EntryPoint = "calculate_rotation")]
public static float CalculateRotation(float deltaTime) { ... }
```

### 3. Rust 加载 DLL

```rust
use libloading::{Library, Symbol};

let lib = Library::new("RotationScript.dll")?;
let calculate: Symbol<extern "C" fn(f32) -> f32> = 
    unsafe { lib.get("calculate_rotation".as_bytes())? };
let result = calculate(0.016f32);
```

---

## 方式三：Mono JIT 动态加载（第三优先级）

### 概述

通过 Mono 嵌入实现运行时动态加载、热重载 C# DLL，适合开发期快速迭代。

### 1. 整体架构

```
┌─────────────────────────────────────────────────────────┐
│                   Rust 主程序                            │
│  ┌─────────────────────────────────────────────────┐   │
│  │  Mono Runtime (JIT)                              │   │
│  │    ↓                                             │   │
│  │  动态加载器                                       │   │
│  │    ↓                                             │   │
│  │  Assembly Cache (已加载的 DLL 缓存)              │   │
│  │    ↓                                             │   │
│  │  Method Registry (方法查找表)                    │   │
│  └─────────────────────────────────────────────────┘   │
│                          ↓                              │
│  用户接口: load_script() / execute() / unload()        │
└─────────────────────────────────────────────────────────┘
```

### 2. Rust 端实现

```rust
use wrapped_mono::*;
use std::collections::HashMap;

pub struct ScriptManager {
    domain: Domain,
    assemblies: HashMap<String, Assembly>,
}

impl ScriptManager {
    pub fn new() -> Self {
        let domain = jit::init("ScriptDomain", None);
        Self {
            domain,
            assemblies: HashMap::new(),
        }
    }
    
    pub fn load_script(&mut self, dll_path: &str) -> Result<String, String> {
        let assembly = self.domain.assembly_open(dll_path)
            .ok_or("无法加载程序集")?;
        
        let name = PathBuf::from(dll_path)
            .file_name().unwrap().to_str().unwrap();
        
        self.assemblies.insert(name.to_string(), assembly);
        Ok(name.to_string())
    }
    
    pub fn execute(&self, assembly: &str, ns: &str, class: &str, method: &str, 
                  args: (i32, i32)) -> Result<i32, String> {
        let asm = self.assemblies.get(assembly).ok_or("未加载")?;
        let image = asm.get_image();
        let cls = Class::from_name(&image, ns, class).ok_or("找不到类")?;
        let m = Method::get_from_name(&cls, method, 2).ok_or("找不到方法")?;
        
        m.invoke(None, args)
            .map(|r| r.map_or(0, |_| 0))
            .map_err(|e| format!("调用失败: {:?}", e))
    }
    
    pub fn reload(&mut self, dll_path: &str) -> Result<String, String> {
        let name = PathBuf::from(dll_path).file_name().unwrap().to_str().unwrap();
        self.assemblies.remove(name);
        self.load_script(dll_path)
    }
}
```

### 3. 使用示例

```rust
fn main() {
    let mut manager = ScriptManager::new();
    
    manager.load_script("./Scripts/UserPlugin.dll").unwrap();
    
    let result = manager.execute(
        "UserPlugin.dll",
        "UserPlugins",
        "Calculator",
        "Add",
        (10, 20)
    ).unwrap();
    
    println!("结果: {}", result);
}
```

### 4. Mono 反射：获取 DLL 信息

```rust
pub fn inspect_assembly(&self, assembly_name: &str) -> Vec<ClassInfo> {
    let assembly = self.assemblies.get(assembly_name).unwrap();
    let image = assembly.get_image();
    
    unsafe {
        let table = mono_image_get_table_info(image.get_raw(), MONO_TABLE_TYPEDEF);
        let count = mono_table_info_get_rows(table);
        
        (0..count).map(|i| {
            let class_ptr = mono_class_get(image.get_raw(), i + 1);
            // ... 遍历类和方法
        }).collect()
    }
}
```

---

## 实现路线图

```
Phase 1 (生产环境): Thunk 函数指针
├── 实现 register_callback / trigger_callback FFI
├── C# [UnmanagedCallersOnly] 标记方法
├── 静态字段捕获状态
├── 性能测试验证
└── 集成到渲染循环

Phase 2 (插件系统): NativeAOT DLL
├── C# NativeAOT 编译配置
├── Rust libloading 加载器
├── 多 DLL 管理器
└── 插件生命周期管理

Phase 3 (开发期): Mono JIT 热重载
├── Mono 嵌入初始化
├── Assembly 加载/卸载
├── 反射 API 集成
└── 开发工具集成
```

---

## 技术决策

| 场景 | 推荐方式 | 理由 |
|------|----------|------|
| 每帧计算（旋转、物理） | **Thunk** | 10ns 调用开销，无 GC |
| 预编译插件模块 | NativeAOT | 独立 DLL，按需加载 |
| 开发期快速迭代 | Mono JIT | 热重载，即时生效 |
| 跨平台一致性 | Thunk | 所有平台同一 API |

---

## 参考

- **C# 函数指针**: https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/proposals/csharp-9.0/function-pointers
- **UnmanagedCallersOnly**: https://learn.microsoft.com/en-us/dotnet/api/system.runtime.interopservices.unmanagedcallersonlyattribute
- **NativeAOT**: https://learn.microsoft.com/en-us/dotnet/core/deploying/native-aot/
- **Mono 嵌入**: https://www.mono-project.com/docs/advanced/embedding/