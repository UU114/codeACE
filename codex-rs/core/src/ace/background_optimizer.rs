// 后台优化系统 - 无感知智能优化 Playbook
use crate::ace::similarity::SimilarityCalculator;
use crate::ace::storage::BulletStorage;
use crate::ace::types::Bullet;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};

/// 后台优化器配置
#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    /// 优化间隔(秒)
    pub interval_secs: u64,
    /// 是否启用去重
    pub dedup_enabled: bool,
    /// 是否启用清理
    pub cleanup_enabled: bool,
    /// 每 N 次调用触发优化
    pub trigger_every_n_calls: u64,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            interval_secs: 300, // 5分钟
            dedup_enabled: true,
            cleanup_enabled: true,
            trigger_every_n_calls: 100,
        }
    }
}

/// 后台优化器
pub struct BackgroundOptimizer {
    storage: Arc<RwLock<BulletStorage>>,
    call_count: Arc<AtomicU64>,
    config: OptimizerConfig,
}

impl BackgroundOptimizer {
    /// 创建新的后台优化器
    pub fn new(storage: Arc<RwLock<BulletStorage>>, config: OptimizerConfig) -> Self {
        Self {
            storage,
            call_count: Arc::new(AtomicU64::new(0)),
            config,
        }
    }

    /// 启动后台优化任务
    pub fn start(self: Arc<Self>) {
        let optimizer = Arc::clone(&self);
        tokio::spawn(async move {
            loop {
                // 等待指定间隔
                sleep(Duration::from_secs(optimizer.config.interval_secs)).await;

                if let Err(e) = optimizer.optimize().await {
                    tracing::error!("后台优化失败: {}", e);
                }
            }
        });
    }

    /// 记录调用并可能触发优化
    pub async fn record_call(&self, _used_bullet_ids: Vec<String>, _success: bool) {
        let count = self.call_count.fetch_add(1, Ordering::Relaxed) + 1;

        // 每 N 次调用触发优化
        if count.is_multiple_of(self.config.trigger_every_n_calls) {
            tracing::info!("达到 {} 次调用，触发优化", count);

            let storage = Arc::clone(&self.storage);
            let config = self.config.clone();

            tokio::spawn(async move {
                let optimizer = BackgroundOptimizer {
                    storage,
                    call_count: Arc::new(AtomicU64::new(0)),
                    config,
                };

                if let Err(e) = optimizer.optimize().await {
                    tracing::error!("触发优化失败: {}", e);
                }
            });
        }
    }

    /// 执行优化
    pub async fn optimize(&self) -> Result<()> {
        tracing::info!("开始后台优化...");
        let start = std::time::Instant::now();

        // 1. 去重
        let dedup_count = if self.config.dedup_enabled {
            self.deduplicate_similar().await?
        } else {
            0
        };

        // 2. 重算权重
        self.recalculate_weights().await?;

        // 3. 清理低价值内容
        let cleaned_count = if self.config.cleanup_enabled {
            self.cleanup_low_value().await?
        } else {
            0
        };

        tracing::info!(
            "后台优化完成: 去重 {} 条, 清理 {} 条, 耗时 {:?}",
            dedup_count,
            cleaned_count,
            start.elapsed()
        );

        Ok(())
    }

