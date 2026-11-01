//! 最小 C ABI 设计草案（不接入现有 `FfiContext`，供后续 C# 绑定参考）。
//!
//! # 约定
//!
//! - 引擎句柄是**不透明指针**（[`HezhouAudioEngineHandle`]），由 C# 持有，
//!   必须与创建它的线程同一线程使用（内部已加锁，但 Mono/编辑器侧约定单线程调用）。
//! - 返回 `i32`：`0` 成功、`-1` 失败；失败详情通过
//!   [`hezhou_audio_engine_last_error`] 读取（返回 UTF-8 C 字符串，指针在下次错误写入前有效）。
//! - 返回 `u64`：声部 id（`0` 表示失败）；后续通过 `hezhou_audio_voice_*` 操作。
//! - 字符串参数均为 UTF-8 C 字符串，可为空指针（按失败处理）。
//! - 本文件只是**草案**：正式接入时由集成负责人决定放在 `FfiContext` 的哪些字段，
//!   以及是否需要 `cdylib` 导出。
//!
//! # 建议的 C# 绑定（P/Invoke）
//!
//! ```csharp
//! [DllImport("hezhou_audio", CallingConvention = CallingConvention.Cdecl)]
//! static extern IntPtr hezhou_audio_engine_create();
//! [DllImport("hezhou_audio", CallingConvention = CallingConvention.Cdecl)]
//! static extern long hezhou_audio_play(IntPtr engine, string soundId);
//! // ...其余函数按相同规则声明，字符串用 [MarshalAs(UnmanagedType.LPStr)]
//! ```

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Mutex;

use crate::engine::AudioEngine;
use crate::mixer::PlaybackState;
use crate::sound::SoundData;
use crate::spatial::{Listener, Vec3};

/// 不透明引擎句柄（C ABI 侧只持有指针，不直接访问字段）。
pub struct HezhouAudioEngineHandle {
    engine: AudioEngine,
    last_error: Mutex<Option<CString>>,
    backend_name: CString,
}

impl HezhouAudioEngineHandle {
    fn new(engine: AudioEngine) -> Self {
        let backend_name = CString::new(engine.status().backend().to_string())
            .unwrap_or_else(|_| CString::new("unknown").expect("静态字符串不含 NUL"));
        Self {
            engine,
            last_error: Mutex::new(None),
            backend_name,
        }
    }

    fn set_error(&self, message: impl Into<String>) {
        let message = message.into();
        let mut slot = self
            .last_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *slot = CString::new(message).ok();
    }

    fn clear_error(&self) {
        let mut slot = self
            .last_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *slot = None;
    }
}

/// 读取 C 字符串参数；空指针或非 UTF-8 返回 `None`。
unsafe fn read_c_string(pointer: *const c_char) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    // SAFETY: 调用方保证非空指针指向以 NUL 结尾的有效 C 字符串。
    let cstr = unsafe { CStr::from_ptr(pointer) };
    cstr.to_str().ok().map(str::to_string)
}

/// 把引擎指针转成引用；空指针返回 `None`。
unsafe fn engine_ref<'a>(
    handle: *const HezhouAudioEngineHandle,
) -> Option<&'a HezhouAudioEngineHandle> {
    if handle.is_null() {
        None
    } else {
        // SAFETY: 调用方保证指针来自 `hezhou_audio_engine_create` 且未销毁。
        Some(unsafe { &*handle })
    }
}

/// 创建默认音频引擎（当前为无声后端；未来接入真实设备后自动尝试打开设备）。
///
/// 返回 `NULL` 仅可能发生在极端内存分配失败时。
#[no_mangle]
pub extern "C" fn hezhou_audio_engine_create() -> *mut HezhouAudioEngineHandle {
    let engine = AudioEngine::null();
    Box::into_raw(Box::new(HezhouAudioEngineHandle::new(engine)))
}

/// 销毁引擎；传入 `NULL` 安全。
///
/// # Safety
/// `handle` 必须是 [`hezhou_audio_engine_create`] 返回且尚未销毁的指针。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_engine_destroy(handle: *mut HezhouAudioEngineHandle) {
    if handle.is_null() {
        return;
    }
    // SAFETY: 由 create 返回的 Box 指针，调用方保证只销毁一次。
    drop(unsafe { Box::from_raw(handle) });
}

