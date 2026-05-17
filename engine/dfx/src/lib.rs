pub mod crash;
pub mod log_types;
pub mod logger;
pub mod perf;
pub mod trace;

pub use crash::*;
pub use log_types::*;
pub use logger::*;
pub use perf::*;
pub use trace::*;

use parking_lot::Mutex;
use std::sync::Arc;

pub struct DfxSystem {
    logger: Arc<Mutex<Logger>>,
    crash_handler: Arc<Mutex<CrashHandler>>,
    trace_analyzer: Arc<Mutex<TraceAnalyzer>>,
    perf_monitor: Arc<Mutex<PerformanceMonitor>>,
}

impl DfxSystem {
    pub fn new() -> Self {
        Self {
            logger: Arc::new(Mutex::new(Logger::new())),
            crash_handler: Arc::new(Mutex::new(CrashHandler::new())),
            trace_analyzer: Arc::new(Mutex::new(TraceAnalyzer::new())),
            perf_monitor: Arc::new(Mutex::new(PerformanceMonitor::new())),
        }
    }

    pub fn enable_all(&mut self) {
        self.logger.lock().set_level(LogLevel::Trace);
        self.crash_handler.lock().enable();
        self.trace_analyzer.lock().enable();
        self.perf_monitor.lock().enable();
    }

    pub fn disable_all(&mut self) {
        self.crash_handler.lock().disable();
        self.trace_analyzer.lock().disable();
        self.perf_monitor.lock().disable();
    }

    pub fn get_logger(&self) -> Arc<Mutex<Logger>> {
        self.logger.clone()
    }

    pub fn get_crash_handler(&self) -> Arc<Mutex<CrashHandler>> {
        self.crash_handler.clone()
    }

    pub fn get_trace_analyzer(&self) -> Arc<Mutex<TraceAnalyzer>> {
        self.trace_analyzer.clone()
    }

    pub fn get_perf_monitor(&self) -> Arc<Mutex<PerformanceMonitor>> {
        self.perf_monitor.clone()
    }
}

