use hezhou_rhi_vulkan::UIVulkanRenderer;
use hezhou_dfx::*;
use hezhou_ui::ffi as ui_ffi;
use std::ffi::CString;
use std::fs;
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    let trace_file = args.iter().position(|a| a == "--file")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| {
            list_available_traces();
            "traces/trace_latest.json".to_string()
        });
    
    let screenshot_mode = args.iter().any(|a| a == "--screenshot");
    let screenshot_delay = if screenshot_mode { 
        args.iter().position(|a| a == "--delay")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(1.0)
    } else { 0.0 };
    let screenshot_path = if screenshot_mode {
        args.iter().position(|a| a == "--output")
            .and_then(|i| args.get(i + 1))
            .cloned()
            .unwrap_or_else(|| "screenshots/trace_viewer.png".to_string())
    } else { String::new() };
    
    let dfx = init_dfx();
    dfx.lock().get_logger().lock().set_level(LogLevel::Info);
    
    let log_path = format!("logs/trace_viewer_{}.log", chrono::Local::now().format("%Y-%m-%d"));
    let _ = dfx.lock().get_logger().lock().enable_file_output(&log_path);
    
    dfx_info!("TraceViewer", "=== Trace Viewer (Enhanced) ===");
    dfx_info!("TraceViewer", "Loading: {}", trace_file);
    
    let trace_data = load_trace_file(&trace_file);
    
    if trace_data.is_none() {
        dfx_error!("TraceViewer", "No trace file found: {}", trace_file);
        dfx_info!("TraceViewer", "Run mono_editor_demo first to generate trace.");
        std::process::exit(1);
    }
    
    let trace = trace_data.unwrap();
    dfx_info!("TraceViewer", "Loaded {} events, {} counters", trace.events.len(), trace.counter_events.len());
    
    let stats = calculate_stats(&trace);
    let counter_stats = calculate_counter_stats(&trace);
    dfx_info!("TraceViewer", "Stats: {} frames, avg {:.2}ms, max {:.2}ms", 
             stats.frame_count, stats.avg_frame_time, stats.max_frame_time);
    if !counter_stats.is_empty() {
        for (name, cs) in &counter_stats {
            dfx_info!("TraceViewer", "Counter {}: avg={}, min={}, max={}", name, cs.avg, cs.min, cs.max);
        }
    }
    
    dfx_info!("TraceViewer", "Creating window...");
    let mut renderer = UIVulkanRenderer::new(1280, 720, "Trace Viewer (Enhanced)")
        .expect("Failed to create renderer");
    
    renderer.setup_ui_for_script();
    
    let handle = renderer.get_widget_tree_handle() as ui_ffi::WidgetTreeHandle;
    let root_id = ui_ffi::ui_get_root_id(handle);
    
    dfx_info!("TraceViewer", "Building UI...");
    let (initial_w, initial_h) = renderer.get_extent();
    build_trace_ui(handle, root_id, &trace, &stats, &counter_stats, initial_w as f32, initial_h as f32);
    
    if screenshot_mode {
        std::fs::create_dir_all("screenshots").ok();
        dfx_info!("TraceViewer", "[Screenshot mode] delay={}s, output={}", screenshot_delay, screenshot_path);
    } else {
        dfx_info!("TraceViewer", "Starting main loop...");
        dfx_info!("TraceViewer", "Press S to screenshot the swimlane visualization");
        std::fs::create_dir_all("screenshots").ok();
    }
    
    let start_time = Instant::now();
    let mut screenshot_taken = false;
    let mut last_width = initial_w;
    let mut last_height = initial_h;

    loop {
        renderer.process_events();
        
        // Check for window resize and rebuild UI
        let (current_w, current_h) = renderer.get_extent();
        if current_w != last_width || current_h != last_height {
            dfx_info!("TraceViewer", "Window resized: {}x{} -> {}x{}", last_width, last_height, current_w, current_h);
            ui_ffi::ui_clear_widget_tree(handle);
            build_trace_ui(handle, root_id, &trace, &stats, &counter_stats, current_w as f32, current_h as f32);
            last_width = current_w;
            last_height = current_h;
        }
        
        if screenshot_mode && !screenshot_taken {
            let elapsed = start_time.elapsed().as_secs_f32();
            if elapsed >= screenshot_delay {
                dfx_info!("TraceViewer", "Taking screenshot...");
                if let Err(e) = renderer.capture_screenshot(&screenshot_path) {
                    dfx_error!("TraceViewer", "Screenshot failed: {}", e);
                } else {
                    dfx_info!("TraceViewer", "Screenshot saved: {}", screenshot_path);
                }
                screenshot_taken = true;
                break;
            }
        }
        
        if !screenshot_mode {
            let should_screenshot = renderer.is_s_pressed();
            if should_screenshot {
                renderer.consume_s_press();
                
                let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
                let filename = format!("screenshots/trace_viewer_{}.png", timestamp);
                
                dfx_info!("TraceViewer", "Taking screenshot...");
                if let Err(e) = renderer.capture_screenshot(&filename) {
                    dfx_error!("TraceViewer", "Screenshot failed: {}", e);
                } else {
                    dfx_info!("TraceViewer", "Screenshot saved: {}", filename);
                }
            }
        }
        
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
        
        std::thread::sleep(Duration::from_millis(16));
    }
    
    dfx_info!("TraceViewer", "Cleanup...");
    
    if screenshot_mode {
        dfx_info!("TraceViewer", "Screenshot mode - skipping cleanup");
    } else {
        renderer.cleanup();
    }
    dfx_info!("TraceViewer", "=== Trace Viewer Closed ===");
}

