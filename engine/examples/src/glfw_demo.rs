use hezhou_platform::*;
use hezhou_core::*;
use hezhou_core::ffi::*;
use hezhou_scripting::*;
use std::ffi::CString;

fn main() {
    println!("=== Hezhou Engine - GLFW Platform Demo ===\n");
    
    println!("[1] 初始化 GLFW 平台...");
    let manager_ptr = platform_manager_create();
    let result = platform_init_glfw(manager_ptr);
    if result != 0 {
        println!("    GLFW 初始化失败!");
        return;
    }
    println!("    GLFW 初始化成功!\n");
    
    println!("[2] 创建窗口...");
    let title = CString::new("Hezhou Engine - GLFW").unwrap();
    let window = platform_create_window(manager_ptr, title.as_ptr(), 800, 600);
    println!("    Window: type={}, ptr={}, size={}x{}\n", 
        window.window_type as i32, window.ptr, window.width, window.height);
    
    println!("[3] 初始化引擎...");
    let engine = engine_create();
    engine_start(engine);
    println!("    Engine started\n");
    
    println!("[4] 初始化脚本系统...");
    let script_mgr = scripting_init();
    
    extern "C" fn on_frame(_arg: ScriptValue, context: usize) -> ScriptValue {
        let frame = context as i32;
        println!("    Frame callback: {}", frame);
        ScriptValue::from_int(frame + 1)
    }
    
    let cb_name = CString::new("on_frame").unwrap();
    let cb_desc = CString::new("Frame callback").unwrap();
    let cb_sig = CString::new("int -> int").unwrap();
    
    scripting_register_sync_callback(
        script_mgr, cb_name.as_ptr(),
        on_frame, cb_desc.as_ptr(), cb_sig.as_ptr(),
        0,
    );
    println!("    Script callback registered\n");
    
    println!("[5] 主循环...");
    let mut frame_count = 0;
    let max_frames = 100;
    
    while platform_is_running(manager_ptr) && frame_count < max_frames {
        let event_count = platform_poll_events(manager_ptr);
        let time = platform_get_time(manager_ptr);
        
        engine_run_frame(engine, 0.016);
        
        frame_count += 1;
        
        if event_count > 0 || frame_count % 30 == 0 {
            println!("    Frame {}: time={:.3}s, events={}", frame_count, time, event_count);
        }
        
        platform_sleep(manager_ptr, 0.016);
    }
    println!();
    
    println!("[6] 清理...");
    scripting_shutdown(script_mgr);
    engine_stop(engine);
    engine_destroy(engine);
    
    platform_request_quit(manager_ptr);
    platform_manager_destroy(manager_ptr);
    println!("    清理完成\n");
    
    println!("=== Demo Complete ===");
}

extern "C" fn platform_sleep(manager: *mut hezhou_platform::PlatformManager, seconds: f64) {
    std::thread::sleep(std::time::Duration::from_secs_f64(seconds));
}