//! Storage for bullet-based playbook
//!
//! Playbook storage system supporting incremental updates (Delta merging).
//! Uses JSON format to store entire Playbook, supports in-place bullet updates.

use super::similarity::SimilarityCalculator;
use super::types::Bullet;
use super::types::BulletSection;
use super::types::DeltaContext;
use super::types::Playbook;
use anyhow::Context;
use anyhow::Result;
use chrono::Utc;
use std::path::Path;
use std::path::PathBuf;
use tokio::fs;

/// Bullet-based Storage
///
/// Responsible for Playbook persistence, loading and incremental updates.
pub struct BulletStorage {
    /// Playbook file path
    playbook_path: PathBuf,

    /// Archive directory
    archive_dir: PathBuf,

    /// Maximum number of bullets
    max_bullets: usize,
}

/// 英文停用词表（高频无意义词）
/// 这些词在搜索时会被过滤，以提高匹配精确度
const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "is", "are", "was", "were", "be", "been", "being", "to", "of", "in", "for",
    "on", "with", "at", "by", "from", "as", "it", "its", "this", "that", "these", "those", "which",
    "what", "who", "how", "when", "where", "why", "all", "each", "every", "both", "few", "and",
    "or", "but", "not", "no", "nor", "so", "yet", "if", "then", "else", "can", "could", "will",
    "would", "shall", "should", "may", "might", "must", "do", "does", "did", "done", "doing",
    "has", "have", "had", "having", "am", "your", "my", "our", "his", "her", "their", "me", "you",
    "we", "he", "she",
];

/// 简单英文词干提取（无外部依赖）
///
/// 处理常见的英文单词后缀，提高召回率。
/// 例如: "testing" -> "test", "users" -> "user"
fn simple_stem(word: &str) -> Option<String> {
    let word = word.to_lowercase();
    let len = word.len();

    // 处理 -ing 后缀（如 testing -> test, running -> run）
    if word.ends_with("ing") && len > 5 {
        let stem = &word[..len - 3];
        // 处理双写辅音（如 running -> run）
        if stem.len() >= 2 {
            let chars: Vec<char> = stem.chars().collect();
            let last = chars[chars.len() - 1];
            let second_last = chars[chars.len() - 2];
            if last == second_last && !matches!(last, 'a' | 'e' | 'i' | 'o' | 'u') {
                return Some(stem[..stem.len() - 1].to_string());
            }
        }
        return Some(stem.to_string());
    }

    // 处理 -ed 后缀（如 tested -> test, created -> create）
    if word.ends_with("ed") && len > 4 {
        let stem = &word[..len - 2];
        // 如果以 e 结尾的动词（如 created -> create）
        if !stem.ends_with('e') {
            return Some(stem.to_string());
        }
        // 否则可能是 tested -> test
        return Some(stem.to_string());
    }

    // 处理 -es 后缀（如 matches -> match, boxes -> box）
    if word.ends_with("es") && len > 4 {
        let without_es = &word[..len - 2];
        // 如果以 ch, sh, x, s, z 结尾，去掉 es
        if without_es.ends_with("ch")
            || without_es.ends_with("sh")
            || without_es.ends_with('x')
            || without_es.ends_with('s')
            || without_es.ends_with('z')
        {
            return Some(without_es.to_string());
        }
        // 否则尝试去掉 s
        return Some(word[..len - 1].to_string());
    }

    // 处理 -s 后缀（如 users -> user, tests -> test）
    if word.ends_with('s') && len > 3 && !word.ends_with("ss") && !word.ends_with("us") {
        return Some(word[..len - 1].to_string());
    }

    // 处理 -ly 后缀（如 quickly -> quick）
    if word.ends_with("ly") && len > 4 {
        return Some(word[..len - 2].to_string());
    }

    // 处理 -er 后缀（如 faster -> fast，但保留 user, never 等）
    if word.ends_with("er") && len > 4 {
        let stem = &word[..len - 2];
        // 只有当去掉 er 后仍是有意义的词时才处理
        // 避免 user -> us, never -> nev 等错误
        if stem
            .chars()
            .last()
            .map(char::is_alphabetic)
            .unwrap_or(false)
        {
            // 只对形容词比较级处理（保守策略）
            if stem.ends_with("fast") || stem.ends_with("slow") || stem.ends_with("quick") {
                return Some(stem.to_string());
            }
        }
    }

    None
}

