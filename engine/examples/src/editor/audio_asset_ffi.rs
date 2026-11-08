//! 0.2 音频 / 资产数据库 编辑器 FFI（task-16）。
//!
//! 职责：
//! - 持有编辑器级 `AudioEngine`（启动时初始化；无声卡自动降级 NullBackend）；
//! - 持有工程级 `AssetDatabase`（启动时扫描当前工作目录 `assets/`）；
//! - 向 C# 暴露最小 FFI：音频初始化/播放/暂停/恢复/停止/主音量/监听者/摘要，
//!   资产打开/扫描/导入 glTF/计数/摘要/列表查询。
//!
//! 失败路径只写日志并返回错误码，绝不 panic/崩溃（编辑器可继续工作）。
//! 本模块不修改 audio/asset crate 源码，仅作为集成层使用其公开 API。

use std::ffi::{CStr, c_char};
use std::path::Path;

use hezhou_asset::{AssetDatabase, AssetKind, LogicalPath};
use hezhou_audio::{AudioEngine, Listener, PlayOptions, SoundData, Vec3};
use hezhou_dfx::*;

/// 编辑器级音频引擎（延迟到启动时初始化；无声卡时自动降级）。
static mut AUDIO: Option<Box<AudioEngine>> = None;
/// 工程资产数据库。
static mut ASSET_DB: Option<Box<AssetDatabase>> = None;
/// 最近一次播放的声部 ID（暂停/恢复/停止目标）。
static mut LAST_VOICE: Option<u64> = None;
/// 最近一次播放的声音 ID（日志用）。
static mut LAST_SOUND: Option<String> = None;

/// 内置测试音（工程内没有音频资产时使用）。
const TEST_TONE_ID: &str = "editor_test_tone";

// === 静态访问辅助（编辑器单线程；与现有 static mut 全局模式一致） ===

#[allow(static_mut_refs)]
fn audio_ref() -> Option<&'static AudioEngine> {
    unsafe { (*std::ptr::addr_of!(AUDIO)).as_deref() }
}

#[allow(static_mut_refs)]
fn audio_mut() -> Option<&'static mut AudioEngine> {
    unsafe { (*std::ptr::addr_of_mut!(AUDIO)).as_deref_mut() }
}

#[allow(static_mut_refs)]
fn db_ref() -> Option<&'static AssetDatabase> {
    unsafe { (*std::ptr::addr_of!(ASSET_DB)).as_deref() }
}

#[allow(static_mut_refs)]
fn db_mut() -> Option<&'static mut AssetDatabase> {
    unsafe { (*std::ptr::addr_of_mut!(ASSET_DB)).as_deref_mut() }
}

// === 启动/帧/退出钩子（由 editor/mod.rs 调用） ===

/// 编辑器启动时初始化音频与资产数据库（失败仅日志，不阻塞启动）。
pub(crate) fn startup_init() {
    let audio_rc = audio_init_impl();
    dfx_info!(
        "Audio",
        "音频引擎初始化: rc={} | {}",
        audio_rc,
        audio_summary()
    );

    let asset_rc = init_asset_db(Some(Path::new(".")));
    dfx_info!(
        "Asset",
        "资产数据库初始化: rc={} | {}",
        asset_rc,
        asset_summary()
    );
}

/// 每帧推进音频状态机（真实设备后端由设备线程推进，`update` 内部自动跳过）。
pub(crate) fn tick(dt_secs: f32) {
    let Some(engine) = audio_ref() else {
        return;
    };
    engine.update(dt_secs);
    // 播放结束后清理失效句柄。
    if let Some(voice) = unsafe { LAST_VOICE } {
        if engine.voice_handle(voice).is_none() {
            unsafe { LAST_VOICE = None };
        }
    }
}

/// 编辑器退出时释放音频后端与资产数据库。
pub(crate) fn shutdown_all() {
    let _ = audio_shutdown_impl();
    let summary = asset_summary();
    unsafe {
        if let Some(db) = (*std::ptr::addr_of_mut!(ASSET_DB)).take() {
            dfx_info!("Asset", "编辑器退出，资产数据库关闭: {}", summary);
            drop(db);
        }
    }
}