/// 读取最近一次错误信息（UTF-8 C 字符串）；没有错误返回 `NULL`。
///
/// 返回指针在下一次错误写入或引擎销毁前有效，调用方不要释放。
///
/// # Safety
/// `handle` 必须是有效引擎指针或 `NULL`。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_engine_last_error(
    handle: *const HezhouAudioEngineHandle,
) -> *const c_char {
    let Some(handle) = (unsafe { engine_ref(handle) }) else {
        return std::ptr::null();
    };
    let slot = handle
        .last_error
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    slot.as_ref()
        .map(|message| message.as_ptr())
        .unwrap_or(std::ptr::null())
}

/// 后端名称（如 `"null"`），静态字符串，无需释放。
///
/// # Safety
/// `handle` 必须是有效引擎指针或 `NULL`。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_engine_backend_name(
    handle: *const HezhouAudioEngineHandle,
) -> *const c_char {
    match unsafe { engine_ref(handle) } {
        Some(handle) => handle.backend_name.as_ptr(),
        None => std::ptr::null(),
    }
}

/// 是否处于无声（无设备）模式：1 = 静音降级，0 = 有真实设备。
///
/// # Safety
/// `handle` 必须是有效引擎指针或 `NULL`。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_engine_is_silent(
    handle: *const HezhouAudioEngineHandle,
) -> i32 {
    match unsafe { engine_ref(handle) } {
        Some(handle) => i32::from(handle.engine.status().is_silent()),
        None => 1,
    }
}

/// 加载音频文件（WAV；启用 `ogg` feature 后也支持 OGG/Vorbis）并缓存；成功返回 0。
///
/// 相当于 Rust API 的 `AudioEngine::load_audio_file`。
///
/// # Safety
/// `handle`、`id`、`path` 必须有效（`id`/`path` 为 UTF-8 C 字符串）。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_load_audio_file(
    handle: *mut HezhouAudioEngineHandle,
    id: *const c_char,
    path: *const c_char,
) -> i32 {
    let Some(handle) = (unsafe { engine_ref(handle) }) else {
        return -1;
    };
    let (Some(id), Some(path)) = (unsafe { read_c_string(id) }, unsafe { read_c_string(path) })
    else {
        handle.set_error("参数非法：id/path 为空或不是 UTF-8");
        return -1;
    };
    match handle.engine.load_audio_file(id, path) {
        Ok(()) => {
            handle.clear_error();
            0
        }
        Err(error) => {
            handle.set_error(error.to_string());
            -1
        }
    }
}

/// 加载 WAV 文件并缓存；成功返回 0。
///
/// 兼容旧 API 的别名，等价于 [`hezhou_audio_load_audio_file`]。
///
/// # Safety
/// `handle`、`id`、`path` 必须有效（`id`/`path` 为 UTF-8 C 字符串）。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_load_wav_file(
    handle: *mut HezhouAudioEngineHandle,
    id: *const c_char,
    path: *const c_char,
) -> i32 {
    unsafe { hezhou_audio_load_audio_file(handle, id, path) }
}

/// 从内存加载 WAV 音频并缓存；成功返回 0。
///
/// # Safety
/// `handle`、`id` 必须有效；`data` 必须指向 `len` 字节的可读内存。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_load_wav_memory(
    handle: *mut HezhouAudioEngineHandle,
    id: *const c_char,
    data: *const u8,
    len: usize,
) -> i32 {
    let Some(handle) = (unsafe { engine_ref(handle) }) else {
        return -1;
    };
    let Some(id) = (unsafe { read_c_string(id) }) else {
        handle.set_error("参数非法：id 为空或不是 UTF-8");
        return -1;
    };
    if data.is_null() || len == 0 {
        handle.set_error("参数非法：内存数据为空");
        return -1;
    }
    // SAFETY: 调用方保证 data 指向 len 字节可读内存。
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    match handle.engine.load_from_memory(id, bytes) {
        Ok(()) => {
            handle.clear_error();
            0
        }
        Err(error) => {
            handle.set_error(error.to_string());
            -1
        }
    }
}

/// 生成并注册一个正弦测试音（无需资源文件，便于 C# 冒烟/编辑器试听）；成功返回 0。
///
/// # Safety
/// `handle`、`id` 必须有效（`id` 为 UTF-8 C 字符串）。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_load_tone(
    handle: *mut HezhouAudioEngineHandle,
    id: *const c_char,
    frequency_hz: f32,
    duration_secs: f32,
) -> i32 {
    let Some(handle) = (unsafe { engine_ref(handle) }) else {
        return -1;
    };
    let Some(id) = (unsafe { read_c_string(id) }) else {
        handle.set_error("参数非法：id 为空或不是 UTF-8");
        return -1;
    };
    let sample_rate = handle.engine.sample_rate();
    match SoundData::sine(id.clone(), frequency_hz, duration_secs, sample_rate) {
        Ok(data) => {
            handle.engine.load_sound(id, data);
            handle.clear_error();
            0
        }
        Err(error) => {
            handle.set_error(error.to_string());
            -1
        }
    }
}

