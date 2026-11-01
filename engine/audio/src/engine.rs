//! 音频引擎门面：资源加载/缓存、播放句柄、总线/主音量、监听者与帧推进。
//!
//! # 设计要点
//!
//! - **单锁状态**：解码器注册表单独用 `RwLock`（避免解码大文件时阻塞音频回调），
//!   其余状态（混音器 + 资源缓存 + 声部 id）在一把 `Mutex` 内，回调线程与主线程
//!   共享同一份状态，无跨锁死锁风险。
//! - **无声卡降级**：后端 `open` 失败时自动换成 [`NullBackend`]，
//!   播放/暂停/停止/循环/音量/3D 衰减逻辑全部照常工作，只是不发声。
//! - **可离线测试**：默认后端为无声后端；`update(dt)` 或 `render_interleaved`
//!   手动推进混音时钟，测试不依赖真实声卡。
//!
//! # 基本用法
//!
//! ```
//! use hezhou_audio::{AudioEngine, SoundData};
//!
//! let engine = AudioEngine::null();
//! engine.load_sound("click", SoundData::sine("click", 880.0, 0.05, 48000).unwrap());
//! let handle = engine.play("click").unwrap();
//! engine.update(0.02);
//! assert_eq!(handle.state(), hezhou_audio::PlaybackState::Playing);
//! handle.stop();
//! ```

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, RwLock, Weak};

use crate::backend::{AudioBackend, BackendConfig, MixerSink, NullBackend};
use crate::bus::DEFAULT_BUS;
use crate::cache::AssetCache;
use crate::decoder::{AudioDecoder, DecoderRegistry};
use crate::error::{AudioError, AudioResult};
use crate::mixer::{Mixer, PlaybackState, Voice, VoiceId};
use crate::sound::{SoundData, OUTPUT_CHANNELS};
use crate::spatial::{Listener, SpatialParams, Vec3};

/// `update` 单次最大推进时长（秒）。
///
/// 断点/卡顿导致 `dt` 过大时限制推进量，避免一次渲染几十秒音频造成卡死。
pub const MAX_UPDATE_SECONDS: f32 = 10.0;

/// 引擎配置。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineConfig {
    /// 混音/输出采样率（Hz）。
    pub sample_rate: u32,
    /// 同时活动声部上限（超过后新播放请求被丢弃并计数）。
    pub max_voices: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            max_voices: 64,
        }
    }
}

/// 音频输出模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioOutputMode {
    /// 已打开真实音频设备（后端自动拉取混音数据）。
    Device,
    /// 静音模式：没有可用设备或后端打开失败，逻辑照常推进但不发声。
    Silent,
}

/// 引擎状态快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineStatus {
    backend: String,
    mode: AudioOutputMode,
    sample_rate: u32,
    degraded_reason: Option<String>,
}

impl EngineStatus {
    /// 当前后端名称（如 `"null"`）。
    pub fn backend(&self) -> &str {
        &self.backend
    }

    /// 输出模式。
    pub fn mode(&self) -> AudioOutputMode {
        self.mode
    }

    /// 混音输出采样率。
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// 降级原因（设备打开失败时为 `Some`；主动使用无声后端时为 `None`）。
    pub fn degraded_reason(&self) -> Option<&str> {
        self.degraded_reason.as_deref()
    }

    /// 是否处于静音（无设备）模式。
    pub fn is_silent(&self) -> bool {
        self.mode == AudioOutputMode::Silent
    }
}

/// 引擎运行统计快照。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EngineStats {
    /// 活动声部数（播放中 + 暂停）。
    pub active_voices: usize,
    /// 已缓存音频资源数。
    pub cached_sounds: usize,
    /// 资源缓存命中次数。
    pub cache_hits: u64,
    /// 资源缓存未命中次数。
    pub cache_misses: u64,
    /// 混音器已渲染总帧数。
    pub rendered_frames: u64,
    /// 因超过声部上限被丢弃的播放请求数。
    pub dropped_plays: u64,
}

/// 播放参数（[`AudioEngine::play_with`]）。
#[derive(Debug, Clone, PartialEq)]
pub struct PlayOptions {
    /// 已加载资源 id。
    pub sound: String,
    /// 声部音量（`0..=1`）。
    pub volume: f32,
    /// 是否循环。
    pub looping: bool,
    /// 所属总线（空字符串视为默认总线）。
    pub bus: String,
    /// 3D 参数；`None` 表示 2D 播放。
    pub spatial: Option<SpatialParams>,
}

impl PlayOptions {
    /// 以默认参数创建播放选项（音量 1、不循环、默认总线、2D）。
    pub fn new(sound: impl Into<String>) -> Self {
        Self {
            sound: sound.into(),
            volume: 1.0,
            looping: false,
            bus: DEFAULT_BUS.to_string(),
            spatial: None,
        }
    }

    /// 设置音量。
    pub fn volume(mut self, volume: f32) -> Self {
        self.volume = volume;
        self
    }

    /// 设置循环。
    pub fn looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    /// 设置总线。
    pub fn bus(mut self, bus: impl Into<String>) -> Self {
        self.bus = bus.into();
        self
    }

    /// 设置为 3D 播放。
    pub fn spatial(mut self, spatial: SpatialParams) -> Self {
        self.spatial = Some(spatial);
        self
    }
}