// === 音频实现 ===

fn audio_init_impl() -> i32 {
    if audio_ref().is_some() {
        return 0;
    }
    let engine = AudioEngine::new(hezhou_audio::default_backend());
    let status = engine.status();
    if status.is_silent() {
        dfx_warn!(
            "Audio",
            "无可用音频输出设备，已降级为无声模式（播放逻辑照常）: {}",
            status.degraded_reason().unwrap_or("unknown")
        );
    } else {
        dfx_info!(
            "Audio",
            "音频后端就绪: {} @ {}Hz",
            status.backend(),
            status.sample_rate()
        );
    }
    unsafe { AUDIO = Some(Box::new(engine)) };
    0
}

fn audio_shutdown_impl() -> i32 {
    let Some(mut engine) = (unsafe { (*std::ptr::addr_of_mut!(AUDIO)).take() }) else {
        return 0;
    };
    engine.shutdown();
    unsafe {
        LAST_VOICE = None;
        LAST_SOUND = None;
    }
    dfx_info!("Audio", "音频引擎已关闭");
    0
}

/// 工程内第一个音频资产（AssetId 字符串 + 文件系统路径）。
fn first_audio_asset() -> Option<(String, std::path::PathBuf)> {
    let db = db_ref()?;
    let entry = db.entries().find(|entry| entry.kind == AssetKind::Audio)?;
    let path = entry.locator.path().to_fs_path(db.project_root());
    Some((entry.id.to_string(), path))
}

fn play_sound(engine: &AudioEngine, sound_id: &str, looping: bool) -> i32 {
    match engine.play_with(PlayOptions::new(sound_id).looping(looping)) {
        Some(handle) => {
            unsafe {
                LAST_VOICE = Some(handle.id());
                LAST_SOUND = Some(sound_id.to_string());
            }
            dfx_info!(
                "Audio",
                "播放: {} (voice={}, looping={})",
                sound_id,
                handle.id(),
                looping
            );
            0
        }
        None => {
            dfx_error!("Audio", "播放失败: {}（声部数量已达上限或声音未加载）", sound_id);
            -1
        }
    }
}

fn audio_play_first_asset_impl(looping: bool) -> i32 {
    if audio_init_impl() != 0 {
        return -1;
    }
    let Some(engine) = audio_ref() else {
        return -1;
    };

    if let Some((asset_id, path)) = first_audio_asset() {
        match engine.load_audio_file(asset_id.clone(), &path) {
            Ok(()) => {
                return play_sound(engine, &asset_id, looping);
            }
            Err(error) => {
                dfx_warn!(
                    "Audio",
                    "加载工程音频失败（{}），回退内置测试音: {}",
                    path.display(),
                    error
                );
            }
        }
    }

    // 无音频资产/加载失败：播放内置 440Hz 测试音，验证音频链路。
    match SoundData::sine(TEST_TONE_ID, 440.0, 0.5, 48_000) {
        Ok(tone) => {
            engine.load_sound(TEST_TONE_ID, tone);
            dfx_info!("Audio", "工程内无音频资产，播放内置 440Hz 测试音");
            play_sound(engine, TEST_TONE_ID, looping)
        }
        Err(error) => {
            dfx_error!("Audio", "生成测试音失败: {}", error);
            -1
        }
    }
}

fn current_handle() -> Option<(u64, hezhou_audio::SoundHandle)> {
    let engine = audio_ref()?;
    let voice = unsafe { LAST_VOICE }?;
    let handle = engine.voice_handle(voice)?;
    Some((voice, handle))
}

fn audio_summary() -> String {
    match audio_ref() {
        None => "audio=off".to_string(),
        Some(engine) => {
            let status = engine.status();
            let stats = engine.stats();
            let last_voice = unsafe { LAST_VOICE }
                .map(|voice| voice.to_string())
                .unwrap_or_else(|| "-".to_string());
            format!(
                "audio=on backend={} mode={:?} silent={} sample_rate={} master={:.2} voices={} cached={} last_voice={}",
                status.backend(),
                status.mode(),
                status.is_silent(),
                status.sample_rate(),
                engine.master_volume(),
                stats.active_voices,
                stats.cached_sounds,
                last_voice
            )
        }
    }
}