/// 卸载资源；成功返回 0，资源不存在返回 -1。
///
/// # Safety
/// `handle`、`id` 必须有效。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_unload(
    handle: *mut HezhouAudioEngineHandle,
    id: *const c_char,
) -> i32 {
    let Some(handle) = (unsafe { engine_ref(handle) }) else {
        return -1;
    };
    let Some(id) = (unsafe { read_c_string(id) }) else {
        return -1;
    };
    i32::from(!handle.engine.unload(&id))
}

/// 播放 2D 音效，返回声部 id（0 = 失败）。
///
/// # Safety
/// `handle`、`sound_id` 必须有效。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_play(
    handle: *mut HezhouAudioEngineHandle,
    sound_id: *const c_char,
) -> u64 {
    let Some(handle) = (unsafe { engine_ref(handle) }) else {
        return 0;
    };
    let Some(sound_id) = (unsafe { read_c_string(sound_id) }) else {
        return 0;
    };
    handle
        .engine
        .play(&sound_id)
        .map(|voice| voice.id())
        .unwrap_or(0)
}

/// 播放 3D 音效，返回声部 id（0 = 失败）。
///
/// # Safety
/// `handle`、`sound_id` 必须有效。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_play_3d(
    handle: *mut HezhouAudioEngineHandle,
    sound_id: *const c_char,
    x: f32,
    y: f32,
    z: f32,
) -> u64 {
    let Some(handle) = (unsafe { engine_ref(handle) }) else {
        return 0;
    };
    let Some(sound_id) = (unsafe { read_c_string(sound_id) }) else {
        return 0;
    };
    handle
        .engine
        .play_3d(&sound_id, Vec3::new(x, y, z))
        .map(|voice| voice.id())
        .unwrap_or(0)
}

/// 查询声部状态：0=Playing、1=Paused、2=Stopped/无效。
///
/// # Safety
/// `handle` 必须有效或为 NULL。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_voice_state(
    handle: *const HezhouAudioEngineHandle,
    voice_id: u64,
) -> i32 {
    let Some(handle) = (unsafe { engine_ref(handle) }) else {
        return PlaybackState::Stopped.as_i32();
    };
    handle
        .engine
        .voice_handle(voice_id)
        .map(|voice| voice.state().as_i32())
        .unwrap_or(PlaybackState::Stopped.as_i32())
}

/// 声部操作：暂停（1）、恢复（2）、停止（3）。成功返回 0。
///
/// # Safety
/// `handle` 必须有效。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_voice_command(
    handle: *mut HezhouAudioEngineHandle,
    voice_id: u64,
    command: i32,
) -> i32 {
    let Some(handle) = (unsafe { engine_ref(handle) }) else {
        return -1;
    };
    let Some(voice) = handle.engine.voice_handle(voice_id) else {
        return -1;
    };
    let ok = match command {
        1 => voice.pause(),
        2 => voice.resume(),
        3 => voice.stop(),
        _ => false,
    };
    i32::from(!ok)
}

/// 设置声部音量（0..=1）；成功返回 0。
///
/// # Safety
/// `handle` 必须有效。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_voice_set_volume(
    handle: *mut HezhouAudioEngineHandle,
    voice_id: u64,
    volume: f32,
) -> i32 {
    let Some(handle) = (unsafe { engine_ref(handle) }) else {
        return -1;
    };
    let Some(voice) = handle.engine.voice_handle(voice_id) else {
        return -1;
    };
    i32::from(!voice.set_volume(volume))
}

/// 设置声部循环（`looping != 0` 为循环）；成功返回 0。
///
/// # Safety
/// `handle` 必须有效。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_voice_set_loop(
    handle: *mut HezhouAudioEngineHandle,
    voice_id: u64,
    looping: i32,
) -> i32 {
    let Some(handle) = (unsafe { engine_ref(handle) }) else {
        return -1;
    };
    let Some(voice) = handle.engine.voice_handle(voice_id) else {
        return -1;
    };
    i32::from(!voice.set_looping(looping != 0))
}

