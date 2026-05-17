use hezhou_scripting::MonoUIExecutor;
use hezhou_ui::{UISystem, trigger_update_callback, trigger_onclick_callback, ui_clear_callbacks};
use hezhou_dfx::{DfxSystem, LogLevel};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn main() {
    println!("=== Thunk + Mono JIT UI Demo ===\n");
    
    println!("[架构说明]");
    println!("  C#: Initialize() → 创建UI + 注册Thunk");
    println!("  Rust: trigger_update_callback(dt) → thunk(dt)");
    println!("  Rust: trigger_onclick_callback(id) → thunk(id)");
    println!("  调用开销: ~10ns (函数指针直接调用)");
    println!("  热重载: 按 R 键重新加载DLL\n");
    
    let dfx = Arc::new(Mutex::new(DfxSystem::new()));
    dfx.lock().get_logger().lock().set_level(LogLevel::Trace);
    
    println!("[1] 创建UISystem...");
    let ui_system = Arc::new(Mutex::new(UISystem::new()));
    
    println!("[2] 编译C#脚本...");
    let dll_path = compile_ui_script();
    println!("    编译成功: {}\n", dll_path);
    
    println!("[3] 加载Mono DLL...");
    let executor = MonoUIExecutor::new(&dll_path)
        .expect("Failed to load Mono DLL");
    println!("    加载成功!\n");
    
    println!("[4] 调用Initialize...");
    executor.call_static_void("UIScript", "Initialize", &[])
        .expect("Initialize failed");
    println!("    Initialize调用成功!\n");
    
    println!("[5] 模拟运行循环...\n");
    
    let mut frame = 0;
    let mut last_time = Instant::now();
    
    for i in 0..100 {
        frame = i;
        
        let now = Instant::now();
        let delta_time = now.duration_since(last_time).as_secs_f32();
        last_time = now;
        
        if frame % 20 == 0 {
            println!("[Frame {}] 调用Update Thunk...", frame);
            trigger_update_callback(delta_time);
            
            let fps = ui_system.lock().get_fps();
            println!("    FPS: {:.1}", fps);
        }
        
        if frame == 40 {
            println!("\n[模拟点击] Button被点击!");
            let button_id = get_button_id();
            trigger_onclick_callback(button_id);
            println!("    OnClick Thunk调用完成\n");
        }
        
        std::thread::sleep(Duration::from_millis(16));
    }
    
    println!("[6] 测试热重载...");
    ui_clear_callbacks();
    
    println!("    重新编译C#脚本...");
    let new_dll_path = compile_ui_script_with_timestamp();
    
    println!("    加载新DLL...");
    let executor2 = MonoUIExecutor::new(&new_dll_path)
        .expect("Failed to reload");
    
    executor2.call_static_void("UIScript", "Initialize", &[])
        .expect("Initialize failed");
    println!("    热重载成功!\n");
    
    println!("[7] 继续运行...");
    for i in 100..120 {
        frame = i;
        
        let delta_time = 0.016;
        
        if frame % 20 == 0 {
            println!("[Frame {}] 调用Update Thunk...", frame);
            trigger_update_callback(delta_time);
        }
        
        if frame == 110 {
            println!("\n[模拟点击] Button被点击 (热重载后)!");
            let button_id = get_button_id();
            trigger_onclick_callback(button_id);
            println!("    OnClick Thunk调用完成\n");
        }
        
        std::thread::sleep(Duration::from_millis(16));
    }
    
    println!("\n=== Demo Complete ===");
    println!("\n总结:");
    println!("  - Thunk调用开销: ~10ns (函数指针)");
    println!("  - Mono反射调用: ~100μs (仅Initialize)");
    println!("  - 热重载成功");
}

fn compile_ui_script() -> String {
    use std::process::Command;
    
    let scripts_dir = "scripts";
    let output_dir = "scripts/bin/Mono";
    
    std::fs::create_dir_all(output_dir).unwrap();
    
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let output_dll = format!("{}/UIScript_{}.dll", output_dir, timestamp);
    
    let args = [
        "-target:library".to_string(),
        format!("-out:{}", output_dll),
        "-unsafe".to_string(),
        format!("{}/UIScript.cs", scripts_dir),
        format!("{}/DFX.cs", scripts_dir),
        format!("{}/UI.cs", scripts_dir),
    ];
    
    let result = Command::new("mcs")
        .args(&args)
        .output();
    
    match result {
        Ok(output) => {
            if output.status.success() {
                output_dll
            } else {
                println!("Compilation failed: {}", String::from_utf8_lossy(&output.stderr));
                panic!("Mono compilation failed");
            }
        }
        Err(_) => {
            println!("mcs not found, using precompiled DLL");
            find_latest_dll(output_dir).unwrap_or_else(|| panic!("No precompiled DLL found in {}", output_dir))
        }
    }
}

fn find_latest_dll(output_dir: &str) -> Option<String> {
    use std::fs;
    
    let entries: Vec<_> = fs::read_dir(output_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "dll").unwrap_or(false))
        .filter(|e| e.file_name().to_str().map(|name| name.starts_with("UIScript_")).unwrap_or(false))
        .collect();
    
    if entries.is_empty() {
        return None;
    }
    
    let latest = entries
        .iter()
        .max_by_key(|e| e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH));
    
    latest.map(|e| e.path().to_str().unwrap().to_string())
}

fn compile_ui_script_with_timestamp() -> String {
    compile_ui_script()
}

fn get_button_id() -> u64 {
    123
}