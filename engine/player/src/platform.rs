//! 目标平台差异：可执行文件命名、动态库查找、文件权限位、归档格式。
//!
//! Player 只支持 Windows / Linux 两个目标（见 PROJECT_PLAN D4）。
//! 平台差异集中在本模块，避免散落在构建器与运行时里。

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Player 支持的目标平台。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetPlatform {
    Windows,
    Linux,
}

impl TargetPlatform {
    /// 当前编译目标平台。
    #[cfg(target_os = "windows")]
    pub fn host() -> Self {
        Self::Windows
    }

    /// 当前编译目标平台。
    #[cfg(target_os = "linux")]
    pub fn host() -> Self {
        Self::Linux
    }

    /// 当前编译目标平台。
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    compile_error!("hezhou-player 仅支持 Windows 与 Linux 目标平台");

    /// 双平台固定顺序，用于生成可重复的双平台清单。
    pub const ALL: [TargetPlatform; 2] = [TargetPlatform::Windows, TargetPlatform::Linux];

    /// 清单/日志中的小写标识（`windows` / `linux`）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Linux => "linux",
        }
    }

    /// 人类可读名称。
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Windows => "Windows",
            Self::Linux => "Linux",
        }
    }

    /// 可执行文件后缀（Windows `.exe`，Linux 无后缀）。
    pub fn executable_suffix(self) -> &'static str {
        match self {
            Self::Windows => ".exe",
            Self::Linux => "",
        }
    }

    /// 生成目标平台的可执行文件名。
    pub fn executable_file_name(self, stem: &str) -> String {
        format!("{stem}{}", self.executable_suffix())
    }

    /// 运行时目录名（Mono、Vulkan、VC runtime、字体等）。
    pub fn runtime_dir_name(self) -> &'static str {
        "runtime"
    }

    /// 可执行文件在 Unix 下的权限位；Windows 无 Unix 权限概念，返回 `None`。
    pub fn unix_executable_mode(self) -> Option<u32> {
        match self {
            Self::Windows => None,
            Self::Linux => Some(0o755),
        }
    }

    /// 普通数据文件在 Unix 下的权限位。
    pub fn unix_data_mode(self) -> Option<u32> {
        match self {
            Self::Windows => None,
            Self::Linux => Some(0o644),
        }
    }

    /// 动态库查找时优先追加到哪个环境变量。
    ///
    /// - Windows：`PATH`（DLL 加载器按 PATH 搜索，exe 同目录优先）。
    /// - Linux：`LD_LIBRARY_PATH`（仅影响 `dlopen`；可执行文件自身的依赖必须
    ///   在链接期写入 `$ORIGIN/runtime` rpath）。
    pub fn library_search_env(self) -> &'static str {
        match self {
            Self::Windows => "PATH",
            Self::Linux => "LD_LIBRARY_PATH",
        }
    }

    /// 路径列表分隔符（Windows `;`，Linux `:`）。
    pub fn path_list_separator(self) -> char {
        match self {
            Self::Windows => ';',
            Self::Linux => ':',
        }
    }

    /// 把 `runtime/` 目录加入动态库搜索路径后的环境变量值（新目录优先）。
    ///
    /// - Windows：DLL 搜索使用 `PATH`（`LoadLibrary` 会读）。
    /// - Linux：`dlopen` 使用 `LD_LIBRARY_PATH`；可执行文件自身依赖必须在
    ///   链接期写 rpath（见 [`Self::rpath_link_arg`]）。
    pub fn runtime_library_search_value(self, runtime_dir: &Path) -> String {
        let runtime_dir = runtime_dir.to_string_lossy();
        let existing = std::env::var_os(self.library_search_env()).unwrap_or_default();
        if existing.is_empty() {
            return runtime_dir.to_string();
        }
        format!(
            "{runtime_dir}{}{}",
            self.path_list_separator(),
            existing.to_string_lossy()
        )
    }

    /// Linux 链接期需要的 rpath 参数（构建 CI 集成点）。
    ///
    /// 需要由构建 CI 以 `RUSTFLAGS="-C link-arg=-Wl,-rpath,$ORIGIN/runtime"` 方式注入，
    /// 这样解压后的 Player 无需设置 `LD_LIBRARY_PATH` 即可找到 `runtime/` 下的 `.so`。
    pub fn rpath_link_arg(self) -> Option<&'static str> {
        match self {
            Self::Windows => None,
            Self::Linux => Some("-Wl,-rpath,$ORIGIN/runtime"),
        }
    }

    /// 发布归档后缀（Windows zip / Linux tar.gz，见 PROJECT_PLAN 决策）。
    pub fn archive_extension(self) -> &'static str {
        match self {
            Self::Windows => "zip",
            Self::Linux => "tar.gz",
        }
    }
}

impl fmt::Display for TargetPlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
