use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PerformanceSnapshot {
    pub timestamp: u64,
    pub fps: f32,
    pub frame_time_ms: f32,
    pub cpu_usage_percent: f32,
    pub memory_used_mb: f32,
    pub memory_available_mb: f32,
    pub draw_calls: u32,
    pub triangle_count: u32,
}

pub struct PerformanceMonitor {
    snapshots: Arc<Mutex<VecDeque<PerformanceSnapshot>>>,
    max_snapshots: usize,
    frame_start_time: Option<u64>,
    frame_count: u64,
    last_fps_time: u64,
    fps_frame_count: u32,
    current_fps: f32,
    enabled: bool,
    // CPU tracking state (FILETIME 100ns units)
    prev_process_kernel_time: u64,
    prev_process_user_time: u64,
    prev_system_kernel_time: u64,
    prev_system_user_time: u64,
    has_prev_cpu_times: bool,
    // Pending renderer stats (set before end_frame)
    pending_draw_calls: u32,
    pending_triangle_count: u32,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            snapshots: Arc::new(Mutex::new(VecDeque::new())),
            max_snapshots: 300,
            frame_start_time: None,
            frame_count: 0,
            last_fps_time: Self::get_timestamp_ms(),
            fps_frame_count: 0,
            current_fps: 0.0,
            enabled: false,
            prev_process_kernel_time: 0,
            prev_process_user_time: 0,
            prev_system_kernel_time: 0,
            prev_system_user_time: 0,
            has_prev_cpu_times: false,
            pending_draw_calls: 0,
            pending_triangle_count: 0,
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn begin_frame(&mut self) {
        if !self.enabled {
            return;
        }

        self.frame_start_time = Some(Self::get_timestamp_ms());
    }

    pub fn end_frame(&mut self) {
        if !self.enabled {
            return;
        }

        let end_time = Self::get_timestamp_ms();

        if let Some(start_time) = self.frame_start_time {
            let frame_time_ms = (end_time - start_time) as f32;

            self.frame_count += 1;
            self.fps_frame_count += 1;

            if end_time - self.last_fps_time >= 1000 {
                self.current_fps =
                    self.fps_frame_count as f32 * 1000.0 / (end_time - self.last_fps_time) as f32;
                self.fps_frame_count = 0;
                self.last_fps_time = end_time;
            }

            let cpu_usage = self.get_cpu_usage();
            let (memory_used, memory_available) = Self::get_memory_info();

            let snapshot = PerformanceSnapshot {
                timestamp: end_time,
                fps: self.current_fps,
                frame_time_ms,
                cpu_usage_percent: cpu_usage,
                memory_used_mb: memory_used,
                memory_available_mb: memory_available,
                draw_calls: self.pending_draw_calls,
                triangle_count: self.pending_triangle_count,
            };

            // Reset pending renderer stats
            self.pending_draw_calls = 0;
            self.pending_triangle_count = 0;

            {
                let mut snapshots = self.snapshots.lock();
                if snapshots.len() >= self.max_snapshots {
                    snapshots.pop_front();
                }
                snapshots.push_back(snapshot);
            }
        }

        self.frame_start_time = None;
    }

    pub fn set_draw_calls(&mut self, count: u32) {
        self.pending_draw_calls = count;
    }

    pub fn set_triangle_count(&mut self, count: u32) {
        self.pending_triangle_count = count;
    }

