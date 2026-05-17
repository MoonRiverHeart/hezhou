pub struct Time {
    pub elapsed: f64,
    pub delta: f32,
    pub frame_count: u64,
    pub fixed_delta: f32,
    pub fixed_accumulator: f32,
}

impl Time {
    pub fn new() -> Self {
        Self {
            elapsed: 0.0,
            delta: 0.0,
            frame_count: 0,
            fixed_delta: 1.0 / 60.0,
            fixed_accumulator: 0.0,
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        self.elapsed += delta_time as f64;
        self.delta = delta_time;
        self.frame_count += 1;
        self.fixed_accumulator += delta_time;
    }

    pub fn needs_fixed_update(&self) -> bool {
        self.fixed_accumulator >= self.fixed_delta
    }

    pub fn consume_fixed_delta(&mut self) -> f32 {
        let delta = self.fixed_delta;
        self.fixed_accumulator -= delta;
        delta
    }

    pub fn reset(&mut self) {
        self.elapsed = 0.0;
        self.delta = 0.0;
        self.frame_count = 0;
        self.fixed_accumulator = 0.0;
    }
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MainLoop {
    target_fps: f32,
    vsync: bool,
    is_running: bool,
}

impl MainLoop {
    pub fn new(target_fps: f32, vsync: bool) -> Self {
        Self {
            target_fps,
            vsync,
            is_running: false,
        }
    }

    pub fn start(&mut self) {
        self.is_running = true;
    }

    pub fn stop(&mut self) {
        self.is_running = false;
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn ideal_delta_time(&self) -> f32 {
        if self.target_fps > 0.0 {
            1.0 / self.target_fps
        } else {
            1.0 / 60.0
        }
    }
}

impl Default for MainLoop {
    fn default() -> Self {
        Self::new(60.0, true)
    }
}