/// 检查是否是 CJK（中日韩）字符
fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}' |  // CJK Unified Ideographs
        '\u{3400}'..='\u{4DBF}' |  // CJK Extension A
        '\u{20000}'..='\u{2A6DF}' | // CJK Extension B
        '\u{F900}'..='\u{FAFF}'    // CJK Compatibility Ideographs
    )
}

impl BulletStorage {
    /// Create new storage
    pub fn new(base_path: impl AsRef<Path>, max_bullets: usize) -> Result<Self> {
        let base_path = base_path.as_ref();
        let playbook_path = base_path.join("playbook.json");
        let archive_dir = base_path.join("archive");

        // Create directories
        std::fs::create_dir_all(base_path)?;
        std::fs::create_dir_all(&archive_dir)?;

        Ok(Self {
            playbook_path,
            archive_dir,
            max_bullets,
        })
    }

    /// 提取查询关键词（优化版，支持中英文混合）
    ///
    /// 优化策略：
    /// 1. 提取英文单词并过滤停用词
    /// 2. 对英文单词进行词干提取（提高召回率）
    /// 3. 提取中文 2-gram（保守策略，减少噪音）
    /// 4. 添加完整中文字符串用于精确匹配
    fn extract_keywords(query: &str) -> Vec<String> {
        let mut keywords = Vec::new();
        let query_lower = query.to_lowercase();

        // 1. 提取英文单词（按非字母数字分割）
        for word in query_lower.split(|c: char| !c.is_alphanumeric()) {
            if word.is_empty() {
                continue;
            }

            // 检查是否全是 ASCII（英文单词）
            if word.chars().all(|c| c.is_ascii_alphanumeric()) {
                // 过滤停用词和过短的词
                if word.len() >= 2 && !STOP_WORDS.contains(&word) {
                    keywords.push(word.to_string());

                    // 词干提取（提高召回率，如 "testing" -> "test"）
                    if let Some(stem) = simple_stem(word) {
                        if stem != word && !STOP_WORDS.contains(&stem.as_str()) {
                            keywords.push(stem);
                        }
                    }
                }
            }
        }

        // 2. 提取中文字符（保守策略，减少噪音）
        let chinese_chars: Vec<char> = query_lower.chars().filter(|c| is_cjk(*c)).collect();

        if chinese_chars.len() >= 2 {
            // 只提取 2-gram，不生成 3-gram（减少噪音）
            for i in 0..chinese_chars.len().saturating_sub(1) {
                let bigram: String = chinese_chars[i..=i + 1].iter().collect();
                keywords.push(bigram);
            }

            // 添加完整中文字符串（用于精确匹配）
            let full_chinese: String = chinese_chars.iter().collect();
            keywords.push(full_chinese);
        }

        // 去重
        keywords.sort();
        keywords.dedup();

        tracing::debug!("extract_keywords (optimized): {:?}", keywords);
        keywords
    }

    /// Load playbook
    pub async fn load_playbook(&self) -> Result<Playbook> {
        if !self.playbook_path.exists() {
            return Ok(Playbook::new());
        }

        let content = fs::read_to_string(&self.playbook_path)
            .await
            .context("Failed to read playbook file")?;

        let playbook: Playbook =
            serde_json::from_str(&content).context("Failed to parse playbook JSON")?;

        tracing::debug!(
            "Loaded playbook version {} with {} bullets",
            playbook.version,
            playbook.metadata.total_bullets
        );

        Ok(playbook)
    }

    /// Save playbook
    pub async fn save_playbook(&self, playbook: &Playbook) -> Result<()> {
        let json =
            serde_json::to_string_pretty(playbook).context("Failed to serialize playbook")?;

        fs::write(&self.playbook_path, json)
            .await
            .context("Failed to write playbook file")?;

        tracing::debug!(
            "Saved playbook version {} with {} bullets",
            playbook.version,
            playbook.metadata.total_bullets
        );

        Ok(())
    }

