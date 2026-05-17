# Thunk + Mono JIT UI系统调试总结

## 一、目标

实现Rust UI控件 + Mono JIT C#回调系统：
- Rust创建UI控件（Button、Label）
- C#通过FfiContext获取函数指针
- 点击Button触发C#回调，修改Label文本

## 二、遇到的问题

**C#读取的值与Rust写入的值完全不匹配**

示例输出：
```
[Rust] ui_get_primary_button_id: 0x7ff6fca2dbb0
[C#]  offset 0: 0x4840EC8348575655  ← 这是机器码！不是函数地址
```

C#读取到的是机器码字节（如`0x4840EC83...`），而不是存储的函数指针。

## 三、调试过程

### 3.1 验证结构体布局
添加日志对比Rust和C#的结构体：
```
[Rust] FfiContext size: 48 bytes
[Rust] offset 0, 8, 16, 24, 32, 40
[C#]  FfiContext size: 48 bytes
[C#]  offset 0, 8, 16, 24, 32, 40
```
**结论：结构体布局完全匹配**

### 3.2 验证存储方式
将`Option<FfiContext>`改为`Box<FfiContext>`：
```rust
let boxed = Box::new(ctx);
FFI_CONTEXT = Some(boxed);
```
**结论：堆内存分配，指针稳定**

### 3.3 验证指针传递
对比Rust写入和C#读取的地址：
```
[Rust] Box allocated at: 0x1a7732b59f0
[Rust] get_ffi_context_ptr: 0x1a7732b59f0
[C#]  contextPtr value: 0x7FF7864CDBB0  ← 值错误！
```
C#收到的指针是`0x7FF7864CDBB0`（函数地址），而不是`0x1a7732b59f0`（数据地址）。

## 四、根本原因

**`mono_runtime_invoke` API要求：参数必须是"指针指向的值"，而非"值本身"**

原代码（错误）：
```rust
// script_manager.rs
params.push(ptr_arg as *mut c_void);  // 直接传递值
```

Mono运行时读取时，把`ptr_arg`当作内存地址去读取，导致读到的是随机内存（可能是函数的机器码）。

## 五、解决方案

**将参数存储到局部变量，传递指向该变量的指针**

修正代码：
```rust
// script_manager.rs:234-236
let mut params: Vec<*mut std::os::raw::c_void> = Vec::new();
let mut ptr_storage: usize = ptr_arg;  // 存到局部变量
if param_count > 0 {
    params.push(&mut ptr_storage as *mut usize as *mut std::os::raw::c_void);
}
```

## 六、验证结果

### 6.1 正确传递指针
```
[Rust] Box allocated at: 0x1e1222eab20
[C#]  contextPtr value: 0x1E1222EAB20  ← 匹配！
```

### 6.2 正确读取数据
```
[Rust] ui_get_primary_button_id: 0x7ff6fca2dbb0
[C#]  offset 0: 0x7FF6FCA2DBB0  ← 匹配！
```

### 6.3 完整流程验证
```
[C#] 主按钮ID: 11280627291709506204  ← 正确获取
[Info][UI] 注册OnClick回调: widget=11280627291709506204 callback=0x1a3745f5080
[GestureRecognizer] Tap recognized: target=11280627291709506204
[UI] Button text changed to 'hello' via click!  ← 回调触发成功
```

## 七、关键代码修改

| 文件 | 修改内容 |
|------|----------|
| `scripting/src/script_manager.rs:234-236` | `execute_with_ptr`：参数存储到局部变量再传指针 |
| `scripting/src/ffi_context.rs` | 使用`Box<FfiContext>`存储，确保堆内存稳定 |
| `examples/src/mono_ui_thunk_demo.rs` | 清理调试日志，简化FfiContext创建 |
| `scripts/UI.cs` | 清理调试日志 |

## 八、性能对比

| 调用方式 | 延迟 | 使用场景 |
|----------|------|----------|
| 反射（`mono_runtime_invoke`） | ~100μs | Initialize（一次性） |
| Thunk（函数指针） | ~10ns | Update/OnClick（高频调用） |

## 九、架构说明

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│    Rust     │────▶│  FfiContext  │────▶│    C#       │
│  UI FFI     │     │  (堆内存)     │     │   Mono      │
└─────────────┘     └──────────────┘     └─────────────┘
      │                   │                    │
      │                   ▼                    │
      │           函数指针传递                  │
      │         (Marshal.PtrToStructure)       │
      │                   │                    │
      │                   ▼                    │
      │           GetDelegateForFunctionPointer │
      │                   │                    │
      └───────────────────┼────────────────────┘
                          │
                          ▼
                    C#调用Rust FFI
                          │
                          ▼
                    Button点击 → 回调
```

## 十、运行命令

```bash
# 编译C#（使用Mono mcs）
"C:\Program Files\Mono\bin\mcs.bat" -target:library -out:scripts/bin/Mono/TestScript.dll scripts/TestScript.cs scripts/UI.cs

# 运行Demo（release模式，fontdue需要release）
cargo run --bin mono_ui_thunk_demo --features mono --release
```

## 十一、重要经验

1. **Mono `mono_runtime_invoke` 参数规则**：参数必须是指针指向的值，不是值本身
2. **FFI结构体存储**：使用`Box`确保堆内存稳定，避免静态变量地址问题
3. **结构体布局验证**：先对比Rust和C#的size和offset，排除布局不匹配
4. **逐步调试**：从指针传递 → 内存读取 → 函数调用，逐步验证每个环节