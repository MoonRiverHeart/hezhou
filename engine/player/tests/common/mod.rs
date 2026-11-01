//! 测试用临时目录与样例数据构造。
//!
//! 不引入 `tempfile`，用进程 id + 时间纳秒 + 原子计数保证唯一；
//! 退出时尽力清理，测试失败时也不会污染系统临时目录。

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use hezhou_player::builder::{
    BuildSources, CookManifest, RuntimePayload, RuntimeFileSource, ScriptAssembly,
};
use hezhou_player::config::{PlayerBuildConfig, DEFAULT_FONT_ASSET};
use hezhou_player::manifest::{ManifestAsset, PlayerManifest};
use hezhou_player::platform::TargetPlatform;

static COUNTER: AtomicU64 = AtomicU64::new(0);
static BASE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 每个测试进程独立的基础目录。
///
/// Windows 上 `std::fs::create_dir_all` 在多个进程并发创建同一目录时可能
/// 返回 `PermissionDenied`（已知 std 竞态），因此基础目录带进程 id，且同进程
/// 内只用 `OnceLock` 创建一次；每个用例再使用唯一子目录。
fn base_dir() -> &'static Path {
    BASE_DIR.get_or_init(|| {
        let process_id = std::process::id();
        let candidates = [
            std::env::temp_dir().join(format!("hezhou-player-tests-{process_id}")),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("test-tmp")
                .join(process_id.to_string()),
        ];
        let mut last_error = None;
        for dir in candidates {
            match fs::create_dir_all(&dir) {
                Ok(()) => return dir,
                Err(error) => last_error = Some((dir, error)),
            }
        }
        let (dir, error) = last_error.expect("至少有一个候选目录");
        panic!("创建测试基础目录 {} 失败：{error}", dir.display());
    })
}

pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new(label: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let unique = format!(
            "{}-{}-{}",
            nanos,
            COUNTER.fetch_add(1, Ordering::Relaxed),
            label
        );
        let path = base_dir().join(unique);
        fs::create_dir_all(&path).expect("创建测试临时目录");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn join(&self, relative: &str) -> PathBuf {
        self.path
            .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR))
    }

    pub fn write(&self, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = self.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("创建测试文件父目录");
        }
        fs::write(&path, bytes).expect("写入测试文件");
        path
    }

    pub fn mkdir(&self, relative: &str) -> PathBuf {
        let path = self.join(relative);
        fs::create_dir_all(&path).expect("创建测试目录");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// 构造一份可校验通过的构建输入（资产含启动场景 + 字体，1 个脚本 DLL，运行时含 1 个文件）。
pub fn sample_input(temp: &TempDir, target: TargetPlatform) -> hezhou_player::PlayerBuildInput {
    let config = PlayerBuildConfig::new("Demo Game", "1.0.0", "scenes/main.scene", target);
    let cook = CookManifest::new(vec![
        ManifestAsset::new("scenes/main.scene", "scene").with_size(42),
        ManifestAsset::new(DEFAULT_FONT_ASSET, "font").with_size(1024),
        ManifestAsset::new("textures/hero.png", "texture").with_size(2048),
    ]);

    let script_source = temp.write("source/Game.dll", b"fake-mono-assembly");
    let scripts = vec![ScriptAssembly::with_source("Game", script_source)];

    let runtime_source = temp.write("source-runtime/libmono.so", b"fake-libmono");
    let runtime = RuntimePayload::new(vec![RuntimeFileSource {
        relative_path: "libmono.so".to_string(),
        source: runtime_source,
        size_bytes: b"fake-libmono".len() as u64,
    }]);

    hezhou_player::PlayerBuildInput {
        config,
        cook,
        scripts,
        runtime,
        cooked: None,
    }
}

/// 真实构建所需的额外来源：假 exe + 与 `sample_input` 资产声明大小一致的源树。
pub fn sample_build_sources(temp: &TempDir) -> BuildSources {
    let executable = temp.write("source/player-exe", b"fake-player-executable");
    temp.write("source-assets/scenes/main.scene", &vec![b's'; 42]);
    temp.write(
        "source-assets/fonts/HarmonyOS_Sans_SC.ttf",
        &vec![b'f'; 1024],
    );
    temp.write("source-assets/textures/hero.png", &vec![b't'; 2048]);
    BuildSources::new(executable, temp.join("source-assets"))
}

/// 测试 fixture 目录（`tests/fixtures/minimal`）。
pub fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("minimal")
}