    /// 相似内容去重（使用高级相似度算法）
    ///
    /// 使用组合相似度算法检测重复内容，相比简单哈希更准确。
    async fn deduplicate_similar(&self) -> Result<usize> {
        let storage = self.storage.write().await;
        let mut playbook = storage.load_playbook().await?;
        let all_bullets = playbook.all_bullets();

        if all_bullets.is_empty() {
            return Ok(0);
        }

        tracing::debug!("开始去重检查，共 {} 个 bullets", all_bullets.len());

        let mut to_remove = HashSet::new();
        let similarity_threshold = 0.85; // 相似度阈值

        // 归一化所有内容
        let normalized_contents: Vec<_> = all_bullets
            .iter()
            .map(|b| SimilarityCalculator::normalize_text(&b.content, true))
            .collect();

        // 比较每对 bullet
        for i in 0..all_bullets.len() {
            if to_remove.contains(&all_bullets[i].id) {
                continue; // 已经被标记删除，跳过
            }

            for j in (i + 1)..all_bullets.len() {
                if to_remove.contains(&all_bullets[j].id) {
                    continue; // 已经被标记删除，跳过
                }

                // 计算相似度
                let similarity = SimilarityCalculator::combined_similarity(
                    &normalized_contents[i],
                    &normalized_contents[j],
                );

                // 如果相似度高于阈值，认为是重复
                if similarity >= similarity_threshold {
                    // 比较权重，删除权重较低的
                    let weight_i = all_bullets[i].metadata.calculate_dynamic_weight();
                    let weight_j = all_bullets[j].metadata.calculate_dynamic_weight();

                    let (to_keep, to_delete) = if weight_i >= weight_j {
                        (&all_bullets[i], &all_bullets[j])
                    } else {
                        (&all_bullets[j], &all_bullets[i])
                    };

                    to_remove.insert(to_delete.id.clone());

                    tracing::debug!(
                        "发现相似 bullets (相似度: {:.2}): 保留 '{}', 删除 '{}'",
                        similarity,
                        to_keep.content.chars().take(30).collect::<String>(),
                        to_delete.content.chars().take(30).collect::<String>()
                    );
                }
            }
        }

        // 执行删除
        let removed_count = to_remove.len();
        for id in &to_remove {
            playbook.remove_bullet(id);
        }

        if removed_count > 0 {
            storage.save_playbook(&playbook).await?;
            tracing::info!("去重完成，删除了 {} 个相似 bullets", removed_count);
        } else {
            tracing::debug!("未发现需要去重的 bullets");
        }

        Ok(removed_count)
    }

    /// 计算内容哈希（归一化后）
    fn calculate_content_hash(content: &str) -> u64 {
        // 归一化：转小写，只保留字母数字
        let normalized = content
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>();

        let mut hasher = DefaultHasher::new();
        normalized.hash(&mut hasher);
        hasher.finish()
    }

    /// 重新计算所有权重
    async fn recalculate_weights(&self) -> Result<()> {
        let storage = self.storage.read().await;
        let playbook = storage.load_playbook().await?;

        // 收集所有权重
        let mut weight_stats = Vec::new();

        for bullet in playbook.all_bullets() {
            let dynamic_weight = bullet.metadata.calculate_dynamic_weight();
            weight_stats.push((bullet.id.clone(), dynamic_weight));

            tracing::trace!(
                "Bullet {} 权重: {:.3} (recall: {}, success_rate: {:.2}%)",
                bullet.id,
                dynamic_weight,
                bullet.metadata.recall_count,
                bullet.metadata.success_rate * 100.0
            );
        }

        // 按权重排序，找出 top 和 bottom
        weight_stats.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if !weight_stats.is_empty() {
            tracing::debug!(
                "权重统计: 最高 {:.3}, 最低 {:.3}, 中位 {:.3}",
                weight_stats.first().map(|w| w.1).unwrap_or(0.0),
                weight_stats.last().map(|w| w.1).unwrap_or(0.0),
                weight_stats
                    .get(weight_stats.len() / 2)
                    .map(|w| w.1)
                    .unwrap_or(0.0)
            );
        }

        Ok(())
    }

    /// 清理低价值内容
    async fn cleanup_low_value(&self) -> Result<usize> {
        let storage = self.storage.write().await;
        let mut playbook = storage.load_playbook().await?;
        let all_bullets = playbook.all_bullets();

        let mut to_remove = Vec::new();

        for bullet in all_bullets {
            if self.should_remove(bullet) {
                to_remove.push(bullet.id.clone());

                tracing::debug!(
                    "标记删除低价值 bullet: {} (recall: {}, success_rate: {:.0}%, age: {} days)",
                    bullet.id,
                    bullet.metadata.recall_count,
                    bullet.metadata.success_rate * 100.0,
                    (Utc::now() - bullet.created_at).num_days()
                );
            }
        }

        // 执行删除
        let removed_count = to_remove.len();
        for id in &to_remove {
            playbook.remove_bullet(id);
        }

        if removed_count > 0 {
            storage.save_playbook(&playbook).await?;
        }

        Ok(removed_count)
    }

