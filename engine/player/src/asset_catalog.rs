//! Player 运行时资产目录（task-35）：从 [`PlayerAssetIndex`] 提取 `AssetId/kind/output`
//! 供 C# `PlayerCSharpContext` v4 的资产枚举 API 使用。
//!
//! 仅做只读快照；资产字节加载仍由各子系统（audio/imported 等）按需处理。

use hezhou_asset::{AssetId, AssetKind};

use crate::asset_index::PlayerAssetIndex;

/// 运行时资产目录条目。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerAssetEntry {
    /// cook manifest 的稳定 AssetId。
    pub id: AssetId,
    /// 资产类型。
    pub kind: AssetKind,
    /// 包内相对 `assets/` 的输出路径。
    pub output: String,
}

/// 运行时资产目录（只读）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlayerAssetCatalog {
    entries: Vec<PlayerAssetEntry>,
}

impl PlayerAssetCatalog {
    /// 从已加载的资产索引构建（按 AssetId 排序，包含 glTF 子资产）。
    pub fn from_index(index: &PlayerAssetIndex) -> Self {
        let mut entries: Vec<PlayerAssetEntry> = index
            .entries()
            .map(|entry| PlayerAssetEntry {
                id: entry.id,
                kind: entry.kind,
                output: entry.output.clone(),
            })
            .collect();
        entries.sort_by_key(|entry| entry.id.to_string());
        Self { entries }
    }

    /// 直接从条目构建（测试/嵌入场景）。
    pub fn from_entries(entries: Vec<PlayerAssetEntry>) -> Self {
        Self { entries }
    }

    /// 资产数量。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 全部条目。
    pub fn entries(&self) -> &[PlayerAssetEntry] {
        &self.entries
    }

    /// 按 index 查询条目。
    pub fn entry(&self, index: usize) -> Option<&PlayerAssetEntry> {
        self.entries.get(index)
    }

    /// 第一个指定 kind 的条目。
    pub fn find_first(&self, kind: AssetKind) -> Option<&PlayerAssetEntry> {
        self.entries.iter().find(|entry| entry.kind == kind)
    }

    /// 单行摘要。
    pub fn summary(&self) -> String {
        if self.entries.is_empty() {
            return "资产目录：0 条".to_string();
        }
        format!(
            "资产目录：{} 条（首个 {} / {}）",
            self.entries.len(),
            self.entries[0].kind.as_str(),
            self.entries[0].output
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> AssetId {
        AssetId::parse(text).expect("AssetId")
    }

    #[test]
    fn catalog_from_entries_is_indexable() {
        let catalog = PlayerAssetCatalog::from_entries(vec![
            PlayerAssetEntry {
                id: id("11111111-1111-4111-8111-111111111111"),
                kind: AssetKind::Scene,
                output: "assets/main.scene".to_string(),
            },
            PlayerAssetEntry {
                id: id("22222222-2222-4222-8222-222222222222"),
                kind: AssetKind::Audio,
                output: "assets/audio/test.wav".to_string(),
            },
        ]);
        assert_eq!(catalog.len(), 2);
        assert!(!catalog.is_empty());
        assert_eq!(catalog.entry(1).map(|e| e.kind), Some(AssetKind::Audio));
        assert_eq!(
            catalog.find_first(AssetKind::Audio).map(|e| e.output.as_str()),
            Some("assets/audio/test.wav")
        );
        assert_eq!(catalog.find_first(AssetKind::Texture), None);
        assert!(catalog.summary().contains("2 条"));
    }

    #[test]
    fn empty_catalog_is_safe() {
        let catalog = PlayerAssetCatalog::default();
        assert_eq!(catalog.len(), 0);
        assert!(catalog.is_empty());
        assert!(catalog.entry(0).is_none());
        assert!(catalog.summary().contains("0 条"));
    }
}