/// 可克隆的播放句柄。
///
/// 句柄只是“声部 id + 引擎弱引用”：**丢弃句柄不会停止播放**（音效由引擎持有），
/// 声部播放结束后句柄自动失效（[`SoundHandle::is_valid`] 为 `false`，状态视为
/// [`PlaybackState::Stopped`]）。
#[derive(Clone)]
pub struct SoundHandle {
    id: VoiceId,
    inner: Weak<Mutex<EngineInner>>,
}

impl std::fmt::Debug for SoundHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SoundHandle")
            .field("id", &self.id)
            .field("state", &self.state())
            .finish()
    }
}

impl SoundHandle {
    pub(crate) fn new(id: VoiceId, inner: Weak<Mutex<EngineInner>>) -> Self {
        Self { id, inner }
    }

    fn with_inner<R>(&self, function: impl FnOnce(&mut EngineInner) -> R) -> Option<R> {
        let inner = self.inner.upgrade()?;
        let mut guard = inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Some(function(&mut guard))
    }

    /// 声部 id（在引擎生命周期内唯一，可传给 C ABI）。
    pub fn id(&self) -> VoiceId {
        self.id
    }

    /// 声部是否仍存在（播放中或暂停）。
    pub fn is_valid(&self) -> bool {
        self.with_inner(|inner| inner.mixer.voice(self.id).is_some())
            .unwrap_or(false)
    }

    /// 当前播放状态（声部已结束/无效时返回 [`PlaybackState::Stopped`]）。
    pub fn state(&self) -> PlaybackState {
        self.with_inner(|inner| inner.mixer.voice(self.id).map(Voice::state))
            .flatten()
            .unwrap_or(PlaybackState::Stopped)
    }

    /// 暂停；仅当正在播放时返回 `true`。
    pub fn pause(&self) -> bool {
        self.with_inner(|inner| inner.mixer.pause_voice(self.id))
            .unwrap_or(false)
    }

    /// 继续播放；仅当处于暂停时返回 `true`。
    pub fn resume(&self) -> bool {
        self.with_inner(|inner| inner.mixer.resume_voice(self.id))
            .unwrap_or(false)
    }

    /// 停止并释放声部；声部已不存在时返回 `false`。
    pub fn stop(&self) -> bool {
        self.with_inner(|inner| inner.mixer.stop_voice(self.id))
            .unwrap_or(false)
    }

    /// 设置声部音量（`0..=1`，自动 clamp）。
    pub fn set_volume(&self, volume: f32) -> bool {
        self.with_inner(|inner| inner.mixer.set_voice_volume(self.id, volume))
            .unwrap_or(false)
    }

    /// 声部音量（无效句柄返回 0）。
    pub fn volume(&self) -> f32 {
        self.with_inner(|inner| inner.mixer.voice(self.id).map(Voice::volume))
            .flatten()
            .unwrap_or(0.0)
    }

    /// 设置循环标记。
    pub fn set_looping(&self, looping: bool) -> bool {
        self.with_inner(|inner| inner.mixer.set_voice_looping(self.id, looping))
            .unwrap_or(false)
    }

    /// 是否循环。
    pub fn is_looping(&self) -> bool {
        self.with_inner(|inner| inner.mixer.voice(self.id).map(Voice::is_looping))
            .flatten()
            .unwrap_or(false)
    }

    /// 更新 3D 位置；2D 声部/无效句柄返回 `false`。
    pub fn set_position(&self, position: Vec3) -> bool {
        self.with_inner(|inner| inner.mixer.set_voice_position(self.id, position))
            .unwrap_or(false)
    }

    /// 3D 声源当前位置；2D 声部返回 `None`。
    pub fn position_3d(&self) -> Option<Vec3> {
        self.with_inner(|inner| {
            inner
                .mixer
                .voice(self.id)
                .and_then(|voice| voice.spatial().map(|spatial| spatial.position))
        })
        .flatten()
    }

    /// 切换所属总线（总线不存在时自动创建）。
    pub fn set_bus(&self, bus: &str) -> bool {
        self.with_inner(|inner| inner.mixer.set_voice_bus(self.id, bus))
            .unwrap_or(false)
    }

    /// 当前所属总线名。
    pub fn bus(&self) -> Option<String> {
        self.with_inner(|inner| {
            inner
                .mixer
                .voice(self.id)
                .map(|voice| voice.bus().to_string())
        })
        .flatten()
    }

    /// 播放位置（秒，源采样率）。
    pub fn position_secs(&self) -> f32 {
        self.with_inner(|inner| inner.mixer.voice(self.id).map(Voice::position_secs))
            .flatten()
            .unwrap_or(0.0)
    }

    /// 音效总时长（秒）。
    pub fn duration_secs(&self) -> f32 {
        self.with_inner(|inner| inner.mixer.voice(self.id).map(Voice::duration_secs))
            .flatten()
            .unwrap_or(0.0)
    }

    /// 关联资源名（调试用）。
    pub fn sound_name(&self) -> Option<String> {
        self.with_inner(|inner| {
            inner
                .mixer
                .voice(self.id)
                .map(|voice| voice.sound().name().to_string())
        })
        .flatten()
    }
}

/// 引擎内部状态（后端回调与引擎 API 共享）。
pub(crate) struct EngineInner {
    pub(crate) mixer: Mixer,
    pub(crate) cache: AssetCache,
    pub(crate) next_voice_id: VoiceId,
    pub(crate) max_voices: usize,
    pub(crate) dropped_plays: u64,
    /// 后端是否自动拉取音频（`true` 时 `update` 不手动渲染）。
    pub(crate) automatic: bool,
    /// `update(dt)` 的帧数累加器（保留不足一帧的余量）。
    pub(crate) frame_accumulator: f64,
}

