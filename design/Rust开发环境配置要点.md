# Rust 开发环境配置要点

## 1. Rust 开发环境配置

### 1.1 版本选择
- **Rust 2024 Edition 尚未稳定发布**，需要使用 nightly 工具链
- 当前 Rust 最新稳定版是 **2021 Edition**
- 设置 `edition = "2024"` 会导致编译器进入实验模式，要求使用 `#[unsafe(no_mangle)]` 替代 `#[no_mangle]`

### 1.2 C# 项目配置要点
```xml
<!-- 必须启用不安全代码 -->
<AllowUnsafeBlocks>true</AllowUnsafeBlocks>
```

- C# 调用 Rust DLL 时，`[DllImport]` 必须指定 `CallingConvention = CallingConvention.Cdecl`
- 字符串处理：Rust 分配内存，C# 调用 `Marshal.PtrToStringAnsi()` 读取后必须调用 `free_*` 函数释放

## 2. NuGet 包管理问题

### 问题
- C# 项目只有离线包源 `Microsoft Visual Studio Offline Packages`
- 缺少 `nuget.org` 导致找不到 `csbindgen`

### 解决方案
```powershell
# 添加 nuget.org 源
dotnet nuget add source "https://api.nuget.org/v3/index.json" --name nuget.org
```

## 3. 跨语言调用 Rust

### 3.1 csbindgen 版本差异

| 平台 | 版本号 |
|------|--------|
| Rust crate (`Cargo.toml`) | 1.9.7 |
| C# NuGet (`.csproj`) | 1.9.7 |

> 注意：两个版本应保持一致，避免生成代码不兼容。

### 3.2 项目结构优化
- 避免同时存在 `lib.rs` 和 `main.rs` 导致冲突
- 使用 `cargo build --lib` 只编译库
- 将示例代码放在 `examples/` 目录

### 3.3 完整项目结构说明

#### 整体目录结构
```
demo/
├── rust/          # Rust 库项目（cdylib）
├── csharp/        # C# 调用端项目
└── harmony/       # 鸿蒙应用项目
```

#### rust/ 文件夹详解
```
rust/
├── Cargo.toml          # 项目配置，定义 cdylib 和 bin 目标
├── Cargo.lock          # 依赖锁定文件
├── build.rs            # csbindgen 代码生成脚本
├── src/
│   ├── lib.rs          # 核心库：导出 C 兼容函数
│   └── main.rs         # 测试用二进制入口（条件编译）
├── tools/
│   ├── build_harmony.ps1   # 鸿蒙交叉编译脚本
│   └── RustMirror.ps1      # Rust 镜像源配置脚本
└── target/             # 编译产物目录
```

**Cargo.toml 关键配置**：
```toml
[package]
name = "csharptorust_lib"
edition = "2024"

[lib]
crate-type = ["cdylib"]      # 生成动态链接库
name = "csharptorust_lib"
path = "src/lib.rs"

[[bin]]                       # 可选：测试用二进制
name = "test_app"
path = "src/main.rs"
target = ["x86_64-pc-windows-msvc"]  # 仅 Windows 编译
doc = false
test = false
bench = false

[build-dependencies]
csbindgen = "1.9.7"          # 代码生成工具

# 鸿蒙交叉编译目标配置
[target.aarch64-unknown-linux-ohos]
linker = "clang++.exe 路径"
ar = "llvm-ar.exe 路径"
rustflags = [
    "-C", "link-arg=--target=aarch64-unknown-linux-ohos",
    "-C", "link-arg=--sysroot=.../sysroot",
    "-C", "link-arg=-L.../sysroot/usr/lib/aarch64-linux-ohos"
]
```

**build.rs 作用**：
```rust
// 扫描 src/lib.rs 中的 extern "C" 函数
// 自动生成 C# P/Invoke 绑定代码
csbindgen::Builder::default()
    .input_extern_file("src/lib.rs")
    .csharp_dll_name("csharptorust_lib")
    .generate_csharp_file("../csharp/CsharpCaller/NativeMethods.cs")
    .unwrap();
```

**lib.rs 函数导出规范**：
```rust
// 2024 Edition 必须使用 #[unsafe(no_mangle)]
#[unsafe(no_mangle)]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}

// 字符串返回：使用 CString 转移所有权
#[unsafe(no_mangle)]
pub extern "C" fn get_message() -> *mut c_char {
    let msg = CString::new("Hello from Rust").unwrap();
    msg.into_raw()  // 防止 Rust 自动释放
}

// 配套释放函数（C# 必须调用）
#[unsafe(no_mangle)]
pub extern "C" fn free_message(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { drop(CString::from_raw(ptr)) };
    }
}
```

#### csharp/ 文件夹详解
```
csharp/
├── CsharpCaller.csproj     # 项目文件，配置 DLL 自动复制
├── Program.cs              # 主程序：演示调用 Rust 函数
├── CsharpCaller/
│   └── NativeMethods.cs    # csbindgen 自动生成的绑定代码
├── bin/                    # 编译输出
└── obj/                    # 中间文件
```

