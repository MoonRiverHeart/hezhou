//! 编辑器 Profiler 指标采集与 FFI（0.3 task-39）。
//!
//! - 维护最近 120 帧的 CPU 帧时间 ring buffer（`record_frame`）；
//! - 启动耗时（`set_startup_ms`，由编辑器 `run()` 在初始化完成后写入）；
//! - DFX 每帧快照（FPS/CPU/内存，`record_dfx`）；
//! - 场景/渲染/资产/音频统计在 `snapshot_json()` 时从编辑器全局读取；
//! - FFI：`profiler_snapshot_json` / `profiler_reset` / `profiler_frame_count`，失败安全。
//!
//! 限制：CPU 帧时间（墙钟），非 GPU timestamp；p95 为 ring buffer 内样本的最近秩估计。

use std::ffi::c_char;

use hezhou_dfx::*;
use serde_json::json;

/// ring buffer 容量（最近 120 帧）。
const RING_CAPACITY: usize = 120;

static mut FRAME_MS: [f64; RING_CAPACITY] = [0.0; RING_CAPACITY];
/// 下一个写入位置。
static mut FRAME_HEAD: usize = 0;
/// 自上次 reset 后累计记录的帧数（单调递增）。
static mut FRAME_COUNT: u64 = 0;
/// 编辑器启动耗时（毫秒）；0 表示尚未测量。
static mut STARTUP_MS: f64 = 0.0;
/// DFX 最近一帧快照（由主循环写入）。
static mut DFX_FPS: f32 = 0.0;
static mut DFX_CPU_PERCENT: f32 = 0.0;
static mut DFX_MEMORY_USED_MB: f32 = 0.0;
static mut DFX_MEMORY_AVAILABLE_MB: f32 = 0.0;

/// 保留小数位（避免 JSON 出现过长浮点）。
fn round(value: f64, digits: i32) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    let factor = 10f64.powi(digits);
    (value * factor).round() / factor
}

/// 编辑器初始化完成时写入启动耗时（毫秒）。
pub(crate) fn set_startup_ms(milliseconds: f64) {
    unsafe {
        STARTUP_MS = if milliseconds.is_finite() && milliseconds >= 0.0 {
            milliseconds
        } else {
            0.0
        };
    }
}

/// 记录一帧 CPU 帧时间（毫秒）；非法值忽略。
pub(crate) fn record_frame(frame_ms: f64) {
    if !frame_ms.is_finite() || frame_ms < 0.0 {
        return;
    }
    unsafe {
        (*std::ptr::addr_of_mut!(FRAME_MS))[FRAME_HEAD] = frame_ms;
        FRAME_HEAD = (FRAME_HEAD + 1) % RING_CAPACITY;
        FRAME_COUNT = FRAME_COUNT.saturating_add(1);
    }
}

/// 记录 DFX 每帧快照（值非法时保持 0）。
pub(crate) fn record_dfx(fps: f32, cpu_percent: f32, memory_used_mb: f32, memory_available_mb: f32) {
    unsafe {
        DFX_FPS = if fps.is_finite() { fps.max(0.0) } else { 0.0 };
        DFX_CPU_PERCENT = if cpu_percent.is_finite() { cpu_percent.max(0.0) } else { 0.0 };
        DFX_MEMORY_USED_MB = if memory_used_mb.is_finite() { memory_used_mb.max(0.0) } else { 0.0 };
        DFX_MEMORY_AVAILABLE_MB = if memory_available_mb.is_finite() {
            memory_available_mb.max(0.0)
        } else {
            0.0
        };
    }
}

/// 按时间顺序返回 ring buffer 中的帧时间样本（最多 120 个）。
fn frame_samples() -> Vec<f64> {
    unsafe {
        let buffer = &*std::ptr::addr_of!(FRAME_MS);
        let head = FRAME_HEAD;
        let count = FRAME_COUNT.min(RING_CAPACITY as u64) as usize;
        if count == 0 {
            return Vec::new();
        }
        // head 指向下一个写入位置：最早样本为 head - count（模容量）。
        let mut samples = Vec::with_capacity(count);
        for offset in 0..count {
            let index = (head + RING_CAPACITY - count + offset) % RING_CAPACITY;
            samples.push(buffer[index]);
        }
        samples
    }
}

/// 累计帧数（自上次 reset 后）。
fn total_frame_count() -> u64 {
    unsafe { FRAME_COUNT }
}

