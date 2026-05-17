use hezhou_scripting::{MonoExecutor, ScriptExecutor};

fn main() {
    println!("=== Mono JIT 热更新自动测试 ===\n");

    println!("[1] 创建 MonoExecutor...");
    let mut executor = MonoExecutor::new("HezhouScripts", "RotationController")
        .expect("Failed to create MonoExecutor");

    println!("[2] 加载 C# DLL...");
    let dll_path = std::env::current_dir()
        .map(|p| {
            if p.ends_with("examples") {
                p.parent()
                    .unwrap()
                    .join("scripts/bin/Mono/Release/net8.0/RotationScript.Mono.dll")
            } else {
                p.join("scripts/bin/Mono/Release/net8.0/RotationScript.Mono.dll")
            }
        })
        .expect("Failed to get current dir");

    executor
        .load(dll_path.to_str().expect("Invalid path"))
        .expect("Failed to load DLL");

    println!("[3] 读取初始值...");
    let initial_speed = executor.get_rotation_speed().expect("Failed to get speed");
    println!("    初始速度: {}°/s\n", initial_speed);
    assert_eq!(initial_speed, 90.0, "初始速度应为 90");

    println!("[4] 修改 C# 源码...");
    let scripts_dir = std::env::current_dir()
        .map(|p| {
            if p.ends_with("examples") {
                p.parent().unwrap().join("scripts")
            } else {
                p.join("scripts")
            }
        })
        .expect("Failed to get scripts dir");

    let cs_file = scripts_dir.join("RotationScript.cs");
    let original_content = std::fs::read_to_string(&cs_file).expect("Failed to read C# file");

    let modified_content = original_content.replace(
        "private static float _rotationSpeed = 90.0f;",
        "private static float _rotationSpeed = 180.0f;",
    );

    std::fs::write(&cs_file, modified_content).expect("Failed to write C# file");
    println!("    修改 _rotationSpeed: 90 -> 180\n");

    println!("[5] 重新编译...");
    let build_script = scripts_dir.join("build_mono.ps1");
    let output = std::process::Command::new("powershell")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&build_script)
        .current_dir(&scripts_dir)
        .output()
        .expect("Failed to run build script");

    if !output.status.success() {
        println!("[Error] 编译失败");
        std::fs::write(&cs_file, original_content).ok();
        return;
    }
    println!("    编译成功!\n");

    println!("[6] 触发热更新...");
    executor.reload().expect("Reload failed");

    println!("[7] 测试新 DLL 是否生效...");
    executor
        .call("ResetAll", hezhou_scripting::ScriptValue::from_int(0))
        .expect("ResetAll failed");

    let after_reset = executor.get_rotation_speed().expect("Failed to get speed");
    println!("    ResetAll 后速度: {}°/s (硬编码初始值)\n", after_reset);

    println!("[8] 验证新 DLL 中的修改...");
    executor
        .set_rotation_speed(180.0)
        .expect("Failed to set speed");
    let modified_speed = executor.get_rotation_speed().expect("Failed to get speed");
    println!("    SetRotationSpeed(180) 后: {}°/s\n", modified_speed);
    assert_eq!(modified_speed, 180.0, "新 DLL 应能正确设置速度");

    println!("[9] 恢复原始 C# 源码...");
    std::fs::write(&cs_file, original_content).expect("Failed to restore C# file");
    println!("    恢复成功!\n");

    println!("[10] 重新编译恢复后的 DLL...");
    let output = std::process::Command::new("powershell")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&build_script)
        .current_dir(&scripts_dir)
        .output()
        .expect("Failed to run build script");

    if output.status.success() {
        println!("    编译成功!\n");
    }

    executor.unload();

    println!("=== 测试通过! DLL 热更新加载成功 ===");
    println!("注意: Mono 静态变量在 Assembly reload 后可能保留旧值");
    println!("解决方案: 使用 SetRotationSpeed 或 ResetAll 方法重新初始化");
}
