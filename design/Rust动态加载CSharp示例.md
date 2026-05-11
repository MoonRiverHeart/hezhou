# Rust 动态加载 C# 示例

## 概述

通过 Mono 嵌入实现 Rust 主程序运行时动态加载、执行用户新增的 C# 代码。

## 1. 整体架构

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
                          ↓
┌─────────────────────────────────────────────────────────┐
│  用户工作流                                              │
│  1. 编写 C# 代码                                        │
│  2. 编译为 DLL                                          │
│  3. 放入 Scripts/ 目录                                  │
│  4. Rust 主程序自动发现并加载                           │
│  5. 通过命令/API 执行                                   │
└─────────────────────────────────────────────────────────┘
```

## 2. Rust 端实现

### 2.1 ScriptManager 核心结构

```rust
// dynamic_script_loader.rs

use wrapped_mono::*;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ScriptManager {
    domain: Domain,
    loaded_assemblies: HashMap<String, Assembly>,
}

impl ScriptManager {
    pub fn new() -> Self {
        let domain = jit::init("ScriptDomain", None);
        Self {
            domain,
            loaded_assemblies: HashMap::new(),
        }
    }
    
    /// 加载新 DLL
    pub fn load_script(&mut self, dll_path: &str) -> Result<String, String> {
        let assembly = self.domain.assembly_open(dll_path)
            .ok_or("无法加载程序集")?;
        
        let name = PathBuf::from(dll_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        
        self.loaded_assemblies.insert(name.to_string(), assembly);
        Ok(name.to_string())
    }
    
    /// 执行任意类的方法
    pub fn execute(
        &self,
        assembly_name: &str,
        namespace: &str,
        class_name: &str,
        method_name: &str,
        args: (i32, i32),
    ) -> Result<Option<Object>, String> {
        let assembly = self.loaded_assemblies.get(assembly_name)
            .ok_or("程序集未加载")?;
        
        let image = assembly.get_image();
        let class = Class::from_name(&image, namespace, class_name)
            .ok_or("找不到类")?;
        
        let method = Method::get_from_name(&class, method_name, 2)
            .ok_or("找不到方法")?;
        
        method.invoke(None, args)
            .map_err(|e| format!("调用失败: {:?}", e))
    }
    
    /// 热重载（卸载旧版本，加载新版本）
    pub fn reload(&mut self, dll_path: &str) -> Result<String, String> {
        let name = PathBuf::from(dll_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        
        // 移除旧缓存
        self.loaded_assemblies.remove(name);
        
        // 加载新版本
        self.load_script(dll_path)
    }
}
```

### 2.2 文件监听自动加载

```rust
use std::fs;
use std::time::Duration;

fn watch_scripts_folder(manager: &mut ScriptManager) {
    let scripts_dir = "./Scripts/";
    
    loop {
        for entry in fs::read_dir(scripts_dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().unwrap() == "dll" {
                let name = path.file_name().unwrap().to_str().unwrap();
                
                if !manager.loaded_assemblies.contains_key(name) {
                    println!("发现新脚本: {}", name);
                    manager.load_script(path.to_str().unwrap()).unwrap();
                }
            }
        }
        
        std::thread::sleep(Duration::from_secs(1));
    }
}
```

### 2.3 命令行交互模式

```rust
fn interactive_mode(manager: &mut ScriptManager) {
    println!("=== Rust Script Runner ===");
    println!("命令: load <dll> | run <assembly> <namespace> <class> <method> <a> <b>");
    
    loop {
        let input = read_user_input();
        
        match parse_command(&input) {
            Command::Load(dll) => {
                manager.load_script(&dll).unwrap();
                println!("✓ 已加载: {}", dll);
            }
            Command::Run { assembly, ns, class, method, a, b } => {
                let result = manager.execute(&assembly, &ns, &class, &method, (a, b));
                println!("✓ 执行结果: {:?}", result);
            }
            Command::Quit => break,
        }
    }
}
```

## 3. 用户 C# 代码示例

### 3.1 插件 A - Calculator

```csharp
// Scripts/UserPluginA.dll
namespace UserPlugins {
    public class Calculator {
        public int Add(int a, int b) {
            Console.WriteLine($"[C#] Add: {a} + {b}");
            return a + b;
        }
    }
}
```

### 3.2 插件 B - DataProcessor（新增）

```csharp
// Scripts/UserPluginB.dll（用户后续新增）
namespace UserPlugins {
    public class DataProcessor {
        public int Process(int input, int factor) {
            Console.WriteLine($"[C#] Processing {input} with factor {factor}");
            return input * factor + 100;
        }
    }
}
```

### 3.3 业务逻辑 - GameRule（可修改）

```csharp
// Scripts/UserLogic.dll（用户随时修改）
namespace BusinessLogic {
    public class GameRule {
        public int CalculateScore(int baseScore, int bonus) {
            // 用户可以随时修改这个逻辑
            int result = baseScore * 2 + bonus;
            Console.WriteLine($"[C#] Score: {result}");
            return result;
        }
    }
}
```

## 4. 使用流程示例

```rust
fn main() {
    let mut manager = ScriptManager::new();
    
    // ========== 第一步：加载用户现有脚本 ==========
    println!("加载用户脚本...");
    manager.load_script("./Scripts/UserPluginA.dll").unwrap();
    
    let result = manager.execute(
        "UserPluginA.dll",
        "UserPlugins",
        "Calculator",
        "Add",
        (10, 20)
    ).unwrap();
    println!("结果: {:?}", result);
    
    // ========== 第二步：用户新增代码 ==========
    println!("\n用户新增了 DataProcessor.dll");
    manager.load_script("./Scripts/UserPluginB.dll").unwrap();
    
    let result = manager.execute(
        "UserPluginB.dll",
        "UserPlugins",
        "DataProcessor",
        "Process",
        (50, 3)
    ).unwrap();
    
    // ========== 第三步：用户修改已有代码 ==========
    println!("\n用户修改了 GameRule.dll，热重载...");
    manager.reload("./Scripts/UserLogic.dll").unwrap();
    
    manager.execute(
        "UserLogic.dll",
        "BusinessLogic",
        "GameRule",
        "CalculateScore",
        (100, 50)
    ).unwrap();
    
    println!("\n运行完成，用户可以继续添加新 DLL");
}
```

## 5. 完整工作流时间线

```
T0: Rust 主程序启动
    ├── 初始化 Mono Runtime
    └── 加载已有 DLL: UserPluginA.dll, UserLogic.dll

T1: 用户编写新 C# 代码
    ├── 创建 DataProcessor.cs
    └── 编译为 UserPluginB.dll

T2: 用户放入 Scripts/ 目录
    ├── Rust 主程序检测到新文件（或用户调用 load 命令）
    ├── 自动加载 UserPluginB.dll
    └── 立即可执行

T3: 用户修改已有代码
    ├── 修改 GameRule.cs 的 CalculateScore 逻辑
    ├── 重新编译 UserLogic.dll（覆盖旧文件）
    ├── Rust 主程序执行 reload
    └── 新逻辑生效，无需重启主程序

T4: 循环继续
    └── 用户可以不断添加、修改、执行
```

## 6. 应用场景

| 场景 | 说明 |
|------|------|
| 游戏脚本系统 | 玩家/UI 脚本热更新 |
| 插件系统 | 用户自定义插件 |
| 配置规则 | 动态业务逻辑 |
| 快速迭代 | 开发期无需重启 Rust 主程序 |

## 7. 技术要点

### 7.1 Assembly 缓存管理

```rust
loaded_assemblies: HashMap<String, Assembly>
```

- 避免重复加载同一 DLL
- 通过文件名作为 key 快速查找

### 7.2 热重载策略

1. 移除旧 Assembly 引用
2. 加载新 Assembly
3. 注意：Mono 不支持真正的 Assembly 卸载，旧引用会残留

### 7.3 类型安全调用

```rust
Method::get_from_name(&class, method_name, param_count)
method.invoke(None/Some(instance), args)
```

### 7.4 错误处理

| 错误类型 | 处理方式 |
|----------|----------|
| 找不到类 | 返回错误信息 |
| 找不到方法 | 返回错误信息 |
| 参数类型不匹配 | invoke 返回 Err |
| 运行时异常 | 捕获 Mono 异常 |

## 8. 与现有项目的关系

| 现有文件 | 用途 | 动态加载扩展 |
|----------|------|-------------|
| `lib.rs` | 导出 Rust 函数供 C# 调用 | 保持不变 |
| `main.rs` | Mono 嵌入测试程序 | 改造为 ScriptManager |
| C# DLLs | 用户代码 | 放入 Scripts/ 目录 |

## 9. 反射：获取 DLL 中的函数信息

### 9.1 问题

Rust 主程序如何知道用户 DLL 中：
- 有哪些类？
- 每个类有哪些方法？
- 方法名称是什么？
- 有几个参数？
- 参数类型是什么？

### 9.2 解决方案：Mono 反射 API

```rust
/// 遍历 Assembly，获取所有类和方法信息
pub fn inspect_assembly(&self, assembly_name: &str) -> Vec<ClassInfo> {
    let assembly = self.loaded_assemblies.get(assembly_name).unwrap();
    let image = assembly.get_image();
    
    // 获取所有类
    let classes = self.get_all_classes(&image);
    
    // 对每个类，获取所有方法
    classes.iter().map(|class| {
        ClassInfo {
            namespace: class.get_namespace(),
            name: class.get_name(),
            methods: self.get_all_methods(class),
        }
    }).collect()
}

fn get_all_classes(&self, image: &Image) -> Vec<Class> {
    // Mono 提供的遍历 API
    // 需要调用 mono_image_get_table_info + mono_class_get
    unsafe {
        let table = mono_image_get_table_info(image.get_raw(), MONO_TABLE_TYPEDEF);
        let count = mono_table_info_get_rows(table);
        
        (0..count).map(|i| {
            let class_ptr = mono_class_get(image.get_raw(), i + 1);
            Class::from_ptr(class_ptr)
        }).collect()
    }
}

fn get_all_methods(&self, class: &Class) -> Vec<MethodInfo> {
    unsafe {
        let method_iter = mono_class_get_methods(class.get_raw());
        
        let mut methods = Vec::new();
        while let Some(method_ptr) = mono_class_get_next_method(method_iter) {
            let method = Method::from_ptr(method_ptr);
            methods.push(MethodInfo {
                name: method.get_name(),
                param_count: method.get_param_count(),
                return_type: method.get_return_type().get_name(),
                params: self.get_params(&method),
            });
        }
        methods
    }
}

fn get_params(&self, method: &Method) -> Vec<ParamInfo> {
    let count = method.get_param_count();
    (0..count).map(|i| {
        ParamInfo {
            name: method.get_param_name(i),
            type_name: method.get_param_type(i).get_name(),
        }
    }).collect()
}
```

### 9.3 数据结构

```rust
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub namespace: String,
    pub name: String,
    pub methods: Vec<MethodInfo>,
}

#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub param_count: usize,
    pub return_type: String,
    pub params: Vec<ParamInfo>,
}

