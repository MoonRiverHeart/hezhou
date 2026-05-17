use hezhou_dfx::*;
use std::ffi::CString;

#[test]
fn test_dfx_create_destroy() {
    let system = dfx_create();
    assert!(!system.is_null());

    dfx_destroy(system);
}

#[test]
fn test_logger_levels() {
    let mut logger = Logger::new();

    assert_eq!(logger.get_level(), LogLevel::Info);

    logger.set_level(LogLevel::Debug);
    assert_eq!(logger.get_level(), LogLevel::Debug);

    logger.set_level(LogLevel::Trace);
    assert_eq!(logger.get_level(), LogLevel::Trace);

    logger.set_level(LogLevel::Error);
    assert_eq!(logger.get_level(), LogLevel::Error);
}

#[test]
fn test_log_output() {
    let mut logger = Logger::new();
    logger.set_level(LogLevel::Trace);

    logger.trace("TestModule", "Trace message", "test.rs", 1);
    logger.debug("TestModule", "Debug message", "test.rs", 2);
    logger.info("TestModule", "Info message", "test.rs", 3);
    logger.warn("TestModule", "Warn message", "test.rs", 4);
    logger.error("TestModule", "Error message", "test.rs", 5);

    let buffer = logger.get_buffer();
    assert_eq!(buffer.len(), 5);
}

#[test]
fn test_log_filtering() {
    let mut logger = Logger::new();
    logger.set_level(LogLevel::Warn);

    logger.trace("Test", "Trace", "test.rs", 1);
    logger.debug("Test", "Debug", "test.rs", 2);
    logger.info("Test", "Info", "test.rs", 3);
    logger.warn("Test", "Warn", "test.rs", 4);
    logger.error("Test", "Error", "test.rs", 5);

    let buffer = logger.get_buffer();
    assert_eq!(buffer.len(), 2);
}

#[test]
fn test_log_buffer_size() {
    let mut logger = Logger::new();
    logger.set_level(LogLevel::Info);
    logger.set_buffer_size(5);

    for i in 0..10 {
        logger.info("Test", &format!("Message {}", i), "test.rs", i);
    }

    let buffer = logger.get_buffer();
    assert_eq!(buffer.len(), 5);
}

#[test]
fn test_log_clear_buffer() {
    let mut logger = Logger::new();
    logger.set_level(LogLevel::Info);

    logger.info("Test", "Message 1", "test.rs", 1);
    logger.info("Test", "Message 2", "test.rs", 2);

    assert_eq!(logger.get_buffer().len(), 2);

    logger.clear_buffer();
    assert_eq!(logger.get_buffer().len(), 0);
}

#[test]
fn test_trace_analyzer_enable_disable() {
    let mut analyzer = TraceAnalyzer::new();

    assert!(!analyzer.is_enabled());

    analyzer.enable();
    assert!(analyzer.is_enabled());

    analyzer.disable();
    assert!(!analyzer.is_enabled());
}

#[test]
fn test_trace_points() {
    let mut analyzer = TraceAnalyzer::new();
    analyzer.enable();

    analyzer.begin_point("test_function", "test_category");
    std::thread::sleep(std::time::Duration::from_millis(10));
    analyzer.end_point("test_function", "test_category");

    let points = analyzer.get_points();
    assert_eq!(points.len(), 1);
    assert!(points[0].duration_ns > 0);
}

#[test]
fn test_trace_counters() {
    let mut analyzer = TraceAnalyzer::new();
    analyzer.enable();

    analyzer.set_counter("fps", "performance", 60);
    analyzer.set_counter("fps", "performance", 59);
    analyzer.set_counter("fps", "performance", 61);

    let counters = analyzer.get_counters("fps");
    assert_eq!(counters.len(), 3);
}

#[test]
fn test_trace_counter_increment() {
    let mut analyzer = TraceAnalyzer::new();
    analyzer.enable();

    for _ in 0..5 {
        analyzer.increment_counter("frame_count", "performance");
    }

    let counters = analyzer.get_counters("frame_count");
    assert_eq!(counters.len(), 5);

    let last = counters.last().unwrap();
    assert_eq!(last.value, 5);
}

#[test]
fn test_trace_clear() {
    let mut analyzer = TraceAnalyzer::new();
    analyzer.enable();

    analyzer.begin_point("test", "cat");
    analyzer.end_point("test", "cat");
    analyzer.set_counter("test", "cat", 1);

    assert_eq!(analyzer.get_points().len(), 1);
    assert_eq!(analyzer.get_counters("test").len(), 1);

    analyzer.clear();

    assert_eq!(analyzer.get_points().len(), 0);
    assert_eq!(analyzer.get_counters("test").len(), 0);
}

#[test]
fn test_trace_export_json() {
    let mut analyzer = TraceAnalyzer::new();
    analyzer.enable();

    analyzer.begin_point("test_func", "test_cat");
    analyzer.end_point("test_func", "test_cat");

    analyzer.set_counter("fps", "perf", 60);

    let json = analyzer.export_json();
    assert!(json.contains("traceEvents"));
    assert!(json.contains("test_func"));
    assert!(json.contains("fps"));
}

#[test]
fn test_perf_monitor_enable_disable() {
    let mut monitor = PerformanceMonitor::new();

    assert!(!monitor.is_enabled());

    monitor.enable();
    assert!(monitor.is_enabled());

    monitor.disable();
    assert!(!monitor.is_enabled());
}

