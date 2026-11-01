//! cpal 真实音频输出后端（feature `cpal-backend`）。
//!
//! 设计（对应 ADR-001：cpal + 自研混音）：
//! - [`crate::engine::AudioEngine`] 仍持有纯 Rust [`crate::mixer::Mixer`]；
//! - 本后端在**独立线程**中创建 `cpal::Stream` 并保持存活，设备回调线程通过
//!   [`MixerSink::render`] 拉取混音数据 —— 这样不依赖 `cpal::Stream: Send`
//!   （部分平台不满足），也避免阻塞引擎主线程；
//! - 只使用 f32 立体声输出（引擎混音输出固定 48kHz 左右、2 声道）；
//! - 没有设备/配置不支持/打开失败时 `open` 返回 [`BackendError`]，
//!   [`crate::engine::AudioEngine`] 会自动降级为 [`crate::backend::NullBackend`]。
//!
//! 使用方式：
//! ```no_run
//! # #[cfg(feature = "cpal-backend")] {
//! use hezhou_audio::{AudioEngine, default_backend};
//! let engine = AudioEngine::new(default_backend()); // 启用 feature 时为 cpal
//! # let _ = engine; }
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, StreamConfig};

use crate::backend::{AudioBackend, BackendConfig, BackendError, MixerSink};

/// 线程轮询间隔（用于等待 stop 信号；20ms 对关闭延迟足够小）。
const THREAD_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// 引擎允许的采样率范围（超出直接返回参数错误，避免打开异常设备配置）。
const MIN_SAMPLE_RATE: u32 = 8_000;
const MAX_SAMPLE_RATE: u32 = 384_000;

/// cpal 音频输出后端。
///
/// 线程安全：`Stream` 由内部线程持有，本结构只保存停止信号与线程句柄。
pub struct CpalBackend {
    stop: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
    opened: bool,
    device_name: Option<String>,
}

impl std::fmt::Debug for CpalBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CpalBackend")
            .field("opened", &self.opened)
            .field("device_name", &self.device_name)
            .finish()
    }
}

impl CpalBackend {
    /// 创建未打开的 cpal 后端。
    pub fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            thread: Mutex::new(None),
            opened: false,
            device_name: None,
        }
    }

    /// 以 `Box<dyn AudioBackend>` 形式返回（便于传给引擎）。
    pub fn boxed() -> Box<dyn AudioBackend> {
        Box::new(Self::new())
    }

    /// 实际打开的输出设备名（未打开/未成功时为 `None`）。
    ///
    /// 用于编辑器/日志展示；引擎的 `EngineStatus` 只记录后端名 `"cpal"`。
    pub fn device_name(&self) -> Option<&str> {
        self.device_name.as_deref()
    }

    fn thread_lock(&self) -> std::sync::MutexGuard<'_, Option<JoinHandle<()>>> {
        self.thread
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Default for CpalBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioBackend for CpalBackend {
    fn name(&self) -> &'static str {
        "cpal"
    }

    fn open(&mut self, config: &BackendConfig, sink: MixerSink) -> Result<(), BackendError> {
        // 参数校验放在最前面：保证“明确的错误路径”不依赖真实声卡，可离线测试。
        if config.output_channels != 2 {
            return Err(BackendError::new(format!(
                "cpal 后端只支持立体声输出（引擎请求 {} 声道）",
                config.output_channels
            )));
        }
        if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&config.sample_rate) {
            return Err(BackendError::new(format!(
                "采样率 {} 超出 cpal 后端支持范围（{MIN_SAMPLE_RATE}..={MAX_SAMPLE_RATE} Hz）",
                config.sample_rate
            )));
        }

        // 允许重复 open（先关闭旧流）。
        self.close();

        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = Arc::clone(&stop);
        let sample_rate = config.sample_rate;
        let (ready_tx, ready_rx) = mpsc::channel::<Result<String, String>>();

        let thread = std::thread::Builder::new()
            .name("hezhou-audio-cpal".to_string())
            .spawn(move || run_output_stream(sample_rate, sink, stop_for_thread, ready_tx))
            .map_err(|error| BackendError::new(format!("创建 cpal 音频线程失败: {error}")))?;

        match ready_rx.recv() {
            Ok(Ok(device_name)) => {
                self.stop = stop;
                *self.thread_lock() = Some(thread);
                self.opened = true;
                self.device_name = Some(device_name);
                Ok(())
            }
            Ok(Err(message)) => {
                // 线程已退出，回收句柄避免泄漏。
                let _ = thread.join();
                Err(BackendError::new(message))
            }
            Err(_) => {
                let _ = thread.join();
                Err(BackendError::new("cpal 音频线程在初始化完成前退出"))
            }
        }
    }

    fn close(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let thread = self.thread_lock().take();
        if let Some(thread) = thread {
            let _ = thread.join();
        }
        self.opened = false;
        self.device_name = None;
    }

    fn is_open(&self) -> bool {
        self.opened
    }

    // is_automatic() 使用默认实现 true：设备回调线程自动拉取混音数据，
    // 引擎的 update(dt) 在此模式下返回 0（避免双倍推进）。
}

