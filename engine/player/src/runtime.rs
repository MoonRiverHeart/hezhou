//! `hezhou-player` 最小运行时启动流程。
//!
//! 本轮职责（不接渲染/脚本，只建立可验收的启动契约）：
//! 1. 定位包根目录与 `manifest.json`；
//! 2. 校验清单结构、格式版本、目标平台、引擎版本兼容性；
//! 3. 校验资产/脚本/运行时/字体文件是否齐全（跨平台路径规则）；
//! 4. 初始化文件日志（无终端），记录启动摘要；
//! 5. `--validate` 模式只做启动检查，便于 CI 冒烟测试。
//!
//! 真正的窗口创建、Vulkan 初始化、Mono 加载在后续轮次接入
//! `render` / `scripting` / `platform` crate（见 README 集成点）。

use std::fmt;
use std::path::{Path, PathBuf};

use crate::logging::{format_timestamp_utc, FileLogger, STARTUP_ERROR_LOG};
use crate::manifest::{ManifestError, PlayerManifest};
use crate::paths::to_native_path;
use crate::platform::TargetPlatform;
use crate::RUNTIME_ENGINE_VERSION;

/// 运行时命令行选项。
#[derive(Clone, Debug, Default)]
pub struct RuntimeOptions {
    /// 包根目录；默认取可执行文件所在目录。
    pub player_root: Option<PathBuf>,
    /// 清单路径；默认 `<player_root>/manifest.json`。
    pub manifest_path: Option<PathBuf>,
    /// 日志目录覆盖；默认用户数据目录。
    pub log_dir: Option<PathBuf>,
    /// 只做启动检查，不进入游戏循环。
    pub validate_only: bool,
}

/// 已解析的包内布局（本机路径）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeLayout {
    pub assets_dir: PathBuf,
    pub scripts_dir: PathBuf,
    pub runtime_dir: PathBuf,
    pub start_scene: PathBuf,
    /// 必检字体路径（清单未配置时为 `None`）。
    pub font: Option<PathBuf>,
}

/// 启动成功后的报告。
pub struct RuntimeReport {
    pub manifest: PlayerManifest,
    pub root: PathBuf,
    pub layout: RuntimeLayout,
    pub log_path: PathBuf,
}

impl RuntimeReport {
    /// 单行启动摘要（带控制台时打印；无终端时只进日志）。
    pub fn summary(&self) -> String {
        format!(
            "hezhou-player {} 启动检查通过：{} v{}（{}，质量 {}）场景 {}，日志 {}",
            RUNTIME_ENGINE_VERSION,
            self.manifest.product_name,
            self.manifest.version,
            self.manifest.target.display_name(),
            self.manifest.quality.as_str(),
            self.layout.start_scene.display(),
            self.log_path.display(),
        )
    }
}

/// 运行时错误。
#[derive(Debug)]
pub enum RuntimeError {
    Manifest(ManifestError),
    Io {
        context: String,
        source: std::io::Error,
    },
    InvalidRoot {
        path: PathBuf,
    },
    MissingFile {
        path: PathBuf,
    },
    MissingDirectory {
        path: PathBuf,
    },
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest(source) => write!(f, "{source}"),
            Self::Io { context, source } => write!(f, "{context} 失败：{source}"),
            Self::InvalidRoot { path } => write!(
                f,
                "无法确定 Player 包根目录（可执行文件路径非法）：{}",
                path.display()
            ),
            Self::MissingFile { path } => write!(f, "缺少必需文件：{}", path.display()),
            Self::MissingDirectory { path } => write!(f, "缺少必需目录：{}", path.display()),
        }
    }
}

impl std::error::Error for RuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Manifest(source) => Some(source),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// 当前可执行文件所在目录（Player 包根目录的默认值）。
pub fn resolve_player_root() -> Result<PathBuf, RuntimeError> {
    let exe = std::env::current_exe().map_err(|source| RuntimeError::Io {
        context: "获取可执行文件路径".to_string(),
        source,
    })?;
    exe.parent()
        .map(Path::to_path_buf)
        .filter(|dir| !dir.as_os_str().is_empty())
        .ok_or(RuntimeError::InvalidRoot { path: exe })
}

