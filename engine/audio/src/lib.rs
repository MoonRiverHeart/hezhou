//! # hezhou-audio
//!
//! Hezhou 引擎 0.2 的**音频能力模块**（`engine/audio`）。
//!
//! 目标是提供一套可测试、可组合的通用音频能力（不做具体游戏音频）：
//! 播放/暂停/恢复/停止/循环、主音量与总线音量、2D/3D 距离衰减、无声卡优雅降级、
//! WAV/OGG 解码与资源缓存，以及 C#/FFI 接入草案。
//!
//! ## Features（ADR-001：cpal + 自研混音，OGG 用 lewton）
//!
//! - 默认 **零外部依赖**：`default_backend()` 返回 [`NullBackend`]，
//!   `cargo test -p hezhou-audio` 可在离线/无声卡环境通过；
//! - `cpal-backend`：启用真实输出设备（`CpalBackend`），无设备/打开失败自动降级；
//! - `ogg`：启用 lewton OGG/Vorbis 解码（`LewtonOggDecoder`），自动注册到
//!   [`DecoderRegistry`]。
//!
//! ```toml
//! hezhou-audio = { path = "engine/audio", features = ["cpal-backend", "ogg"] }
//! ```
//!
//! ## 架构
//!
//! ```text
//!          ┌──────────────── AudioEngine（门面）────────────────┐
//!          │ 资源缓存 AssetCache · 解码注册 DecoderRegistry      │
//!          │ 混音器 Mixer（voice/bus/master/3D）· 状态机          │
//!          └───────────────┬────────────────────────────────────┘
//!                          │ MixerSink（共享锁）
//!          ┌───────────────┴────────────────┐
//!          │ AudioBackend trait             │
//!          ├───────────────┬────────────────┤
//!          │ NullBackend   │ CpalBackend     │  ← feature `cpal-backend`
//!          └───────────────┴────────────────┘
//! ```
//!
//! - [`AudioEngine`]：对外唯一入口，所有方法可 `&self` 调用，可放进 `Arc` 跨线程共享。
//! - [`SoundHandle`]：可克隆播放句柄；丢弃句柄不停止播放，声部结束后自动失效。
//! - [`AudioBackend`]：设备输出抽象；[`NullBackend`] 为无声后端，
//!   启用 feature 后 [`default_backend`] 返回 `CpalBackend`。
//! - [`Mixer`]：纯 Rust 混音器，可离线渲染为 f32 交错立体声，便于测试。
//! - [`DecoderRegistry`]：解码器注册表，内置纯 Rust WAV 解码器；
//!   启用 `ogg` feature 后自动注册 lewton OGG/Vorbis 解码器。
//!
//! ## 快速开始
//!
//! ```
//! use hezhou_audio::{AudioEngine, PlayOptions, SoundData, Vec3};
//!
//! // 无设备/CI 环境：无声引擎，逻辑照常运行。
//! let engine = AudioEngine::null();
//!
//! // 资源：程序化音频或 WAV 文件/内存
//! engine.load_sound("click", SoundData::sine("click", 880.0, 0.05, 48_000).unwrap());
//!
//! // 播放 2D
//! let click = engine.play("click").unwrap();
//! // 播放 3D（默认反比距离衰减）
//! let roar = engine.play_3d("click", Vec3::new(10.0, 0.0, 0.0)).unwrap();
//! // 完整参数
//! let bgm = engine.play_with(PlayOptions::new("click").looping(true).volume(0.5).bus("bgm")).unwrap();
//!
//! // 每帧推进（无声后端下必须调用；真实设备后端由设备线程推进）
//! engine.update(1.0 / 60.0);
//!
//! // 音量 / 总线 / 监听者
//! engine.set_master_volume(0.8);
//! engine.set_bus_volume("bgm", 0.6);
//! engine.set_listener(hezhou_audio::Listener::default());
//!
//! // 状态机
//! click.pause();
//! click.resume();
//! click.stop();
//! # let _ = (roar, bgm);
//! ```
//!
//! ## 无声卡降级
//!
//! [`AudioEngine::new`] 会尝试打开后端（启用 `cpal-backend` 时为真实设备）；
//! 失败自动降级为 [`NullBackend`]，状态通过 [`AudioEngine::status`] 查询
//! （[`EngineStatus::is_silent`]、[`EngineStatus::degraded_reason`]）。
//! 播放/暂停/循环/3D 衰减逻辑完全不受影响。
//!
//! ## 集成点
//!
//! - 真实设备：`AudioEngine::new(default_backend())`（启用 `cpal-backend`）；
//! - OGG/Vorbis：启用 `ogg` feature 自动注册，或手动
//!   [`AudioEngine::register_decoder`]；
//! - C#/FFI：见 [`ffi`] 模块的最小 C ABI 草案与 crate `README.md` 的绑定说明。
//!
//! 详细 API、集成步骤与已知限制见 `engine/audio/README.md`。

#![warn(missing_docs)]

pub mod backend;
#[cfg(feature = "cpal-backend")]
pub mod backend_cpal;
pub mod bus;
pub mod cache;
pub mod decoder;
pub mod engine;
pub mod error;
pub mod ffi;
pub mod mixer;
#[cfg(feature = "ogg")]
pub mod ogg_vorbis;
pub mod sound;
pub mod spatial;
pub mod wav;

pub use backend::{
    default_backend, AudioBackend, BackendConfig, BackendError, MixerSink, NullBackend,
};
#[cfg(feature = "cpal-backend")]
pub use backend_cpal::CpalBackend;
pub use bus::{clamp_volume, AudioBus, DEFAULT_BUS};
pub use cache::{AssetCache, CacheStats};
pub use decoder::{detect_format, AudioDecoder, AudioFormat, DecoderRegistry, WavDecoder};
pub use engine::{
    AudioEngine, AudioOutputMode, EngineConfig, EngineStats, EngineStatus, PlayOptions,
    SoundHandle, MAX_UPDATE_SECONDS,
};
pub use error::{AudioError, AudioResult};
pub use mixer::{Mixer, PlaybackState, Voice, VoiceId};
#[cfg(feature = "ogg")]
pub use ogg_vorbis::LewtonOggDecoder;
pub use sound::{SoundData, OUTPUT_CHANNELS};
pub use spatial::{
    pan_to_gains, Attenuation, AttenuationModel, Listener, SpatialOutput, SpatialParams, Vec3,
};