impl Drop for CpalBackend {
    fn drop(&mut self) {
        // 防止调用方忘记 close 导致音频线程泄漏。
        self.close();
    }
}

/// 音频线程主体：创建并持有 `cpal::Stream`，直到收到 stop 信号。
///
/// `ready` 用于把初始化结果同步回 `open`：成功返回设备名，失败返回错误描述。
fn run_output_stream(
    sample_rate: u32,
    sink: MixerSink,
    stop: Arc<AtomicBool>,
    ready: mpsc::Sender<Result<String, String>>,
) {
    let host = cpal::default_host();
    let Some(device) = host.default_output_device() else {
        let _ = ready.send(Err("没有可用的默认音频输出设备".to_string()));
        return;
    };
    let device_name = device.name().unwrap_or_else(|_| "<unknown>".to_string());

    let stream_config = match choose_stream_config(&device, sample_rate) {
        Ok(config) => config,
        Err(message) => {
            let _ = ready.send(Err(format!("默认输出设备 {device_name}: {message}")));
            return;
        }
    };

    let build_result = device.build_output_stream(
        &stream_config,
        move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
            // 设备回调线程：从混音器拉取一块交错立体声数据。
            sink.render(data);
        },
        |error| eprintln!("[hezhou-audio] cpal 输出流错误: {error}"),
        None,
    );
    let stream = match build_result {
        Ok(stream) => stream,
        Err(error) => {
            let _ = ready.send(Err(format!(
                "默认输出设备 {device_name}: 创建输出流失败: {error}"
            )));
            return;
        }
    };
    if let Err(error) = stream.play() {
        let _ = ready.send(Err(format!(
            "默认输出设备 {device_name}: 启动输出流失败: {error}"
        )));
        return;
    }

    let _ = ready.send(Ok(device_name));
    while !stop.load(Ordering::SeqCst) {
        std::thread::sleep(THREAD_POLL_INTERVAL);
    }
    // 显式停止并释放设备；必须在创建它的线程内 drop（部分平台要求）。
    drop(stream);
}

