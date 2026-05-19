use hezhou_rhi_vulkan::UIVulkanRenderer;
use hezhou_dfx::*;
use hezhou_ui::ffi as ui_ffi;
use std::ffi::CString;
use std::fs;

fn main() {
    let dfx = init_dfx();
    dfx.lock().get_logger().lock().set_level(LogLevel::Info);
    
    let log_path = format!("logs/trace_viewer_{}.log", chrono::Local::now().format("%Y-%m-%d"));
    let _ = dfx.lock().get_logger().lock().enable_file_output(&log_path);
    
    dfx_info!("TraceViewer", "=== Trace Viewer ===");
    
    dfx_info!("TraceViewer", "Loading trace file...");
    let trace_data = load_trace_file("traces/trace_latest.json");
    
    if trace_data.is_none() {
        dfx_error!("TraceViewer", "No trace file found. Run mono_editor_demo first to generate trace.");
        std::process::exit(1);
    }
    
    let trace = trace_data.unwrap();
    dfx_info!("TraceViewer", "Loaded {} trace events", trace.events.len());
    
    dfx_info!("TraceViewer", "Creating window...");
    let mut renderer = UIVulkanRenderer::new(1280, 720, "Trace Viewer")
        .expect("Failed to create renderer");
    
    renderer.setup_ui_for_script();
    
    let handle = renderer.get_widget_tree_handle() as ui_ffi::WidgetTreeHandle;
    let root_id = ui_ffi::ui_get_root_id(handle);
    
    dfx_info!("TraceViewer", "Building UI...");
    build_trace_ui(handle, root_id, &trace);
    
    dfx_info!("TraceViewer", "Starting main loop...");
    
    loop {
        renderer.process_events();
        
        match renderer.draw_frame() {
            Ok(running) => {
                if !running {
                    break;
                }
            }
            Err(e) => {
                dfx_error!("TraceViewer", "{}", e);
                break;
            }
        }
        
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    
    dfx_info!("TraceViewer", "Cleanup...");
    renderer.cleanup();
    dfx_info!("TraceViewer", "=== Trace Viewer Closed ===");
}

struct TraceData {
    events: Vec<TraceEvent>,
    min_time: u64,
    max_time: u64,
    threads: Vec<u64>,
}

struct TraceEvent {
    name: String,
    category: String,
    phase: String,
    timestamp: u64,
    duration: u64,
    thread_id: u64,
}

fn load_trace_file(path: &str) -> Option<TraceData> {
    let content = fs::read_to_string(path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    
    let events_array = json.get("traceEvents")?.as_array()?;
    
    let mut events: Vec<TraceEvent> = Vec::new();
    let mut min_time = u64::MAX;
    let mut max_time = 0u64;
    let mut threads: Vec<u64> = Vec::new();
    
    for event_val in events_array {
        let name = event_val.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let category = event_val.get("cat")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let phase = event_val.get("ph")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let timestamp = event_val.get("ts")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        
        let duration = event_val.get("dur")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        
        let thread_id = event_val.get("tid")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        
        if timestamp < min_time {
            min_time = timestamp;
        }
        if timestamp + duration > max_time {
            max_time = timestamp + duration;
        }
        
        if !threads.contains(&thread_id) {
            threads.push(thread_id);
        }
        
        events.push(TraceEvent {
            name,
            category,
            phase,
            timestamp,
            duration,
            thread_id,
        });
    }
    
    Some(TraceData {
        events,
        min_time,
        max_time,
        threads,
    })
}

fn build_trace_ui(handle: ui_ffi::WidgetTreeHandle, root_id: u64, trace: &TraceData) {
    let title_c = CString::new("Trace Viewer").unwrap();
    ui_ffi::ui_create_label_in_parent(handle, root_id, 500.0, 30.0, title_c.as_ptr());
    
    let time_range = trace.max_time.saturating_sub(trace.min_time);
    let scale = if time_range > 0 { 1100.0 / time_range as f32 } else { 1.0 };
    
    let swimlane_height = 50.0;
    let swimlane_start_y = 60.0;
    
    for (i, thread_id) in trace.threads.iter().enumerate() {
        let lane_y = swimlane_start_y + i as f32 * swimlane_height;
        
        ui_ffi::ui_create_panel_in_parent(
            handle,
            root_id,
            160.0,
            lane_y,
            1100.0,
            swimlane_height - 5.0,
            0.2,
            0.2,
            0.25,
            1.0
        );
        
        for event in trace.events.iter().filter(|e| e.thread_id == *thread_id) {
            if event.phase != "X" {
                continue;
            }
            
            let event_x = 160.0 + (event.timestamp.saturating_sub(trace.min_time)) as f32 * scale;
            let event_width = (event.duration as f32 * scale).max(2.0);
            
            let color = get_category_color(&event.category);
            
            ui_ffi::ui_create_panel_in_parent(
                handle,
                root_id,
                event_x,
                lane_y + 5.0,
                event_width,
                swimlane_height - 15.0,
                color.0,
                color.1,
                color.2,
                1.0
            );
            
            if event_width > 50.0 {
                let event_label = CString::new(event.name.clone()).unwrap();
                let label_id = ui_ffi::ui_create_label_in_parent(handle, root_id, event_width - 4.0, 15.0, event_label.as_ptr());
                ui_ffi::ui_set_widget_layout(handle, label_id, event_x + 2.0, lane_y + 10.0, event_width - 4.0, 15.0);
            }
        }
    }
}

fn get_category_color(category: &str) -> (f32, f32, f32) {
    match category {
        "render" => (0.2, 0.6, 0.8),
        "logic" => (0.8, 0.4, 0.2),
        "ui" => (0.4, 0.8, 0.4),
        "script" => (0.8, 0.6, 0.2),
        _ => (0.5, 0.5, 0.5),
    }
}