    /// 判断是否应该删除某个 bullet
    fn should_remove(&self, bullet: &Bullet) -> bool {
        // 1. 保护最近使用的
        if let Some(last_recall) = bullet.metadata.last_recall {
            let days_since = (Utc::now() - last_recall).num_days();
            if days_since < 7 {
                return false; // 7天内使用过，保留
            }
        }

        // 2. 从未被召回且创建超过30天
        if bullet.metadata.recall_count == 0 {
            let age_days = (Utc::now() - bullet.created_at).num_days();
            if age_days > 30 {
                return true;
            }
        }

        // 3. 失败率太高（> 80%）且召回次数 > 5
        if bullet.metadata.recall_count > 5 && bullet.metadata.success_rate < 0.2 {
            return true;
        }

        // 4. 内容太短且重要性低
        if bullet.content.len() < 30 && bullet.metadata.importance < 0.3 {
            return true;
        }

        false
    }

    /// 压缩存储（可选功能）
    async fn _compress_storage(&self) -> Result<()> {
        // 未来可以实现：
        // - 将旧的 bullets 移到 archive
        // - 压缩 JSON 文件
        // - 清理过期的备份

        Ok(())
    }

    /// 获取优化统计
    pub async fn get_stats(&self) -> Result<OptimizerStats> {
        let storage = self.storage.read().await;
        let playbook = storage.load_playbook().await?;
        let all_bullets = playbook.all_bullets();

        let mut stats = OptimizerStats::default();
        stats.total_bullets = all_bullets.len();

        let mut weight_sum = 0.0;
        let mut recall_sum = 0;
        let mut success_sum = 0.0;

        for bullet in all_bullets {
            weight_sum += bullet.metadata.calculate_dynamic_weight();
            recall_sum += bullet.metadata.recall_count as i32;
            success_sum += bullet.metadata.success_rate;

            // 统计各个年龄段
            let age_days = (Utc::now() - bullet.created_at).num_days();
            if age_days < 7 {
                stats.bullets_last_week += 1;
            } else if age_days < 30 {
                stats.bullets_last_month += 1;
            } else {
                stats.bullets_older += 1;
            }

            // 统计召回频率
            if bullet.metadata.recall_count == 0 {
                stats.never_recalled += 1;
            } else if bullet.metadata.recall_count < 5 {
                stats.low_recall += 1;
            } else {
                stats.high_recall += 1;
            }
        }

        if stats.total_bullets > 0 {
            stats.avg_weight = weight_sum / stats.total_bullets as f32;
            stats.avg_recall = recall_sum as f32 / stats.total_bullets as f32;
            stats.avg_success_rate = success_sum / stats.total_bullets as f32;
        }

        stats.call_count = self.call_count.load(Ordering::Relaxed);

        Ok(stats)
    }
}

/// 优化器统计信息
#[derive(Debug, Default)]
pub struct OptimizerStats {
    pub total_bullets: usize,
    pub avg_weight: f32,
    pub avg_recall: f32,
    pub avg_success_rate: f32,
    pub bullets_last_week: usize,
    pub bullets_last_month: usize,
    pub bullets_older: usize,
    pub never_recalled: usize,
    pub low_recall: usize,
    pub high_recall: usize,
    pub call_count: u64,
}

