//! 音频后端抽象。
//!
//! 引擎核心（解码/缓存/混音/3D）与“声音如何送到声卡”解耦：
//! - [`AudioBackend`]：设备后端 trait；
//! - [`NullBackend`]：无设备后端。播放逻辑与状态机照常推进，只是不发声，
//!   用于无声卡环境、CI 和离线测试；
//! - [`MixerSink`]：提供给真实后端（cpal/kira）在设备回调线程里拉取混音数据的句柄。
//!
//! 接入真实后端的推荐做法（见 crate README「后端接入点」）：
//! 1. 新建模块/独立 crate 实现 [`AudioBackend`]；
//! 2. 在 `open` 里用 `sink.sample_rate()` 打开设备，把 `sink.clone()` 移入回调；
//! 3. 回调中调用 `sink.render(&mut data)`，`is_automatic()` 返回 `true`；
//! 4. 在 [`crate::backend::default_backend`] 里按平台返回该后端。

use std::sync::{Arc, Mutex};

use crate::engine::EngineInner;

/// 后端打开参数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendConfig {
    /// 期望输出采样率（Hz）。
    pub sample_rate: u32,
    /// 输出声道数（引擎固定为 2，立体声）。
    pub output_channels: u16,
}

/// 音频后端错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendError {
    /// 错误描述（会写入引擎降级原因）。
    pub message: String,
}

impl BackendError {
    /// 构造后端错误。
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for BackendError {}

/// 混音数据拉取句柄：后端设备回调线程通过它读取混音结果。
///
/// 内部与 [`crate::engine::AudioEngine`] 共享同一把锁，因此直接调用
/// [`MixerSink::render`] 即可，无需了解引擎内部结构。
#[derive(Clone)]
pub struct MixerSink {
    inner: Arc<Mutex<EngineInner>>,
}

impl std::fmt::Debug for MixerSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MixerSink")
            .field("sample_rate", &self.sample_rate())
            .finish()
    }
}

impl MixerSink {
    /// 由引擎创建（外部后端只接收、不构造）。
    pub(crate) fn new(inner: Arc<Mutex<EngineInner>>) -> Self {
        Self { inner }
    }

    /// 混音输出采样率。
    pub fn sample_rate(&self) -> u32 {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.mixer.sample_rate()
    }

    /// 渲染一块交错立体声 f32 样本（长度应为 `帧数 * 2`）。
    ///
    /// 返回实际渲染帧数。设备回调线程直接调用即可。
    pub fn render(&self, out: &mut [f32]) -> usize {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.mixer.render_interleaved(out)
    }
}

/// 音频设备后端。
///
/// 实现者需要是 `Send + Sync`：引擎可能被编辑器主线程与资源加载线程同时使用。
pub trait AudioBackend: Send + Sync {
    /// 后端名称（日志/状态展示）。
    fn name(&self) -> &'static str;

    /// 打开输出设备并开始播放。
    ///
    /// - 成功后，若 [`AudioBackend::is_automatic`] 为 `true`，后端负责周期性调用
    ///   [`MixerSink::render`]；引擎的 `update` 将不再手动推进帧。
    /// - 失败时返回 [`BackendError`]，引擎会自动降级到 [`NullBackend`]。
    fn open(&mut self, config: &BackendConfig, sink: MixerSink) -> Result<(), BackendError>;

    /// 关闭设备、停止回调线程（幂等）。
    fn close(&mut self);

    /// 设备是否已打开。
    fn is_open(&self) -> bool;

    /// 后端是否自动拉取混音数据。
    ///
    /// `true`：有独立设备线程；`false`：需要引擎 `update(dt)` 手动推进（NullBackend）。
    fn is_automatic(&self) -> bool {
        true
    }
}

/// 无声卡后端：忽略所有数据，不产生声音，但引擎状态机照常推进。
///
/// 适用场景：CI/无音频设备/音频依赖不可用。`open` 永不失败。
#[derive(Debug, Default)]
pub struct NullBackend {
    opened: bool,
}

impl NullBackend {
    /// 创建无声后端。
    pub fn new() -> Self {
        Self::default()
    }

    /// 以 `Box<dyn AudioBackend>` 形式返回（便于传给引擎）。
    pub fn boxed() -> Box<dyn AudioBackend> {
        Box::new(Self::new())
    }
}

impl AudioBackend for NullBackend {
    fn name(&self) -> &'static str {
        "null"
    }

    fn open(&mut self, _config: &BackendConfig, _sink: MixerSink) -> Result<(), BackendError> {
        self.opened = true;
        Ok(())
    }

    fn close(&mut self) {
        self.opened = false;
    }

    fn is_open(&self) -> bool {
        self.opened
    }

    fn is_automatic(&self) -> bool {
        false
    }
}

/// 默认后端工厂。
///
/// - 启用 `cpal-backend` feature：返回 `CpalBackend`，
///   由引擎尝试打开默认输出设备（失败自动降级为 [`NullBackend`]）；
/// - 未启用：返回 [`NullBackend`]，保证任何机器/CI 都可运行、可离线测试。
pub fn default_backend() -> Box<dyn AudioBackend> {
    #[cfg(feature = "cpal-backend")]
    let backend: Box<dyn AudioBackend> = crate::backend_cpal::CpalBackend::boxed();
    #[cfg(not(feature = "cpal-backend"))]
    let backend: Box<dyn AudioBackend> = NullBackend::boxed();
    backend
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_backend_lifecycle() {
        let mut backend = NullBackend::new();
        assert_eq!(backend.name(), "null");
        assert!(!backend.is_open());
        assert!(!backend.is_automatic());
        // 构造一个 sink 需要引擎，这里只验证 open 的空操作语义由引擎测试覆盖。
        assert_eq!(NullBackend::boxed().name(), "null");
        backend.close();
        assert!(!backend.is_open());
    }

    #[test]
    fn default_backend_matches_feature() {
        let backend = default_backend();
        // 未启用 cpal feature 时为 null；启用后为 cpal（设备是否可用不影响名称）。
        #[cfg(not(feature = "cpal-backend"))]
        assert_eq!(backend.name(), "null");
        #[cfg(feature = "cpal-backend")]
        assert_eq!(backend.name(), "cpal");
        assert!(!backend.is_open());
    }

    #[test]
    fn backend_error_is_display_and_error() {
        let error = BackendError::new("没有可用的音频设备");
        assert_eq!(error.to_string(), "没有可用的音频设备");
        let boxed: Box<dyn std::error::Error> = Box::new(error.clone());
        assert!(boxed.to_string().contains("音频设备"));
    }
}
