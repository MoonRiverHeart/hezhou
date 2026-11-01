//! 资产管线统一错误类型。
//!
//! 所有公开 API 返回 [`AssetResult`]，错误枚举保持可匹配，便于编辑器/Player
//! 在 UI 上给出可读提示。

use crate::id::{AssetId, AssetKind};
use crate::path::LogicalPath;

/// 资产管线结果别名。
pub type AssetResult<T> = Result<T, AssetError>;

/// 资产管线错误。
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    /// 文件系统 I/O 错误（读元数据、扫描目录、写 cook 产物等）。
    #[error("资产 I/O 错误: {0}")]
    Io(#[from] std::io::Error),

    /// `.meta` / cook manifest 等 JSON 序列化或解析失败。
    #[error("资产 JSON 错误: {0}")]
    Json(#[from] serde_json::Error),

    /// 逻辑路径不合法（绝对路径、越界 `..`、非法字符等）。
    #[error("非法逻辑路径: {0}")]
    InvalidPath(String),

    /// 请求的资产 ID 不在数据库中。
    #[error("未知资产 ID: {0}")]
    UnknownAsset(AssetId),

    /// 请求的逻辑路径不在数据库中。
    #[error("未知资产路径: {0}")]
    UnknownPath(LogicalPath),

    /// 期望的资产类型与实际类型不一致（类型化引用校验失败）。
    #[error("资产类型不匹配: 期望 {expected:?}, 实际 {actual:?}")]
    KindMismatch {
        /// 引用声明的类型。
        expected: AssetKind,
        /// 数据库记录的类型。
        actual: AssetKind,
    },

    /// 依赖图成环，cook / 拓扑排序无法进行。
    #[error("资产依赖成环: {0:?}")]
    DependencyCycle(Vec<AssetId>),

    /// 两个逻辑路径在大小写不敏感比较下冲突。
    #[error("资产路径大小写冲突: {0} 与 {1}")]
    DuplicateLocator(String, String),

    /// `.meta` 版本高于当前实现支持的版本。
    #[error("不支持的 meta 版本: {0}（当前支持 {META_VERSION_SUPPORTED}）")]
    UnsupportedMetaVersion(u32),

    /// glTF/GLB 导入失败或导入器能力限制。
    #[error("资产导入失败: {0}")]
    Import(String),

    /// 当前构建未启用对应导入器 feature。
    #[error("导入器未启用: {0}（请启用对应 cargo feature）")]
    ImporterDisabled(&'static str),

    /// 其他资产管线错误。
    #[error("资产错误: {0}")]
    Other(String),
}

/// 当前实现支持的最高 meta 版本（用于错误提示）。
pub(crate) const META_VERSION_SUPPORTED: u32 = 1;