struct TraceData {
    events: Vec<TraceEvent>,
    counter_events: Vec<CounterEvent>,
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

struct CounterEvent {
    name: String,
    category: String,
    timestamp: u64,
    value: i64,
}

struct TraceStats {
    frame_count: u32,
    avg_frame_time: f32,
    max_frame_time: f32,
    min_frame_time: f32,
    total_time_ms: f32,
}

struct CounterStat {
    avg: f64,
    min: f64,
    max: f64,
    count: u32,
}

fn calculate_stats(trace: &TraceData) -> TraceStats {
    let frame_events: Vec<&TraceEvent> = trace.events.iter()
        .filter(|e| e.name == "Frame" && e.phase == "X")
        .collect();
    
    let frame_count = frame_events.len() as u32;
    
    if frame_count == 0 {
        return TraceStats {
            frame_count: 0,
            avg_frame_time: 0.0,
            max_frame_time: 0.0,
            min_frame_time: 0.0,
            total_time_ms: 0.0,
        };
    }
    
    let frame_times: Vec<f32> = frame_events.iter()
        .map(|e| e.duration as f32 / 1_000_000.0)
        .collect();
    
    let avg_frame_time = frame_times.iter().sum::<f32>() / frame_count as f32;
    let max_frame_time = frame_times.iter().fold(0.0f32, |a: f32, b: &f32| a.max(*b));
    let min_frame_time = frame_times.iter().fold(f32::MAX, |a: f32, b: &f32| a.min(*b));
    
    let time_range_ns = trace.max_time.saturating_sub(trace.min_time);
    let total_time_ms = time_range_ns as f32 / 1_000_000.0;
    
    TraceStats {
        frame_count,
        avg_frame_time,
        max_frame_time,
        min_frame_time,
        total_time_ms,
    }
}

fn calculate_counter_stats(trace: &TraceData) -> Vec<(String, CounterStat)> {
    // Group counter events by name
    let mut groups: std::collections::HashMap<String, Vec<&CounterEvent>> = std::collections::HashMap::new();
    for ce in &trace.counter_events {
        groups.entry(ce.name.clone()).or_default().push(ce);
    }
    
    let mut result: Vec<(String, CounterStat)> = Vec::new();
    
    // Process in a stable order: prioritize perf counters
    let priority_order = ["FPS", "CPU%", "MemUsedMB", "MemAvailMB", "FrameMs"];
    
    for name in &priority_order {
        if let Some(events) = groups.get(*name) {
            let values: Vec<f64> = events.iter().map(|e| e.value as f64).collect();
            let count = values.len() as u32;
            let avg = values.iter().sum::<f64>() / count as f64;
            let min = values.iter().fold(f64::MAX, |a, b| a.min(*b));
            let max = values.iter().fold(0.0f64, |a, b| a.max(*b));
            result.push((name.to_string(), CounterStat { avg, min, max, count }));
        }
    }
    
    // Add remaining counters not in priority list
    for (name, events) in &groups {
        if !priority_order.contains(&name.as_str()) {
            let values: Vec<f64> = events.iter().map(|e| e.value as f64).collect();
            let count = values.len() as u32;
            let avg = values.iter().sum::<f64>() / count as f64;
            let min = values.iter().fold(f64::MAX, |a, b| a.min(*b));
            let max = values.iter().fold(0.0f64, |a, b| a.max(*b));
            result.push((name.clone(), CounterStat { avg, min, max, count }));
        }
    }
    
    result
}

fn load_trace_file(path: &str) -> Option<TraceData> {
    let content = fs::read_to_string(path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    
    let events_array = json.get("traceEvents")?.as_array()?;
    
    let mut events: Vec<TraceEvent> = Vec::new();
    let mut counter_events: Vec<CounterEvent> = Vec::new();
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
        
        if phase == "C" {
            // Counter event
            let value = event_val.get("value")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            
            if timestamp < min_time { min_time = timestamp; }
            if timestamp > max_time { max_time = timestamp; }
            
            counter_events.push(CounterEvent {
                name,
                category,
                timestamp,
                value,
            });
            continue;
        }
        
        // Duration event (ph="X") or other
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
        counter_events,
        min_time,
        max_time,
        threads,
    })
}