/// 真实 cook 输出 fixture 目录（`tests/fixtures/cooked`）。
pub fn cooked_fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("cooked")
}

/// 蒙皮资产 fixture 目录（`tests/fixtures/skinned`）。
pub fn skinned_fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("skinned")
}

/// 生成最小 PCM16 单声道 WAV（0.2s @ 44100Hz）并写入临时目录，返回路径。
pub fn write_test_wav(temp: &TempDir, relative: &str) -> PathBuf {
    let sample_rate: u32 = 44_100;
    let samples: u32 = 8_820; // 0.2s
    let data_size = samples * 2;
    let mut bytes = Vec::with_capacity(44 + data_size as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
    bytes.extend_from_slice(&2u16.to_le_bytes()); // block align
    bytes.extend_from_slice(&16u16.to_le_bytes()); // bits
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    for index in 0..samples {
        // 440Hz 正弦，幅度 0.1，避免爆音。
        let sample = (3000.0 * (2.0 * std::f64::consts::PI * 440.0 * index as f64 / sample_rate as f64).sin()) as i16;
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    temp.write(relative, &bytes)
}

/// 编译 `tests/fixtures/mono/PlayerGame.cs` 为临时 DLL（需要本机 mcs/Mono）。
///
/// 用于 mono-host 真实调用测试；mcs 不存在或编译失败时返回 `None`（测试跳过）。
#[cfg(feature = "mono-host")]
pub fn compile_player_game_fixture(temp: &TempDir) -> Option<PathBuf> {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("mono")
        .join("PlayerGame.cs");
    compile_csharp_source(temp, &source, "PlayerGame.dll")
}

/// 编译 task-30/32 的真实 `engine/scripts/PlayerGame.cs`（Player C# 入口）。
#[cfg(feature = "mono-host")]
pub fn compile_engine_player_game(temp: &TempDir) -> Option<PathBuf> {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("scripts")
        .join("PlayerGame.cs");
    compile_csharp_source(temp, &source, "PlayerGame.dll")
}

/// 用本机 mcs 编译任意 C# 源为临时 DLL；mcs 不存在/编译失败返回 `None`。
#[cfg(feature = "mono-host")]
pub fn compile_csharp_source(temp: &TempDir, source: &Path, dll_name: &str) -> Option<PathBuf> {
    use std::process::Command;

    let output_dir = temp.join("mono");
    std::fs::create_dir_all(&output_dir).ok()?;
    let dll = output_dir.join(dll_name);

    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "mcs.bat"]);
        command
    };
    #[cfg(not(windows))]
    let mut command = Command::new("mcs");

    let status = command
        .arg("-target:library")
        .arg(format!("-out:{}", dll.display()))
        .arg(source)
        .status()
        .ok()?;
    if status.success() && dll.is_file() {
        Some(dll)
    } else {
        eprintln!(
            "跳过 mono-host 真实调用测试：mcs 编译 {} 失败（status={status:?}）",
            source.display()
        );
        None
    }
}

/// 递归复制目录（fixture 损坏/缺失场景测试用）。
pub fn copy_dir_all(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).expect("创建复制目标目录");
    for entry in std::fs::read_dir(source).expect("读取复制源目录") {
        let entry = entry.expect("读取目录项");
        let file_type = entry.file_type().expect("读取文件类型");
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &target);
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), target).expect("复制文件");
        }
    }
}

/// 在临时目录里铺一个完整可启动的 Player 包（用于运行时 CLI 测试）。
pub fn write_player_package(temp: &TempDir, target: TargetPlatform) -> PathBuf {
    let config = PlayerBuildConfig::new("Demo Game", "1.0.0", "scenes/main.scene", target);
    let manifest = PlayerManifest::new(
        &config,
        vec![
            ManifestAsset::new("scenes/main.scene", "scene").with_size(8),
            ManifestAsset::new(DEFAULT_FONT_ASSET, "font").with_size(8),
        ],
        vec!["Game.dll".to_string()],
    );

    temp.write("assets/scenes/main.scene", b"scene-data");
    temp.write("assets/fonts/HarmonyOS_Sans_SC.ttf", b"font-data");
    temp.write("scripts/Game.dll", b"script-data");
    temp.mkdir("runtime");
    manifest
        .save(&temp.path().join("manifest.json"))
        .expect("写入测试清单");
    temp.path().to_path_buf()
}