    /// **Core method**: Merge delta (incremental update)
    ///
    /// This is the key method of Bullet-based architecture, supporting:
    /// - Adding new bullets
    /// - Updating existing bullets metadata
    /// - Auto-archiving (when exceeding limit)
    pub async fn merge_delta(&self, delta: DeltaContext) -> Result<()> {
        if delta.is_empty() {
            tracing::debug!("Delta is empty, skipping merge");
            return Ok(());
        }

        // Load existing playbook
        let mut playbook = self.load_playbook().await?;

        tracing::info!(
            "Merging delta: {} new bullets, {} updated bullets",
            delta.new_bullets.len(),
            delta.updated_bullets.len()
        );

        // 1. Add new bullets
        for bullet in delta.new_bullets {
            playbook.add_bullet(bullet);
        }

        // 2. Update existing bullets
        for bullet in delta.updated_bullets {
            if !playbook.update_bullet(bullet) {
                tracing::warn!("Failed to update bullet (not found)");
            }
        }

        // 3. Check if archiving is needed
        if playbook.metadata.total_bullets > self.max_bullets {
            self.auto_archive(&mut playbook).await?;
        }

        // 4. Save
        self.save_playbook(&playbook).await?;

        tracing::info!(
            "Delta merged successfully. Total bullets: {}",
            playbook.metadata.total_bullets
        );

        Ok(())
    }

    /// Query bullets (for context loading)
    ///
    /// 优化版查询，使用简化的 3 层评分策略：
    /// - 层1: 精确匹配（高权重）
    /// - 层2: 模糊匹配（仅当精确匹配不足时）
    /// - 层3: 元数据加成（仅对高质量匹配）
    ///
    /// 同时添加质量惩罚机制，防止噪音累加
    pub async fn query_bullets(&self, query: &str, max_results: usize) -> Result<Vec<Bullet>> {
        let playbook = self.load_playbook().await?;
        let query_lower = query.to_lowercase();
        let query_normalized = SimilarityCalculator::normalize_text(&query_lower, true);
        let mut results = Vec::new();

        // 提取查询关键词（优化版）
        let keywords = Self::extract_keywords(&query_lower);

        // 诊断日志
        tracing::info!(
            "query_bullets: query='{}', keywords={:?}, total_bullets={}",
            query_lower,
            keywords,
            playbook.metadata.total_bullets
        );

        // 提高阈值，减少误匹配
        const FUZZY_THRESHOLD: f32 = 0.5; // 从 0.4 提高到 0.5
        const HIGH_MATCH_THRESHOLD: f32 = 0.7; // 高匹配阈值

        for bullets in playbook.bullets.values() {
            for bullet in bullets {
                let content_lower = bullet.content.to_lowercase();
                let content_normalized = SimilarityCalculator::normalize_text(&content_lower, true);
                let tags_str = bullet.tags.join(" ").to_lowercase();

                let mut score: f32 = 0.0;
                let mut match_count: i32 = 0; // 用于计算匹配质量

                // === 层1: 精确匹配（高权重） ===

                // 完整查询匹配
                if content_lower.contains(&query_lower) {
                    score += 15.0;
                    match_count += 3;
                }

                // 关键词精确匹配
                for keyword in &keywords {
                    // 内容匹配
                    if content_lower.contains(keyword) {
                        let word_score = match keyword.len() {
                            2..=3 => 2.0, // 短词低分（如 "js"）
                            4..=6 => 4.0, // 中等词
                            _ => 5.0,     // 长词高分
                        };
                        score += word_score;
                        match_count += 1;
                    }

                    // 标签匹配（bonus）
                    if tags_str.contains(keyword) {
                        score += 3.0;
                        match_count += 1;
                    }
                }

                // 中文精确匹配
                let content_chinese: String =
                    content_lower.chars().filter(|c| is_cjk(*c)).collect();
                for keyword in &keywords {
                    let is_chinese_keyword = keyword.chars().all(is_cjk);
                    if is_chinese_keyword && content_chinese.contains(keyword) {
                        let keyword_len = keyword.chars().count();
                        score += (keyword_len as f32).min(4.0);
                        match_count += 1;
                    }
                }

                // === 层2: 模糊匹配（仅当精确匹配不足时） ===
                if match_count < 2 {
                    let overall_similarity = SimilarityCalculator::combined_similarity(
                        &query_normalized,
                        &content_normalized,
                    );

                    if overall_similarity > HIGH_MATCH_THRESHOLD {
                        score += overall_similarity * 8.0;
                        match_count += 1;
                    } else if overall_similarity > FUZZY_THRESHOLD {
                        score += overall_similarity * 4.0;
                    }
                }

                // === 层3: 元数据加成（仅对高质量匹配） ===
                if match_count >= 2 {
                    // Importance 权重
                    score += bullet.metadata.importance * 3.0;

                    // 成功率权重
                    let success_rate = bullet.success_rate();
                    if success_rate > 0.7 {
                        score += 2.0;
                    }

                    // 工具匹配（bonus）
                    for tool in &bullet.metadata.related_tools {
                        if query_lower.contains(&tool.to_lowercase()) {
                            score += 3.0;
                        }
                    }

                    // 语言标签匹配（bonus）
                    for keyword in &keywords {
                        for tag in &bullet.tags {
                            let tag_lower = tag.to_lowercase();
                            if let Some(lang) = tag_lower.strip_prefix("lang:") {
                                if lang == *keyword || keyword.contains(lang) {
                                    score += 2.0;
                                }
                            }
                        }
                    }
                }

                // === 质量惩罚机制 ===
                // 如果关键词很多但匹配很少，降低分数
                if !keywords.is_empty() {
                    let match_ratio = match_count as f32 / keywords.len() as f32;
                    if match_ratio < 0.3 && score > 0.0 {
                        score *= 0.5; // 惩罚低质量匹配
                    }
                }

                // 提高最低分数阈值（从 0.5 提高到 2.0）
                if score > 2.0 {
                    results.push((bullet.clone(), score));
                }
            }
        }

        // 按分数降序排序
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        tracing::info!(
            "query_bullets: found {} matches (returning top {})",
            results.len(),
            max_results
        );

        // 返回 top N
        Ok(results
            .into_iter()
            .take(max_results)
            .map(|(bullet, _)| bullet)
            .collect())
    }

