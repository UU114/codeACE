//! Storage for bullet-based playbook
//!
//! Playbook storage system supporting incremental updates (Delta merging).
//! Uses JSON format to store entire Playbook, supports in-place bullet updates.

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

    /// 提取查询关键词（支持中英文混合）
    ///
    /// 策略：
    /// 1. 先按空格分词（处理英文词和混合词）
    /// 2. 对于每个分词结果，提取连续的中文字符和英文词
    fn extract_keywords(query: &str) -> Vec<String> {
        let mut keywords = Vec::new();

        // 1. 按空格分词
        for word in query.split_whitespace() {
            // 添加完整的词
            keywords.push(word.to_string());

            // 2. 提取中文字符组（2个字符以上的连续中文）
            let mut chinese_chars = String::new();
            for ch in word.chars() {
                if ch.is_ascii_alphabetic() || ch.is_ascii_digit() {
                    // 遇到英文/数字，保存之前的中文词
                    if chinese_chars.len() >= 2 { // 至少2个中文字
                        keywords.push(chinese_chars.clone());
                    }
                    chinese_chars.clear();
                } else if !ch.is_ascii_punctuation() && !ch.is_whitespace() {
                    // 中文或其他非ASCII字符
                    chinese_chars.push(ch);
                }
            }
            // 保存最后的中文词
            if chinese_chars.len() >= 2 {
                keywords.push(chinese_chars);
            }
        }

        // 去重并转小写
        keywords.sort();
        keywords.dedup();
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
    /// Uses simple keyword matching (MVP), sorted by relevance score.
    pub async fn query_bullets(&self, query: &str, max_results: usize) -> Result<Vec<Bullet>> {
        let playbook = self.load_playbook().await?;
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        // 提取查询关键词（支持中英文混合）
        let keywords = Self::extract_keywords(&query_lower);

        // Simple keyword matching (MVP)
        for bullets in playbook.bullets.values() {
            for bullet in bullets {
                let content_lower = bullet.content.to_lowercase();
                let tags_str = bullet.tags.join(" ").to_lowercase();

                // Calculate relevance score (only score basic matches)
                let mut score = 0;

                // Content match - 完整查询字符串匹配
                if content_lower.contains(&query_lower) {
                    score += 3;
                }

                // Content match - 按提取的关键词匹配
                for keyword in &keywords {
                    if content_lower.contains(keyword) {
                        score += 2;
                    }
                }

                // Tag match
                for keyword in &keywords {
                    if tags_str.contains(keyword) {
                        score += 2;
                    }
                }

                // Tool match
                for tool in &bullet.metadata.related_tools {
                    if query_lower.contains(&tool.to_lowercase()) {
                        score += 2;
                    }
                }

                // Only apply importance and success rate weighting when content/tags/tools match
                if score > 0 {
                    // Importance weighting
                    score += (bullet.metadata.importance * 10.0) as i32;

                    // Success rate weighting
                    let success_rate = bullet.success_rate();
                    if success_rate > 0.7 {
                        score += 2;
                    }

                    results.push((bullet.clone(), score));
                }
            }
        }

        // Sort by score
        results.sort_by(|a, b| b.1.cmp(&a.1));

        // Return top N
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
