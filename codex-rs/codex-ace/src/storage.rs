//! 简单存储系统 - JSONL格式
//!
//! 使用JSON Lines格式，每行一个JSON对象，便于追加和流式读取。

use crate::types::PlaybookEntry;
use anyhow::{Context, Result};
use chrono::Utc;
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// 简单存储管理器
pub struct SimpleStorage {
    /// Playbook文件路径
    playbook_path: PathBuf,

    /// 归档目录路径
    archive_dir: PathBuf,

    /// 最大条目数
    max_entries: usize,
}

impl SimpleStorage {
    /// 创建新的存储管理器
    pub fn new(base_path: impl AsRef<Path>, max_entries: usize) -> Self {
        let base_path = base_path.as_ref();
        let playbook_path = base_path.join("playbook.jsonl");
        let archive_dir = base_path.join("archive");

        Self {
            playbook_path,
            archive_dir,
            max_entries,
        }
    }

    /// 初始化存储目录
    pub async fn init(&self) -> Result<()> {
        // 确保基础目录存在
        if let Some(parent) = self.playbook_path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create storage directory")?;
        }

        // 确保归档目录存在
        fs::create_dir_all(&self.archive_dir)
            .await
            .context("Failed to create archive directory")?;

        Ok(())
    }

    /// 追加新条目
    pub async fn append_entry(&self, entry: &PlaybookEntry) -> Result<()> {
        // 先写入条目
        self.write_entry_internal(entry).await?;

        // 检查是否需要自动归档
        self.auto_archive_if_needed().await?;

        Ok(())
    }

    /// 内部写入方法，不触发归档检查
    async fn write_entry_internal(&self, entry: &PlaybookEntry) -> Result<()> {
        // 确保目录存在
        self.init().await?;

        // 序列化为JSON
        let json_line = serde_json::to_string(entry).context("Failed to serialize entry")?;

        // 追加到文件
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.playbook_path)
            .await
            .context("Failed to open playbook file")?;

        file.write_all(json_line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        tracing::debug!("Appended entry {} to playbook", entry.id);
        Ok(())
    }

    /// 读取所有条目
    pub async fn load_all(&self) -> Result<Vec<PlaybookEntry>> {
        if !self.playbook_path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&self.playbook_path)
            .await
            .context("Failed to open playbook file")?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut entries = Vec::new();

        while let Some(line) = lines.next_line().await? {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            match serde_json::from_str::<PlaybookEntry>(line) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    tracing::warn!("Failed to parse playbook entry: {}", e);
                    continue;
                }
            }
        }

        tracing::debug!("Loaded {} entries from playbook", entries.len());
        Ok(entries)
    }

    /// 清空Playbook（归档后清空）
    pub async fn clear(&self) -> Result<()> {
        if self.playbook_path.exists() {
            // 生成归档文件名
            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            let archive_name = format!("playbook_{}.jsonl", timestamp);
            let archive_path = self.archive_dir.join(archive_name);

            // 确保归档目录存在
            fs::create_dir_all(&self.archive_dir).await?;

            // 移动文件到归档目录
            fs::rename(&self.playbook_path, &archive_path)
                .await
                .context("Failed to archive playbook")?;

            tracing::info!("Archived playbook to {}", archive_path.display());
        }

        Ok(())
    }

    /// 检查并自动归档（超过限制时）
    async fn auto_archive_if_needed(&self) -> Result<()> {
        let entries = self.load_all().await?;

        if entries.len() > self.max_entries {
            tracing::info!(
                "Playbook has {} entries, exceeding limit of {}. Auto-archiving...",
                entries.len(),
                self.max_entries
            );

            self.clear().await?;

            // 重新创建文件，保留最近的一半条目
            let keep_count = self.max_entries / 2;
            let skip_count = entries.len().saturating_sub(keep_count);
            let recent_entries = entries.into_iter().skip(skip_count).collect::<Vec<_>>();

            for entry in recent_entries {
                self.write_entry_internal(&entry).await?;
            }

            tracing::info!("Auto-archive complete, kept {} recent entries", keep_count);
        }

        Ok(())
    }

    /// 获取存储统计信息
    pub async fn get_stats(&self) -> Result<StorageStats> {
        let entries = self.load_all().await?;
        let total_entries = entries.len();

        let success_count = entries.iter().filter(|e| e.execution_success).count();

        let mut tool_counts = std::collections::HashMap::new();
        for entry in &entries {
            for tool in &entry.tools_used {
                *tool_counts.entry(tool.clone()).or_insert(0) += 1;
            }
        }

        Ok(StorageStats {
            total_entries,
            success_count,
            success_rate: if total_entries > 0 {
                success_count as f32 / total_entries as f32
            } else {
                0.0
            },
            tool_usage: tool_counts,
        })
    }

    /// 搜索条目（简单的关键词匹配）
    pub async fn search(&self, query: &str) -> Result<Vec<PlaybookEntry>> {
        let entries = self.load_all().await?;
        let query_lower = query.to_lowercase();

        let matches = entries
            .into_iter()
            .filter(|entry| {
                entry.user_query.to_lowercase().contains(&query_lower)
                    || entry
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
                    || entry
                        .insights
                        .iter()
                        .any(|insight| insight.content.to_lowercase().contains(&query_lower))
            })
            .collect();

        Ok(matches)
    }
}

/// 存储统计信息
#[derive(Debug)]
pub struct StorageStats {
    pub total_entries: usize,
    pub success_count: usize,
    pub success_rate: f32,
    pub tool_usage: std::collections::HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_storage_basic_operations() {
        let temp_dir = tempdir().unwrap();
        let storage = SimpleStorage::new(temp_dir.path(), 100);

        // 测试初始化
        storage.init().await.unwrap();

        // 创建测试条目
        let mut entry = PlaybookEntry::new("test query".to_string(), "test response".to_string());
        entry.execution_success = true;
        entry.tools_used.push("bash".to_string());

        // 测试追加
        storage.append_entry(&entry).await.unwrap();

        // 测试加载
        let entries = storage.load_all().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].user_query, "test query");

        // 测试搜索
        let search_results = storage.search("test").await.unwrap();
        assert_eq!(search_results.len(), 1);

        // 测试清空
        storage.clear().await.unwrap();
        let entries = storage.load_all().await.unwrap();
        assert_eq!(entries.len(), 0);
    }
}
