//! 召回记录器 - LAPS 系统的使用跟踪组件
//!
//! 负责记录 bullet 的使用情况，更新召回统计和动态权重。

use crate::ace::storage::BulletStorage;
use crate::ace::types::Bullet;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 召回记录器
///
/// 跟踪 bullet 的使用情况，维护召回统计数据。
pub struct RecallTracker {
    /// Storage 引用
    storage: Arc<RwLock<BulletStorage>>,
}

impl RecallTracker {
    /// 创建新的召回记录器
    ///
    /// # 参数
    /// - `storage`: Storage 的共享引用
    pub fn new(storage: Arc<RwLock<BulletStorage>>) -> Self {
        Self { storage }
    }

    /// 记录 bullet 使用
    ///
    /// 当 bullets 被召回使用时调用此方法，更新每个 bullet 的统计信息。
    ///
    /// # 参数
    /// - `bullet_ids`: 使用的 bullet ID 列表
    /// - `context`: 使用的上下文描述
    /// - `success`: 是否成功应用
    ///
    /// # 返回
    /// 成功时返回 Ok(())，失败时返回错误
    pub async fn record_bullet_usage(
        &self,
        bullet_ids: Vec<String>,
        context: String,
        success: bool,
    ) -> Result<()> {
        let storage = self.storage.write().await;

        // 从 storage 加载 playbook
        let mut playbook = storage.load_playbook().await?;

        // 记录每个 bullet 的使用
        for bullet_id in &bullet_ids {
            if let Some(bullet) = playbook.find_bullet_mut(bullet_id) {
                // 记录召回
                bullet.metadata.record_recall(context.clone(), success);

                tracing::debug!(
                    "记录 bullet {} 召回，总次数: {}, 成功率: {:.2}%",
                    bullet_id,
                    bullet.metadata.recall_count,
                    bullet.metadata.success_rate * 100.0
                );
            } else {
                tracing::warn!("Bullet {} 不存在，无法记录召回", bullet_id);
            }
        }

        // 保存更新后的 playbook
        storage.save_playbook(&playbook).await?;

        tracing::info!(
            "召回记录完成: {} 个 bullets, 上下文: {}, 成功: {}",
            bullet_ids.len(),
            context,
            success
        );

        Ok(())
    }