    /// Find bullet by ID
    pub async fn find_bullet(&self, id: &str) -> Result<Option<Bullet>> {
        let playbook = self.load_playbook().await?;
        Ok(playbook.find_bullet(id).cloned())
    }

    /// Update bullet (single)
    pub async fn update_bullet(&self, bullet: Bullet) -> Result<bool> {
        let mut playbook = self.load_playbook().await?;
        let updated = playbook.update_bullet(bullet);

        if updated {
            self.save_playbook(&playbook).await?;
        }

        Ok(updated)
    }

    /// Auto-archive old bullets
    ///
    /// When playbook exceeds limit, archive current version and keep latest portion of bullets.
    async fn auto_archive(&self, playbook: &mut Playbook) -> Result<()> {
        tracing::info!(
            "Auto-archiving: {} bullets exceed limit {}",
            playbook.metadata.total_bullets,
            self.max_bullets
        );

        // Generate archive filename
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let archive_path = self.archive_dir.join(format!("playbook_{timestamp}.json"));

        // Save current playbook to archive
        let json = serde_json::to_string_pretty(playbook)?;
        fs::write(&archive_path, json).await?;

        tracing::info!("Archived to: {}", archive_path.display());

        // Clear current playbook (keep recent portion)
        // MVP: Simple truncation strategy
        let keep_ratio = 0.7; // Keep 70%
        let keep_count = (self.max_bullets as f32 * keep_ratio) as usize;

        // Sort by update time, keep latest
        let mut all_bullets: Vec<_> = playbook.bullets.values().flatten().cloned().collect();
        all_bullets.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        // Rebuild playbook
        *playbook = Playbook::new();
        for bullet in all_bullets.into_iter().take(keep_count) {
            playbook.add_bullet(bullet);
        }

        tracing::info!(
            "Archive completed: {} bullets retained",
            playbook.metadata.total_bullets
        );

        Ok(())
    }

    /// Clear playbook (archive)
    pub async fn clear(&self) -> Result<()> {
        // TODO: Implement archiving logic
        // Currently just clear
        let playbook = Playbook::new();
        self.save_playbook(&playbook).await?;
        tracing::info!("Playbook cleared");
        Ok(())
    }