fn build_trace_ui(
    handle: ui_ffi::WidgetTreeHandle,
    root_id: u64,
    trace: &TraceData,
    stats: &TraceStats,
    counter_stats: &Vec<(String, CounterStat)>,
    window_width: f32,
    window_height: f32,
) {
    let content_scale = 1.0;
    
    // Dynamic layout dimensions based on window size
    let left_margin = 10.0;
    let lane_label_width = 150.0;
    let swimlane_left = lane_label_width + left_margin;
    let swimlane_width = window_width - swimlane_left - 10.0;
    let swimlane_width = swimlane_width.max(200.0); // minimum usable width
    
    // === Title ===
    let title_text = CString::new("Trace Viewer - Performance Analysis").unwrap();
    let title_id = ui_ffi::ui_create_label_in_parent(handle, root_id, 500.0, 24.0 * content_scale, title_text.as_ptr());
    ui_ffi::ui_widget_set_position(handle, title_id, 10.0, 10.0);
    
    // === Frame Stats Summary ===
    let stats_text = CString::new(format!(
        "Frames: {} | Avg: {:.2}ms | Max: {:.2}ms | Min: {:.2}ms | Total: {:.1}ms",
        stats.frame_count, stats.avg_frame_time, stats.max_frame_time, stats.min_frame_time, stats.total_time_ms
    ).as_str()).unwrap();
    let stats_id = ui_ffi::ui_create_label_in_parent(handle, root_id, 600.0, 18.0 * content_scale, stats_text.as_ptr());
    ui_ffi::ui_widget_set_position(handle, stats_id, 10.0, 35.0);
    
    // === Performance Counter Stats ===
    let counter_y_start = 55.0;
    
    // Section header
    if !counter_stats.is_empty() {
        let perf_header = CString::new("Performance Metrics:").unwrap();
        let perf_header_id = ui_ffi::ui_create_label_in_parent(handle, root_id, 200.0, 16.0 * content_scale, perf_header.as_ptr());
        ui_ffi::ui_widget_set_position(handle, perf_header_id, 10.0, counter_y_start);
        
        let mut counter_line_y = counter_y_start + 18.0;
        for (name, cs) in counter_stats {
            let display_name = match name.as_str() {
                "FPS" => "FPS",
                "CPU%" => "CPU%",
                "MemUsedMB" => "Mem Used",
                "MemAvailMB" => "Mem Avail",
                "FrameMs" => "Frame ms",
                _ => name.as_str(),
            };
            
            let unit = match name.as_str() {
                "FPS" => "",
                "CPU%" => "%",
                "MemUsedMB" | "MemAvailMB" => "MB",
                "FrameMs" => "ms",
                _ => "",
            };
            
            let (r, g, b) = get_counter_color(name);
            
            // Color indicator
            ui_ffi::ui_create_panel_in_parent(handle, root_id, 10.0, counter_line_y + 2.0, 8.0, 8.0, r, g, b, 1.0);
            
            let counter_text = CString::new(format!(
                "{}: avg={:.1}{}, min={:.1}{}, max={:.1}{} ({} samples)",
                display_name, cs.avg, unit, cs.min, unit, cs.max, unit, cs.count
            ).as_str()).unwrap();
            let counter_label = ui_ffi::ui_create_label_in_parent(handle, root_id, 500.0, 14.0 * content_scale, counter_text.as_ptr());
            ui_ffi::ui_widget_set_position(handle, counter_label, 22.0, counter_line_y);
            
            counter_line_y += 16.0;
        }
    }
    
    // === Color Legend for event categories ===
    let legend_y = if !counter_stats.is_empty() {
        counter_y_start + 18.0 + counter_stats.len() as f32 * 16.0 + 5.0
    } else {
        55.0
    };
    
    let legend_items = [
        ("Render", 0.2, 0.6, 0.8),
        ("Logic", 0.8, 0.4, 0.2),
        ("UI", 0.4, 0.8, 0.4),
        ("Script", 0.8, 0.6, 0.2),
        ("Other", 0.5, 0.5, 0.5),
    ];
    
    let mut legend_x = 10.0;
    for (name, r, g, b) in legend_items.iter() {
        ui_ffi::ui_create_panel_in_parent(handle, root_id, legend_x, legend_y, 15.0, 15.0, *r, *g, *b, 1.0);
        let label_text = CString::new(*name).unwrap();
        let label_id = ui_ffi::ui_create_label_in_parent(handle, root_id, 50.0, 14.0 * content_scale, label_text.as_ptr());
        ui_ffi::ui_widget_set_position(handle, label_id, legend_x + 18.0, legend_y);
        legend_x += 70.0;
    }
    
    // === Time Axis ===
    let time_axis_y = legend_y + 25.0;
    ui_ffi::ui_create_panel_in_parent(handle, root_id, swimlane_left, time_axis_y, swimlane_width, 20.0, 0.15, 0.15, 0.2, 1.0);
    
    let time_range_ns = trace.max_time.saturating_sub(trace.min_time);
    let time_range_ms = time_range_ns as f32 / 1_000_000.0;
    
    let num_ticks = 10;
    let tick_interval_ms = time_range_ms / num_ticks as f32;
    
    for i in 0..=num_ticks {
        let tick_x = swimlane_left + (i as f32 / num_ticks as f32) * swimlane_width;
        let tick_ms = i as f32 * tick_interval_ms;
        
        ui_ffi::ui_create_panel_in_parent(handle, root_id, tick_x, time_axis_y + 15.0, 2.0, 5.0, 0.4, 0.4, 0.4, 1.0);
        
        if i % 2 == 0 || num_ticks <= 5 {
            let tick_text = CString::new(format!("{:.0}ms", tick_ms).as_str()).unwrap();
            let tick_label = ui_ffi::ui_create_label_in_parent(handle, root_id, 40.0, 12.0 * content_scale, tick_text.as_ptr());
            ui_ffi::ui_widget_set_position(handle, tick_label, tick_x - 20.0, time_axis_y + 22.0);
        }
    }
    
    // === Thread Swimlanes ===
    let swimlane_height = 40.0;
    let swimlane_start_y = time_axis_y + 40.0;
    
    let scale = if time_range_ns > 0 { swimlane_width / time_range_ns as f32 } else { 1.0 };
    
    for (i, thread_id) in trace.threads.iter().enumerate() {
        let lane_y = swimlane_start_y + i as f32 * swimlane_height;
        
        ui_ffi::ui_create_panel_in_parent(
            handle,
            root_id,
            10.0,
            lane_y,
            lane_label_width,
            swimlane_height - 2.0,
            0.12,
            0.12,
            0.15,
            1.0
        );
        
        let thread_text = CString::new(format!("Thread {}", thread_id).as_str()).unwrap();
        let thread_label = ui_ffi::ui_create_label_in_parent(handle, root_id, lane_label_width - 10.0, 14.0 * content_scale, thread_text.as_ptr());
        ui_ffi::ui_widget_set_position(handle, thread_label, 15.0, lane_y + 12.0);
        
        ui_ffi::ui_create_panel_in_parent(
            handle,
            root_id,
            swimlane_left,
            lane_y,
            swimlane_width,
            swimlane_height - 2.0,
            0.18,
            0.18,
            0.22,
            1.0
        );
        
        let thread_events: Vec<&TraceEvent> = trace.events.iter()
            .filter(|e| e.thread_id == *thread_id && e.phase == "X")
            .collect();
        
        for event in thread_events {
            let event_x = swimlane_left + (event.timestamp.saturating_sub(trace.min_time)) as f32 * scale;
            let event_width = (event.duration as f32 * scale).max(3.0);
            
            let (r, g, b) = get_category_color(&event.category);
            
            ui_ffi::ui_create_panel_in_parent(
                handle,
                root_id,
                event_x,
                lane_y + 4.0,
                event_width,
                swimlane_height - 10.0,
                r,
                g,
                b,
                0.9
            );
            
            if event_width > 30.0 {
                let short_name = if event.name.len() > 12 {
                    &event.name[..12]
                } else {
                    &event.name
                };
                let event_text = CString::new(short_name).unwrap();
                let event_label = ui_ffi::ui_create_label_in_parent(handle, root_id, event_width - 2.0, 12.0 * content_scale, event_text.as_ptr());
                ui_ffi::ui_widget_set_position(handle, event_label, event_x + 1.0, lane_y + 8.0);
            } else if event_width > 10.0 {
                let dur_ms = event.duration as f32 / 1_000_000.0;
                let dur_text = CString::new(format!("{:.1}", dur_ms).as_str()).unwrap();
                let dur_label = ui_ffi::ui_create_label_in_parent(handle, root_id, event_width - 1.0, 10.0 * content_scale, dur_text.as_ptr());
                ui_ffi::ui_widget_set_position(handle, dur_label, event_x + 1.0, lane_y + 8.0);
            }
        }
    }
    
    // === Counter Swimlanes ===
    let counter_lane_start_y = swimlane_start_y + trace.threads.len() as f32 * swimlane_height + 5.0;
    let counter_lane_height = 30.0;
    
    // Group counter events by name for swimlane display
    let counter_groups: std::collections::HashMap<String, Vec<&CounterEvent>> = {
        let mut groups: std::collections::HashMap<String, Vec<&CounterEvent>> = std::collections::HashMap::new();
        for ce in &trace.counter_events {
            groups.entry(ce.name.clone()).or_default().push(ce);
        }
        groups
    };
    
    // Display priority counters as swimlanes
    let display_counters = ["FPS", "CPU%", "MemUsedMB", "FrameMs"];
    
    for (i, counter_name) in display_counters.iter().enumerate() {
        if let Some(counter_values) = counter_groups.get(*counter_name) {
            if counter_values.is_empty() {
                continue;
            }
            
            let lane_y = counter_lane_start_y + i as f32 * counter_lane_height;
            
            let (r, g, b) = get_counter_color(counter_name);
            
            // Lane label
            let display_name = match *counter_name {
                "FPS" => "FPS",
                "CPU%" => "CPU%",
                "MemUsedMB" => "Mem Used",
                "FrameMs" => "Frame ms",
                _ => *counter_name,
            };
            
            ui_ffi::ui_create_panel_in_parent(
                handle,
                root_id,
                10.0,
                lane_y,
                lane_label_width,
                counter_lane_height - 2.0,
                r * 0.3,
                g * 0.3,
                b * 0.3,
                1.0
            );
            let label_text = CString::new(display_name).unwrap();
            let label_id = ui_ffi::ui_create_label_in_parent(handle, root_id, lane_label_width - 10.0, 14.0 * content_scale, label_text.as_ptr());
            ui_ffi::ui_widget_set_position(handle, label_id, 15.0, lane_y + 7.0);
            
            // Lane background
            ui_ffi::ui_create_panel_in_parent(
                handle,
                root_id,
                swimlane_left,
                lane_y,
                swimlane_width,
                counter_lane_height - 2.0,
                0.18,
                0.18,
                0.22,
                1.0
            );
            
            // Find min/max for this counter to normalize display
            let values: Vec<f64> = counter_values.iter().map(|e| e.value as f64).collect();
            let val_min = values.iter().fold(f64::MAX, |a, b| a.min(*b));
            let val_max = values.iter().fold(0.0f64, |a, b| a.max(*b));
            let val_range = val_max - val_min;
            
            // Draw counter value dots/bars along the timeline
            let max_bar_height = counter_lane_height - 6.0;
            
            for ce in counter_values.iter() {
                let ce_x = swimlane_left + (ce.timestamp.saturating_sub(trace.min_time)) as f32 * scale;
                
                // Normalize value to bar height
                let normalized = if val_range > 0.0 {
                    ((ce.value as f64 - val_min) / val_range) as f32
                } else {
                    0.5 // Flat line if all values are same
                };
                
                let bar_height = (normalized * max_bar_height).max(2.0);
                let bar_y = lane_y + (counter_lane_height - 2.0) - bar_height;
                
                // Small bar for each counter sample
                ui_ffi::ui_create_panel_in_parent(
                    handle,
                    root_id,
                    ce_x,
                    bar_y,
                    3.0, // 3px wide dots
                    bar_height,
                    r,
                    g,
                    b,
                    0.8
                );
            }
        }
    }
    
    // === Bottom Summary ===
    let bottom_y = counter_lane_start_y + display_counters.len() as f32 * counter_lane_height + 10.0;
    let event_count_text = CString::new(format!(
        "Events: {} | Threads: {} | Counters: {}",
        trace.events.len(), trace.threads.len(), trace.counter_events.len()
    ).as_str()).unwrap();
    let event_count_label = ui_ffi::ui_create_label_in_parent(handle, root_id, 400.0, 14.0 * content_scale, event_count_text.as_ptr());
    ui_ffi::ui_widget_set_position(handle, event_count_label, 10.0, bottom_y);
    
    let help_text = CString::new("Press S to save screenshot").unwrap();
    let help_label = ui_ffi::ui_create_label_in_parent(handle, root_id, 200.0, 14.0 * content_scale, help_text.as_ptr());
    ui_ffi::ui_widget_set_position(handle, help_label, 500.0, bottom_y);
}