/// 用户数据目录下的默认日志路径。
///
/// - Windows：`%LOCALAPPDATA%\Hezhou\<product>\logs`
/// - Linux：`$XDG_DATA_HOME/hezhou/<product>/logs` 或 `$HOME/.local/share/hezhou/<product>/logs`
/// - 都没有时回退系统临时目录，保证仍能写启动日志。
pub fn default_log_dir(product_name: &str) -> PathBuf {
    let slug = product_slug(product_name);

    #[cfg(target_os = "windows")]
    {
        if let Some(base) = std::env::var_os("LOCALAPPDATA") {
            if !base.is_empty() {
                return PathBuf::from(base)
                    .join("Hezhou")
                    .join(&slug)
                    .join("logs");
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(base) = std::env::var_os("XDG_DATA_HOME") {
            if !base.is_empty() {
                return PathBuf::from(base)
                    .join("hezhou")
                    .join(&slug)
                    .join("logs");
            }
        }
        if let Some(home) = std::env::var_os("HOME") {
            if !home.is_empty() {
                return PathBuf::from(home)
                    .join(".local")
                    .join("share")
                    .join("hezhou")
                    .join(&slug)
                    .join("logs");
            }
        }
    }

    std::env::temp_dir()
        .join("hezhou-player")
        .join(slug)
        .join("logs")
}

/// 产品名转目录 slug（ASCII 小写字母数字与 `-`，其余折叠为 `-`）。
pub fn product_slug(product_name: &str) -> String {
    let mut out = String::new();
    let mut last_was_dash = false;
    for ch in product_name.chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_was_dash = false;
        } else if !last_was_dash && !out.is_empty() {
            out.push('-');
            last_was_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "player".to_string()
    } else {
        trimmed
    }
}

/// 执行启动流程。
pub fn run(options: RuntimeOptions) -> Result<RuntimeReport, RuntimeError> {
    let manifest_override = options.manifest_path.clone();
    // 包根目录优先级：显式 --player-root > manifest 所在目录 > 可执行文件目录。
    let root = match (options.player_root.clone(), manifest_override.as_deref()) {
        (Some(root), _) => root,
        (None, Some(manifest)) => manifest
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or(resolve_player_root()?),
        (None, None) => resolve_player_root()?,
    };
    let manifest_path = manifest_override.unwrap_or_else(|| PlayerManifest::manifest_path(&root));

    // 1) 读取并校验清单（失败信息由 main 写入 startup-error.log）。
    let manifest = PlayerManifest::load(&manifest_path)
        .map_err(RuntimeError::Manifest)
        .and_then(|manifest| {
            manifest
                .validate_for_runtime(TargetPlatform::host())
                .map_err(RuntimeError::Manifest)?;
            Ok(manifest)
        })?;

    // 2) 解析包内布局。
    let layout = resolve_layout(&root, &manifest)?;

    // 3) 动态库查找：把 runtime/ 注入平台搜索路径（必须在加载 Mono/Vulkan 之前）。
    let (library_env, library_value) = apply_runtime_library_search(&manifest, &layout.runtime_dir);

    // 4) 打开日志：优先用户数据目录，失败回退包内 logs/。
    let preferred_log_dir = options
        .log_dir
        .unwrap_or_else(|| default_log_dir(&manifest.product_name));
    let (mut logger, log_fallback_note) = open_logger(&preferred_log_dir, &root)?;

    log_info(
        &mut logger,
        &format!(
            "hezhou-player {} 启动；清单 {}（format {}）",
            RUNTIME_ENGINE_VERSION,
            manifest_path.display(),
            manifest.format_version
        ),
    )?;
    if let Some(note) = log_fallback_note {
        log_warn(&mut logger, &note)?;
    }
    log_info(
        &mut logger,
        &format!(
            "产品 {} v{}；目标 {}；质量 {}；窗口 {}x{} {}",
            manifest.product_name,
            manifest.version,
            manifest.target.display_name(),
            manifest.quality.as_str(),
            manifest.window.width,
            manifest.window.height,
            manifest.window.mode.as_str()
        ),
    )?;
    log_info(
        &mut logger,
        &format!("启动场景：{}", layout.start_scene.display()),
    )?;
    log_info(
        &mut logger,
        &format!(
            "资产目录 {}；脚本目录 {}；运行时目录 {}",
            layout.assets_dir.display(),
            layout.scripts_dir.display(),
            layout.runtime_dir.display()
        ),
    )?;
    log_info(
        &mut logger,
        &format!("动态库查找：{library_env}={library_value}"),
    )?;

    if let Some(font) = &layout.font {
        log_info(&mut logger, &format!("必检字体：{}", font.display()))?;
    }

    // 5) 运行时目录为空时给出警告（不阻塞骨架启动）。
    let runtime_empty = std::fs::read_dir(&layout.runtime_dir)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(true);
    if runtime_empty {
        log_warn(
            &mut logger,
            "runtime/ 为空：本轮骨架允许，正式发布必须随包内置运行时",
        )?;
    }

    // 6) 完成（本轮不创建窗口/渲染/加载 Mono）。
    if options.validate_only {
        log_info(&mut logger, "validate-only 模式：启动检查完成")?;
    } else {
        log_info(
            &mut logger,
            "运行时骨架启动成功：窗口/渲染/脚本加载待接入（见 engine/player/README.md 集成点）",
        )?;
    }

    let log_path = logger.path().to_path_buf();
    Ok(RuntimeReport {
        manifest,
        root,
        layout,
        log_path,
    })
}

/// 解析并校验包内布局（目录/文件存在性 + 逻辑路径转换）。
pub fn resolve_layout(
    root: &Path,
    manifest: &PlayerManifest,
) -> Result<RuntimeLayout, RuntimeError> {
    let assets_dir = manifest.assets_dir(root);
    let scripts_dir = manifest.scripts_dir(root);
    let runtime_dir = manifest.runtime_dir(root);
    let start_scene = manifest.start_scene_path(root);
    let font = manifest.font_path(root);

    if !assets_dir.is_dir() {
        return Err(RuntimeError::MissingDirectory { path: assets_dir });
    }
    if !scripts_dir.is_dir() {
        return Err(RuntimeError::MissingDirectory {
            path: scripts_dir,
        });
    }
    if !runtime_dir.is_dir() {
        return Err(RuntimeError::MissingDirectory {
            path: runtime_dir,
        });
    }
    if !start_scene.is_file() {
        return Err(RuntimeError::MissingFile { path: start_scene });
    }
    if let Some(font) = &font {
        if !font.is_file() {
            return Err(RuntimeError::MissingFile { path: font.clone() });
        }
    }
    for script in &manifest.scripts {
        let path = scripts_dir.join(to_native_path(script, manifest.target));
        if !path.is_file() {
            return Err(RuntimeError::MissingFile { path });
        }
    }

    Ok(RuntimeLayout {
        assets_dir,
        scripts_dir,
        runtime_dir,
        start_scene,
        font,
    })
}

/// 把 `runtime/` 注入平台动态库搜索路径，并返回 `(变量名, 新值)` 供日志记录。
///
/// 必须在校启动早期（加载 Mono/Vulkan 之前）调用：Windows 下 `LoadLibrary`
/// 直接读 `PATH`；Linux 下 `dlopen` 读 `LD_LIBRARY_PATH`，但可执行文件自身的
/// 依赖需要在链接期写 `$ORIGIN/runtime` rpath（`TargetPlatform::rpath_link_arg`）。
fn apply_runtime_library_search(
    manifest: &PlayerManifest,
    runtime_dir: &Path,
) -> (String, String) {
    let env_name = manifest.target.library_search_env().to_string();
    let value = manifest.target.runtime_library_search_value(runtime_dir);
    // 单线程启动早期设置，只有后续 dlopen/LoadLibrary 读取该变量。
    std::env::set_var(&env_name, &value);
    (env_name, value)
}

/// 打开日志文件；主目录失败时回退到包内 `logs/`。
fn open_logger(
    preferred: &Path,
    root: &Path,
) -> Result<(FileLogger, Option<String>), RuntimeError> {
    match FileLogger::create_in(preferred) {
        Ok(logger) => Ok((logger, None)),
        Err(error) => {
            let fallback = root.join(crate::manifest::LOGS_DIR);
            let logger = FileLogger::create_in(&fallback).map_err(|fallback_error| {
                RuntimeError::Io {
                    context: format!(
                        "创建日志目录 {}（回退 {} 也失败：{fallback_error}）",
                        preferred.display(),
                        fallback.display()
                    ),
                    source: error,
                }
            })?;
            Ok((
                logger,
                Some(format!(
                    "日志目录 {} 不可写，已回退到 {}",
                    preferred.display(),
                    fallback.display()
                )),
            ))
        }
    }
}

fn log_info(logger: &mut FileLogger, message: &str) -> Result<(), RuntimeError> {
    logger.info(message).map_err(|source| RuntimeError::Io {
        context: "写入 Player 日志".to_string(),
        source,
    })
}

fn log_warn(logger: &mut FileLogger, message: &str) -> Result<(), RuntimeError> {
    logger.warn(message).map_err(|source| RuntimeError::Io {
        context: "写入 Player 日志".to_string(),
        source,
    })
}

/// 启动失败时写 `startup-error.log`（Windows 无终端场景的唯一可见诊断）。
///
/// 优先写包内 `logs/`；安装目录只读时回退系统临时目录。
pub fn write_failure_log(root: &Path, error: &RuntimeError) -> Option<PathBuf> {
    let line = format!(
        "[{}] hezhou-player 启动失败：{}\n",
        format_timestamp_utc(std::time::SystemTime::now()),
        error
    );
    let candidates = [
        root.join(crate::manifest::LOGS_DIR),
        std::env::temp_dir().join("hezhou-player").join("logs"),
    ];
    for dir in candidates {
        if std::fs::create_dir_all(&dir).is_err() {
            continue;
        }
        let path = dir.join(STARTUP_ERROR_LOG);
        if std::fs::write(&path, &line).is_ok() {
            return Some(path);
        }
    }
    None
}