/// 更新 3D 声部位置；2D 声部返回 -1。
///
/// # Safety
/// `handle` 必须有效。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_voice_set_position(
    handle: *mut HezhouAudioEngineHandle,
    voice_id: u64,
    x: f32,
    y: f32,
    z: f32,
) -> i32 {
    let Some(handle) = (unsafe { engine_ref(handle) }) else {
        return -1;
    };
    let Some(voice) = handle.engine.voice_handle(voice_id) else {
        return -1;
    };
    i32::from(!voice.set_position(Vec3::new(x, y, z)))
}

/// 设置主音量（0..=1）。
///
/// # Safety
/// `handle` 必须有效。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_set_master_volume(
    handle: *mut HezhouAudioEngineHandle,
    volume: f32,
) {
    if let Some(handle) = unsafe { engine_ref(handle) } {
        handle.engine.set_master_volume(volume);
    }
}

/// 读取主音量。
///
/// # Safety
/// `handle` 必须有效。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_master_volume(handle: *const HezhouAudioEngineHandle) -> f32 {
    match unsafe { engine_ref(handle) } {
        Some(handle) => handle.engine.master_volume(),
        None => 0.0,
    }
}

/// 主静音开关（`muted != 0` 为静音）。
///
/// # Safety
/// `handle` 必须有效。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_set_muted(handle: *mut HezhouAudioEngineHandle, muted: i32) {
    if let Some(handle) = unsafe { engine_ref(handle) } {
        handle.engine.set_muted(muted != 0);
    }
}

/// 设置总线音量（总线不存在时自动创建）。
///
/// # Safety
/// `handle`、`bus` 必须有效。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_set_bus_volume(
    handle: *mut HezhouAudioEngineHandle,
    bus: *const c_char,
    volume: f32,
) -> i32 {
    let Some(handle) = (unsafe { engine_ref(handle) }) else {
        return -1;
    };
    let Some(bus) = (unsafe { read_c_string(bus) }) else {
        return -1;
    };
    handle.engine.set_bus_volume(&bus, volume);
    0
}

/// 设置总线静音。
///
/// # Safety
/// `handle`、`bus` 必须有效。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_set_bus_muted(
    handle: *mut HezhouAudioEngineHandle,
    bus: *const c_char,
    muted: i32,
) -> i32 {
    let Some(handle) = (unsafe { engine_ref(handle) }) else {
        return -1;
    };
    let Some(bus) = (unsafe { read_c_string(bus) }) else {
        return -1;
    };
    handle.engine.set_bus_muted(&bus, muted != 0);
    0
}

/// 设置监听者位置与朝向（上方向固定 +Y）。C# 侧通常每帧同步相机。
///
/// # Safety
/// `handle` 必须有效。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_set_listener(
    handle: *mut HezhouAudioEngineHandle,
    x: f32,
    y: f32,
    z: f32,
    forward_x: f32,
    forward_y: f32,
    forward_z: f32,
) {
    if let Some(handle) = unsafe { engine_ref(handle) } {
        handle.engine.set_listener(Listener::new(
            Vec3::new(x, y, z),
            Vec3::new(forward_x, forward_y, forward_z),
            Vec3::new(0.0, 1.0, 0.0),
        ));
    }
}

/// 帧更新：无声后端下推进混音时钟；返回推进帧数。
///
/// # Safety
/// `handle` 必须有效。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_update(
    handle: *mut HezhouAudioEngineHandle,
    dt_secs: f32,
) -> usize {
    match unsafe { engine_ref(handle) } {
        Some(handle) => handle.engine.update(dt_secs),
        None => 0,
    }
}

/// 活动声部数量。
///
/// # Safety
/// `handle` 必须有效。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_active_voice_count(
    handle: *const HezhouAudioEngineHandle,
) -> usize {
    match unsafe { engine_ref(handle) } {
        Some(handle) => handle.engine.active_voice_count(),
        None => 0,
    }
}