// === Profiler 只读指标（task-39） ===

/// Profiler：资产统计（文件资产数, 全部条目数）；数据库未初始化返回 `None`。
#[allow(static_mut_refs)]
pub(crate) fn asset_stats() -> Option<(usize, usize)> {
    let db = db_ref()?;
    let total = db.entries().count();
    let files = db.entries().filter(|entry| !entry.is_sub_asset()).count();
    Some((files, total))
}

/// Profiler：音频后端名（未初始化返回 `off`）。
#[allow(static_mut_refs)]
pub(crate) fn audio_backend_name() -> String {
    match audio_ref() {
        None => "off".to_string(),
        Some(engine) => engine.status().backend().to_string(),
    }
}

// === 资产实现 ===

fn init_asset_db(root: Option<&Path>) -> i32 {
    if db_ref().is_some() {
        return asset_scan_impl();
    }
    let root = root.unwrap_or_else(|| Path::new("."));
    match AssetDatabase::open(root) {
        Ok(db) => {
            let summary = summarize_db(&db);
            let warnings: Vec<String> = db.warnings().to_vec();
            unsafe { ASSET_DB = Some(Box::new(db)) };
            if warnings.is_empty() {
                dfx_info!("Asset", "资产数据库就绪: {}", summary);
            } else {
                dfx_warn!(
                    "Asset",
                    "资产数据库就绪: {} | 告警 {} 条（首条: {}）",
                    summary,
                    warnings.len(),
                    warnings[0]
                );
            }
            0
        }
        Err(error) => {
            dfx_error!("Asset", "打开资产数据库失败（{}）: {}", root.display(), error);
            -1
        }
    }
}

fn summarize_db(db: &AssetDatabase) -> String {
    let file_assets = db
        .entries()
        .filter(|entry| !entry.is_sub_asset())
        .count();
    let sub_assets = db.entries().filter(|entry| entry.is_sub_asset()).count();
    let gltf_assets = db
        .entries()
        .filter(|entry| entry.kind == AssetKind::Gltf)
        .count();
    format!(
        "project={} assets={} sub_assets={} gltf={} warnings={}",
        db.project_root().display(),
        file_assets,
        sub_assets,
        gltf_assets,
        db.warnings().len()
    )
}

fn asset_summary() -> String {
    match db_ref() {
        None => "assets=0 project=-".to_string(),
        Some(db) => summarize_db(db),
    }
}

fn asset_scan_impl() -> i32 {
    let Some(db) = db_mut() else {
        return -1;
    };
    match db.scan() {
        Ok(report) => {
            let warnings = db.warnings().to_vec();
            dfx_info!(
                "Asset",
                "资产扫描完成: assets={} sub_assets={} created_meta={} loaded_meta={} warnings={}",
                report.asset_count,
                report.sub_asset_count,
                report.created_meta_files,
                report.loaded_meta_files,
                warnings.len()
            );
            for warning in report.warnings.iter().take(5) {
                dfx_warn!("Asset", "扫描告警: {}", warning);
            }
            0
        }
        Err(error) => {
            dfx_error!("Asset", "资产扫描失败: {}", error);
            -1
        }
    }
}

