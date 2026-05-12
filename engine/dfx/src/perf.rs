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
                self.current_fps = self.fps_frame_count as f32 * 1000.0 / (end_time - self.last_fps_time) as f32;
                self.fps_frame_count = 0;
                self.last_fps_time = end_time;
            }
            
            let cpu_usage = Self::get_cpu_usage();
            let (memory_used, memory_available) = Self::get_memory_info();
            
            let snapshot = PerformanceSnapshot {
                timestamp: end_time,
                fps: self.current_fps,
                frame_time_ms,
                cpu_usage_percent: cpu_usage,
                memory_used_mb: memory_used,
                memory_available_mb: memory_available,
                draw_calls: 0,
                triangle_count: 0,
            };
            
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
    
    fn get_timestamp_ms() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
    
    fn get_cpu_usage() -> f32 {
        #[cfg(windows)]
        {
            0.0
        }
        
        #[cfg(unix)]
        {
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
            (0.0, 0.0)
        }
        
        #[cfg(unix)]
        {
            (0.0, 0.0)
        }
        
        #[cfg(not(any(windows, unix)))]
        {
            (0.0, 0.0)
        }
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