use hezhou_scripting::{MonoExecutor, ScriptExecutor, ScriptValue};

fn main() {
    println!("=== Mono JIT 热更新 Demo ===\n");
    
    println!("[说明]");
    println!("  开发期使用 Mono JIT（支持热更新）");
    println!("  发布版使用 NativeAOT（高性能）");
    println!("  按 R 键触发热更新\n");
    
    println!("[1] 创建 MonoExecutor...");
    let mut executor = MonoExecutor::new("HezhouScripts", "RotationController")
        .expect("Failed to create MonoExecutor");
    println!("    MonoExecutor 初始化成功!\n");
    
    println!("[2] 加载 C# DLL...");
    let dll_path = std::env::current_dir()
        .map(|p| p.join("scripts/bin/Mono/Release/net8.0/RotationScript.Mono.dll"))
        .expect("Failed to get current dir");
    
    println!("    DLL path: {}", dll_path.display());
    
    if !dll_path.exists() {
        println!("    ERROR: DLL 文件不存在!");
        return;
    }
    
    executor.load(dll_path.to_str().expect("Invalid path"))
        .expect("Failed to load DLL");
    
    println!("    assembly_name: {}", executor.assembly_name);
    
    let speed = executor.get_rotation_speed().expect("Failed to get speed");
    println!("    DLL 加载成功! rotation_speed = {}°/s\n", speed);
    
    println!("[3] 运行渲染循环...");
    println!("    每 60 帧输出一次\n");
    
    let mut frame_count = 0u32;
    let mut last_speed = speed;
    
    loop {
        std::thread::sleep(std::time::Duration::from_millis(16));
        
        let dt = 0.016f32;
        let increment = executor.call("UpdateRotation", ScriptValue::from_float(dt))
            .expect("Call failed");
        
        frame_count += 1;
        
        if frame_count % 60 == 0 {
            let current_speed = executor.get_rotation_speed().expect("Failed");
            
            if current_speed != last_speed {
                println!("\n[HotReload] Speed changed: {}°/s -> {}°/s", last_speed, current_speed);
                last_speed = current_speed;
            }
            
            let angle_result = executor.call("GetCurrentAngle", ScriptValue::from_int(0))
                .expect("Failed");
            let angle = angle_result.float_value;
            
            println!("    Frame {}: angle = {:.1}°, speed = {}°/s", frame_count, angle, current_speed);
        }
        
        if frame_count % 180 == 0 {
            println!("\n[提示] 按 R 键热更新（需要外部输入模拟）");
            println!("       编辑 RotationScript.cs 修改 _rotationSpeed");
            println!("       然后运行: dotnet build RotationScript.Mono.csproj -c Release");
            println!("       本 demo 需要手动触发 reload（按 Enter）\n");
            
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();
            
            println!("\n[HotReload] 重新编译...");
            recompile_mono_dll();
            
            println!("[HotReload] 调用 executor.reload()...");
            executor.reload().expect("Reload failed");
            
            let new_speed = executor.get_rotation_speed().expect("Failed");
            println!("[HotReload] 成功! 新速度: {}°/s\n", new_speed);
            last_speed = new_speed;
        }
        
        if frame_count >= 600 {
            println!("\n[Demo] 完成 600 帧，退出");
            break;
        }
    }
    
    executor.unload();
    println!("\n=== Demo Complete ===");
}

fn recompile_mono_dll() {
    let scripts_dir = std::env::current_dir()
        .map(|p| p.join("scripts"))
        .expect("Failed to get scripts dir");
    
    let output = std::process::Command::new("dotnet")
        .arg("build")
        .arg("RotationScript.Mono.csproj")
        .arg("-c")
        .arg("Release")
        .current_dir(&scripts_dir)
        .output()
        .expect("Failed to run dotnet build");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("[HotReload] 编译失败: {}", stderr);
    } else {
        println!("[HotReload] 编译成功!");
    }
}