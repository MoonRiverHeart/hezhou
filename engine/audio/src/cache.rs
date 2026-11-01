//! 音频资源缓存。
//!
//! 以字符串 id 为键缓存已解码的 [`SoundData`]，避免同一音效重复解码。
//! 引擎层在 [`crate::engine::AudioEngine`] 中持有一个 [`AssetCache`]，
//! 统计命中率用于编辑器/QA 观察。

use std::collections::HashMap;
use std::sync::Arc;

use crate::sound::SoundData;

/// 资源缓存统计快照。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheStats {
    /// 当前缓存资源数。
    pub len: usize,
    /// 当前缓存占用的样本内存（字节，不含 Arc/HashMap 开销）。
    pub bytes: usize,
    /// 命中次数。
    pub hits: u64,
    /// 未命中次数。
    pub misses: u64,
}

/// 已解码音频资源缓存（id -> `Arc<SoundData>`）。
#[derive(Debug, Default)]
pub struct AssetCache {
    entries: HashMap<String, Arc<SoundData>>,
    hits: u64,
    misses: u64,
}

impl AssetCache {
    /// 创建空缓存。
    pub fn new() -> Self {
        Self::default()
    }

    /// 插入（或覆盖）一个资源，返回写入的引用。
    pub fn insert(&mut self, id: impl Into<String>, data: Arc<SoundData>) -> Arc<SoundData> {
        self.entries.insert(id.into(), Arc::clone(&data));
        data
    }

    /// 按 id 查询；命中/未命中都会计入统计。
    pub fn get(&mut self, id: &str) -> Option<Arc<SoundData>> {
        match self.entries.get(id) {
            Some(data) => {
                self.hits += 1;
                Some(Arc::clone(data))
            }
            None => {
                self.misses += 1;
                None
            }
        }
    }

    /// 只看不计数（编辑器展示用）。
    pub fn peek(&self, id: &str) -> Option<&Arc<SoundData>> {
        self.entries.get(id)
    }

    /// 是否已缓存。
    pub fn contains(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }

    /// 移除资源，返回被移除的引用。
    pub fn remove(&mut self, id: &str) -> Option<Arc<SoundData>> {
        self.entries.remove(id)
    }

    /// 清空缓存（命中统计保留，便于对比）。
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 当前缓存资源数量。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 所有已缓存 id（排序后返回，保证可复现）。
    pub fn ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.entries.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// 统计快照。
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            len: self.entries.len(),
            bytes: self.entries.values().map(|data| data.memory_bytes()).sum(),
            hits: self.hits,
            misses: self.misses,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone() -> Arc<SoundData> {
        Arc::new(SoundData::silent("tone", 48000, 1, 64).unwrap())
    }

    #[test]
    fn insert_get_and_stats() {
        let mut cache = AssetCache::new();
        let data = cache.insert("hit", tone());
        assert!(Arc::ptr_eq(&data, cache.peek("hit").unwrap()));
        assert!(cache.contains("hit"));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.ids(), vec!["hit".to_string()]);

        assert!(cache.get("hit").is_some());
        assert!(cache.get("miss").is_none());
        assert!(cache.get("miss").is_none());
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 2);
        assert_eq!(stats.len, 1);
        assert_eq!(stats.bytes, 64 * std::mem::size_of::<f32>());
    }

    #[test]
    fn remove_and_clear() {
        let mut cache = AssetCache::new();
        cache.insert("a", tone());
        cache.insert("b", tone());
        assert!(cache.remove("a").is_some());
        assert!(cache.remove("a").is_none());
        assert_eq!(cache.len(), 1);
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.stats().len, 0);
    }
}