fn get_category_color(category: &str) -> (f32, f32, f32) {
    match category {
        "render" => (0.2, 0.6, 0.8),
        "logic" => (0.8, 0.4, 0.2),
        "ui" => (0.4, 0.8, 0.4),
        "script" => (0.8, 0.6, 0.2),
        "perf" => (0.6, 0.3, 0.9),
        "memory" => (0.9, 0.5, 0.2),
        "editor" => (0.3, 0.7, 0.7),
        "create" => (0.7, 0.3, 0.6),
        "load" => (0.6, 0.6, 0.2),
        _ => (0.5, 0.5, 0.5),
    }
}

fn get_counter_color(name: &str) -> (f32, f32, f32) {
    match name {
        "FPS" => (0.3, 0.9, 0.3),    // Green
        "CPU%" => (0.9, 0.3, 0.3),    // Red
        "MemUsedMB" => (0.9, 0.5, 0.2), // Orange
        "MemAvailMB" => (0.2, 0.6, 0.9), // Blue
        "FrameMs" => (0.6, 0.3, 0.9),   // Purple
        _ => (0.5, 0.5, 0.5),           // Gray
    }
}

fn list_available_traces() {
    println!("\n=== Available Trace Files ===");
    
    if let Ok(entries) = fs::read_dir("traces") {
        let mut files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
            .collect();
        
        files.sort_by(|a, b| {
            b.metadata().and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                .cmp(&a.metadata().and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH))
        });
        
        if files.is_empty() {
            println!("  No trace files found in traces/");
            println!("  Run mono_editor_demo first to generate traces.");
        } else {
            println!("  Usage: trace_viewer_demo --file <path>");
            println!("\n  Recent files:");
            for (i, file) in files.iter().take(10).enumerate() {
                let path_str = file.path().to_string_lossy().to_string();
                let size = file.metadata().map(|m| m.len()).unwrap_or(0);
                println!("    [{}] {} ({:.1} KB)", i + 1, path_str, size as f32 / 1024.0);
            }
            println!("\n  Default: traces/trace_latest.json");
        }
    } else {
        println!("  traces/ directory not found");
    }
    println!();
}