/// 停止所有声部，返回停止数量。
///
/// # Safety
/// `handle` 必须有效。
#[no_mangle]
pub unsafe extern "C" fn hezhou_audio_stop_all(handle: *mut HezhouAudioEngineHandle) -> usize {
    match unsafe { engine_ref(handle) } {
        Some(handle) => handle.engine.stop_all(),
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    fn c_string(value: &str) -> CString {
        CString::new(value).expect("测试字符串不含 NUL")
    }

    #[test]
    fn ffi_full_lifecycle() {
        // SAFETY: 测试内使用有效指针，按 create/destroy 配对。
        unsafe {
            let engine = hezhou_audio_engine_create();
            assert!(!engine.is_null());
            assert_eq!(
                CStr::from_ptr(hezhou_audio_engine_backend_name(engine))
                    .to_str()
                    .unwrap(),
                "null"
            );
            assert_eq!(hezhou_audio_engine_is_silent(engine), 1);

            let id = c_string("tone");
            assert_eq!(hezhou_audio_load_tone(engine, id.as_ptr(), 440.0, 0.01), 0);
            assert_eq!(hezhou_audio_engine_last_error(engine), ptr::null());

            let voice = hezhou_audio_play(engine, id.as_ptr());
            assert_ne!(voice, 0);
            assert_eq!(hezhou_audio_voice_state(engine, voice), 0); // Playing

            assert_eq!(hezhou_audio_voice_command(engine, voice, 1), 0); // 暂停
            assert_eq!(hezhou_audio_voice_state(engine, voice), 1); // Paused
            assert_eq!(hezhou_audio_voice_command(engine, voice, 2), 0); // 恢复
            assert_eq!(hezhou_audio_voice_state(engine, voice), 0);

            assert_eq!(hezhou_audio_voice_command(engine, voice, 3), 0); // 停止
            assert_eq!(hezhou_audio_voice_state(engine, voice), 2); // Stopped

            // 3D 播放 + 监听者
            let spatial_voice = hezhou_audio_play_3d(engine, id.as_ptr(), 0.0, 0.0, -5.0);
            assert_ne!(spatial_voice, 0);
            assert_eq!(
                hezhou_audio_voice_set_position(engine, spatial_voice, 1.0, 2.0, 3.0),
                0
            );
            hezhou_audio_set_listener(engine, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0);
            hezhou_audio_set_master_volume(engine, 0.5);
            assert!((hezhou_audio_master_volume(engine) - 0.5).abs() < 1e-6);
            hezhou_audio_set_muted(engine, 1);
            assert_eq!(
                CStr::from_ptr(hezhou_audio_engine_backend_name(engine))
                    .to_str()
                    .unwrap(),
                "null"
            );

            assert_eq!(hezhou_audio_active_voice_count(engine), 1);
            assert_eq!(hezhou_audio_stop_all(engine), 1);
            assert_eq!(hezhou_audio_active_voice_count(engine), 0);

            hezhou_audio_engine_destroy(engine);
        }
    }

    #[test]
    fn ffi_reports_errors_without_panicking() {
        unsafe {
            let engine = hezhou_audio_engine_create();
            let id = c_string("bad");
            // 空指针参数
            assert_eq!(
                hezhou_audio_load_wav_memory(engine, id.as_ptr(), ptr::null(), 0),
                -1
            );
            assert!(!hezhou_audio_engine_last_error(engine).is_null());

            // 未知格式
            let not_audio = b"definitely not audio";
            assert_eq!(
                hezhou_audio_load_wav_memory(
                    engine,
                    id.as_ptr(),
                    not_audio.as_ptr(),
                    not_audio.len()
                ),
                -1
            );
            let message = CStr::from_ptr(hezhou_audio_engine_last_error(engine))
                .to_str()
                .unwrap()
                .to_string();
            assert!(message.contains("无法识别"), "错误信息应可读: {message}");
            hezhou_audio_engine_destroy(engine);
        }
    }

    #[test]
    fn ffi_null_pointer_is_safe() {
        unsafe {
            assert!(hezhou_audio_engine_last_error(ptr::null()).is_null());
            assert!(hezhou_audio_engine_backend_name(ptr::null()).is_null());
            assert_eq!(hezhou_audio_engine_is_silent(ptr::null()), 1);
            assert_eq!(hezhou_audio_play(ptr::null_mut(), ptr::null()), 0);
            assert_eq!(hezhou_audio_voice_state(ptr::null(), 1), 2);
            assert_eq!(hezhou_audio_voice_command(ptr::null_mut(), 1, 1), -1);
            assert_eq!(hezhou_audio_active_voice_count(ptr::null()), 0);
            assert_eq!(hezhou_audio_stop_all(ptr::null_mut()), 0);
            assert_eq!(hezhou_audio_update(ptr::null_mut(), 0.1), 0);
            hezhou_audio_engine_destroy(ptr::null_mut());
        }
    }
}
