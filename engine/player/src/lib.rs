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
pub mod animation_control;
pub mod asset_catalog;
pub mod animation_graph;
pub mod audio_runtime;
pub mod builder;
pub mod gameplay_runtime;
pub mod config;
pub mod cook_bridge;
pub mod csharp_context;
pub mod host;
pub mod imported_assets;
pub mod logging;
pub mod manifest;
#[cfg(feature = "mono-host")]
pub mod mono_host;
pub mod paths;
pub mod platform;
pub mod runtime;
pub mod runtime_setup;
pub mod scene_loader;
pub mod skinned_asset;
pub mod version;

/// 可选的真实窗口渲染 host（`--features window-host`）。
#[cfg(feature = "window-host")]
pub mod window_host;

#[cfg(feature = "mono-host")]
pub use mono_host::{
    find_player_game_script, MonoHostError, PlayerGameScript, PLAYER_GAME_CLASS,
    PLAYER_GAME_NAMESPACE,
};

pub use asset_index::{AssetIndexEntry, AssetIndexError, PlayerAssetIndex};
pub use asset_catalog::{PlayerAssetCatalog, PlayerAssetEntry};
pub use animation_graph::{
    AnimationGraphClip, AnimationGraphError, PlayerAnimationGraph,
};
pub use audio_runtime::PlayerAudioRuntime;
pub use gameplay_runtime::{
    GameplayRuntimeError, PlayerCharacterState, PlayerGameplayRuntime,
    DEFAULT_CHARACTER_HEIGHT, DEFAULT_CHARACTER_RADIUS,
};
pub use animation_control::{
    AnimationClipEntry, AnimationControlError, PlayerAnimationState,
    DEFAULT_ANIMATION_SPEED, MAX_ANIMATION_SPEED,
};
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
pub use csharp_context::{
    player_context_log, scene_create_cube, scene_create_entity, scene_destroy_entity,
    scene_entity_count, scene_entity_exists, scene_get_entity_position, scene_get_entity_rotation,
    scene_get_entity_scale, scene_set_entity_position, scene_set_entity_rotation,
    scene_set_entity_scale, PlayerCSharpContext, SceneCreateCubeFn, SceneCreateEntityFn,
    SceneDestroyEntityFn, SceneEntityCountFn, SceneEntityExistsFn, SceneGetEntityPositionFn,
    SceneGetEntityRotationFn, SceneGetEntityScaleFn, SceneSetEntityPositionFn,
    SceneSetEntityRotationFn, SceneSetEntityScaleFn, PLAYER_CSHARP_CONTEXT_VERSION,
};
pub use host::{
    resolve_host, DefaultPlayerHost, HostError, HostLog, HostOutcome, PlayerHost,
    PlayerHostContext, DEFAULT_HOST_NAME, HOST_PENDING_EXIT_CODE,
};
pub use imported_assets::{
    ImportedAssetBundle, ImportedAssetError, ImportedAssetStats, ImportedAssetSummary,
};
pub use scene_loader::{
    builtin_test_scene, load_start_scene, LoadedStartScene, StartSceneSource, StartSceneSummary,
};
pub use skinned_asset::{
    compute_joint_palette, primitive_to_skinned_mesh, select_first_skinned_asset,
    SelectedSkinnedAsset, SkinnedAnimationInfo, SkinnedAssetError, SkinnedSkeletonInfo,
    PLAYER_SKINNED_ENTITY_ID,
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
pub use runtime_setup::{
    setup_mono_runtime, setup_mono_runtime_for, MonoRuntimeSetup, MONO_PATH_ENV,
    MONO_RUNTIME_DIR,
};
pub use version::{SemVer, VersionError};

/// 生成/运行清单的引擎版本（= crate 版本，来源 `engine/Cargo.toml` workspace.version）。
pub const RUNTIME_ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
