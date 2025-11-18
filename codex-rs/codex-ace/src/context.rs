//! 上下文管理器 - 加载相关的历史知识
//!
//! MVP版本，使用简单的关键词匹配。

use crate::storage::SimpleStorage;
use crate::types::{ContextConfig, PlaybookEntry};
use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;

/// 简单的上下文加载器
pub struct SimpleContextLoader {
    storage: Arc<SimpleStorage>,
    config: ContextConfig,
}

impl SimpleContextLoader {
    /// 创建新的上下文加载器
    pub fn new(storage: Arc<SimpleStorage>, config: ContextConfig) -> Self {
        Self { storage, config }
    }

    /// 为新对话加载相关上下文
    pub async fn load_context(&self, user_query: &str) -> Result<String> {
        let entries = self.storage.load_all().await?;

        if entries.is_empty() {
            return Ok(String::new());
        }

        // 查找相关条目
        let relevant_entries = self.find_relevant_entries(&entries, user_query);

        // 生成上下文文本
        let context = self.format_context(&relevant_entries);

        Ok(context)
    }

    /// 查找相关条目（MVP版：简单关键词匹配）
    fn find_relevant_entries(&self, entries: &[PlaybookEntry], query: &str) -> Vec<PlaybookEntry> {
        // 提取查询关键词
        let query_words: HashSet<String> = query
            .split_whitespace()
            .filter(|w| w.len() > 3) // 忽略短词
            .map(|w| w.to_lowercase())
            .collect();

        if query_words.is_empty() {
            // 如果没有关键词，返回最近的成功案例
            return self.get_recent_successes(entries);
        }

        // 计算每个条目的相关性分数
        let mut scored_entries: Vec<(PlaybookEntry, usize)> = entries
            .iter()
            .filter_map(|entry| {
                let mut score = 0;

                // 用户查询匹配（权重最高）
                for word in &query_words {
                    if entry.user_query.to_lowercase().contains(word) {
                        score += 3;
                    }
                }

                // 标签匹配（权重中等）
                for word in &query_words {
                    if entry
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(word))
                    {
                        score += 2;
                    }
                }

                // 洞察内容匹配（权重较低）
                for word in &query_words {
                    if entry
                        .insights
                        .iter()
                        .any(|i| i.content.to_lowercase().contains(word))
                    {
                        score += 1;
                    }
                }

                // 只返回有分数的条目
                if score > 0 {
                    Some((entry.clone(), score))
                } else {
                    None
                }
            })
            .collect();

        // 按分数排序
        scored_entries.sort_by_key(|(_, score)| std::cmp::Reverse(*score));

        // 取前N个
        let selected: Vec<PlaybookEntry> = scored_entries
            .into_iter()
            .take(self.config.max_recent_entries)
            .map(|(entry, _)| entry)
            .collect();

        // 如果没有相关的，返回最近的成功案例
        if selected.is_empty() {
            self.get_recent_successes(entries)
        } else {
            selected
        }
    }

    /// 获取最近的成功案例
    fn get_recent_successes(&self, entries: &[PlaybookEntry]) -> Vec<PlaybookEntry> {
        let mut successes: Vec<PlaybookEntry> = entries
            .iter()
            .filter(|e| e.execution_success || !e.learned_strategies.is_empty())
            .cloned()
            .collect();

        // 按时间倒序（最新的在前）
        successes.sort_by_key(|e| std::cmp::Reverse(e.timestamp));

        successes
            .into_iter()
            .take(self.config.max_recent_entries)
            .collect()
    }

    /// 格式化条目为上下文
    fn format_context(&self, entries: &[PlaybookEntry]) -> String {
        if entries.is_empty() {
            return String::new();
        }

        let mut context = String::from("# 📚 Previous Learning\n\n");
        context.push_str(&format!(
            "Found {} relevant previous experiences:\n\n",
            entries.len()
        ));

        let mut total_chars = context.len();

        for (i, entry) in entries.iter().enumerate() {
            let entry_text = self.format_entry(entry, i + 1);

            // 检查字符数限制
            if total_chars + entry_text.len() > self.config.max_context_chars {
                context.push_str(&format!(
                    "\n... ({} more entries omitted due to length limit)\n",
                    entries.len() - i
                ));
                break;
            }

            context.push_str(&entry_text);
            context.push_str("\n---\n\n");
            total_chars += entry_text.len() + 5; // 包括分隔符
        }

        context
    }

    /// 格式化单个条目
    fn format_entry(&self, entry: &PlaybookEntry, index: usize) -> String {
        let mut text = String::new();

        // 标题
        text.push_str(&format!("## {}. ", index));
        if entry.execution_success {
            text.push_str("✅ ");
        } else {
            text.push_str("⚠️ ");
        }

        // 简短的查询描述
        let query_summary = Self::truncate(&entry.user_query, 80);
        text.push_str(&format!("{}\n\n", query_summary));

        // 重要洞察
        if !entry.insights.is_empty() {
            text.push_str("**Key Insights:**\n");
            for insight in entry.insights.iter().filter(|i| i.importance > 0.6).take(3) {
                text.push_str(&format!("- {}\n", Self::truncate(&insight.content, 150)));
            }
            text.push('\n');
        }

        // 成功策略
        if !entry.learned_strategies.is_empty() {
            text.push_str("**Successful Strategies:**\n");
            for strategy in entry.learned_strategies.iter().take(2) {
                text.push_str(&format!("- {}\n", Self::truncate(strategy, 200)));
            }
            text.push('\n');
        }

        // 使用的工具
        if !entry.tools_used.is_empty() {
            text.push_str(&format!(
                "**Tools Used:** {}\n",
                entry.tools_used.join(", ")
            ));
        }

        // 识别的模式
        if !entry.patterns.is_empty() {
            text.push_str(&format!("**Patterns:** {}\n", entry.patterns.join(", ")));
        }

        // 标签
        if !entry.tags.is_empty() {
            text.push_str(&format!("**Tags:** {}\n", entry.tags.join(", ")));
        }

        text
    }

    /// 截断字符串
    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Insight, InsightCategory};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_context_loading() {
        let temp_dir = tempdir().unwrap();
        let storage = Arc::new(SimpleStorage::new(temp_dir.path(), 100));
        let config = ContextConfig::default();
        let loader = SimpleContextLoader::new(Arc::clone(&storage), config);

        // 添加一些测试条目
        let mut entry1 = PlaybookEntry::new(
            "How to run tests?".to_string(),
            "Use cargo test".to_string(),
        );
        entry1.execution_success = true;
        entry1.tools_used.push("cargo".to_string());
        entry1.tags.push("testing".to_string());
        entry1.insights.push(Insight {
            content: "Cargo test runs all tests".to_string(),
            category: InsightCategory::Knowledge,
            importance: 0.8,
        });

        storage.append_entry(&entry1).await.unwrap();

        let mut entry2 = PlaybookEntry::new(
            "Build the project".to_string(),
            "Use cargo build".to_string(),
        );
        entry2.execution_success = true;
        entry2.tools_used.push("cargo".to_string());
        entry2.tags.push("building".to_string());

        storage.append_entry(&entry2).await.unwrap();

        // 测试相关查询
        let context = loader.load_context("run tests").await.unwrap();
        assert!(context.contains("testing"));
        assert!(context.contains("cargo"));

        // 测试不相关查询（应该返回最近的成功案例）
        let context = loader.load_context("deploy application").await.unwrap();
        assert!(!context.is_empty());
    }
}