#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub name: String,
    pub type_name: String,
}
```

### 9.4 使用示例

```rust
// 加载 DLL 后，打印所有可用方法
let manager = ScriptManager::new();
manager.load_script("./Scripts/UserPlugin.dll").unwrap();

let info = manager.inspect_assembly("UserPlugin.dll");
for class in info {
    println!("类: {}.{}", class.namespace, class.name);
    for method in class.methods {
        println!("  方法: {}({})", method.name, method.param_count);
        println!("    返回: {}", method.return_type);
        for param in method.params {
            println!("    参数 {}: {}", param.name, param.type_name);
        }
    }
}
```

### 9.5 输出示例

```
类: UserPlugins.Calculator
  方法: Add(2)
    返回: System.Int32
    参数 a: System.Int32
    参数 b: System.Int32
  方法: Multiply(2)
    返回: System.Int32
    参数 x: System.Int32
    参数 y: System.Int32
  方法: GetResult(0)
    返回: System.Int32

类: UserPlugins.DataProcessor
  方法: Process(2)
    返回: System.Int32
    参数 input: System.Int32
    参数 factor: System.Int32
```

### 9.6 wrapped_mono 提供的 API

| 方法 | 功能 |
|------|------|
| `Class::from_name(image, ns, name)` | 按名称查找类 |
| `Method::get_from_name(class, name, param_count)` | 按名称查找方法 |
| `method.get_param_count()` | 获取参数数量 |
| `method.get_return_type()` | 获取返回类型 |
| `method.invoke(instance, args)` | 执行方法 |

### 9.7 完整 inspect 实现（使用 wrapped_mono）

```rust
impl ScriptManager {
    /// 打印 Assembly 的所有公共方法
    pub fn print_available_methods(&self, assembly_name: &str) {
        let assembly = self.loaded_assemblies.get(assembly_name).unwrap();
        let image = assembly.get_image();
        
        println!("\n=== {} 可用方法 ===", assembly_name);
        
        // 遍历常见命名空间的常见类名
        for ns in ["UserPlugins", "BusinessLogic", "MyGame"] {
            for class_name in ["Calculator", "DataProcessor", "GameRule", "Helper"] {
                if let Some(class) = Class::from_name(&image, ns, class_name) {
                    println!("\n类: {}.{}", ns, class_name);
                    
                    // 尝试不同参数数量的方法名
                    for method_name in ["Add", "Multiply", "Process", "CalculateScore", "Run", "Execute"] {
                        for param_count in 0..=4 {
                            if let Some(method) = Method::get_from_name(&class, method_name, param_count) {
                                println!("  ✓ {}({} 个参数)", method_name, param_count);
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// 自动发现并缓存所有方法
    pub fn auto_register_methods(&mut self, assembly_name: &str) {
        let assembly = self.loaded_assemblies.get(assembly_name).unwrap();
        let image = assembly.get_image();
        
        // 需要用 Mono 底层 API 遍历
        // 或让用户在 C# 中提供注册方法
    }
}
```

### 9.8 更好的方案：让 C# 自注册

```csharp
// Scripts/UserPlugin.dll
namespace UserPlugins {
    // 让用户实现一个标准接口
    public class PluginInfo {
        public static string[] GetAvailableMethods() {
            return new string[] {
                "Calculator.Add(int, int) -> int",
                "Calculator.Multiply(int, int) -> int",
                "DataProcessor.Process(int, int) -> int"
            };
        }
        
        public static Dictionary<string, Func<int, int, int>> GetMethodTable() {
            return new Dictionary<string, Func<int, int, int>> {
                { "Calculator.Add", (a, b) => new Calculator().Add(a, b) },
                { "Calculator.Multiply", (a, b) => new Calculator().Multiply(a, b) },
            };
        }
    }
}
```

Rust 端：

```rust
// 加载后自动调用 PluginInfo.GetAvailableMethods()
let info_class = Class::from_name(&image, "UserPlugins", "PluginInfo");
let get_methods = Method::from_name(&info_class, "GetAvailableMethods", 0);
let result = get_methods.invoke(None, ());
// 解析返回的字符串数组，构建方法注册表
```

## 10. 实现路线

```
Phase 1: 基础框架
├── ScriptManager 结构
├── load_script / execute 方法
└── 基本错误处理

Phase 2: 交互能力
├── 命令行交互模式
├── reload 热重载
└── 参数解析

Phase 3: 自动化
├── 文件监听
├── 自动加载新 DLL
└── 日志记录

Phase 4: 高级特性
├── 返回值类型转换
├── 多参数支持
└── 异常捕获
```

## 10. 参考实现

完整实现可参考 Unity 引擎架构：

```
Unity 架构：
┌──────────────────┐
│  C++ Core Engine │  ← Rust 主程序
│      ↓           │
│  Mono Runtime    │  ← Mono JIT
│      ↓           │
│  C# Scripts      │  ← 用户 DLLs
└──────────────────┘

热更新流程：
Editor 编译 C# → Assembly 加载 → 执行 → 修改 → 重编译 → 重加载
```

---

**结论**：Rust 通过 Mono 嵌入可以实现运行时动态加载用户新增 C# 代码，无需重启主程序，支持热重载和交互式执行。