//! 文件日志。
//!
//! Windows Player 以 GUI 子系统启动（无控制台），所有诊断信息必须落文件；
//! Linux 同样统一写文件，避免终端输出依赖。
//!
//! 日志目录优先级：命令行 `--log-dir` > 用户数据目录（`%LOCALAPPDATA%` /
//! `$XDG_DATA_HOME`）> 包内 `logs/` 兜底。时间戳使用 UTC，避免跨时区构建差异。

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Player 运行时日志文件名（追加模式）。
pub const PLAYER_LOG_FILE: &str = "player.log";
/// 启动失败日志文件名（每次失败覆盖）。
pub const STARTUP_ERROR_LOG: &str = "startup-error.log";

/// 日志级别。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

/// 追加式文件日志。
pub struct FileLogger {
    file: File,
    path: PathBuf,
}

impl FileLogger {
    /// 在指定目录创建/追加 `player.log`。
    pub fn create_in(dir: &Path) -> std::io::Result<Self> {
        fs::create_dir_all(dir)?;
        let path = dir.join(PLAYER_LOG_FILE);
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self { file, path })
    }

    /// 日志文件绝对路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 写一行日志（含 UTC 时间戳与级别）。
    pub fn log(&mut self, level: LogLevel, message: &str) -> std::io::Result<()> {
        let line = format!(
            "[{}] {:<5} {}\n",
            format_timestamp_utc(SystemTime::now()),
            level.as_str(),
            message
        );
        self.file.write_all(line.as_bytes())
    }

    pub fn info(&mut self, message: &str) -> std::io::Result<()> {
        self.log(LogLevel::Info, message)
    }

    pub fn warn(&mut self, message: &str) -> std::io::Result<()> {
        self.log(LogLevel::Warn, message)
    }

    pub fn error(&mut self, message: &str) -> std::io::Result<()> {
        self.log(LogLevel::Error, message)
    }
}

/// 把时间格式化为 `YYYY-MM-DDTHH:MM:SS.mmmZ`（UTC，无第三方依赖）。
pub fn format_timestamp_utc(time: SystemTime) -> String {
    let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let total_secs = duration.as_secs() as i64;
    let millis = duration.subsec_millis();
    let days = total_secs.div_euclid(86_400);
    let secs_of_day = total_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let second = secs_of_day % 60;
    format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z"
    )
}

/// 由 Unix 纪元天数换算公历年月日（Howard Hinnant civil_from_days 算法）。
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097; // [0, 146096]
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365; // [0, 399]
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    let year = if month <= 2 { year + 1 } else { year };
    (year, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn epoch_timestamp_is_iso_utc() {
        assert_eq!(
            format_timestamp_utc(UNIX_EPOCH),
            "1970-01-01T00:00:00.000Z"
        );
    }

    #[test]
    fn timestamp_handles_leap_day() {
        let time = UNIX_EPOCH + Duration::from_secs(1_709_164_800); // 2024-02-29T00:00:00Z
        assert_eq!(format_timestamp_utc(time), "2024-02-29T00:00:00.000Z");
    }
}