fn asset_import_impl(path_text: &str) -> i32 {
    let Some(db) = db_mut() else {
        return -1;
    };
    let logical = if Path::new(path_text).is_absolute() {
        match LogicalPath::from_fs_path(db.project_root(), Path::new(path_text)) {
            Ok(path) => path,
            Err(error) => {
                dfx_error!("Asset", "导入路径不在工程内（{}）: {}", path_text, error);
                return -4;
            }
        }
    } else {
        match LogicalPath::new(path_text) {
            Ok(path) => path,
            Err(error) => {
                dfx_error!("Asset", "导入路径非法（{}）: {}", path_text, error);
                return -4;
            }
        }
    };

    let Some(asset_id) = auto_register_if_needed(db, &logical) else {
        dfx_error!(
            "Asset",
            "导入失败: 路径不在资产库（先扫描工程）: {}",
            logical.as_str()
        );
        return -2;
    };

    match db.import_gltf(asset_id) {
        Ok(outcome) => {
            let mut counts = [0usize; 5];
            let kinds = [
                AssetKind::Mesh,
                AssetKind::Material,
                AssetKind::Texture,
                AssetKind::Skeleton,
                AssetKind::Animation,
            ];
            for (slot, kind) in counts.iter_mut().zip(kinds) {
                *slot = outcome.sub_assets_of_kind(kind).count();
            }
            dfx_info!(
                "Asset",
                "glTF 导入成功: {} | mesh={} material={} texture={} skeleton={} animation={}",
                logical.as_str(),
                counts[0],
                counts[1],
                counts[2],
                counts[3],
                counts[4]
            );
            for warning in outcome.scene.warnings.iter().take(5) {
                dfx_warn!("Asset", "导入告警: {}", warning);
            }
            0
        }
        Err(error) => {
            dfx_error!("Asset", "glTF 导入失败: {}: {}", logical.as_str(), error);
            -3
        }
    }
}

/// 已注册则直接返回 ID；未注册的新文件若位于工程 `assets/` 内，自动登记（创建 `.meta`）。
///
/// 这样「导入 glTF」按钮无需先手动扫描，符合编辑器用户直觉。
fn auto_register_if_needed(
    db: &mut AssetDatabase,
    logical: &LogicalPath,
) -> Option<hezhou_asset::AssetId> {
    if let Some(id) = db.id_of(logical) {
        return Some(id);
    }
    let fs_path = logical.to_fs_path(db.project_root());
    if !fs_path.is_file() || !logical.is_under(db.assets_root()) {
        return None;
    }
    match db.register_file_asset(logical) {
        Ok(id) => {
            dfx_info!("Asset", "新资产自动登记: {}", logical.as_str());
            Some(id)
        }
        Err(error) => {
            dfx_error!("Asset", "新资产登记失败（{}）: {}", logical.as_str(), error);
            None
        }
    }
}

/// NUL 结尾字符串安全写入：返回写入字节数（不含 `\0`）。
fn write_c_string(buffer: *mut c_char, buffer_size: usize, text: &str) -> usize {
    if buffer.is_null() || buffer_size == 0 {
        return 0;
    }
    let bytes = text.as_bytes();
    let copy_len = std::cmp::min(bytes.len(), buffer_size - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, buffer, copy_len);
        *buffer.add(copy_len) = 0;
    }
    copy_len
}

fn read_c_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr).to_str().ok().map(str::to_string) }
}

// === 资产数据库导出辅助（供 Player 导出 dry-run 使用） ===

/// Player 导出用的资产行（相对 `assets/` 的路径 + 元数据）。
#[derive(Clone, Debug)]
pub(crate) struct ExportAssetRow {
    /// 相对 `assets/` 的路径（Player manifest 的 `path`）。
    pub path: String,
    /// 资产类型（snake_case）。
    pub kind: String,
    /// 源文件字节数。
    pub size_bytes: Option<u64>,
    /// 内容校验（`fnv1a64:...`）。
    pub checksum: Option<String>,
    /// 稳定 AssetId（UUID 字符串）。
    pub asset_id: String,
}

/// 收集 AssetDatabase 中所有文件资产（不含子资产），供 Player dry-run manifest 使用。
pub(crate) fn export_asset_rows() -> Vec<ExportAssetRow> {
    let Some(db) = db_ref() else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for entry in db.entries() {
        if entry.is_sub_asset() {
            continue;
        }
        let Some(relative) = entry.locator.path().strip_prefix(db.assets_root()) else {
            continue;
        };
        let fs_path = entry.locator.path().to_fs_path(db.project_root());
        let metadata = std::fs::metadata(&fs_path).ok();
        let checksum = std::fs::read(&fs_path)
            .ok()
            .map(|bytes| format!("fnv1a64:{}", hezhou_asset::fnv1a64_hex(&bytes)));
        rows.push(ExportAssetRow {
            path: relative.as_str().to_string(),
            kind: entry.kind.as_str().to_string(),
            size_bytes: metadata.map(|meta| meta.len()),
            checksum,
            asset_id: entry.id.to_string(),
        });
    }
    rows.sort_by(|left, right| left.path.cmp(&right.path));
    rows
}

