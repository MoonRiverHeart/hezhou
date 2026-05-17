use crate::types::*;
use crate::widget::*;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum AnimationCurve {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Spring(SpringParams),
    Bounce,
}

impl AnimationCurve {
    pub fn interpolate(&self, progress: f32) -> f32 {
        match self {
            Self::Linear => progress,

            Self::EaseIn => progress * progress,

            Self::EaseOut => 1.0 - (1.0 - progress) * (1.0 - progress),

            Self::EaseInOut => {
                if progress < 0.5 {
                    2.0 * progress * progress
                } else {
                    1.0 - 2.0 * (1.0 - progress) * (1.0 - progress)
                }
            }

            Self::Spring(params) => self.spring_curve(progress, params),

            Self::Bounce => self.bounce_curve(progress),
        }
    }

    fn spring_curve(&self, progress: f32, params: &SpringParams) -> f32 {
        let damping = params.damping;
        let stiffness = params.stiffness;

        let omega = (stiffness / damping).sqrt();
        let decay = (-damping * progress).exp();
        let oscillation = (omega * progress).cos();

        1.0 - decay * oscillation * (1.0 - progress)
    }

    fn bounce_curve(&self, progress: f32) -> f32 {
        let n1 = 7.5625;
        let d1 = 2.75;

        if progress < 1.0 / d1 {
            n1 * progress * progress
        } else if progress < 2.0 / d1 {
            let t = progress - 1.5 / d1;
            n1 * t * t + 0.75
        } else if progress < 2.5 / d1 {
            let t = progress - 2.25 / d1;
            n1 * t * t + 0.9375
        } else {
            let t = progress - 2.625 / d1;
            n1 * t * t + 0.984375
        }
    }
}

impl Default for AnimationCurve {
    fn default() -> Self {
        Self::Linear
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SpringParams {
    pub damping: f32,
    pub stiffness: f32,
}

impl SpringParams {
    pub fn new() -> Self {
        Self {
            damping: 1.0,
            stiffness: 100.0,
        }
    }
}

impl Default for SpringParams {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimatedProperty {
    Opacity,
    Width,
    Height,
    X,
    Y,
    Scale,
    Rotation,
    BorderRadius,
    Color,
    Custom(u32),
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnimationId {
    pub id: u64,
}

impl AnimationId {
    pub fn new() -> Self {
        use uuid::Uuid;
        Self {
            id: Uuid::new_v4().as_u128() as u64,
        }
    }

    pub fn invalid() -> Self {
        Self { id: 0 }
    }
}

impl Default for AnimationId {
    fn default() -> Self {
        Self::invalid()
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Animation {
    pub id: AnimationId,
    pub target: WidgetId,
    pub property: AnimatedProperty,
    pub from_value: f32,
    pub to_value: f32,
    pub duration: f32,
    pub delay: f32,
    pub curve: AnimationCurve,
    pub repeat_count: u32,
    pub auto_reverse: bool,
    pub running: bool,
    pub paused: bool,
    pub elapsed_time: f32,
}

impl Animation {
    pub fn new(
        target: WidgetId,
        property: AnimatedProperty,
        from: f32,
        to: f32,
        duration: f32,
    ) -> Self {
        Self {
            id: AnimationId::new(),
            target,
            property,
            from_value: from,
            to_value: to,
            duration,
            delay: 0.0,
            curve: AnimationCurve::EaseInOut,
            repeat_count: 1,
            auto_reverse: false,
            running: false,
            paused: false,
            elapsed_time: 0.0,
        }
    }

    pub fn with_delay(mut self, delay: f32) -> Self {
        self.delay = delay;
        self
    }

    pub fn with_curve(mut self, curve: AnimationCurve) -> Self {
        self.curve = curve;
        self
    }

    pub fn with_repeat(mut self, count: u32) -> Self {
        self.repeat_count = count;
        self
    }

    pub fn with_auto_reverse(mut self) -> Self {
        self.auto_reverse = true;
        self
    }

    pub fn current_value(&self) -> f32 {
        let progress = if self.duration > 0.0 {
            (self.elapsed_time / self.duration).min(1.0)
        } else {
            1.0
        };

        let curve_progress = self.curve.interpolate(progress);
        self.from_value + (self.to_value - self.from_value) * curve_progress
    }

    pub fn is_complete(&self) -> bool {
        self.elapsed_time >= self.duration
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationState {
    Started,
    Running,
    Paused,
    Completed,
    Cancelled,
}

pub type AnimationCallback = extern "C" fn(AnimationId, AnimationState);
