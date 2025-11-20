//! 轻量级索引系统 - LAPS 系统的高效检索组件
//!
//! 提供基于内存的快速索引和搜索功能，无需外部数据库。

use crate::ace::similarity::SimilarityCalculator;
use crate::ace::types::{Bullet, BulletSection, Playbook};
use lru::LruCache;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::num::NonZeroUsize;
use std::sync::Arc;

/// 轻量级索引
///
/// 使用纯内存结构实现快速的 bullet 检索，包括：
/// - ID 索引（O(1) 查找）
/// - 分类索引（O(log n) 范围查询）
/// - 关键词倒排索引
/// - LRU 热度缓存
pub struct LightweightIndex {
    /// 主索引：ID -> Bullet（O(1) 查找）
    by_id: HashMap<String, Arc<Bullet>>,

    /// 分类索引：Section -> Bullet IDs（O(log n) 范围查询）
    by_section: BTreeMap<BulletSection, Vec<String>>,

    /// 关键词倒排索引：Keyword -> Bullet IDs
    keywords: HashMap<String, HashSet<String>>,

    /// 热度缓存（LRU，最多缓存100个）
    hot_cache: LruCache<String, Arc<Bullet>>,
}

impl LightweightIndex {
    /// 创建新的空索引
    pub fn new() -> Self {
        Self {
            by_id: HashMap::new(),
            by_section: BTreeMap::new(),
            keywords: HashMap::new(),
            hot_cache: LruCache::new(NonZeroUsize::new(100).unwrap()),
        }
    }

    /// 获取索引中的 bullet 数量
    pub fn size(&self) -> usize {
        self.by_id.len()
    }

    /// 从 Playbook 构建索引
    ///
    /// # 参数
    /// - `playbook`: 要索引的 playbook
    ///
    /// # 返回
    /// 构建好的索引
    pub fn build_from_playbook(playbook: &Playbook) -> Self {
        let mut index = Self::new();

        let all_bullets = playbook.all_bullets();
        tracing::debug!("开始构建索引，共 {} 个 bullets", all_bullets.len());

        for bullet in all_bullets {
            let bullet_arc = Arc::new(bullet.clone());

            // 1. ID 索引
            index.by_id.insert(bullet.id.clone(), bullet_arc.clone());

            // 2. 分类索引
            index
                .by_section
                .entry(bullet.section.clone())
                .or_default()
                .push(bullet.id.clone());

            // 3. 关键词索引
            let keywords = Self::extract_keywords(&bullet.content);
            for keyword in keywords {
                index
                    .keywords
                    .entry(keyword)
                    .or_default()
                    .insert(bullet.id.clone());
            }
        }

        tracing::info!(
            "索引构建完成: {} bullets, {} 分类, {} 关键词",
            index.by_id.len(),
            index.by_section.len(),
            index.keywords.len()
        );

        index
    }