// === extern "C" FFI（注册到 FfiContext；见 scripting/src/ffi_context/audio_asset.rs） ===

#[unsafe(no_mangle)]
pub extern "C" fn audio_init() -> i32 {
    audio_init_impl()
}

#[unsafe(no_mangle)]
pub extern "C" fn audio_shutdown() -> i32 {
    audio_shutdown_impl()
}

#[unsafe(no_mangle)]
pub extern "C" fn audio_play_first_asset(looping: i32) -> i32 {
    audio_play_first_asset_impl(looping != 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn audio_pause() -> i32 {
    match current_handle() {
        Some((_, handle)) if handle.pause() => 0,
        _ => {
            dfx_warn!("Audio", "暂停失败: 没有可暂停的声部");
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn audio_resume() -> i32 {
    match current_handle() {
        Some((_, handle)) if handle.resume() => 0,
        _ => {
            dfx_warn!("Audio", "恢复失败: 没有可恢复的声部");
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn audio_stop() -> i32 {
    match current_handle() {
        Some((voice, handle)) => {
            handle.stop();
            unsafe { LAST_VOICE = None };
            dfx_info!("Audio", "停止声部: voice={}", voice);
            0
        }
        None => {
            dfx_warn!("Audio", "停止失败: 没有正在播放的声部");
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn audio_set_master_volume(volume: f32) -> i32 {
    match audio_mut() {
        Some(engine) => {
            engine.set_master_volume(volume);
            dfx_info!("Audio", "主音量: {:.2}", engine.master_volume());
            0
        }
        None => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn audio_set_listener(x: f32, y: f32, z: f32) -> i32 {
    match audio_mut() {
        Some(engine) => {
            let mut listener = Listener::default();
            listener.position = Vec3::new(x, y, z);
            engine.set_listener(listener);
            0
        }
        None => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn audio_get_summary(buffer: *mut c_char, buffer_size: usize) -> usize {
    write_c_string(buffer, buffer_size, &audio_summary())
}

#[unsafe(no_mangle)]
pub extern "C" fn asset_open_project(path: *const c_char) -> i32 {
    let root_text = read_c_string(path);
    let root = root_text.as_deref().filter(|text| !text.is_empty());
    match root {
        Some(text) => {
            let path = Path::new(text);
            // 重新打开时先丢弃旧数据库，避免工程切换后残留映射。
            unsafe {
                if let Some(old) = (*std::ptr::addr_of_mut!(ASSET_DB)).take() {
                    drop(old);
                }
            }
            init_asset_db(Some(path))
        }
        None => init_asset_db(None),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn asset_scan() -> i32 {
    asset_scan_impl()
}

#[unsafe(no_mangle)]
pub extern "C" fn asset_import_gltf(path: *const c_char) -> i32 {
    match read_c_string(path) {
        Some(text) if !text.is_empty() => asset_import_impl(&text),
        _ => {
            dfx_error!("Asset", "导入失败: 空路径");
            -4
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn asset_count() -> i32 {
    db_ref().map(|db| db.len() as i32).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn asset_get_summary(buffer: *mut c_char, buffer_size: usize) -> usize {
    write_c_string(buffer, buffer_size, &asset_summary())
}

#[unsafe(no_mangle)]
pub extern "C" fn asset_get_asset(
    index: i32,
    id_buffer: *mut c_char,
    id_size: usize,
    path_buffer: *mut c_char,
    path_size: usize,
) -> i32 {
    let Some(db) = db_ref() else {
        return -1;
    };
    if index < 0 {
        return -1;
    }
    let Some(entry) = db.entries().nth(index as usize) else {
        return -1;
    };
    write_c_string(id_buffer, id_size, &entry.id.to_string());
    write_c_string(path_buffer, path_size, &entry.locator.to_string());
    0
}