    /// Clear playbook (no archive, direct delete)
    pub async fn clear_without_archive(&self) -> Result<()> {
        let playbook = Playbook::new();
        self.save_playbook(&playbook).await?;
        tracing::warn!("Playbook cleared without archive");
        Ok(())
    }

    /// Get statistics
    pub async fn get_stats(&self) -> Result<StorageStats> {
        let playbook = self.load_playbook().await?;

        let mut tool_usage = std::collections::HashMap::new();
        let mut bullets_by_section = std::collections::HashMap::new();
        let mut total_successes = 0;
        let mut total_attempts = 0;
        let mut sessions = std::collections::HashSet::new();

        for bullet in playbook.all_bullets() {
            // Count tool usage
            for tool in &bullet.metadata.related_tools {
                *tool_usage.entry(tool.clone()).or_insert(0) += 1;
            }

            // Count bullets per section
            *bullets_by_section
                .entry(bullet.section.clone())
                .or_insert(0) += 1;

            // Count success rate
            total_successes += bullet.metadata.success_count;
            total_attempts += bullet.metadata.success_count + bullet.metadata.failure_count;

            // Collect session IDs
            sessions.insert(bullet.source_session_id.clone());
        }

        Ok(StorageStats {
            total_bullets: playbook.metadata.total_bullets,
            total_sessions: sessions.len(),
            playbook_version: playbook.version,
            total_sections: playbook.metadata.section_counts.len(),
            bullets_by_section,
            tool_usage,
            overall_success_rate: if total_attempts > 0 {
                total_successes as f32 / total_attempts as f32
            } else {
                0.0
            },
        })
    }
}

/// Storage statistics
#[derive(Debug)]
pub struct StorageStats {
    pub total_bullets: usize,
    pub total_sessions: usize,
    pub playbook_version: u32,
    pub total_sections: usize,
    pub bullets_by_section: std::collections::HashMap<BulletSection, usize>,
    pub tool_usage: std::collections::HashMap<String, usize>,
    pub overall_success_rate: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::types::BulletSection;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_storage_basic_operations() {
        let temp_dir = tempdir().unwrap();
        let storage = BulletStorage::new(temp_dir.path(), 100).unwrap();

        // Test loading empty playbook
        let playbook = storage.load_playbook().await.unwrap();
        assert_eq!(playbook.metadata.total_bullets, 0);

        // Test save and load
        let mut playbook = Playbook::new();
        let bullet = Bullet::new(
            BulletSection::StrategiesAndRules,
            "Test strategy".to_string(),
            "session-1".to_string(),
        );
        playbook.add_bullet(bullet);

        storage.save_playbook(&playbook).await.unwrap();

        let loaded = storage.load_playbook().await.unwrap();
        assert_eq!(loaded.metadata.total_bullets, 1);
    }

    #[tokio::test]
    async fn test_storage_merge_delta() {
        let temp_dir = tempdir().unwrap();
        let storage = BulletStorage::new(temp_dir.path(), 100).unwrap();

        // Create delta
        let bullet = Bullet::new(
            BulletSection::ToolUsageTips,
            "Test tool usage".to_string(),
            "session-1".to_string(),
        );

        let mut delta = DeltaContext::new("session-1".to_string());
        delta.new_bullets.push(bullet);
        delta.metadata.new_bullets_count = 1;

        // Merge
        storage.merge_delta(delta).await.unwrap();

        // Verify
        let playbook = storage.load_playbook().await.unwrap();
        assert_eq!(playbook.metadata.total_bullets, 1);
    }