impl Default for DfxSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_create() -> *mut DfxSystem {
    let system = Box::new(DfxSystem::new());
    Box::into_raw(system)
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_destroy(system: *mut DfxSystem) {
    if !system.is_null() {
        unsafe {
            let _ = Box::from_raw(system);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_enable_all(system: *mut DfxSystem) {
    if !system.is_null() {
        unsafe {
            (*system).enable_all();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_disable_all(system: *mut DfxSystem) {
    if !system.is_null() {
        unsafe {
            (*system).disable_all();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_set_log_level(system: *mut DfxSystem, level: u8) {
    if !system.is_null() {
        unsafe {
            (*system).logger.lock().set_level(LogLevel::from_u8(level));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_log(
    system: *mut DfxSystem,
    level: u8,
    module: *const std::os::raw::c_char,
    message: *const std::os::raw::c_char,
    file: *const std::os::raw::c_char,
    line: u32,
) {
    if system.is_null() || module.is_null() || message.is_null() || file.is_null() {
        return;
    }

    unsafe {
        let module_str = std::ffi::CStr::from_ptr(module).to_str().unwrap_or("");
        let message_str = std::ffi::CStr::from_ptr(message).to_str().unwrap_or("");
        let file_str = std::ffi::CStr::from_ptr(file).to_str().unwrap_or("");

        (*system).logger.lock().log(
            LogLevel::from_u8(level),
            module_str,
            message_str,
            file_str,
            line,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_enable_crash_handler(system: *mut DfxSystem) {
    if !system.is_null() {
        unsafe {
            (*system).crash_handler.lock().enable();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_enable_trace(system: *mut DfxSystem) {
    if !system.is_null() {
        unsafe {
            (*system).trace_analyzer.lock().enable();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_enable_perf_monitor(system: *mut DfxSystem) {
    if !system.is_null() {
        unsafe {
            (*system).perf_monitor.lock().enable();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_trace_begin(
    system: *mut DfxSystem,
    name: *const std::os::raw::c_char,
    category: *const std::os::raw::c_char,
) {
    if system.is_null() || name.is_null() || category.is_null() {
        return;
    }

    unsafe {
        let name_str = std::ffi::CStr::from_ptr(name).to_str().unwrap_or("");
        let category_str = std::ffi::CStr::from_ptr(category).to_str().unwrap_or("");

        (*system)
            .trace_analyzer
            .lock()
            .begin_point(name_str, category_str);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_trace_end(
    system: *mut DfxSystem,
    name: *const std::os::raw::c_char,
    category: *const std::os::raw::c_char,
) {
    if system.is_null() || name.is_null() || category.is_null() {
        return;
    }

    unsafe {
        let name_str = std::ffi::CStr::from_ptr(name).to_str().unwrap_or("");
        let category_str = std::ffi::CStr::from_ptr(category).to_str().unwrap_or("");

        (*system)
            .trace_analyzer
            .lock()
            .end_point(name_str, category_str);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_perf_begin_frame(system: *mut DfxSystem) {
    if !system.is_null() {
        unsafe {
            (*system).perf_monitor.lock().begin_frame();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_perf_end_frame(system: *mut DfxSystem) {
    if !system.is_null() {
        unsafe {
            (*system).perf_monitor.lock().end_frame();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_get_fps(system: *mut DfxSystem) -> f32 {
    if system.is_null() {
        return 0.0;
    }

    unsafe { (*system).perf_monitor.lock().get_fps() }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_get_frame_count(system: *mut DfxSystem) -> u64 {
    if system.is_null() {
        return 0;
    }

    unsafe { (*system).perf_monitor.lock().get_frame_count() }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_get_perf_snapshot(system: *mut DfxSystem) -> PerformanceSnapshot {
    if system.is_null() {
        return PerformanceSnapshot {
            timestamp: 0,
            fps: 0.0,
            frame_time_ms: 0.0,
            cpu_usage_percent: 0.0,
            memory_used_mb: 0.0,
            memory_available_mb: 0.0,
            draw_calls: 0,
            triangle_count: 0,
        };
    }

    unsafe {
        (*system)
            .perf_monitor
            .lock()
            .get_latest_snapshot()
            .unwrap_or(PerformanceSnapshot {
                timestamp: 0,
                fps: 0.0,
                frame_time_ms: 0.0,
                cpu_usage_percent: 0.0,
                memory_used_mb: 0.0,
                memory_available_mb: 0.0,
                draw_calls: 0,
                triangle_count: 0,
            })
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_save_trace(system: *mut DfxSystem, path: *const std::os::raw::c_char) -> i32 {
    if system.is_null() || path.is_null() {
        return -1;
    }

    unsafe {
        let path_str = std::ffi::CStr::from_ptr(path).to_str().unwrap_or("");

        match (*system).trace_analyzer.lock().save_to_file(path_str) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_capture_stack_trace() -> *mut StackFrame {
    let frames = CrashHandler::capture_stack_trace();

    if frames.is_empty() {
        return std::ptr::null_mut();
    }

    let boxed = frames.into_boxed_slice();
    let ptr = Box::into_raw(boxed) as *mut StackFrame;

    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_get_stack_frame_count(frames: *mut StackFrame) -> u32 {
    if frames.is_null() {
        return 0;
    }

    unsafe { 10 }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_free_stack_trace(frames: *mut StackFrame, count: u32) {
    if frames.is_null() {
        return;
    }

    let slice = unsafe { std::slice::from_raw_parts_mut(frames, count as usize) };
    let _ = unsafe { Box::from_raw(slice as *mut [StackFrame]) };
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_get_log_buffer_count(system: *mut DfxSystem) -> u32 {
    if system.is_null() {
        return 0;
    }

    unsafe { (*system).logger.lock().get_buffer().len() as u32 }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_clear_log_buffer(system: *mut DfxSystem) {
    if !system.is_null() {
        unsafe {
            (*system).logger.lock().clear_buffer();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_clear_trace(system: *mut DfxSystem) {
    if !system.is_null() {
        unsafe {
            (*system).trace_analyzer.lock().clear();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn dfx_clear_perf(system: *mut DfxSystem) {
    if !system.is_null() {
        unsafe {
            (*system).perf_monitor.lock().clear();
        }
    }
}