    /// 提取关键词（简单分词）
    ///
    /// 从文本中提取有意义的关键词，用于倒排索引。
    ///
    /// # 参数
    /// - `content`: 待提取的文本
    ///
    /// # 返回
    /// 关键词列表
    fn extract_keywords(content: &str) -> Vec<String> {
        content
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() >= 3) // 至少3个字符
            .map(std::string::ToString::to_string)
            .collect()
    }

    /// 计算文本相似度（使用高级相似度算法）
    ///
    /// 使用组合相似度算法，结合：
    /// - Levenshtein 编辑距离
    /// - 2-gram 相似度
    /// - 3-gram 相似度
    ///
    /// # 参数
    /// - `query`: 查询文本
    /// - `content`: 目标文本
    ///
    /// # 返回
    /// 相似度分数（0.0 - 1.0）
    fn text_similarity(query: &str, content: &str) -> f32 {
        // 先进行文本归一化，提高匹配准确性
        let normalized_query = SimilarityCalculator::normalize_text(query, false);
        let normalized_content = SimilarityCalculator::normalize_text(content, false);

        // 使用组合相似度算法
        SimilarityCalculator::combined_similarity(&normalized_query, &normalized_content)
    }

    /// 搜索 bullets
    ///
    /// 基于查询文本搜索相关的 bullets，结合：
    /// - 文本相似度
    /// - 动态权重
    ///
    /// # 参数
    /// - `query`: 查询文本
    /// - `limit`: 返回的最大数量
    ///
    /// # 返回
    /// 排序后的 bullets 列表
    pub fn search(&mut self, query: &str, limit: usize) -> Vec<Arc<Bullet>> {
        // 1. 提取查询关键词
        let query_keywords = Self::extract_keywords(query);
        if query_keywords.is_empty() {
            tracing::warn!("查询关键词为空，返回空结果");
            return Vec::new();
        }

        let mut candidates = HashSet::new();

        // 2. 从倒排索引获取候选
        for keyword in &query_keywords {
            if let Some(bullet_ids) = self.keywords.get(keyword) {
                candidates.extend(bullet_ids.clone());
            }
        }

        if candidates.is_empty() {
            tracing::debug!("未找到匹配的 bullets");
            return Vec::new();
        }

        tracing::debug!("找到 {} 个候选 bullets", candidates.len());

        // 3. 计算相关性分数
        let mut scored_results: Vec<(Arc<Bullet>, f32)> = candidates
            .iter()
            .filter_map(|id| {
                // 先检查缓存
                if let Some(cached) = self.hot_cache.get(id) {
                    return Some(cached.clone());
                }

                // 从主索引获取
                self.by_id.get(id).cloned()
            })
            .map(|bullet| {
                // 计算文本相似度分数
                let text_score = Self::text_similarity(query, &bullet.content);

                // 计算动态权重分数（归一化到 0-1）
                let weight_score = bullet.metadata.calculate_dynamic_weight();
                let normalized_weight = (weight_score / 5.0).min(1.0); // 假设最大权重为 5

                // 综合分数：文本相似度占60%，权重占40%
                let final_score = text_score * 0.6 + normalized_weight * 0.4;

                (bullet, final_score)
            })
            .collect();

        // 4. 排序（降序）
        scored_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 5. 更新缓存并返回
        let results: Vec<Arc<Bullet>> = scored_results
            .into_iter()
            .take(limit)
            .map(|(bullet, _score)| {
                // 添加到热缓存
                self.hot_cache.put(bullet.id.clone(), bullet.clone());
                bullet
            })
            .collect();

        tracing::debug!("返回 {} 个搜索结果", results.len());
        results
    }

    /// 按分类获取 bullets
    ///
    /// # 参数
    /// - `section`: bullet 分类
    ///
    /// # 返回
    /// 该分类下的所有 bullets
    pub fn get_by_section(&self, section: &BulletSection) -> Vec<Arc<Bullet>> {
        self.by_section
            .get(section)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.by_id.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 按 ID 获取 bullet
    ///
    /// # 参数
    /// - `id`: bullet ID
    ///
    /// # 返回
    /// Bullet（如果存在）
    pub fn get_by_id(&mut self, id: &str) -> Option<Arc<Bullet>> {
        // 先检查缓存
        if let Some(cached) = self.hot_cache.get(id) {
            return Some(cached.clone());
        }

        // 从主索引获取
        if let Some(bullet) = self.by_id.get(id).cloned() {
            // 添加到缓存
            self.hot_cache.put(id.to_string(), bullet.clone());
            Some(bullet)
        } else {
            None
        }
    }

    /// 增量添加 bullet
    ///
    /// # 参数
    /// - `bullet`: 要添加的 bullet
    pub fn add_bullet(&mut self, bullet: Bullet) {
        let bullet_id = bullet.id.clone();
        let bullet_arc = Arc::new(bullet.clone());

        // 更新各个索引
        self.by_id
            .insert(bullet_id.clone(), Arc::clone(&bullet_arc));

        self.by_section
            .entry(bullet.section.clone())
            .or_default()
            .push(bullet_id.clone());

        let keywords = Self::extract_keywords(&bullet.content);
        for keyword in keywords {
            self.keywords
                .entry(keyword)
                .or_default()
                .insert(bullet_id.clone());
        }

        tracing::debug!("添加 bullet {bullet_id} 到索引");
    }

    /// 删除 bullet
    ///
    /// # 参数
    /// - `bullet_id`: 要删除的 bullet ID
    pub fn remove_bullet(&mut self, bullet_id: &str) {
        if let Some(bullet) = self.by_id.remove(bullet_id) {
            // 从分类索引移除
            if let Some(ids) = self.by_section.get_mut(&bullet.section) {
                ids.retain(|id| id != bullet_id);
            }

            // 从关键词索引移除
            let keywords = Self::extract_keywords(&bullet.content);
            for keyword in keywords {
                if let Some(ids) = self.keywords.get_mut(&keyword) {
                    ids.remove(bullet_id);
                    // 如果该关键词不再关联任何 bullet，删除该关键词
                    if ids.is_empty() {
                        self.keywords.remove(&keyword);
                    }
                }
            }

            // 从缓存移除
            self.hot_cache.pop(bullet_id);

            tracing::debug!("从索引移除 bullet {}", bullet_id);
        }
    }

    /// 获取索引统计信息
    pub fn statistics(&self) -> IndexStatistics {
        IndexStatistics {
            total_bullets: self.by_id.len(),
            total_sections: self.by_section.len(),
            total_keywords: self.keywords.len(),
            cache_size: self.hot_cache.len(),
        }
    }
}

impl Default for LightweightIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// 索引统计信息
#[derive(Debug, Clone)]
pub struct IndexStatistics {
    /// 总 bullet 数
    pub total_bullets: usize,
    /// 总分类数
    pub total_sections: usize,
    /// 总关键词数
    pub total_keywords: usize,
    /// 缓存大小
    pub cache_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::types::{Bullet, BulletSection, Playbook};

