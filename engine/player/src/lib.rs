//! `hezhou-player` — Hezhou 0.2 Player 导出构建器与最小运行时。
//!
//! 模块划分：
//! - [`config`]：`PlayerBuildConfig`（启动场景/窗口/版本/平台/质量档）；
//! - [`manifest`]：产物根目录 `manifest.json` 契约与校验；
//! - [`builder`]：dry-run 构建器（cook 清单 + 脚本 DLL + 运行时 → 双平台文件清单）；
//! - [`paths`] / [`platform`]：跨平台路径与平台差异；
//! - [`runtime`] / [`logging`]：`hezhou-player` 启动流程与文件日志。
//!
//! 产物目录格式（详见 `README.md`）：
//! ```text
//! <PlayerRoot>/
//! ├── hezhou-game.exe / hezhou-game
//! ├── manifest.json
//! ├── assets/
//! ├── scripts/
//! └── runtime/
//! ```

pub mod asset_index;
pub mod builder;
pub mod config;
pub mod cook_bridge;
pub mod host;
pub mod logging;
pub mod manifest;
pub mod paths;
pub mod platform;
pub mod runtime;
pub mod version;

/// 可选的真实窗口渲染 host（`--features window-host`）。
#[cfg(feature = "window-host")]
pub mod window_host;

pub use asset_index::{AssetIndexEntry, AssetIndexError, PlayerAssetIndex};
pub use builder::{
    checksum_bytes, checksum_file, normalize_asset_relative, ArtifactCategory, BuildError,
    BuildSources, BuildWarning, BuiltFile, CookManifest, PlannedFile, PlayerBuildInput,
    PlayerBuildOutput, PlayerBuildPlan, PlayerBuilder, RuntimeFileSource, RuntimePayload,
    ScriptAssembly,
};
pub use config::{
    default_executable_stem, ConfigErrors, ConfigIssue, PlayerBuildConfig, QualityTier,
    WindowConfig, WindowMode, DEFAULT_FONT_ASSET,
};
pub use cook_bridge::{find_entry_by_project_path, CookedAssets};
pub use host::{
    resolve_host, DefaultPlayerHost, HostError, HostLog, HostOutcome, PlayerHost,
    PlayerHostContext, DEFAULT_HOST_NAME, HOST_PENDING_EXIT_CODE,
};

#[cfg(feature = "window-host")]
pub use window_host::{
    parse_frame_limit, WindowPlayerHost, DEFAULT_FRAME_LIMIT, FRAME_LIMIT_ENV, MAX_FRAME_LIMIT,
    WINDOW_HOST_NAME,
};
pub use logging::{FileLogger, LogLevel};
pub use manifest::{
    ManifestAsset, ManifestError, PlayerManifest, RuntimePaths, ASSETS_DIR, LOGS_DIR,
    MANIFEST_FILE_NAME, MANIFEST_FORMAT_VERSION, RUNTIME_DIR, SCRIPTS_DIR,
};
pub use paths::{
    case_collision_key, normalize_logical_path, to_native_path, PathError,
};
pub use platform::TargetPlatform;
pub use runtime::{
    resolve_layout, resolve_player_root, HostRunReport, RuntimeError, RuntimeLayout,
    RuntimeOptions, RuntimeReport,
};
pub use version::{SemVer, VersionError};

/// 生成/运行清单的引擎版本（= crate 版本，来源 `engine/Cargo.toml` workspace.version）。
pub const RUNTIME_ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
