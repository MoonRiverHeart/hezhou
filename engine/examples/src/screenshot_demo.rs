use hezhou_rhi_vulkan::UIVulkanRenderer;
use hezhou_ui::ffi as ui_ffi;
use hezhou_ui::ffi::WidgetTreeHandle;
use hezhou_dfx::*;
use chrono::Local;
use std::ffi::CString;

fn main() {
    let dfx = init_dfx();
    dfx.lock().get_logger().lock().set_level(LogLevel::Info);
    
    let log_path = format!("logs/screenshot_{}.log", Local::now().format("%Y-%m-%d"));
    std::fs::create_dir_all("logs").ok();
    if let Err(e) = dfx.lock().get_logger().lock().enable_file_output(&log_path) {
        dfx_error!("Screenshot", "Failed to enable file output: {}", e);
    }
    
    dfx_info!("Screenshot", "=== Screenshot Tool ===");
    dfx_info!("Screenshot", "Log file: {}", log_path);
    
    std::fs::create_dir_all("screenshots").ok();
    dfx_info!("Screenshot", "Screenshots will be saved to screenshots/");
    
    dfx_info!("Screenshot", "[1] Creating window...");
    let mut renderer = UIVulkanRenderer::new(800, 600, "Screenshot Tool")
        .expect("Failed to create renderer");
    dfx_info!("Screenshot", "Window created!");
    
    dfx_info!("Screenshot", "[2] Setting up UI...");
    renderer.setup_ui_for_script();
    
    let (width, height) = renderer.get_extent();
    let content_scale = renderer.get_content_scale();
    ui_ffi::ui_set_screen_size(width as f32, height as f32);
    ui_ffi::ui_set_content_scale(content_scale);
    
    let handle: WidgetTreeHandle = renderer.get_widget_tree_handle() as WidgetTreeHandle;
    
    let root_id = ui_ffi::ui_get_root_id(handle);
    dfx_info!("Screenshot", "RootId={}", root_id);
    
    let title_text = CString::new("Screenshot Tool").unwrap();
    let title_id = ui_ffi::ui_create_label_in_parent(
        handle, 
        root_id, 
        width as f32 - 40.0 * content_scale,
        40.0 * content_scale,
        title_text.as_ptr()
    );
    ui_ffi::ui_widget_set_position(handle, title_id, 20.0 * content_scale, 20.0 * content_scale);
    dfx_info!("Screenshot", "Title created: {}", title_id);
    
    let button_text = CString::new("Take Screenshot (S)").unwrap();
    let button_x = (width as f32 - 200.0 * content_scale) / 2.0;
    let button_y = (height as f32 - 60.0 * content_scale) / 2.0 - 30.0 * content_scale;
    let button_id = ui_ffi::ui_create_button_in_parent(
        handle,
        root_id,
        200.0 * content_scale,
        40.0 * content_scale,
        button_text.as_ptr()
    );
    ui_ffi::ui_widget_set_position(handle, button_id, button_x, button_y);
    dfx_info!("Screenshot", "Button created: {}", button_id);
    
    let status_text = CString::new("Press S to take screenshot").unwrap();
    let status_y = (height as f32 - 60.0 * content_scale) / 2.0 + 30.0 * content_scale;
    let status_id = ui_ffi::ui_create_label_in_parent(
        handle,
        root_id,
        width as f32 - 40.0 * content_scale,
        30.0 * content_scale,
        status_text.as_ptr()
    );
    ui_ffi::ui_widget_set_position(handle, status_id, 20.0 * content_scale, status_y);
    dfx_info!("Screenshot", "Status created: {}", status_id);
    
    dfx_info!("Screenshot", "UI created!");
    dfx_info!("Screenshot", "[3] Starting main loop...");
    dfx_info!("Screenshot", "Press S key to take screenshot");
    
    let mut screenshot_count = 0u32;
    
    loop {
        renderer.process_events();
        
        let should_screenshot = renderer.is_s_pressed();
        
        if should_screenshot {
            renderer.consume_s_press();
            
            screenshot_count += 1;
            let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
            let filename = format!("screenshots/screenshot_{}.png", timestamp);
            
            dfx_info!("Screenshot", "Taking screenshot #{}...", screenshot_count);
            
            if let Err(e) = renderer.capture_screenshot(&filename) {
                dfx_error!("Screenshot", "Failed to capture: {}", e);
            } else {
                dfx_info!("Screenshot", "Screenshot saved: {}", filename);
                let new_text = CString::new(format!("Saved: {}", filename)).unwrap();
                ui_ffi::ui_widget_set_text(handle, status_id, new_text.as_ptr());
            }
        }
        
        match renderer.draw_frame() {
            Ok(true) => {}
            Ok(false) => break,
            Err(e) => {
                dfx_error!("Screenshot", "Draw error: {}", e);
                break;
            }
        }
    }
    
    dfx_info!("Screenshot", "Cleanup...");
    renderer.cleanup();
    dfx_info!("Screenshot", "=== Screenshot Tool Closed ===");
}