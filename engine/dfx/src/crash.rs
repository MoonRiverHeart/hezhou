use backtrace::Backtrace;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::ffi::CString;
use std::sync::Arc;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StackFrame {
    pub address: u64,
    pub symbol: *const std::os::raw::c_char,
    pub file: *const std::os::raw::c_char,
    pub line: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CrashReport {
    pub crash_type: CrashType,
    pub timestamp: u64,
    pub message: *const std::os::raw::c_char,
    pub frames: *mut StackFrame,
    pub frame_count: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrashType {
    Panic = 0,
    Segfault = 1,
    StackOverflow = 2,
    NullPointer = 3,
    OutOfMemory = 4,
    Custom = 5,
}

pub struct CrashHandler {
    reports: Arc<Mutex<VecDeque<CrashReport>>>,
    max_reports: usize,
    crash_file: Option<std::path::PathBuf>,
    enabled: bool,
}

impl CrashHandler {
    pub fn new() -> Self {
        Self {
            reports: Arc::new(Mutex::new(VecDeque::new())),
            max_reports: 10,
            crash_file: None,
            enabled: false,
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
        self.setup_panic_hook();
        #[cfg(unix)]
        self.setup_signal_handlers();
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn set_crash_file(&mut self, path: &str) {
        self.crash_file = Some(std::path::PathBuf::from(path));
    }

    fn setup_panic_hook(&mut self) {
        std::panic::set_hook(Box::new(|panic_info| {
            let message = panic_info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| {
                    panic_info
                        .payload()
                        .downcast_ref::<String>()
                        .map(|s| s.clone())
                })
                .unwrap_or_else(|| "Unknown panic".to_string());

            let location = panic_info
                .location()
                .map(|loc| format!("{}:{}", loc.file(), loc.line()))
                .unwrap_or_else(|| "Unknown location".to_string());

            let full_message = format!("Panic at {}: {}", location, message);

            Self::capture_crash(CrashType::Panic, &full_message);
        }));
    }

    #[cfg(unix)]
    fn setup_signal_handlers(&mut self) {
        use libc::{SIGABRT, SIGBUS, SIGFPE, SIGILL, SIGSEGV};

        extern "C" fn signal_handler(sig: i32) {
            let crash_type = match sig {
                SIGSEGV => CrashType::Segfault,
                SIGBUS => CrashType::NullPointer,
                SIGFPE => CrashType::Custom,
                SIGILL => CrashType::Custom,
                SIGABRT => CrashType::Custom,
                _ => CrashType::Custom,
            };

            Self::capture_crash(crash_type, &format!("Signal received: {}", sig));
        }

        unsafe {
            libc::signal(SIGSEGV, signal_handler as usize);
            libc::signal(SIGBUS, signal_handler as usize);
            libc::signal(SIGFPE, signal_handler as usize);
            libc::signal(SIGILL, signal_handler as usize);
            libc::signal(SIGABRT, signal_handler as usize);
        }
    }

    fn capture_crash(crash_type: CrashType, message: &str) {
        let timestamp = chrono::Local::now().timestamp_millis() as u64;

        let backtrace = Backtrace::new();
        let frames: Vec<StackFrame> = backtrace
            .frames()
            .iter()
            .flat_map(|frame| {
                frame.symbols().iter().map(|sym| {
                    let symbol_name = sym
                        .name()
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    let filename = sym
                        .filename()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    let lineno = sym.lineno().unwrap_or(0);

                    StackFrame {
                        address: frame.ip() as u64,
                        symbol: CString::new(symbol_name).unwrap().into_raw(),
                        file: CString::new(filename).unwrap().into_raw(),
                        line: lineno,
                    }
                })
            })
            .collect();

        let message_c = CString::new(message).unwrap().into_raw();
        let frame_count = frames.len() as u32;
        let frames_ptr = frames.as_ptr() as *mut StackFrame;

        let report = CrashReport {
            crash_type,
            timestamp,
            message: message_c,
            frames: frames_ptr,
            frame_count,
        };

        Self::write_crash_file(&report);

        std::mem::forget(frames);
    }

    fn write_crash_file(report: &CrashReport) {
        let crash_file = std::path::PathBuf::from("crash_report.txt");

        let time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let message = unsafe {
            std::ffi::CStr::from_ptr(report.message)
                .to_str()
                .unwrap_or("")
        };

        let content = format!(
            "=== Crash Report ===\nTime: {}\nType: {:?}\nMessage: {}\n\nStack Trace:\n",
            time, report.crash_type, message
        );

        if let Ok(mut file) = std::fs::File::create(&crash_file) {
            use std::io::Write;
            let mut file = file;
            file.write_all(content.as_bytes()).unwrap();

            let frames =
                unsafe { std::slice::from_raw_parts(report.frames, report.frame_count as usize) };

            for (i, frame) in frames.iter().enumerate() {
                let symbol = unsafe {
                    std::ffi::CStr::from_ptr(frame.symbol)
                        .to_str()
                        .unwrap_or("")
                };
                let file_name =
                    unsafe { std::ffi::CStr::from_ptr(frame.file).to_str().unwrap_or("") };
                let line = format!(
                    "#{} {} @ {}:{} (0x{:x})\n",
                    i, symbol, file_name, frame.line, frame.address
                );
                file.write_all(line.as_bytes()).unwrap();
            }
        }
    }

    pub fn capture_stack_trace() -> Vec<StackFrame> {
        let backtrace = Backtrace::new();

        backtrace
            .frames()
            .iter()
            .flat_map(|frame| {
                frame.symbols().iter().map(|sym| {
                    let symbol_name = sym
                        .name()
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    let filename = sym
                        .filename()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    let lineno = sym.lineno().unwrap_or(0);

                    StackFrame {
                        address: frame.ip() as u64,
                        symbol: CString::new(symbol_name).unwrap().into_raw(),
                        file: CString::new(filename).unwrap().into_raw(),
                        line: lineno,
                    }
                })
            })
            .collect()
    }

    pub fn get_reports(&self) -> Vec<CrashReport> {
        self.reports.lock().iter().cloned().collect()
    }

    pub fn clear_reports(&mut self) {
        self.reports.lock().clear();
    }
}

impl Default for CrashHandler {
    fn default() -> Self {
        Self::new()
    }
}