    #[tokio::test]
    async fn test_storage_query_bullets() {
        let temp_dir = tempdir().unwrap();
        let storage = BulletStorage::new(temp_dir.path(), 100).unwrap();

        // Add some bullets
        let mut delta = DeltaContext::new("session-1".to_string());

        let mut bullet1 = Bullet::new(
            BulletSection::ToolUsageTips,
            "Use cargo test to run tests".to_string(),
            "session-1".to_string(),
        );
        bullet1.tags = vec!["testing".to_string(), "rust".to_string()];

        let bullet2 = Bullet::new(
            BulletSection::StrategiesAndRules,
            "Run tests before build".to_string(),
            "session-1".to_string(),
        );

        delta.new_bullets.push(bullet1);
        delta.new_bullets.push(bullet2);

        storage.merge_delta(delta).await.unwrap();

        // Query
        let results = storage.query_bullets("test", 10).await.unwrap();
        assert_eq!(results.len(), 2);

        // More specific query
        let results = storage.query_bullets("rust", 10).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_storage_update_bullet() {
        let temp_dir = tempdir().unwrap();
        let storage = BulletStorage::new(temp_dir.path(), 100).unwrap();

        // Add bullet
        let bullet = Bullet::new(
            BulletSection::General,
            "Original content".to_string(),
            "session-1".to_string(),
        );

        let bullet_id = bullet.id.clone();
        let mut delta = DeltaContext::new("session-1".to_string());
        delta.new_bullets.push(bullet);

        storage.merge_delta(delta).await.unwrap();

        // Update bullet
        let mut updated_bullet = storage.find_bullet(&bullet_id).await.unwrap().unwrap();
        updated_bullet.content = "Updated content".to_string();
        updated_bullet.record_success();

        let success = storage.update_bullet(updated_bullet).await.unwrap();
        assert!(success);

        // Verify update
        let loaded = storage.find_bullet(&bullet_id).await.unwrap().unwrap();
        assert_eq!(loaded.content, "Updated content");
        assert_eq!(loaded.metadata.success_count, 1);
    }

    #[tokio::test]
    async fn test_storage_auto_archive() {
        let temp_dir = tempdir().unwrap();
        // Set small limit to trigger archiving
        let storage = BulletStorage::new(temp_dir.path(), 5).unwrap();

        // Add bullets exceeding limit
        for i in 0..10 {
            let bullet = Bullet::new(
                BulletSection::General,
                format!("Bullet {}", i),
                format!("session-{}", i),
            );

            let mut delta = DeltaContext::new(format!("session-{}", i));
            delta.new_bullets.push(bullet);

            storage.merge_delta(delta).await.unwrap();
        }

        // Verify archiving occurred
        let playbook = storage.load_playbook().await.unwrap();
        // Should keep about 70% of limit (3-4 items)
        assert!(playbook.metadata.total_bullets <= 5);
        assert!(playbook.metadata.total_bullets >= 3);

        // Verify archive file exists
        let mut archive_files = Vec::new();
        let mut entries = fs::read_dir(storage.archive_dir).await.unwrap();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            archive_files.push(entry.path());
        }
        assert!(!archive_files.is_empty());
    }

    #[tokio::test]
    async fn test_storage_stats() {
        let temp_dir = tempdir().unwrap();
        let storage = BulletStorage::new(temp_dir.path(), 100).unwrap();

        // Add some bullets
        let mut delta = DeltaContext::new("session-1".to_string());

        let mut bullet1 = Bullet::new(
            BulletSection::ToolUsageTips,
            "Use bash".to_string(),
            "session-1".to_string(),
        );
        bullet1.metadata.related_tools = vec!["bash".to_string()];
        bullet1.record_success();

        let mut bullet2 = Bullet::new(
            BulletSection::General,
            "Use cargo".to_string(),
            "session-1".to_string(),
        );
        bullet2.metadata.related_tools = vec!["cargo".to_string()];
        bullet2.record_success();
        bullet2.record_success();

        delta.new_bullets.push(bullet1);
        delta.new_bullets.push(bullet2);

        storage.merge_delta(delta).await.unwrap();

        // Get statistics
        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.total_bullets, 2);
        assert_eq!(stats.tool_usage.get("bash"), Some(&1));
        assert_eq!(stats.tool_usage.get("cargo"), Some(&1));
        assert_eq!(stats.overall_success_rate, 1.0); // All succeeded
    }

    #[tokio::test]
    async fn test_storage_clear() {
        let temp_dir = tempdir().unwrap();
        let storage = BulletStorage::new(temp_dir.path(), 100).unwrap();

        // Add bullet
        let bullet = Bullet::new(
            BulletSection::General,
            "Test".to_string(),
            "session-1".to_string(),
        );

        let mut delta = DeltaContext::new("session-1".to_string());
        delta.new_bullets.push(bullet);

        storage.merge_delta(delta).await.unwrap();

        // Clear
        storage.clear().await.unwrap();

        // Verify
        let playbook = storage.load_playbook().await.unwrap();
        assert_eq!(playbook.metadata.total_bullets, 0);
    }
}
