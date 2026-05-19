use parking_lot::Mutex;
use std::collections::HashMap;
use std::ffi::CString;
use std::sync::Arc;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TracePoint {
    pub name: *const std::os::raw::c_char,
    pub category: *const std::os::raw::c_char,
    pub start_time: u64,
    pub end_time: u64,
    pub duration_ns: u64,
    pub thread_id: u64,
}

unsafe impl Send for TracePoint {}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CounterPoint {
    pub name: *const std::os::raw::c_char,
    pub category: *const std::os::raw::c_char,
    pub value: i64,
    pub timestamp: u64,
}

unsafe impl Send for CounterPoint {}

pub struct TraceAnalyzer {
    points: Arc<Mutex<Vec<TracePoint>>>,
    counters: Arc<Mutex<HashMap<String, Vec<CounterPoint>>>>,
    active_points: Arc<Mutex<HashMap<String, u64>>>,
    max_points: usize,
    enabled: bool,
}

impl TraceAnalyzer {
    pub fn new() -> Self {
        Self {
            points: Arc::new(Mutex::new(Vec::new())),
            counters: Arc::new(Mutex::new(HashMap::new())),
            active_points: Arc::new(Mutex::new(HashMap::new())),
            max_points: 10000,
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

    pub fn begin_point(&mut self, name: &str, _category: &str) {
        if !self.enabled {
            return;
        }

        let start_time = Self::get_timestamp_ns();
        self.active_points
            .lock()
            .insert(name.to_string(), start_time);
    }

    pub fn end_point(&mut self, name: &str, category: &str) {
        if !self.enabled {
            return;
        }

        let end_time = Self::get_timestamp_ns();

        if let Some(start_time) = self.active_points.lock().remove(name) {
            let duration_ns = end_time - start_time;
            let thread_id = Self::get_thread_id();

            let name_c = CString::new(name).unwrap().into_raw();
            let category_c = CString::new(category).unwrap().into_raw();

            let point = TracePoint {
                name: name_c,
                category: category_c,
                start_time,
                end_time,
                duration_ns,
                thread_id,
            };

            {
                let mut points = self.points.lock();
                if points.len() >= self.max_points {
                    points.remove(0);
                }
                points.push(point);
            }
        }
    }

    pub fn set_counter(&mut self, name: &str, category: &str, value: i64) {
        if !self.enabled {
            return;
        }

        let timestamp = Self::get_timestamp_ns();

        let name_c = CString::new(name).unwrap().into_raw();
        let category_c = CString::new(category).unwrap().into_raw();

        let counter = CounterPoint {
            name: name_c,
            category: category_c,
            value,
            timestamp,
        };

        self.counters
            .lock()
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(counter);
    }

    pub fn increment_counter(&mut self, name: &str, category: &str) {
        if !self.enabled {
            return;
        }

        let current = {
            let counters = self.counters.lock();
            counters
                .get(name)
                .and_then(|v| v.last())
                .map(|c| c.value)
                .unwrap_or(0)
        };

        self.set_counter(name, category, current + 1);
    }

    fn get_timestamp_ns() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};

        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    fn get_thread_id() -> u64 {
        #[cfg(windows)]
        {
            unsafe { winapi::um::processthreadsapi::GetCurrentThreadId() as u64 }
        }

        #[cfg(unix)]
        {
            unsafe { libc::pthread_self() as u64 }
        }

        #[cfg(not(any(windows, unix)))]
        {
            0
        }
    }

    pub fn get_points(&self) -> Vec<TracePoint> {
        self.points.lock().clone()
    }

    pub fn get_counters(&self, name: &str) -> Vec<CounterPoint> {
        self.counters.lock().get(name).cloned().unwrap_or_default()
    }

    pub fn get_all_counters(&self) -> HashMap<String, Vec<CounterPoint>> {
        self.counters.lock().clone()
    }

    pub fn clear(&mut self) {
        self.points.lock().clear();
        self.counters.lock().clear();
        self.active_points.lock().clear();
    }

    pub fn export_json(&self) -> String {
        let points = self.points.lock();
        let counters = self.counters.lock();

        let mut json = String::from("{\"traceEvents\": [");

        for (i, point) in points.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }

            let name = unsafe { std::ffi::CStr::from_ptr(point.name).to_str().unwrap_or("") };
            let cat = unsafe {
                std::ffi::CStr::from_ptr(point.category)
                    .to_str()
                    .unwrap_or("")
            };

            json.push_str(&format!(
                "{{\"name\":\"{}\",\"cat\":\"{}\",\"ph\":\"X\",\"ts\":{},\"dur\":{},\"tid\":{}}}",
                name, cat, point.start_time, point.duration_ns, point.thread_id
            ));
        }

        for (_, counter_values) in counters.iter() {
            for counter in counter_values.iter() {
                json.push(',');

                let name = unsafe {
                    std::ffi::CStr::from_ptr(counter.name)
                        .to_str()
                        .unwrap_or("")
                };
                let cat = unsafe {
                    std::ffi::CStr::from_ptr(counter.category)
                        .to_str()
                        .unwrap_or("")
                };

                json.push_str(&format!(
                    "{{\"name\":\"{}\",\"cat\":\"{}\",\"ph\":\"C\",\"ts\":{},\"value\":{}}}",
                    name, cat, counter.timestamp, counter.value
                ));
            }
        }

        json.push_str("]}");
        json
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), String> {
        let json = self.export_json();

        std::fs::write(path, json).map_err(|e| format!("Failed to write trace file: {}", e))?;

        Ok(())
    }
}

impl Default for TraceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ScopedTrace {
    analyzer: Arc<Mutex<TraceAnalyzer>>,
    name: String,
    category: String,
}

impl ScopedTrace {
    pub fn new(analyzer: Arc<Mutex<TraceAnalyzer>>, name: &str, category: &str) -> Self {
        analyzer.lock().begin_point(name, category);

        Self {
            analyzer,
            name: name.to_string(),
            category: category.to_string(),
        }
    }
}

impl Drop for ScopedTrace {
    fn drop(&mut self) {
        self.analyzer.lock().end_point(&self.name, &self.category);
    }
}
