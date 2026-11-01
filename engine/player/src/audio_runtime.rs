//! Player 音频运行时（task-35）：窗口 host 持有的 `AudioEngine` 封装。
//!
//! - 默认 `default_backend()`（未启用 `audio-cpal` feature 时为 null backend，
//!   无声卡/CI 环境安全；启用后尝试真实 cpal 设备，失败自动降级 null）；
//! - 从 Player 包内第一个 `AssetKind::Audio` 资产加载并播放（WAV/OGG 由
//!   hezhou-audio 解码器注册表处理）；
//! - 提供 play_first/pause/resume/stop/set_volume/is_playing/backend 给 C# ABI v4；
//! - 每帧 `update(dt)` 推进无声后端状态机（真实设备后端由设备线程推进）。
//!
//! 限制（0.2）：只加载/播放第一个音频资产；无 3D 位置/总线/循环设置（后续扩展）。

use std::path::{Path, PathBuf};

use hezhou_audio::{default_backend, AudioEngine, PlaybackState, SoundHandle};
use hezhou_asset::AssetKind;

use crate::asset_index::PlayerAssetIndex;

/// Player 音频运行时。
pub struct PlayerAudioRuntime {
    engine: AudioEngine,
    handle: Option<SoundHandle>,
    sound_id: Option<String>,
    source: Option<PathBuf>,
}

impl PlayerAudioRuntime {
    /// 使用默认后端（未开 `audio-cpal` 时 null；开启后尝试设备，失败降级）。
    pub fn new() -> Self {
        Self {
            engine: AudioEngine::new(default_backend()),
            handle: None,
            sound_id: None,
            source: None,
        }
    }

    /// 强制 null backend（测试/确定性回放）。
    pub fn null() -> Self {
        Self {
            engine: AudioEngine::null(),
            handle: None,
            sound_id: None,
            source: None,
        }
    }

    /// 后端名称（如 `"null"` / `"cpal"`）。
    pub fn backend(&self) -> String {
        self.engine.status().backend().to_string()
    }

    /// 是否静音降级（无真实设备）。
    pub fn is_silent(&self) -> bool {
        self.engine.status().is_silent()
    }

    /// 已加载的音频资源 id。
    pub fn loaded_id(&self) -> Option<&str> {
        self.sound_id.as_deref()
    }

    /// 源文件路径（诊断用）。
    pub fn source(&self) -> Option<&Path> {
        self.source.as_deref()
    }

    /// 从包内 `assets/` 找第一个 `AssetKind::Audio` 条目并加载。
    ///
    /// 返回 `Ok(None)` 表示无音频资产（调用方 WARN，不阻塞）；加载失败返回可读错误。
    pub fn load_first_audio(
        &mut self,
        assets_dir: &Path,
        index: &PlayerAssetIndex,
    ) -> Result<Option<String>, String> {
        let Some(entry) = index.entries().find(|entry| entry.kind == AssetKind::Audio) else {
            return Ok(None);
        };
        let native = translate_output(&entry.output);
        let path = assets_dir.join(native);
        if !path.is_file() {
            return Err(format!("音频资产文件不存在：{}", path.display()));
        }
        let sound_id = entry.id.to_string();
        self.engine
            .load_audio_file(&sound_id, &path)
            .map_err(|error| format!("加载音频 {} 失败：{error}", path.display()))?;
        self.sound_id = Some(sound_id.clone());
        self.source = Some(path);
        Ok(Some(sound_id))
    }

    /// 直接加载 WAV 文件（测试/嵌入资源用）。
    pub fn load_wav_file(&mut self, sound_id: &str, path: &Path) -> Result<(), String> {
        self.engine
            .load_audio_file(sound_id, path)
            .map_err(|error| format!("加载音频 {} 失败：{error}", path.display()))?;
        self.sound_id = Some(sound_id.to_string());
        self.source = Some(path.to_path_buf());
        Ok(())
    }

    /// 播放已加载的第一个音频（会先停止当前播放）；成功 true。
    pub fn play_first(&mut self) -> bool {
        let Some(sound_id) = self.sound_id.clone() else {
            return false;
        };
        self.stop();
        self.handle = self.engine.play(&sound_id);
        self.handle.is_some()
    }

    /// 暂停当前播放；成功 true。
    pub fn pause(&mut self) -> bool {
        self.handle
            .as_ref()
            .map(|handle| handle.pause())
            .unwrap_or(false)
    }

    /// 恢复当前播放；成功 true。
    pub fn resume(&mut self) -> bool {
        self.handle
            .as_ref()
            .map(|handle| handle.resume())
            .unwrap_or(false)
    }

    /// 停止当前播放；成功 true。
    pub fn stop(&mut self) -> bool {
        let Some(handle) = self.handle.take() else {
            return false;
        };
        handle.stop()
    }

    /// 是否正在播放。
    pub fn is_playing(&self) -> bool {
        self.handle
            .as_ref()
            .map(|handle| handle.state() == PlaybackState::Playing)
            .unwrap_or(false)
    }

    /// 设置主音量（0..=1，clamp）；返回实际生效值。
    pub fn set_volume(&self, volume: f32) -> f32 {
        self.engine.set_master_volume(volume);
        self.engine.master_volume()
    }

    /// 当前主音量。
    pub fn volume(&self) -> f32 {
        self.engine.master_volume()
    }

    /// 每帧推进（null backend 必需；cpal 后端的设备线程也会推进，调用安全）。
    pub fn update(&self, dt_seconds: f32) {
        self.engine.update(dt_seconds);
    }

    /// 活动声部数（诊断）。
    pub fn active_voice_count(&self) -> usize {
        self.engine.active_voice_count()
    }

    /// 单行摘要（启动/结束日志）。
    pub fn summary(&self) -> String {
        let backend = self.backend();
        let loaded = self
            .sound_id
            .as_deref()
            .unwrap_or("<none>");
        let playing = if self.is_playing() { "playing" } else { "stopped" };
        let source = self
            .source
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<none>".to_string());
        format!(
            "音频：backend={backend}（silent={}）loaded={loaded} state={playing} volume={:.2} source={source}",
            self.is_silent(),
            self.volume()
        )
    }
}

impl std::fmt::Debug for PlayerAudioRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerAudioRuntime")
            .field("backend", &self.backend())
            .field("sound_id", &self.sound_id)
            .field("playing", &self.is_playing())
            .field("volume", &self.volume())
            .finish()
    }
}

/// `assets/<output>` 逻辑路径 → 本机相对路径。
fn translate_output(output: &str) -> PathBuf {
    let mut path = PathBuf::new();
    for segment in output.split('/') {
        if !segment.is_empty() {
            path.push(segment);
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_runtime_is_safe_without_audio() {
        let mut runtime = PlayerAudioRuntime::null();
        assert_eq!(runtime.backend(), "null");
        assert!(runtime.is_silent());
        assert_eq!(runtime.loaded_id(), None);
        assert!(!runtime.play_first());
        assert!(!runtime.pause());
        assert!(!runtime.resume());
        assert!(!runtime.stop());
        assert!(!runtime.is_playing());
        assert_eq!(runtime.set_volume(0.5), 0.5);
        assert_eq!(runtime.volume(), 0.5);
        runtime.update(1.0 / 60.0);
        assert!(runtime.summary().contains("backend=null"));
    }
}