impl EngineInner {
    fn new(sample_rate: u32, max_voices: usize) -> Self {
        Self {
            mixer: Mixer::new(sample_rate),
            cache: AssetCache::new(),
            next_voice_id: 1,
            max_voices: max_voices.max(1),
            dropped_plays: 0,
            automatic: false,
            frame_accumulator: 0.0,
        }
    }
}

/// 音频引擎门面。
///
/// 线程安全：所有方法接收 `&self`，可放进 `Arc` 跨线程共享（编辑器主线程播放，
/// 资源线程加载）。真实后端的设备回调持有同一把内部锁。
pub struct AudioEngine {
    inner: Arc<Mutex<EngineInner>>,
    decoders: Arc<RwLock<DecoderRegistry>>,
    backend: Box<dyn AudioBackend>,
    status: EngineStatus,
}

impl std::fmt::Debug for AudioEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioEngine")
            .field("status", &self.status)
            .finish()
    }
}

impl AudioEngine {
    /// 使用默认配置 + 给定后端创建引擎。
    ///
    /// 后端 `open` 失败会自动降级到 [`NullBackend`]，不会 panic/返回错误。
    pub fn new(backend: Box<dyn AudioBackend>) -> Self {
        Self::with_config(backend, EngineConfig::default())
    }

    /// 使用自定义配置创建引擎。
    pub fn with_config(backend: Box<dyn AudioBackend>, config: EngineConfig) -> Self {
        let sample_rate = config.sample_rate.max(1_000);
        let inner = Arc::new(Mutex::new(EngineInner::new(sample_rate, config.max_voices)));
        let sink = MixerSink::new(Arc::clone(&inner));
        let backend_config = BackendConfig {
            sample_rate,
            output_channels: OUTPUT_CHANNELS as u16,
        };
        // 内置 WAV 解码器；启用 `ogg` feature 时自动追加 lewton OGG 解码器；
        // 其他格式由接入方通过 `register_decoder` 追加。
        let decoders = Arc::new(RwLock::new(DecoderRegistry::with_builtin()));

        let mut backend = backend;
        let status = match backend.open(&backend_config, sink.clone()) {
            Ok(()) => {
                let automatic = backend.is_automatic();
                EngineStatus {
                    backend: backend.name().to_string(),
                    mode: if automatic {
                        AudioOutputMode::Device
                    } else {
                        AudioOutputMode::Silent
                    },
                    sample_rate,
                    degraded_reason: None,
                }
            }
            Err(error) => {
                let reason = format!(
                    "音频设备打开失败（后端 {}）：{}；已降级为无声模式，播放逻辑照常运行",
                    backend.name(),
                    error
                );
                let mut fallback = NullBackend::new();
                let _ = fallback.open(&backend_config, sink);
                backend = Box::new(fallback);
                EngineStatus {
                    backend: backend.name().to_string(),
                    mode: AudioOutputMode::Silent,
                    sample_rate,
                    degraded_reason: Some(reason),
                }
            }
        };
        let automatic = backend.is_automatic();
        if let Ok(mut guard) = inner.lock() {
            guard.automatic = automatic;
        }

        Self {
            inner,
            decoders,
            backend,
            status,
        }
    }

    /// 创建**始终无声**的引擎（无声卡环境/CI/离线测试/确定性回放）。
    ///
    /// 与 [`AudioEngine::new`] 不同：即使启用了 `cpal-backend` feature 也不会打开
    /// 真实音频设备，状态机由 [`AudioEngine::update`]/[`AudioEngine::advance_frames`]
    /// 确定性推进。
    pub fn null() -> Self {
        Self::new(NullBackend::boxed())
    }

    /// 引擎状态快照。
    pub fn status(&self) -> &EngineStatus {
        &self.status
    }

    /// 是否有真实音频输出设备（`false` 表示静音降级，但播放逻辑可用）。
    pub fn audio_output_available(&self) -> bool {
        self.status.mode == AudioOutputMode::Device
    }

    /// 混音/输出采样率。
    pub fn sample_rate(&self) -> u32 {
        self.status.sample_rate
    }

    /// 注册自定义解码器（例如基于 lewton 的 OGG 解码器）。
    pub fn register_decoder(&self, decoder: Box<dyn AudioDecoder>) {
        let mut guard = self
            .decoders
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.register(decoder);
    }

