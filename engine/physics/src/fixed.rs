/// 固定时间步累加器。
///
/// 3D 动作游戏的手感要求物理步长与帧率解耦：渲染帧率可以变化，
/// 物理始终以固定 `step` 推进，并通过 `alpha()` 做渲染插值。
///
/// 使用方式：
/// ```ignore
/// let mut timestep = FixedTimestep::new(60.0);
/// // 每帧
/// let steps = world.advance_fixed(&mut timestep, frame_dt);
/// let alpha = timestep.alpha(); // 0..1，用于渲染插值
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FixedTimestep {
    step: f32,
    accumulator: f32,
    max_steps: u32,
    max_frame_dt: f32,
    alpha: f32,
}

impl FixedTimestep {
    /// 默认物理频率（Hz）。
    pub const DEFAULT_FPS: f32 = 60.0;
    /// 单帧最多补跑的物理步数，防止“死亡螺旋”。
    pub const DEFAULT_MAX_STEPS: u32 = 8;
    /// 单帧最大输入时间（秒），超过部分被截断（如切后台回来时的大 dt）。
    pub const DEFAULT_MAX_FRAME_DT: f32 = 0.25;

    /// 以频率（Hz）创建，例如 `FixedTimestep::new(60.0)`。
    pub fn new(fps: f32) -> Self {
        let fps = if fps.is_finite() && fps > 0.0 {
            fps
        } else {
            Self::DEFAULT_FPS
        };
        Self::with_step(1.0 / fps)
    }

    /// 以步长（秒）创建。
    pub fn with_step(step: f32) -> Self {
        let step = if step.is_finite() && step > 0.0 {
            step
        } else {
            1.0 / Self::DEFAULT_FPS
        };
        Self {
            step,
            accumulator: 0.0,
            max_steps: Self::DEFAULT_MAX_STEPS,
            max_frame_dt: Self::DEFAULT_MAX_FRAME_DT,
            alpha: 0.0,
        }
    }

    /// 固定步长（秒）。
    pub fn step(&self) -> f32 {
        self.step
    }

    /// 固定频率（Hz）。
    pub fn fps(&self) -> f32 {
        1.0 / self.step
    }

    /// 未消耗的时间余量（秒），用于渲染插值。
    pub fn accumulator(&self) -> f32 {
        self.accumulator
    }

    /// 渲染插值系数，范围 `0..=1`。
    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    pub fn max_steps(&self) -> u32 {
        self.max_steps
    }

    /// 设置单帧最多补跑的步数（0 视为 1）。
    pub fn set_max_steps(&mut self, max_steps: u32) {
        self.max_steps = max_steps.max(1);
    }

    pub fn max_frame_dt(&self) -> f32 {
        self.max_frame_dt
    }

    /// 设置单帧最大输入时间（秒）。
    pub fn set_max_frame_dt(&mut self, max_frame_dt: f32) {
        if max_frame_dt.is_finite() && max_frame_dt > 0.0 {
            self.max_frame_dt = max_frame_dt;
        }
    }

    /// 累加一帧的真实时间，返回本帧应执行的固定步数。
    ///
    /// - 负的 `frame_dt` 按 0 处理；
    /// - 超过 `max_frame_dt` 的部分被丢弃；
    /// - 补跑步数达到 `max_steps` 后丢弃剩余积压时间，避免螺旋。
    pub fn advance(&mut self, frame_dt: f32) -> u32 {
        let dt = if frame_dt.is_finite() {
            frame_dt.clamp(0.0, self.max_frame_dt)
        } else {
            0.0
        };
        self.accumulator += dt;

        let mut steps = 0;
        // 容差避免浮点误差导致少跑一步（例如 2 * (1/120) 略小于 1/60）。
        let tolerance = self.step * 1e-4;
        while self.accumulator + tolerance >= self.step && steps < self.max_steps {
            self.accumulator -= self.step;
            steps += 1;
        }
        // 补跑达到上限仍有积压：丢弃，防止时间越欠越多。
        if steps >= self.max_steps && self.accumulator >= self.step {
            self.accumulator %= self.step;
        }
        if self.accumulator < 0.0 {
            self.accumulator = 0.0;
        }

        self.alpha = (self.accumulator / self.step).clamp(0.0, 1.0);
        steps
    }

    /// 清空累加器（传送/切场景后调用）。
    pub fn reset(&mut self) {
        self.accumulator = 0.0;
        self.alpha = 0.0;
    }
}

impl Default for FixedTimestep {
    fn default() -> Self {
        Self::new(Self::DEFAULT_FPS)
    }
}