/// 为设备选择 f32 立体声输出配置。
///
/// 优先在 `supported_output_configs` 中寻找“f32 + 2 声道 + 目标采样率”的精确配置；
/// 找不到时给出包含可用配置列表的可读错误（引擎会据此降级为无声后端）。
fn choose_stream_config(device: &cpal::Device, sample_rate: u32) -> Result<StreamConfig, String> {
    let target_rate = SampleRate(sample_rate);
    let configs = device
        .supported_output_configs()
        .map_err(|error| format!("查询输出配置失败: {error}"))?;

    let mut available = Vec::new();
    let mut chosen: Option<StreamConfig> = None;
    for range in configs {
        let format = range.sample_format();
        let channels = range.channels();
        let min_rate = range.min_sample_rate().0;
        let max_rate = range.max_sample_rate().0;
        available.push(format!("{format:?}/{channels}ch/{min_rate}-{max_rate}Hz"));
        if format == SampleFormat::F32
            && channels == 2
            && min_rate <= sample_rate
            && sample_rate <= max_rate
        {
            chosen = Some(range.with_sample_rate(target_rate).config());
            break;
        }
    }
    if let Some(config) = chosen {
        return Ok(config);
    }

    // 兜底：设备默认配置恰好是 f32 双声道且采样率一致。
    if let Ok(default) = device.default_output_config() {
        if default.sample_format() == SampleFormat::F32
            && default.channels() == 2
            && default.sample_rate() == target_rate
        {
            return Ok(default.config());
        }
    }

    Err(format!(
        "没有找到 f32/2 声道/{sample_rate}Hz 输出配置；设备可用配置: [{}]",
        available.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{AudioEngine, EngineConfig};
    use crate::mixer::PlaybackState;
    use crate::sound::SoundData;

    #[test]
    fn backend_name_and_initial_state() {
        let backend = CpalBackend::new();
        assert_eq!(backend.name(), "cpal");
        assert!(!backend.is_open());
        assert!(backend.is_automatic());
        assert!(backend.device_name().is_none());
        // Close 幂等（未打开时安全）。
        let mut backend = backend;
        backend.close();
        assert!(!backend.is_open());
    }

    #[test]
    fn default_uses_new() {
        let backend = CpalBackend::default();
        assert_eq!(backend.name(), "cpal");
        assert!(!backend.is_open());
    }

    #[test]
    fn open_rejects_unsupported_sample_rate_without_touching_device() {
        // 引擎会把 sample_rate 下限钳到 1000；cpal 后端校验会拒绝并触发降级。
        let engine = AudioEngine::with_config(
            CpalBackend::boxed(),
            EngineConfig {
                sample_rate: 1_000,
                max_voices: 4,
            },
        );
        assert_eq!(engine.status().backend(), "null");
        assert!(engine.status().is_silent());
        let reason = engine.status().degraded_reason().unwrap_or_default();
        assert!(
            reason.contains("采样率"),
            "降级原因应说明采样率不合法: {reason}"
        );
        // 降级后仍可正常播放/推进。
        engine.load_sound(
            "tone",
            SoundData::sine("tone", 440.0, 0.05, 48_000).unwrap(),
        );
        assert!(engine.play("tone").is_some());
        assert!(engine.update(0.01) > 0);
    }

    /// 真实设备集成测试：有声卡时走 cpal，没有声卡时安全降级；两种环境都必须不 panic。
    ///
    /// CI 不要求声卡（该测试只在有设备时验证 cpal 路径，否则验证降级路径）。
    #[test]
    fn engine_with_cpal_backend_is_device_optional() {
        let engine = AudioEngine::new(CpalBackend::boxed());
        let status = engine.status();
        // 便于在 CI/本机用 `-- --nocapture` 观察实际走的是设备还是降级路径。
        eprintln!(
            "[hezhou-audio] cpal 引擎状态: backend={}, mode={:?}, reason={:?}",
            status.backend(),
            status.mode(),
            status.degraded_reason()
        );
        if engine.audio_output_available() {
            assert_eq!(status.backend(), "cpal");
            assert!(!status.is_silent());
        } else {
            // 无声卡环境：必须降级到 null 且原因可读。
            assert_eq!(status.backend(), "null");
            assert!(status.is_silent());
            assert!(status.degraded_reason().is_some());
        }

        engine.load_sound(
            "tone",
            SoundData::sine("tone", 440.0, 0.05, engine.sample_rate()).unwrap(),
        );
        let handle = engine.play("tone").expect("cpal 引擎应能播放");
        // 静音降级时必须手动推进；真实设备由回调线程推进。
        if !engine.audio_output_available() {
            assert!(engine.update(0.01) > 0);
        }
        // 不论哪种模式，都不允许 panic；状态是合法枚举值。
        let state = handle.state();
        assert!(matches!(
            state,
            PlaybackState::Playing | PlaybackState::Paused | PlaybackState::Stopped
        ));
    }

    #[test]
    fn repeated_open_and_close_are_safe() {
        // 打开-关闭-再打开（有设备时是真实设备，无设备时是快速失败）。
        let mut backend = CpalBackend::new();
        // 没有引擎无法构造 MixerSink，这里只验证 close 的幂等性与可重复性。
        backend.close();
        backend.close();
        assert!(!backend.is_open());
    }
}