    fn create_test_playbook() -> Playbook {
        let mut playbook = Playbook::new();

        for i in 0..10 {
            let content = match i {
                0 => "使用 Rust 的 async await 处理异步操作".to_string(),
                1 => "Python 的列表推导式简化代码".to_string(),
                2 => "JavaScript Promise 和 async函数".to_string(),
                3 => "Rust 错误处理使用 Result 类型".to_string(),
                4 => "使用 cargo test 运行测试".to_string(),
                5 => "Git commit 提交代码变更".to_string(),
                6 => "Docker 容器化部署应用".to_string(),
                7 => "Rust lifetime 生命周期管理".to_string(),
                8 => "API 接口设计最佳实践".to_string(),
                9 => "数据库事务处理方法".to_string(),
                _ => format!("Test bullet {}", i),
            };

            let section = match i % 3 {
                0 => BulletSection::StrategiesAndRules,
                1 => BulletSection::CodeSnippetsAndTemplates,
                _ => BulletSection::ToolUsageTips,
            };

            let bullet = Bullet::new(section, content, "test-session".to_string());
            playbook.add_bullet(bullet);
        }

        playbook
    }

    #[test]
    fn test_index_creation() {
        let playbook = create_test_playbook();
        let index = LightweightIndex::build_from_playbook(&playbook);

        assert_eq!(index.by_id.len(), 10);
        assert!(index.keywords.len() > 0);
    }

    #[test]
    fn test_keyword_extraction() {
        let content = "使用 Rust 的 async/await 处理异步操作";
        let keywords = LightweightIndex::extract_keywords(content);

        assert!(keywords.contains(&"rust".to_string()));
        assert!(keywords.contains(&"async".to_string()));
        assert!(keywords.contains(&"await".to_string()));
    }

    #[test]
    fn test_text_similarity() {
        let query = "rust async";
        let content1 = "使用 Rust 的 async/await 处理异步操作";
        let content2 = "Python 的列表推导式简化代码";

        let score1 = LightweightIndex::text_similarity(query, content1);
        let score2 = LightweightIndex::text_similarity(query, content2);

        assert!(score1 > score2);
        assert!(score1 > 0.0);
    }

    #[test]
    fn test_search() {
        let playbook = create_test_playbook();
        let mut index = LightweightIndex::build_from_playbook(&playbook);

        let results = index.search("Rust async", 5);

        assert!(!results.is_empty());
        assert!(results.len() <= 5);

        // 第一个结果应该是最相关的
        assert!(
            results[0].content.to_lowercase().contains("rust")
                || results[0].content.to_lowercase().contains("async")
        );
    }

    #[test]
    fn test_search_empty_query() {
        let playbook = create_test_playbook();
        let mut index = LightweightIndex::build_from_playbook(&playbook);

        let results = index.search("", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_no_match() {
        let playbook = create_test_playbook();
        let mut index = LightweightIndex::build_from_playbook(&playbook);

        let results = index.search("xyz123notfound", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_get_by_section() {
        let playbook = create_test_playbook();
        let index = LightweightIndex::build_from_playbook(&playbook);

        let results = index.get_by_section(&BulletSection::StrategiesAndRules);
        assert!(!results.is_empty());

        for bullet in results {
            assert_eq!(bullet.section, BulletSection::StrategiesAndRules);
        }
    }

    #[test]
    fn test_get_by_id() {
        let playbook = create_test_playbook();
        let mut index = LightweightIndex::build_from_playbook(&playbook);

        let all_bullets = playbook.all_bullets();
        let bullet_id = &all_bullets[0].id;

        let result = index.get_by_id(bullet_id);
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, *bullet_id);

        // 再次获取，应该从缓存读取
        let result2 = index.get_by_id(bullet_id);
        assert!(result2.is_some());
    }

    #[test]
    fn test_add_bullet() {
        let playbook = create_test_playbook();
        let mut index = LightweightIndex::build_from_playbook(&playbook);

        let initial_count = index.by_id.len();

        let new_bullet = Bullet::new(
            BulletSection::General,
            "新增的测试 bullet".to_string(),
            "test-session".to_string(),
        );
        let new_id = new_bullet.id.clone();

        index.add_bullet(new_bullet);

        assert_eq!(index.by_id.len(), initial_count + 1);
        assert!(index.get_by_id(&new_id).is_some());
    }

    #[test]
    fn test_remove_bullet() {
        let playbook = create_test_playbook();
        let mut index = LightweightIndex::build_from_playbook(&playbook);

        let all_bullets = playbook.all_bullets();
        let bullet_id = all_bullets[0].id.clone();

        let initial_count = index.by_id.len();

        index.remove_bullet(&bullet_id);

        assert_eq!(index.by_id.len(), initial_count - 1);
        assert!(index.get_by_id(&bullet_id).is_none());
    }

    #[test]
    fn test_statistics() {
        let playbook = create_test_playbook();
        let index = LightweightIndex::build_from_playbook(&playbook);

        let stats = index.statistics();

        assert_eq!(stats.total_bullets, 10);
        assert!(stats.total_keywords > 0);
        assert!(stats.total_sections > 0);
    }
}
