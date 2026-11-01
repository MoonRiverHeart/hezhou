//! glTF 导入器桩：关闭 `gltf` feature 时的替代实现。
//!
//! 保持与真实导入器完全相同的公开函数签名，便于数据库/cook 代码无条件编译；
//! 运行时调用会返回 [`AssetError::ImporterDisabled`]，由上层给出可读错误。

use std::path::Path;

use crate::error::{AssetError, AssetResult};
use crate::meta::GltfImportSettings;

use super::ImportedScene;

/// 是否编译了真实 glTF 导入器（桩实现返回 false）。
pub const fn is_enabled() -> bool {
    false
}

/// 从文件路径导入 glTF/GLB（桩：`gltf` feature 未启用）。
pub fn import_gltf_file(
    _path: &Path,
    _settings: &GltfImportSettings,
) -> AssetResult<ImportedScene> {
    Err(AssetError::ImporterDisabled("gltf"))
}

/// 从内存字节导入 glTF/GLB（桩：`gltf` feature 未启用）。
pub fn import_gltf_bytes(
    _bytes: &[u8],
    _base_dir: Option<&Path>,
    _settings: &GltfImportSettings,
) -> AssetResult<ImportedScene> {
    Err(AssetError::ImporterDisabled("gltf"))
}