/// 构建 Profiler 快照 JSON（字段稳定，供 C#/文档使用）。
pub(crate) fn snapshot_json() -> String {
    let samples = frame_samples();
    let sample_count = samples.len();
    let (avg, p95, max, last) = if samples.is_empty() {
        (0.0, 0.0, 0.0, 0.0)
    } else {
        let sum: f64 = samples.iter().sum();
        let avg = sum / sample_count as f64;
        let mut sorted = samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p95_index = ((sample_count as f64 - 1.0) * 0.95).round() as usize;
        let p95 = sorted[p95_index.min(sample_count - 1)];
        let max = sorted[sample_count - 1];
        let last = *samples.last().unwrap_or(&0.0);
        (avg, p95, max, last)
    };
    let fps = if avg > 0.0 { 1000.0 / avg } else { 0.0 };

    // 场景/渲染统计：全局指针由编辑器主线程维护，这里只读。
    let (entity_count, pbr_entity_count, skinned_mesh_count) = unsafe {
        match (super::SCENE, super::RENDERER) {
            (Some(scene_ptr), Some(renderer_ptr)) => {
                let scene = &*scene_ptr;
                let renderer = &*renderer_ptr;
                (
                    scene.get_all_entities().len(),
                    renderer.pbr_material_override_count(),
                    renderer.skinned_mesh_count(),
                )
            }
            (Some(scene_ptr), None) => ((*scene_ptr).get_all_entities().len(), 0, 0),
            _ => (0, 0, 0),
        }
    };
    let (asset_count, asset_total_count) = super::audio_asset_ffi::asset_stats().unwrap_or((0, 0));
    let audio_backend = super::audio_asset_ffi::audio_backend_name();

    unsafe {
        let startup_ms = STARTUP_MS;
        let dfx_fps = DFX_FPS;
        let dfx_cpu = DFX_CPU_PERCENT;
        let dfx_mem_used = DFX_MEMORY_USED_MB;
        let dfx_mem_available = DFX_MEMORY_AVAILABLE_MB;
        let frame_count = FRAME_COUNT;

        let frames: Vec<f64> = samples.iter().map(|value| round(*value, 3)).collect();
        json!({
            "frame_count": frame_count,
            "sample_count": sample_count,
            "ring_capacity": RING_CAPACITY,
            "fps": round(fps, 1),
            "frame_ms_avg": round(avg, 3),
            "frame_ms_p95": round(p95, 3),
            "frame_ms_max": round(max, 3),
            "frame_ms_last": round(last, 3),
            "startup_ms": round(startup_ms, 1),
            "dfx_fps": round(dfx_fps as f64, 1),
            "cpu_percent": round(dfx_cpu as f64, 1),
            "memory_used_mb": round(dfx_mem_used as f64, 1),
            "memory_available_mb": round(dfx_mem_available as f64, 1),
            "entity_count": entity_count,
            "pbr_entity_count": pbr_entity_count,
            "skinned_mesh_count": skinned_mesh_count,
            "asset_count": asset_count,
            "asset_total_count": asset_total_count,
            "audio_backend": audio_backend,
            "frames": frames,
        })
        .to_string()
    }
}

/// 重置帧统计（保留启动耗时；下一次 record_frame 重新开始）。
pub(crate) fn reset_stats() {
    unsafe {
        FRAME_HEAD = 0;
        FRAME_COUNT = 0;
    }
}

fn write_c_string(buffer: *mut c_char, buffer_size: usize, text: &str) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let bytes = text.as_bytes();
    let copy_len = bytes.len().min(buffer_size - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer as *mut u8, copy_len);
        *buffer.add(copy_len) = 0;
    }
    copy_len
}

/// FFI：写入 Profiler 快照 JSON；返回字节数（0=失败/空）。
#[unsafe(no_mangle)]
pub extern "C" fn profiler_snapshot_json(buffer: *mut c_char, buffer_size: usize) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let snapshot = snapshot_json();
    if snapshot.is_empty() {
        unsafe { *buffer = 0; }
        return 0;
    }
    write_c_string(buffer, buffer_size, &snapshot)
}

/// FFI：重置帧统计；返回 0。
#[unsafe(no_mangle)]
pub extern "C" fn profiler_reset() -> i32 {
    reset_stats();
    dfx_info!("Profiler", "帧统计已重置（启动耗时保留）");
    0
}

/// FFI：累计记录帧数；返回 i32（饱和到 i32::MAX）。
#[unsafe(no_mangle)]
pub extern "C" fn profiler_frame_count() -> i32 {
    total_frame_count().min(i32::MAX as u64) as i32
}