    fn get_timestamp_ms() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};

        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    fn get_cpu_usage(&mut self) -> f32 {
        #[cfg(windows)]
        {
            use winapi::um::processthreadsapi::{GetCurrentProcess, GetProcessTimes, GetSystemTimes};
            use winapi::shared::minwindef::FILETIME;

            let mut process_creation = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
            let mut process_exit = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
            let mut process_kernel = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
            let mut process_user = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };

            let mut system_idle = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
            let mut system_kernel = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
            let mut system_user = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };

            unsafe {
                GetProcessTimes(
                    GetCurrentProcess(),
                    &mut process_creation,
                    &mut process_exit,
                    &mut process_kernel,
                    &mut process_user,
                );
                GetSystemTimes(&mut system_idle, &mut system_kernel, &mut system_user);
            }

            let process_kernel_100ns = Self::filetime_to_u64(&process_kernel);
            let process_user_100ns = Self::filetime_to_u64(&process_user);
            let system_kernel_100ns = Self::filetime_to_u64(&system_kernel);
            let system_user_100ns = Self::filetime_to_u64(&system_user);

            if !self.has_prev_cpu_times {
                self.prev_process_kernel_time = process_kernel_100ns;
                self.prev_process_user_time = process_user_100ns;
                self.prev_system_kernel_time = system_kernel_100ns;
                self.prev_system_user_time = system_user_100ns;
                self.has_prev_cpu_times = true;
                return 0.0; // No delta on first frame
            }

            let process_delta = (process_kernel_100ns - self.prev_process_kernel_time)
                + (process_user_100ns - self.prev_process_user_time);
            let system_delta = (system_kernel_100ns - self.prev_system_kernel_time)
                + (system_user_100ns - self.prev_system_user_time);

            self.prev_process_kernel_time = process_kernel_100ns;
            self.prev_process_user_time = process_user_100ns;
            self.prev_system_kernel_time = system_kernel_100ns;
            self.prev_system_user_time = system_user_100ns;

            if system_delta == 0 {
                return 0.0;
            }

            // Process CPU time / Total system CPU time * 100
            // On multi-core: 100% = all cores used by this process
            (process_delta as f32 / system_delta as f32) * 100.0
        }

        #[cfg(unix)]
        {
            // TODO: implement via /proc/self/stat for Linux
            0.0
        }

        #[cfg(not(any(windows, unix)))]
        {
            0.0
        }
    }

    fn get_memory_info() -> (f32, f32) {
        #[cfg(windows)]
        {
            use winapi::um::sysinfoapi::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
            use winapi::um::psapi::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
            use winapi::um::processthreadsapi::GetCurrentProcess;

            // System memory (available physical RAM)
            let mut mem_status = MEMORYSTATUSEX {
                dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
                dwMemoryLoad: 0,
                ullTotalPhys: 0,
                ullAvailPhys: 0,
                ullTotalPageFile: 0,
                ullAvailPageFile: 0,
                ullTotalVirtual: 0,
                ullAvailVirtual: 0,
                ullAvailExtendedVirtual: 0,
            };

            unsafe {
                GlobalMemoryStatusEx(&mut mem_status);
            }

            let avail_phys_mb = mem_status.ullAvailPhys as f32 / (1024.0 * 1024.0);

            // Process memory (working set = physical pages used by this process)
            let mut counters = PROCESS_MEMORY_COUNTERS {
                cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                PageFaultCount: 0,
                PeakWorkingSetSize: 0,
                WorkingSetSize: 0,
                QuotaPeakPagedPoolUsage: 0,
                QuotaPagedPoolUsage: 0,
                QuotaPeakNonPagedPoolUsage: 0,
                QuotaNonPagedPoolUsage: 0,
                PagefileUsage: 0,
                PeakPagefileUsage: 0,
            };

            unsafe {
                GetProcessMemoryInfo(
                    GetCurrentProcess(),
                    &mut counters,
                    std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                );
            }

            let process_used_mb = counters.WorkingSetSize as f32 / (1024.0 * 1024.0);

            (process_used_mb, avail_phys_mb)
        }

        #[cfg(unix)]
        {
            // TODO: implement via /proc/self/status for Linux
            (0.0, 0.0)
        }

        #[cfg(not(any(windows, unix)))]
        {
            (0.0, 0.0)
        }
    }

    #[cfg(windows)]
    fn filetime_to_u64(ft: &winapi::shared::minwindef::FILETIME) -> u64 {
        (ft.dwHighDateTime as u64) << 32 | (ft.dwLowDateTime as u64)
    }

    pub fn get_fps(&self) -> f32 {
        self.current_fps
    }

    pub fn get_frame_time_ms(&self) -> f32 {
        if let Some(start_time) = self.frame_start_time {
            let now = Self::get_timestamp_ms();
            (now - start_time) as f32
        } else {
            0.0
        }
    }

    pub fn get_frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn get_snapshots(&self) -> Vec<PerformanceSnapshot> {
        self.snapshots.lock().iter().cloned().collect()
    }

    pub fn get_latest_snapshot(&self) -> Option<PerformanceSnapshot> {
        self.snapshots.lock().back().cloned()
    }

    pub fn clear(&mut self) {
        self.snapshots.lock().clear();
        self.frame_count = 0;
        self.fps_frame_count = 0;
        self.current_fps = 0.0;
        self.has_prev_cpu_times = false;
    }

    pub fn get_average_fps(&self) -> f32 {
        let snapshots = self.snapshots.lock();
        if snapshots.is_empty() {
            return 0.0;
        }

        let sum: f32 = snapshots.iter().map(|s| s.fps).sum();
        sum / snapshots.len() as f32
    }

    pub fn get_average_frame_time(&self) -> f32 {
        let snapshots = self.snapshots.lock();
        if snapshots.is_empty() {
            return 0.0;
        }

        let sum: f32 = snapshots.iter().map(|s| s.frame_time_ms).sum();
        sum / snapshots.len() as f32
    }

    pub fn get_average_memory(&self) -> f32 {
        let snapshots = self.snapshots.lock();
        if snapshots.is_empty() {
            return 0.0;
        }

        let sum: f32 = snapshots.iter().map(|s| s.memory_used_mb).sum();
        sum / snapshots.len() as f32
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}