impl OptimizerStats {
    /// 格式化为可读的字符串
    pub fn format(&self) -> String {
        format!(
            "📊 LAPS 优化器统计
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

总 Bullets 数: {}
平均权重: {:.3}
平均召回次数: {:.1}
平均成功率: {:.1}%

年龄分布:
  - 最近一周: {}
  - 最近一月: {}
  - 更早: {}

召回频率分布:
  - 从未召回: {}
  - 低频召回 (1-4): {}
  - 高频召回 (5+): {}

总调用次数: {}
",
            self.total_bullets,
            self.avg_weight,
            self.avg_recall,
            self.avg_success_rate * 100.0,
            self.bullets_last_week,
            self.bullets_last_month,
            self.bullets_older,
            self.never_recalled,
            self.low_recall,
            self.high_recall,
            self.call_count
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::types::{BulletMetadata, BulletSection, Playbook};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_content_hash() {
        let content1 = "这是测试内容";
        let content2 = "这是测试内容"; // 相同
        let content3 = "这是不同内容";

        let hash1 = BackgroundOptimizer::calculate_content_hash(content1);
        let hash2 = BackgroundOptimizer::calculate_content_hash(content2);
        let hash3 = BackgroundOptimizer::calculate_content_hash(content3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[tokio::test]
    async fn test_should_remove_never_recalled() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage_path = temp_dir.path().to_path_buf();

        let storage = Arc::new(RwLock::new(
            BulletStorage::new(&storage_path, 1000).unwrap(),
        ));
        let optimizer = BackgroundOptimizer::new(storage, OptimizerConfig::default());

        // 创建一个从未被召回且很旧的 bullet
        let mut bullet = Bullet {
            id: "test-1".to_string(),
            content: "测试内容".to_string(),
            section: BulletSection::General,
            created_at: Utc::now() - chrono::Duration::days(35),
            updated_at: Utc::now(),
            source_session_id: "test".to_string(),
            metadata: BulletMetadata::default(),
            tags: vec![],
            code_content: None,
        };

        bullet.metadata.recall_count = 0;

        assert!(optimizer.should_remove(&bullet));
    }

    #[tokio::test]
    async fn test_should_remove_high_failure_rate() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage_path = temp_dir.path().to_path_buf();

        let storage = Arc::new(RwLock::new(
            BulletStorage::new(&storage_path, 1000).unwrap(),
        ));
        let optimizer = BackgroundOptimizer::new(storage, OptimizerConfig::default());

        // 创建一个失败率很高的 bullet
        let mut bullet = Bullet {
            id: "test-2".to_string(),
            content: "测试内容".to_string(),
            section: BulletSection::General,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_session_id: "test".to_string(),
            metadata: BulletMetadata::default(),
            tags: vec![],
            code_content: None,
        };

        bullet.metadata.recall_count = 10;
        bullet.metadata.success_count = 1;
        bullet.metadata.failure_count = 9;
        bullet.metadata.success_rate = 0.1;

        assert!(optimizer.should_remove(&bullet));
    }

    #[tokio::test]
    async fn test_should_not_remove_recent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage_path = temp_dir.path().to_path_buf();

        let storage = Arc::new(RwLock::new(
            BulletStorage::new(&storage_path, 1000).unwrap(),
        ));
        let optimizer = BackgroundOptimizer::new(storage, OptimizerConfig::default());

        // 创建一个最近使用的 bullet
        let mut bullet = Bullet {
            id: "test-3".to_string(),
            content: "测试内容".to_string(),
            section: BulletSection::General,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_session_id: "test".to_string(),
            metadata: BulletMetadata::default(),
            tags: vec![],
            code_content: None,
        };

        bullet.metadata.last_recall = Some(Utc::now() - chrono::Duration::days(3));
        bullet.metadata.recall_count = 5;

        assert!(!optimizer.should_remove(&bullet));
    }

    #[tokio::test]
    async fn test_optimizer_stats() {
        use crate::ace::types::DeltaContext;

        let temp_dir = tempfile::tempdir().unwrap();
        let storage_path = temp_dir.path().to_path_buf();

        let storage = BulletStorage::new(&storage_path, 1000).unwrap();

        // 创建一些测试 bullets
        let mut delta = DeltaContext::new("test-session".to_string());

        for i in 0..10 {
            let mut bullet = Bullet {
                id: format!("test-{}", i),
                content: format!("测试内容 {}", i),
                section: BulletSection::General,
                created_at: Utc::now() - chrono::Duration::days(i as i64),
                updated_at: Utc::now(),
                source_session_id: "test".to_string(),
                metadata: BulletMetadata::default(),
                tags: vec![],
                code_content: None,
            };

            bullet.metadata.recall_count = i;
            bullet.metadata.importance = 0.5 + (i as f32 * 0.05);

            delta.new_bullets.push(bullet);
        }

        storage.merge_delta(delta).await.unwrap();

        let storage_arc = Arc::new(RwLock::new(storage));
        let optimizer = BackgroundOptimizer::new(storage_arc, OptimizerConfig::default());

        let stats = optimizer.get_stats().await.unwrap();

        assert_eq!(stats.total_bullets, 10);
        assert!(stats.avg_recall > 0.0);
    }
}
