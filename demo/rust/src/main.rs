use libloading::Library;
use wrapped_mono::*;

unsafe extern "C" {
    pub fn mono_add_internal_call(name: *const std::os::raw::c_char, method: *const ());
    pub fn mono_object_unbox(obj: *mut std::os::raw::c_void) -> *mut std::os::raw::c_void;
}

// Rust 实现的 Internal Call 函数
#[unsafe(no_mangle)]
pub extern "C" fn fast_add(a: i32, b: i32) -> i32 {
    let result = a + b;
    println!("[Rust] Internal Call: {} + {} = {}", a, b, result);
    result
}

fn main() {
    // 1. 加载 Mono 库
    let mono_lib_path = "C:/Program Files/Mono/bin/mono-2.0-sgen.dll";
    
    unsafe {
        match Library::new(mono_lib_path) {
            Ok(_) => println!("✓ Mono 库加载成功"),
            Err(e) => {
                println!("无法加载 Mono: {}", e);
                return;
            }
        }
    }

    // 2. 初始化 Mono 运行时
    let domain = jit::init("MyGameDomain", None);
    println!("✓ Mono 运行时初始化成功");

    // 3. 注册 Internal Call
    let method_name = std::ffi::CString::new("MyGame.Calculator::FastAdd").unwrap();
    unsafe {
        mono_add_internal_call(method_name.as_ptr(), fast_add as *const ());
    }
    println!("✓ Internal Call 注册成功");

    // 4. 加载 C# 程序集
    let assembly = match domain.assembly_open("./Scripts/MyGameScripts.dll") {
        Some(a) => {
            println!("✓ C# 程序集加载成功");
            a
        }
        None => {
            println!("无法加载程序集");
            return;
        }
    };
    let image = assembly.get_image();

    // 5. 获取 Calculator 类
    let calc_class = match Class::from_name(&image, "MyGame", "Calculator") {
        Some(c) => {
            println!("✓ Calculator 类找到");
            c
        }
        None => {
            println!("找不到 Calculator 类");
            return;
        }
    };

    // 6. 创建实例
    let instance = Object::new(&domain, &calc_class);
    println!("✓ 创建实例成功");

    // 7. 调用构造函数
    if let Some(ctor) = Method::get_from_name(&calc_class, ".ctor", 0) {
        let _ = ctor.invoke(Some(instance.clone()), ());
        println!("✓ 构造函数调用成功");
    } else {
        println!("未找到构造函数");
    }

    // 8. 调用 Add 方法
    if let Some(add_method) = Method::get_from_name(&calc_class, "Add", 2) {
        println!("\n--- 调用 C# Add 方法 ---");
        match add_method.invoke(Some(instance.clone()), (100, 200)) {
            Ok(result_obj) => {
                println!("✅ Add 调用成功");
                if let Some(obj) = result_obj {
                    println!("返回值对象存在");
                } else {
                    println!("返回值为空");
                }
            }
            Err(e) => println!("调用失败: {:?}", e),
        }
    }

    // 9. 调用 FastAdd Internal Call
    if let Some(fast_add_method) = Method::get_from_name(&calc_class, "FastAdd", 2) {
        println!("\n--- 调用 FastAdd (Internal Call) ---");
        match fast_add_method.invoke(None, (300, 400)) {
            Ok(result_obj) => {
                println!("✅ FastAdd Internal Call 执行成功");
                if let Some(obj) = result_obj {
                    println!("返回值对象存在");
                } else {
                    println!("返回值为空");
                }
            }
            Err(e) => println!("调用失败: {:?}", e),
        }
    }

    println!("\n✓ 所有测试完成！");
}