    /// 获取高权重的 bullets
    ///
    /// 按动态权重排序，返回权重最高的 bullets。
    ///
    /// # 参数
    /// - `limit`: 返回的最大数量
    ///
    /// # 返回
    /// 排序后的 bullets 列表
    pub async fn get_top_bullets(&self, limit: usize) -> Result<Vec<Bullet>> {
        let storage = self.storage.read().await;
        let playbook = storage.load_playbook().await?;

        // 获取所有 bullets
        let mut all_bullets: Vec<Bullet> = playbook.all_bullets().into_iter().cloned().collect();

        // 按动态权重排序（降序）
        all_bullets.sort_by(|a, b| {
            let weight_a = a.metadata.calculate_dynamic_weight();
            let weight_b = b.metadata.calculate_dynamic_weight();
            weight_b
                .partial_cmp(&weight_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 返回前 N 个
        Ok(all_bullets.into_iter().take(limit).collect())
    }

    /// 获取召回统计信息
    ///
    /// 返回当前所有 bullets 的召回统计摘要。
    ///
    /// # 返回
    /// 统计信息结构
    pub async fn get_recall_statistics(&self) -> Result<RecallStatistics> {
        let storage = self.storage.read().await;
        let playbook = storage.load_playbook().await?;

        let all_bullets = playbook.all_bullets();
        let total_bullets = all_bullets.len();

        let total_recalls: u32 = all_bullets.iter().map(|b| b.metadata.recall_count).sum();

        let recalled_bullets = all_bullets
            .iter()
            .filter(|b| b.metadata.recall_count > 0)
            .count();

        let total_successes: u32 = all_bullets.iter().map(|b| b.metadata.success_count).sum();

        let total_failures: u32 = all_bullets.iter().map(|b| b.metadata.failure_count).sum();

        let overall_success_rate = if total_successes + total_failures > 0 {
            total_successes as f32 / (total_successes + total_failures) as f32
        } else {
            0.0
        };

        // 找出最常用的 bullets
        let mut top_bullets = all_bullets.iter().cloned().cloned().collect::<Vec<_>>();
        top_bullets.sort_by(|a, b| b.metadata.recall_count.cmp(&a.metadata.recall_count));
        let most_used_bullets = top_bullets
            .into_iter()
            .take(5)
            .map(|b| (b.id.clone(), b.metadata.recall_count))
            .collect();

        Ok(RecallStatistics {
            total_bullets,
            recalled_bullets,
            total_recalls,
            total_successes,
            total_failures,
            overall_success_rate,
            most_used_bullets,
        })
    }
}

/// 召回统计信息
#[derive(Debug, Clone)]
pub struct RecallStatistics {
    /// 总 bullet 数
    pub total_bullets: usize,
    /// 被召回过的 bullet 数
    pub recalled_bullets: usize,
    /// 总召回次数
    pub total_recalls: u32,
    /// 总成功次数
    pub total_successes: u32,
    /// 总失败次数
    pub total_failures: u32,
    /// 总体成功率
    pub overall_success_rate: f32,
    /// 最常用的 bullets (id, recall_count)
    pub most_used_bullets: Vec<(String, u32)>,
}

impl RecallStatistics {
    /// 格式化为人类可读的字符串
    pub fn to_string(&self) -> String {
        let mut output = String::new();
        output.push_str("=== 召回统计 ===\n");
        output.push_str(&format!("总 Bullet 数: {}\n", self.total_bullets));
        output.push_str(&format!("被召回过: {}\n", self.recalled_bullets));
        output.push_str(&format!("总召回次数: {}\n", self.total_recalls));
        output.push_str(&format!("总成功次数: {}\n", self.total_successes));
        output.push_str(&format!("总失败次数: {}\n", self.total_failures));
        output.push_str(&format!(
            "总体成功率: {:.2}%\n",
            self.overall_success_rate * 100.0
        ));
        output.push_str("\n最常用的 Bullets:\n");
        for (i, (id, count)) in self.most_used_bullets.iter().enumerate() {
            output.push_str(&format!("  {}. {} (召回 {} 次)\n", i + 1, id, count));
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::types::{Bullet, BulletSection, Playbook};
    use tempfile::TempDir;

    async fn create_test_tracker() -> (RecallTracker, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().to_path_buf();

        let storage = Arc::new(RwLock::new(
            BulletStorage::new(&storage_path, 1000).unwrap(),
        ));

        let tracker = RecallTracker::new(Arc::clone(&storage));

        // 初始化一些测试 bullets
        {
            let mut storage_lock = storage.write().await;
            let mut playbook = Playbook::new();

            for i in 0..5 {
                let bullet = Bullet::new(
                    BulletSection::StrategiesAndRules,
                    format!("Test bullet {}", i),
                    "test-session".to_string(),
                );
                playbook.add_bullet(bullet);
            }

            storage_lock.save_playbook(&playbook).await.unwrap();
        }

        (tracker, temp_dir)
    }

    #[tokio::test]
    async fn test_record_bullet_usage() {
        let (tracker, _temp) = create_test_tracker().await;

        // 获取 bullet IDs
        let storage = tracker.storage.read().await;
        let playbook = storage.load_playbook().await.unwrap();
        let bullet_ids: Vec<String> = playbook
            .all_bullets()
            .into_iter()
            .take(2)
            .map(|b| b.id.clone())
            .collect();
        drop(storage);

        // 记录使用
        tracker
            .record_bullet_usage(bullet_ids.clone(), "test context".to_string(), true)
            .await
            .unwrap();

        // 验证统计数据
        let storage = tracker.storage.read().await;
        let playbook = storage.load_playbook().await.unwrap();

        for bullet_id in &bullet_ids {
            let bullet = playbook.find_bullet(bullet_id).unwrap();
            assert_eq!(bullet.metadata.recall_count, 1);
            assert_eq!(bullet.metadata.success_count, 1);
            assert!(bullet.metadata.last_recall.is_some());
        }
    }

    #[tokio::test]
    async fn test_get_top_bullets() {
        let (tracker, _temp) = create_test_tracker().await;

        // 记录一些使用
        {
            let storage = tracker.storage.read().await;
            let playbook = storage.load_playbook().await.unwrap();
            let bullet_ids: Vec<String> = playbook
                .all_bullets()
                .into_iter()
                .take(2)
                .map(|b| b.id.clone())
                .collect();
            drop(storage);

            // 第一个 bullet 使用多次
            for _ in 0..5 {
                tracker
                    .record_bullet_usage(vec![bullet_ids[0].clone()], "context".to_string(), true)
                    .await
                    .unwrap();
            }

            // 第二个 bullet 使用一次
            tracker
                .record_bullet_usage(vec![bullet_ids[1].clone()], "context".to_string(), true)
                .await
                .unwrap();
        }

        // 获取 top bullets
        let top = tracker.get_top_bullets(3).await.unwrap();
        assert!(top.len() <= 3);

        // 第一个应该是使用最多的
        assert!(top[0].metadata.recall_count >= top[1].metadata.recall_count);
    }

    #[tokio::test]
    async fn test_get_recall_statistics() {
        let (tracker, _temp) = create_test_tracker().await;

        // 记录一些使用
        {
            let storage = tracker.storage.read().await;
            let playbook = storage.load_playbook().await.unwrap();
            let bullet_ids: Vec<String> = playbook
                .all_bullets()
                .into_iter()
                .take(3)
                .map(|b| b.id.clone())
                .collect();
            drop(storage);

            tracker
                .record_bullet_usage(bullet_ids, "context".to_string(), true)
                .await
                .unwrap();
        }

        // 获取统计
        let stats = tracker.get_recall_statistics().await.unwrap();

        assert_eq!(stats.total_bullets, 5);
        assert_eq!(stats.recalled_bullets, 3);
        assert_eq!(stats.total_recalls, 3);
        assert_eq!(stats.total_successes, 3);
        assert_eq!(stats.total_failures, 0);
        assert_eq!(stats.overall_success_rate, 1.0);
    }

    #[tokio::test]
    async fn test_record_failure() {
        let (tracker, _temp) = create_test_tracker().await;

        let storage = tracker.storage.read().await;
        let playbook = storage.load_playbook().await.unwrap();
        let bullet_id = playbook.all_bullets()[0].id.clone();
        drop(storage);

        // 记录失败
        tracker
            .record_bullet_usage(vec![bullet_id.clone()], "context".to_string(), false)
            .await
            .unwrap();

        // 验证
        let storage = tracker.storage.read().await;
        let playbook = storage.load_playbook().await.unwrap();
        let bullet = playbook.find_bullet(&bullet_id).unwrap();

        assert_eq!(bullet.metadata.recall_count, 1);
        assert_eq!(bullet.metadata.success_count, 0);
        assert_eq!(bullet.metadata.failure_count, 1);
        assert_eq!(bullet.metadata.success_rate, 0.0);
    }
}