#[test]
fn test_perf_monitor_frames() {
    let mut monitor = PerformanceMonitor::new();
    monitor.enable();

    for _ in 0..10 {
        monitor.begin_frame();
        std::thread::sleep(std::time::Duration::from_millis(16));
        monitor.end_frame();
    }

    assert_eq!(monitor.get_frame_count(), 10);
}

#[test]
fn test_perf_monitor_snapshots() {
    let mut monitor = PerformanceMonitor::new();
    monitor.enable();

    for _ in 0..5 {
        monitor.begin_frame();
        std::thread::sleep(std::time::Duration::from_millis(1));
        monitor.end_frame();
    }

    let snapshots = monitor.get_snapshots();
    assert_eq!(snapshots.len(), 5);

    let latest = monitor.get_latest_snapshot();
    assert!(latest.is_some());
}

#[test]
fn test_perf_monitor_averages() {
    let mut monitor = PerformanceMonitor::new();
    monitor.enable();

    for _ in 0..10 {
        monitor.begin_frame();
        std::thread::sleep(std::time::Duration::from_millis(1));
        monitor.end_frame();
    }

    let avg_frame_time = monitor.get_average_frame_time();
    assert!(avg_frame_time >= 0.0);
    let avg_memory = monitor.get_average_memory();
    assert!(avg_memory >= 0.0);
}

#[test]
fn test_perf_monitor_clear() {
    let mut monitor = PerformanceMonitor::new();
    monitor.enable();

    for _ in 0..5 {
        monitor.begin_frame();
        monitor.end_frame();
    }

    assert_eq!(monitor.get_frame_count(), 5);

    monitor.clear();

    assert_eq!(monitor.get_frame_count(), 0);
    assert_eq!(monitor.get_snapshots().len(), 0);
}

#[test]
fn test_dfx_system_enable_all() {
    let mut system = DfxSystem::new();

    system.enable_all();

    assert!(system.get_trace_analyzer().lock().is_enabled());
    assert!(system.get_perf_monitor().lock().is_enabled());
}

#[test]
fn test_dfx_system_disable_all() {
    let mut system = DfxSystem::new();
    system.enable_all();

    system.disable_all();

    assert!(!system.get_trace_analyzer().lock().is_enabled());
    assert!(!system.get_perf_monitor().lock().is_enabled());
}

#[test]
fn test_dfx_ffi_create() {
    let system = dfx_create();
    assert!(!system.is_null());

    dfx_destroy(system);
}

#[test]
fn test_dfx_ffi_log() {
    let system = dfx_create();

    let module = CString::new("TestModule").unwrap();
    let message = CString::new("Test message").unwrap();
    let file = CString::new("test.rs").unwrap();

    dfx_set_log_level(system, 0);
    dfx_log(
        system,
        2,
        module.as_ptr(),
        message.as_ptr(),
        file.as_ptr(),
        1,
    );

    let count = dfx_get_log_buffer_count(system);
    assert_eq!(count, 1);

    dfx_destroy(system);
}

#[test]
fn test_dfx_ffi_trace() {
    let system = dfx_create();

    dfx_enable_trace(system);

    let name = CString::new("test_trace").unwrap();
    let category = CString::new("test_cat").unwrap();

    dfx_trace_begin(system, name.as_ptr(), category.as_ptr());
    std::thread::sleep(std::time::Duration::from_millis(5));
    dfx_trace_end(system, name.as_ptr(), category.as_ptr());

    dfx_destroy(system);
}

#[test]
fn test_dfx_ffi_perf() {
    let system = dfx_create();

    dfx_enable_perf_monitor(system);

    for _ in 0..5 {
        dfx_perf_begin_frame(system);
        std::thread::sleep(std::time::Duration::from_millis(1));
        dfx_perf_end_frame(system);
    }

    let frame_count = dfx_get_frame_count(system);
    assert_eq!(frame_count, 5);

    dfx_destroy(system);
}

#[test]
fn test_stack_trace_capture() {
    let frames = dfx_capture_stack_trace();

    if !frames.is_null() {
        let count = dfx_get_stack_frame_count(frames);
        assert!(count > 0);

        dfx_free_stack_trace(frames, count);
    }
}

#[test]
fn test_log_level_conversion() {
    assert_eq!(LogLevel::from_u8(0), LogLevel::Trace);
    assert_eq!(LogLevel::from_u8(1), LogLevel::Debug);
    assert_eq!(LogLevel::from_u8(2), LogLevel::Info);
    assert_eq!(LogLevel::from_u8(3), LogLevel::Warn);
    assert_eq!(LogLevel::from_u8(4), LogLevel::Error);
    assert_eq!(LogLevel::from_u8(5), LogLevel::Fatal);
    assert_eq!(LogLevel::from_u8(99), LogLevel::Info);
}

#[test]
fn test_log_level_string() {
    assert_eq!(LogLevel::Trace.as_str(), "TRACE");
    assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
    assert_eq!(LogLevel::Info.as_str(), "INFO");
    assert_eq!(LogLevel::Warn.as_str(), "WARN");
    assert_eq!(LogLevel::Error.as_str(), "ERROR");
    assert_eq!(LogLevel::Fatal.as_str(), "FATAL");
}