    /// 已注册解码器名称。
    pub fn decoder_names(&self) -> Vec<&'static str> {
        let guard = self
            .decoders
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.decoder_names()
    }

    /// 从文件加载音频并缓存（按魔数自动选择解码器）。
    ///
    /// 支持的格式取决于已注册解码器：内置 WAV；启用 `ogg` feature 后同时支持 OGG/Vorbis。
    /// WAV 与 OGG 均可用此方法；保留 [`AudioEngine::load_wav_file`] 作为兼容别名。
    pub fn load_audio_file(
        &self,
        id: impl Into<String>,
        path: impl AsRef<Path>,
    ) -> AudioResult<()> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)
            .map_err(|error| AudioError::io(path.display().to_string(), &error))?;
        self.load_from_memory(id, &bytes)
    }

    /// 从 WAV 文件加载音频并缓存（兼容旧 API；等价于 [`AudioEngine::load_audio_file`]）。
    pub fn load_wav_file(&self, id: impl Into<String>, path: impl AsRef<Path>) -> AudioResult<()> {
        self.load_audio_file(id, path)
    }

    /// 从内存字节加载音频并缓存（按魔数选择解码器）。
    ///
    /// 若 id 已缓存则直接返回 `Ok(())`（计入缓存命中，不重复解码）。
    pub fn load_from_memory(&self, id: impl Into<String>, bytes: &[u8]) -> AudioResult<()> {
        let id = id.into();
        {
            let mut inner = self.core();
            // 命中则复用缓存（计命中）；未命中会计一次 miss。
            if inner.cache.get(&id).is_some() {
                return Ok(());
            }
        }
        let data = {
            let guard = self
                .decoders
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.decode(&id, bytes)?
        };
        let mut inner = self.core();
        inner.cache.insert(id, Arc::new(data));
        Ok(())
    }

    /// 加载已解码的音频数据（例如程序化生成/测试资源），返回缓存引用。
    pub fn load_sound(&self, id: impl Into<String>, data: SoundData) -> Arc<SoundData> {
        let mut inner = self.core();
        inner.cache.insert(id, Arc::new(data))
    }

    /// 查询已缓存资源（不改变命中统计）。
    pub fn sound(&self, id: &str) -> Option<Arc<SoundData>> {
        self.core().cache.peek(id).cloned()
    }

    /// 查询已缓存资源 id（排序）。
    pub fn cached_sound_ids(&self) -> Vec<String> {
        self.core().cache.ids()
    }

    /// 卸载资源；返回是否成功移除。
    pub fn unload(&self, id: &str) -> bool {
        self.core().cache.remove(id).is_some()
    }

    /// 播放 2D 音效。
    ///
    /// 资源不存在或超过声部上限时返回 `None`。
    pub fn play(&self, sound: &str) -> Option<SoundHandle> {
        self.play_with(PlayOptions::new(sound))
    }

    /// 播放 3D 音效（默认反比距离衰减）。
    pub fn play_3d(&self, sound: &str, position: Vec3) -> Option<SoundHandle> {
        self.play_with(PlayOptions::new(sound).spatial(SpatialParams::new(position)))
    }

    /// 按完整参数播放。
    pub fn play_with(&self, options: PlayOptions) -> Option<SoundHandle> {
        let mut inner = self.core();
        let sound = inner.cache.get(&options.sound)?;
        if inner.mixer.active_voice_count() >= inner.max_voices {
            inner.dropped_plays += 1;
            return None;
        }
        let bus = if options.bus.is_empty() {
            DEFAULT_BUS.to_string()
        } else {
            options.bus
        };
        inner.mixer.ensure_bus(&bus);
        let id = inner.next_voice_id;
        inner.next_voice_id = inner.next_voice_id.saturating_add(1);
        inner.mixer.add_voice(Voice::new(
            id,
            sound,
            options.volume,
            options.looping,
            bus,
            options.spatial,
        ));
        Some(SoundHandle::new(id, Arc::downgrade(&self.inner)))
    }

    /// 按声部 id 重新获取句柄（C ABI / 存档恢复用）；声部不存在返回 `None`。
    pub fn voice_handle(&self, id: VoiceId) -> Option<SoundHandle> {
        let inner = self.core();
        if inner.mixer.voice(id).is_some() {
            Some(SoundHandle::new(id, Arc::downgrade(&self.inner)))
        } else {
            None
        }
    }

    /// 设置主音量（`0..=1`，自动 clamp）。
    pub fn set_master_volume(&self, volume: f32) {
        self.core().mixer.set_master_volume(volume);
    }

    /// 当前主音量。
    pub fn master_volume(&self) -> f32 {
        self.core().mixer.master_volume()
    }

    /// 主静音开关。
    pub fn set_muted(&self, muted: bool) {
        self.core().mixer.set_muted(muted);
    }

    /// 是否主静音。
    pub fn is_muted(&self) -> bool {
        self.core().mixer.is_muted()
    }

    /// 主输出有效增益（静音时为 0）。
    pub fn master_gain(&self) -> f32 {
        self.core().mixer.master_gain()
    }

    /// 更新监听者（相机）状态，影响后续渲染块的 3D 衰减/声像。
    pub fn set_listener(&self, listener: Listener) {
        self.core().mixer.set_listener(listener);
    }

    /// 当前监听者。
    pub fn listener(&self) -> Listener {
        self.core().mixer.listener()
    }

    /// 设置总线音量（不存在时自动创建）。
    pub fn set_bus_volume(&self, bus: &str, volume: f32) {
        self.core().mixer.set_bus_volume(bus, volume);
    }

    /// 总线音量（不存在返回 1.0）。
    pub fn bus_volume(&self, bus: &str) -> f32 {
        self.core().mixer.bus_volume(bus)
    }

    /// 设置总线静音（不存在时自动创建）。
    pub fn set_bus_muted(&self, bus: &str, muted: bool) {
        self.core().mixer.set_bus_muted(bus, muted);
    }

    /// 总线是否静音。
    pub fn bus_muted(&self, bus: &str) -> bool {
        self.core().mixer.bus_muted(bus)
    }

    /// 所有总线名（排序）。
    pub fn bus_names(&self) -> Vec<String> {
        self.core().mixer.bus_names()
    }

    /// 帧更新：推进混音时钟（无声后端）或仅同步状态（真实设备后端）。
    ///
    /// - 无设备（`NullBackend`，`is_automatic() == false`）：按 `dt` 渲染并丢弃样本，
    ///   状态机/播放进度照常推进；
    /// - 真实设备后端（自动拉取）：返回 0，由设备线程推进；
    /// - `dt` 非正/非有限：返回 0；`dt` 超过 [`MAX_UPDATE_SECONDS`] 会被截断。
    ///
    /// 返回本次实际推进的帧数。
    pub fn update(&self, dt_secs: f32) -> usize {
        if !dt_secs.is_finite() || dt_secs <= 0.0 {
            return 0;
        }
        let dt_secs = dt_secs.min(MAX_UPDATE_SECONDS);
        let mut inner = self.core();
        if inner.automatic {
            return 0;
        }
        let sample_rate = inner.mixer.sample_rate();
        inner.frame_accumulator += f64::from(dt_secs) * f64::from(sample_rate);
        let frames = inner.frame_accumulator.floor();
        if frames < 1.0 {
            return 0;
        }
        let frames = frames as usize;
        inner.frame_accumulator -= frames as f64;
        inner.mixer.advance_frames(frames);
        frames
    }

    /// 手动推进指定帧数（测试/离线渲染用；真实设备后端下不要调用，避免双倍推进）。
    pub fn advance_frames(&self, frames: usize) {
        self.core().mixer.advance_frames(frames);
    }

    /// 手动渲染一块交错立体声 f32 样本（`out.len()/2` 帧）。
    ///
    /// 用于离线测试或由调用方驱动的手动后端；返回实际渲染帧数。
    pub fn render_interleaved(&self, out: &mut [f32]) -> usize {
        let mut inner = self.core();
        inner.mixer.render_interleaved(out)
    }

    /// 停止并移除所有声部，返回停止数量。
    pub fn stop_all(&self) -> usize {
        self.core().mixer.stop_all()
    }

    /// 活动声部数量（播放中 + 暂停）。
    pub fn active_voice_count(&self) -> usize {
        self.core().mixer.active_voice_count()
    }

    /// 运行统计快照。
    pub fn stats(&self) -> EngineStats {
        let inner = self.core();
        let cache = inner.cache.stats();
        EngineStats {
            active_voices: inner.mixer.active_voice_count(),
            cached_sounds: cache.len,
            cache_hits: cache.hits,
            cache_misses: cache.misses,
            rendered_frames: inner.mixer.rendered_frames(),
            dropped_plays: inner.dropped_plays,
        }
    }

    /// 显式关闭后端（等价于 Drop 时的行为，便于在卸载 Mono 前调用）。
    pub fn shutdown(&mut self) {
        if self.backend.is_open() {
            self.backend.close();
        }
    }

    fn core(&self) -> MutexGuard<'_, EngineInner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        if self.backend.is_open() {
            self.backend.close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::BackendError;
    use crate::wav;

    // ---------- 测试辅助 ----------

    fn rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }

    fn tone_engine() -> AudioEngine {
        let engine = AudioEngine::null();
        engine.load_sound("tone", SoundData::sine("tone", 440.0, 1.0, 48_000).unwrap());
        engine
    }

    /// 模拟“后端打开失败”的设备，用于验证无声卡降级。
    struct FailingBackend;

    impl AudioBackend for FailingBackend {
        fn name(&self) -> &'static str {
            "failing-device"
        }

        fn open(&mut self, _config: &BackendConfig, _sink: MixerSink) -> Result<(), BackendError> {
            Err(BackendError::new("模拟：没有可用音频设备"))
        }

        fn close(&mut self) {}

        fn is_open(&self) -> bool {
            false
        }
    }

    /// 模拟“自动拉取”的真实设备后端，并把 sink 暴露给测试。
    struct CaptureBackend {
        sink: Arc<Mutex<Option<MixerSink>>>,
    }

    impl AudioBackend for CaptureBackend {
        fn name(&self) -> &'static str {
            "capture-device"
        }

        fn open(&mut self, _config: &BackendConfig, sink: MixerSink) -> Result<(), BackendError> {
            *self
                .sink
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(sink);
            Ok(())
        }

        fn close(&mut self) {}

        fn is_open(&self) -> bool {
            true
        }
    }

    struct TempFile {
        path: std::path::PathBuf,
    }

    impl TempFile {
        fn new(bytes: &[u8]) -> Self {
            use std::sync::atomic::{AtomicU32, Ordering};
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let index = COUNTER.fetch_add(1, Ordering::Relaxed);
            // 测试产物写到 workspace 的 target 目录（已被 .gitignore 忽略），
            // 避免在部分受限环境下 TEMP 不可写导致测试失败。
            let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/audio-test-tmp");
            std::fs::create_dir_all(&directory).expect("创建测试临时目录失败");
            let path = directory.join(format!("hezhou_audio_{}_{}.wav", std::process::id(), index));
            std::fs::write(&path, bytes).expect("写入临时 WAV 失败");
            Self { path }
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    // ---------- 状态与降级 ----------

    #[test]
    fn null_engine_is_silent_but_fully_usable() {
        let engine = AudioEngine::null();
        assert_eq!(engine.status().backend(), "null");
        assert!(engine.status().is_silent());
        assert!(engine.status().degraded_reason().is_none());
        assert!(!engine.audio_output_available());
        assert_eq!(engine.sample_rate(), 48_000);
        // 无声后端不自动推进，需要 update 手动推进
        assert_eq!(engine.update(0.5), 24_000);
    }

    #[test]
    fn failing_backend_degrades_to_null_without_panicking() {
        let engine = AudioEngine::new(Box::new(FailingBackend));
        assert_eq!(engine.status().backend(), "null");
        assert!(engine.status().is_silent());
        let reason = engine.status().degraded_reason().unwrap();
        assert!(reason.contains("模拟"), "降级原因应包含原始错误: {reason}");

        // 降级后播放/推进/结束全部正常。
        engine.load_sound("tone", SoundData::sine("tone", 440.0, 0.1, 48_000).unwrap());
        let handle = engine.play("tone").expect("降级后仍应能播放");
        assert_eq!(handle.state(), PlaybackState::Playing);
        engine.update(0.25);
        assert_eq!(handle.state(), PlaybackState::Stopped);
        assert_eq!(engine.active_voice_count(), 0);
    }

    #[test]
    fn automatic_backend_marks_device_and_pumps_via_sink() {
        let slot = Arc::new(Mutex::new(None));
        let engine = AudioEngine::new(Box::new(CaptureBackend {
            sink: Arc::clone(&slot),
        }));
        assert!(engine.audio_output_available());
        assert_eq!(engine.status().mode(), AudioOutputMode::Device);

        engine.load_sound("tone", SoundData::sine("tone", 440.0, 1.0, 48_000).unwrap());
        engine.play("tone").unwrap();
        // 自动后端下 update 不推进，由设备回调线程通过 sink 渲染。
        assert_eq!(engine.update(0.5), 0);
        let sink = slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .expect("后端 open 时应拿到 MixerSink");
        assert_eq!(sink.sample_rate(), 48_000);
        let mut buffer = vec![0.0f32; 4_800 * 2];
        let frames = sink.render(&mut buffer);
        assert_eq!(frames, 4_800);
        assert!(rms(&buffer) > 0.01, "sink 渲染应产生真实音频数据");
    }

    #[test]
    fn default_config_is_48k_64_voices() {
        let config = EngineConfig::default();
        assert_eq!(config.sample_rate, 48_000);
        assert_eq!(config.max_voices, 64);
    }

    // ---------- 播放状态机 ----------

    #[test]
    fn play_pause_resume_stop_state_machine() {
        let engine = tone_engine();
        let handle = engine.play("tone").unwrap();
        assert_eq!(handle.state(), PlaybackState::Playing);
        assert!(handle.is_valid());

        assert!(handle.pause());
        assert!(!handle.pause(), "重复暂停应返回 false");
        assert_eq!(handle.state(), PlaybackState::Paused);

        assert!(handle.resume());
        assert!(!handle.resume(), "重复恢复应返回 false");
        assert_eq!(handle.state(), PlaybackState::Playing);

        assert!(handle.stop());
        assert!(!handle.stop(), "重复停止应返回 false");
        assert_eq!(handle.state(), PlaybackState::Stopped);
        assert!(!handle.is_valid());
        assert_eq!(engine.active_voice_count(), 0);
    }

    #[test]
    fn update_advances_and_non_looping_voice_finishes() {
        let engine = AudioEngine::null();
        engine.load_sound(
            "short",
            SoundData::sine("short", 440.0, 0.1, 48_000).unwrap(),
        );
        let handle = engine.play("short").unwrap();

        let advanced = engine.update(0.05);
        assert_eq!(advanced, 2_400);
        assert_eq!(handle.state(), PlaybackState::Playing);
        assert!((handle.position_secs() - 0.05).abs() < 1e-4);

        engine.update(0.05);
        assert_eq!(handle.state(), PlaybackState::Stopped);
        assert_eq!(engine.active_voice_count(), 0);
    }

    #[test]
    fn update_ignores_invalid_dt_and_caps_large_dt() {
        let engine = tone_engine();
        // 用循环声部验证“截断而非一次性推进到结束”。
        let handle = engine
            .play_with(PlayOptions::new("tone").looping(true))
            .unwrap();
        assert_eq!(engine.update(0.0), 0);
        assert_eq!(engine.update(-1.0), 0);
        assert_eq!(engine.update(f32::NAN), 0);
        assert_eq!(engine.update(f32::INFINITY), 0);
        let cap = (MAX_UPDATE_SECONDS * 48_000.0) as usize;
        assert_eq!(engine.update(1_000.0), cap);
        assert!(handle.is_valid());
        assert_eq!(handle.state(), PlaybackState::Playing);
    }

    #[test]
    fn looping_voice_never_ends_until_stopped() {
        let engine = tone_engine();
        let handle = engine
            .play_with(PlayOptions::new("tone").looping(true))
            .unwrap();
        assert!(handle.is_looping());
        engine.advance_frames(96_000); // 2 秒 > 1 秒音效
        assert_eq!(handle.state(), PlaybackState::Playing);
        assert!(handle.is_valid());
        assert!(handle.stop());
        assert_eq!(engine.active_voice_count(), 0);
    }

    #[test]
    fn play_missing_sound_returns_none() {
        let engine = AudioEngine::null();
        assert!(engine.play("not-loaded").is_none());
        assert_eq!(engine.stats().cache_misses, 1);
    }

    #[test]
    fn stop_all_stops_every_voice() {
        let engine = tone_engine();
        let first = engine.play("tone").unwrap();
        let second = engine.play("tone").unwrap();
        assert_eq!(engine.active_voice_count(), 2);
        assert_eq!(engine.stop_all(), 2);
        assert_eq!(engine.active_voice_count(), 0);
        assert_eq!(first.state(), PlaybackState::Stopped);
        assert_eq!(second.state(), PlaybackState::Stopped);
        assert_eq!(engine.stop_all(), 0);
    }

    // ---------- 音量 ----------

    #[test]
    fn master_and_bus_volume_affect_rendered_output() {
        fn render_rms(master: f32, bus: f32) -> f32 {
            let engine = tone_engine();
            engine.set_master_volume(master);
            engine.set_bus_volume(DEFAULT_BUS, bus);
            engine.play("tone").unwrap();
            let mut buffer = vec![0.0f32; 4_800 * 2];
            engine.render_interleaved(&mut buffer);
            rms(&buffer)
        }
        let full = render_rms(1.0, 1.0);
        assert!((render_rms(0.5, 1.0) / full - 0.5).abs() < 1e-3);
        assert!((render_rms(1.0, 0.5) / full - 0.5).abs() < 1e-3);
        assert!((render_rms(0.5, 0.5) / full - 0.25).abs() < 1e-3);
    }

    #[test]
    fn mute_silences_output_but_keeps_playback_progress() {
        let engine = tone_engine();
        let handle = engine.play("tone").unwrap();
        engine.set_muted(true);
        assert!(engine.is_muted());
        assert_eq!(engine.master_gain(), 0.0);
        let mut buffer = vec![1.0f32; 4_800 * 2];
        engine.render_interleaved(&mut buffer);
        assert!(buffer.iter().all(|s| *s == 0.0));
        assert!((handle.position_secs() - 0.1).abs() < 1e-4);

        engine.set_muted(false);
        engine.render_interleaved(&mut buffer);
        assert!(rms(&buffer) > 0.01);
    }

    #[test]
    fn bus_mute_and_volume_queries() {
        let engine = tone_engine();
        engine.set_bus_volume("sfx", 0.3);
        assert!((engine.bus_volume("sfx") - 0.3).abs() < 1e-6);
        assert_eq!(engine.bus_volume("unknown"), 1.0);
        engine.set_bus_muted("sfx", true);
        assert!(engine.bus_muted("sfx"));
        assert!(engine.bus_names().contains(&"sfx".to_string()));
        engine.set_master_volume(f32::NAN);
        assert_eq!(engine.master_volume(), 0.0);
    }

    // ---------- 3D ----------

    #[test]
    fn engine_3d_distance_attenuation_changes_output_level() {
        fn render_near_rms(position: Vec3) -> f32 {
            let engine = tone_engine();
            engine.play_3d("tone", position).unwrap();
            let mut buffer = vec![0.0f32; 4_800 * 2];
            engine.render_interleaved(&mut buffer);
            rms(&buffer)
        }
        let near = render_near_rms(Vec3::ZERO);
        let middle = render_near_rms(Vec3::new(0.0, 0.0, -20.0));
        let far = render_near_rms(Vec3::new(0.0, 0.0, -50.0));
        assert!(near > 0.1, "监听者位置应满增益: {near}");
        assert!(middle < near * 0.5, "中距离应明显衰减: {middle} vs {near}");
        assert!(far < 0.0001, "超过 max_distance 应静音: {far}");
    }

    #[test]
    fn listener_movement_updates_attenuation() {
        let engine = tone_engine();
        let handle = engine.play_3d("tone", Vec3::new(0.0, 0.0, -10.0)).unwrap();
        let mut buffer = vec![0.0f32; 4_800 * 2];
        engine.render_interleaved(&mut buffer);
        let far = rms(&buffer);

        engine.set_listener(Listener::new(
            Vec3::new(0.0, 0.0, -9.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 1.0, 0.0),
        ));
        assert_eq!(handle.position_3d(), Some(Vec3::new(0.0, 0.0, -10.0)));
        engine.render_interleaved(&mut buffer);
        let near = rms(&buffer);
        assert!(
            near > far * 3.0,
            "监听者靠近后应显著变响: near={near} far={far}"
        );
    }

    #[test]
    fn handle_set_position_moves_3d_source_only() {
        let engine = tone_engine();
        let spatial = engine
            .play_with(PlayOptions::new("tone").spatial(SpatialParams::new(Vec3::ZERO)))
            .unwrap();
        assert!(spatial.set_position(Vec3::new(1.0, 2.0, 3.0)));
        assert_eq!(spatial.position_3d(), Some(Vec3::new(1.0, 2.0, 3.0)));

        let flat = engine.play("tone").unwrap();
        assert!(flat.position_3d().is_none());
        assert!(!flat.set_position(Vec3::new(1.0, 0.0, 0.0)));
    }

    // ---------- 句柄释放 ----------

    #[test]
    fn dropping_handle_does_not_stop_playback() {
        let engine = AudioEngine::null();
        engine.load_sound("tone", SoundData::sine("tone", 440.0, 0.1, 48_000).unwrap());
        let handle = engine.play("tone").unwrap();
        let observer = handle.clone();
        drop(handle);
        assert_eq!(observer.state(), PlaybackState::Playing);
        engine.advance_frames(2_400);
        assert_eq!(observer.state(), PlaybackState::Playing);
        // 播放结束后句柄自动失效，不泄漏、不 panic。
        engine.advance_frames(2_400);
        assert!(!observer.is_valid());
        assert_eq!(observer.state(), PlaybackState::Stopped);
        assert_eq!(engine.active_voice_count(), 0);
    }

    #[test]
    fn handle_queries_after_engine_drop_stay_safe() {
        let handle = {
            let engine = AudioEngine::null();
            engine.load_sound("tone", SoundData::sine("tone", 440.0, 0.1, 48_000).unwrap());
            engine.play("tone").unwrap()
        };
        // 引擎已销毁：句柄所有查询安全返回默认值。
        assert_eq!(handle.state(), PlaybackState::Stopped);
        assert!(!handle.is_valid());
        assert!(!handle.pause());
        assert!(!handle.stop());
        assert_eq!(handle.volume(), 0.0);
    }

    // ---------- 资源加载与缓存 ----------

    #[test]
    fn cache_hit_miss_and_unload() {
        let engine = AudioEngine::null();
        let tone = SoundData::sine("tone", 440.0, 0.05, 48_000).unwrap();
        let wav_bytes = wav::encode_pcm16_wav(&tone).unwrap();
        engine.load_sound("tone", tone);

        // 已缓存：再次加载同 id 走命中，不重复解码。
        engine.load_from_memory("tone", &wav_bytes).unwrap();
        let stats = engine.stats();
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cached_sounds, 1);

        // 新 id：未命中 + 解码 + 缓存。
        engine.load_from_memory("tone2", &wav_bytes).unwrap();
        let stats = engine.stats();
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.cached_sounds, 2);
        assert_eq!(engine.cached_sound_ids(), vec!["tone", "tone2"]);

        assert!(engine.unload("tone"));
        assert!(!engine.unload("tone"));
        assert!(engine.sound("tone").is_none());
        assert!(engine.sound("tone2").is_some());
    }

    #[test]
    fn load_wav_file_and_error_paths() {
        let engine = AudioEngine::null();
        let tone = SoundData::sine("tone", 440.0, 0.02, 48_000).unwrap();
        let file = TempFile::new(&wav::encode_pcm16_wav(&tone).unwrap());
        engine.load_wav_file("file-tone", &file.path).unwrap();
        let loaded = engine.sound("file-tone").expect("文件加载后应能查询");
        assert_eq!(loaded.frames(), tone.frames());
        assert_eq!(loaded.sample_rate(), 48_000);

        let missing = engine
            .load_wav_file("missing", "C:/hezhou/definitely/not/here.wav")
            .unwrap_err();
        assert!(matches!(missing, AudioError::Io { .. }));
        assert!(missing.to_string().contains("not/here.wav"));

        let bad = engine
            .load_from_memory("bad", b"definitely not audio")
            .unwrap_err();
        assert!(matches!(bad, AudioError::UnsupportedFormat { .. }));
    }

    #[test]
    #[cfg(not(feature = "ogg"))]
    fn ogg_memory_load_reports_clear_unsupported_error() {
        let engine = AudioEngine::null();
        let page = crate::decoder::ogg::synthetic_vorbis_page(2, 44_100);
        let error = engine.load_from_memory("bgm", &page).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("Vorbis"), "应说明 OGG 编解码器: {message}");
        assert!(message.contains("DecoderRegistry::register"));
    }

    #[test]
    fn custom_decoder_registration_works_through_engine() {
        struct ToyDecoder;
        impl AudioDecoder for ToyDecoder {
            fn name(&self) -> &'static str {
                "toy"
            }
            fn supports(&self, bytes: &[u8]) -> bool {
                bytes.starts_with(b"TOY1")
            }
            fn decode(&self, name: &str, bytes: &[u8]) -> AudioResult<SoundData> {
                let frames = bytes.get(4).copied().unwrap_or(0) as usize;
                SoundData::silent(name, 48_000, 1, frames)
            }
        }
        let engine = AudioEngine::null();
        engine.register_decoder(Box::new(ToyDecoder));
        assert!(engine.decoder_names().contains(&"toy"));
        engine.load_from_memory("toy-sound", b"TOY1\x10").unwrap();
        assert_eq!(engine.sound("toy-sound").unwrap().frames(), 16);
    }

    // ---------- 声部上限 ----------

    #[test]
    fn max_voices_drops_extra_play_requests() {
        // 该测试与设备无关：显式使用无声后端，保证在有无声卡环境下结果一致。
        let engine = AudioEngine::with_config(
            NullBackend::boxed(),
            EngineConfig {
                sample_rate: 48_000,
                max_voices: 2,
            },
        );
        engine.load_sound("tone", SoundData::sine("tone", 440.0, 1.0, 48_000).unwrap());
        assert!(engine.play("tone").is_some());
        assert!(engine.play("tone").is_some());
        assert!(engine.play("tone").is_none());
        let stats = engine.stats();
        assert_eq!(stats.active_voices, 2);
        assert_eq!(stats.dropped_plays, 1);
    }

    // ---------- 线程安全 ----------

    #[test]
    fn engine_and_handle_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AudioEngine>();
        assert_send_sync::<SoundHandle>();
    }

    #[test]
    fn engine_can_be_shared_across_threads() {
        let engine = Arc::new(tone_engine());
        let mut spawns = Vec::new();
        for _ in 0..4 {
            let engine = Arc::clone(&engine);
            spawns.push(std::thread::spawn(move || {
                engine.play("tone").map(|handle| handle.id()).is_some()
            }));
        }
        for spawn in spawns {
            assert!(spawn.join().unwrap());
        }
        assert_eq!(engine.active_voice_count(), 4);
    }
}