**CsharpCaller.csproj 关键配置**：
```xml
<PropertyGroup>
  <OutputType>Exe</OutputType>
  <TargetFramework>net8.0</TargetFramework>
  <Nullable>enable</Nullable>
  <ImplicitUsings>enable</ImplicitUsings>
  <PlatformTarget>x64</PlatformTarget>
  <AllowUnsafeBlocks>true</AllowUnsafeBlocks>  <!-- 必须：csbindgen 生成 unsafe 代码 -->
</PropertyGroup>

<ItemGroup>
  <PackageReference Include="csbindgen" Version="1.9.7" />
</ItemGroup>

<!-- 自动复制 Rust DLL 到 C# 输出目录 -->
<Target Name="CopyRustDll" AfterTargets="Build">
  <Copy SourceFiles="..\rust\target\release\csharptorust_lib.dll" 
        DestinationFolder="$(OutputPath)" 
        SkipUnchangedFiles="true" 
        Condition="Exists('..\rust\target\release\csharptorust_lib.dll')" />
</Target>
```

**Program.cs 调用示例**：
```csharp
// 手动声明 P/Invoke（不依赖生成代码时）
[DllImport("csharptorust_lib.dll", CallingConvention = CallingConvention.Cdecl)]
public static extern int add(int a, int b);

[DllImport("csharptorust_lib.dll", CallingConvention = CallingConvention.Cdecl)]
public static extern IntPtr get_message();

[DllImport("csharptorust_lib.dll", CallingConvention = CallingConvention.Cdecl)]
public static extern void free_message(IntPtr ptr);

// 使用示例
IntPtr ptr = get_message();
string? message = Marshal.PtrToStringAnsi(ptr);
free_message(ptr);  // 必须释放，否则内存泄漏
```

**NativeMethods.cs（csbindgen 自动生成）**：
```csharp
// 注意：此文件由 build.rs 自动生成，不应手动修改
namespace CsBindgen
{
    internal static unsafe partial class NativeMethods
    {
        const string __DllName = "csharptorust_lib";

        [DllImport(__DllName, EntryPoint = "add", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int add(int a, int b);

        [DllImport(__DllName, EntryPoint = "get_message", CallingConvention = CallingConvention.Cdecl)]
        internal static extern byte* get_message();

        [DllImport(__DllName, EntryPoint = "free_message", CallingConvention = CallingConvention.Cdecl)]
        internal static extern void free_message(byte* ptr);
    }
}
```

#### harmony/ 文件夹详解
```
harmony/
├── build-profile.json5     # 应用构建配置（SDK 版本、签名等）
├── oh-package.json5        # 依赖管理（类似 package.json）
├── hvigorfile.ts           # 构建脚本入口
├── local.properties        # 本地配置（SDK 路径等）
├── entry/                  # 主模块
│   ├── build-profile.json5
│   ├── oh-package.json5
│   └── src/
│       ├── main/           # 主源码
│       │   ├── ets/
│       │   │   ├── entryability/
│       │   │   │   └── EntryAbility.ets  # 应用入口
│       │   │   └── pages/
│       │   │       └── Index.ets         # 主页面
│       │   ├── resources/                # 资源文件
│       │   └── module.json5              # 模块配置
│       ├── ohosTest/       # 自动化测试
│       ├── mock/           # Mock 数据
│       └── test/           # 单元测试
└── AppScope/               # 应用全局配置
```

**build-profile.json5 关键配置**：
```json5
{
  "app": {
    "products": [
      {
        "name": "default",
        "targetSdkVersion": "6.0.0(20)",      // 目标 SDK 版本
        "compatibleSdkVersion": "6.0.0(20)",  // 兼容 SDK 版本
        "runtimeOS": "HarmonyOS"
      }
    ],
    "buildModeSet": [
      { "name": "debug" },
      { "name": "release" }
    ]
  },
  "modules": [
    {
      "name": "entry",
      "srcPath": "./entry"
    }
  ]
}
```

**EntryAbility.ets（应用生命周期）**：
```typescript
export default class EntryAbility extends UIAbility {
  onCreate(want: Want, launchParam: AbilityConstant.LaunchParam): void {
    // 应用创建时调用
  }

  onWindowStageCreate(windowStage: window.WindowStage): void {
    // 加载主页面
    windowStage.loadContent('pages/Index', (err) => { ... });
  }

  onDestroy(): void { }
  onForeground(): void { }
  onBackground(): void { }
}
```

**Index.ets（ArkUI 页面）**：
```typescript
@Entry
@Component
struct Index {
  @State message: string = 'Hello World';

  build() {
    RelativeContainer() {
      Text(this.message)
        .fontSize($r('app.float.page_text_font_size'))
        .fontWeight(FontWeight.Bold)
        .onClick(() => {
          this.message = 'Welcome';  // 状态更新触发 UI 重绘
        })
    }
  }
}
```

### 3.4 tools/ 脚本说明

#### build_harmony.ps1（鸿蒙交叉编译）
```powershell
# 设置鸿蒙 SDK 路径
$OhosSdk = "C:\...\commandline-tools-windows-x64\...\native"

# 配置链接器和 sysroot
$env:RUSTFLAGS = "-C linker=$OhosSdk\llvm\bin\clang++.exe -C link-arg=--target=aarch64-linux-ohos -C link-arg=--sysroot=$OhosSdk\sysroot"

# 使用 nightly + build-std 编译
cargo +nightly build -Zbuild-std --target aarch64-unknown-linux-ohos --release --lib

# 输出产物：target\aarch64-unknown-linux-ohos\release\libcsharptorust_lib.so
```

#### RustMirror.ps1（国内镜像源配置）
```powershell
# 启用 rsproxy.cn 镜像
.\RustMirror.ps1 set

# 清除镜像配置
.\RustMirror.ps1 clear

# 查看当前状态
.\RustMirror.ps1 status

# 帮助
.\RustMirror.ps1 help
```

设置的环境变量：
- `RUSTUP_DIST_SERVER=https://rsproxy.cn`
- `RUSTUP_UPDATE_ROOT=https://rsproxy.cn/rustup`

## 4. PowerShell 编码问题

### 问题根源
- **PowerShell 5.x** 默认按 ANSI (GBK) 读取脚本
- **VS Code 默认编码**是 UTF-8 without BOM
- 编码不匹配导致中文乱码

### 解决方案

| 方案 | 说明 |
|------|------|
| 升级到 PowerShell 7 | 原生支持 UTF-8 |
| 保存为 UTF-8 with BOM | VS Code 选择带 BOM 编码 |
| 使用纯英文脚本 | 完全避免中文 |
| 逐行执行不保存文件 | 临时解决方案 |

## 5. 鸿蒙交叉编译

### 5.1 工具链确认
```powershell
# 验证交叉编译器
clang++.exe -target aarch64-linux-ohos --sysroot=... test.cpp -o test
# 成功时无输出，只生成文件
```

### 5.2 Rust 交叉编译配置

#### .cargo/config.toml 配置
```toml
[target.aarch64-unknown-linux-ohos]
linker = "path/to/clang++.exe"
rustflags = [
    "-C", "link-arg=--target=aarch64-linux-ohos",
    "-C", "link-arg=--sysroot=path/to/sysroot",
]
```

#### 编译命令
```powershell
# 必须使用 nightly + -Zbuild-std
cargo +nightly build -Zbuild-std --target aarch64-unknown-linux-ohos --release --lib

# 通过环境变量传递链接器参数
$env:RUSTFLAGS = "-C linker=... -C link-arg=--target=... -C link-arg=--sysroot=..."
```

### 5.3 常见错误及解决

| 错误 | 原因 | 解决 |
|------|------|------|
| `linker 'cc' not found` | Rust 找不到链接器 | 设置 `RUSTFLAGS` 或 `.cargo/config.toml` |
| `can't find crate for 'std'` | 目标无预编译标准库 | 使用 `-Zbuild-std` |
| 引号转义错误 | Windows 路径中的引号问题 | 使用 `.cargo/config.toml` 配置 |

## 6. 关键命令速查

### 6.1 Rust 编译
```bash
# 调试版本
cargo build

# 发布版本
cargo build --release

# 只编译库
cargo build --lib

# 清理
cargo clean
```

### 6.2 交叉编译鸿蒙
```powershell
# C/C++ 测试
clang++.exe -target aarch64-linux-ohos --sysroot=... test.cpp -o test

# Rust
cargo +nightly build -Zbuild-std --target aarch64-unknown-linux-ohos --release --lib
```

### 6.3 C# 项目
```powershell
# 编译
dotnet build

# 运行
dotnet run

# 添加 NuGet 源
dotnet nuget add source https://api.nuget.org/v3/index.json --name nuget.org
```

## 7. 最佳实践建议

1. **日常开发**：使用 Rust 2021 Edition + stable 工具链
2. **C# 调用**：只用 `cdylib`，避免同时编译 bin
3. **编码问题**：推荐升级到 PowerShell 7 彻底解决
4. **跨平台**：优先使用 `.cargo/config.toml` 配置链接器，避免环境变量传递问题
5. **调试顺序**：先用 C/C++ 测试交叉工具链是否正常，再调试 Rust 编译
6. **内存管理**：Rust 返回的字符串必须由 C# 调用对应的 `free_*` 函数释放
7. **代码生成**：csbindgen 的 `build.rs` 会在 `cargo build` 时自动运行，生成的 `NativeMethods.cs` 不应手动修改
8. **DLL 部署**：在 `.csproj` 中配置 `CopyRustDll` Target 实现自动复制
