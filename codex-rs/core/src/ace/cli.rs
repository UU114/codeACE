//! ACE CLI 命令处理
//!
//! 提供用户管理 ACE playbook 的命令行接口。

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use super::config_loader::ACEConfigLoader;
use super::storage::BulletStorage;

/// ACE CLI 命令
#[derive(Debug, Clone)]
pub enum AceCommand {
    /// 显示 ACE 状态和统计信息
    Status,

    /// 显示最近的学习条目
    Show { limit: usize },

    /// 清空 playbook
    Clear {
        /// 是否跳过归档直接删除
        no_archive: bool,
    },

    /// 搜索 playbook
    Search { query: String },

    /// 显示配置信息
    Config,
}

/// CLI 命令处理器
pub struct AceCliHandler {
    codex_home: std::path::PathBuf,
}

impl AceCliHandler {
    /// 创建新的 CLI 处理器
    pub fn new(codex_home: &Path) -> Self {
        Self {
            codex_home: codex_home.to_path_buf(),
        }
    }

    /// 执行命令
    pub async fn execute(&self, command: AceCommand) -> Result<()> {
        match command {
            AceCommand::Status => self.handle_status().await,
            AceCommand::Show { limit } => self.handle_show(limit).await,
            AceCommand::Clear { no_archive } => self.handle_clear(no_archive).await,
            AceCommand::Search { query } => self.handle_search(&query).await,
            AceCommand::Config => self.handle_config().await,
        }
    }

    /// 处理 status 命令
    async fn handle_status(&self) -> Result<()> {
        // 加载配置
        let config_loader = ACEConfigLoader::new(&self.codex_home);
        let config = config_loader
            .load_or_create()
            .await
            .context("Failed to load ACE config")?;

        // 加载 storage
        let storage_path = shellexpand::tilde(&config.storage_path).to_string();
        let storage = BulletStorage::new(&PathBuf::from(storage_path), config.max_entries)
            .context("Failed to open ACE storage")?;

        // 获取统计信息
        let stats = storage.get_stats().await?;

        // 显示状态
        println!("📚 ACE (Agentic Coding Environment) Status\n");
        println!("Configuration:");
        println!(
            "  Enabled: {}",
            if config.enabled { "✅ Yes" } else { "❌ No" }
        );
        println!("  Storage: {}", config.storage_path);
        println!("  Max entries: {}", config.max_entries);
        println!();

        println!("Playbook Statistics:");
        println!("  Total bullets: {}", stats.total_bullets);
        println!("  Total sessions: {}", stats.total_sessions);
        println!();

        if !stats.bullets_by_section.is_empty() {
            println!("Bullets by Section:");
            for (section, count) in &stats.bullets_by_section {
                println!("  {:?}: {}", section, count);
            }
            println!();
        }

        if !stats.tool_usage.is_empty() {
            println!("Top Tools (by usage):");
            let mut tools: Vec<_> = stats.tool_usage.iter().collect();
            tools.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

            for (tool, count) in tools.iter().take(10) {
                println!("  {}: {} times", tool, count);
            }
            println!();
        }

        if stats.total_bullets > 0 {
            println!(
                "Overall Success Rate: {:.1}%",
                stats.overall_success_rate * 100.0
            );
        }

        Ok(())
    }

    /// 处理 show 命令
    async fn handle_show(&self, limit: usize) -> Result<()> {
        // 加载配置
        let config_loader = ACEConfigLoader::new(&self.codex_home);
        let config = config_loader.load_or_create().await?;

        // 加载 storage
        let storage_path = shellexpand::tilde(&config.storage_path).to_string();
        let storage = BulletStorage::new(&PathBuf::from(storage_path), config.max_entries)?;

        // 加载所有 bullets
        let playbook = storage.load_playbook().await?;
        let bullets: Vec<_> = playbook.all_bullets().into_iter().cloned().collect();

        if bullets.is_empty() {
            println!("📭 No learning entries found yet.");
            println!("\nACE will start learning from your conversations automatically.");
            return Ok(());
        }

        println!(
            "📚 Recent ACE Learning Entries (showing {} of {})\n",
            limit.min(bullets.len()),
            bullets.len()
        );

        // 按时间倒序显示
        let mut sorted_bullets = bullets.clone();
        sorted_bullets.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        for (i, bullet) in sorted_bullets.iter().take(limit).enumerate() {
            println!(
                "{}. [{:?}] {}",
                i + 1,
                bullet.section,
                bullet.updated_at.format("%Y-%m-%d %H:%M")
            );

            // 显示内容（截断）
            let content = if bullet.content.len() > 80 {
                format!("{}...", &bullet.content[..80])
            } else {
                bullet.content.clone()
            };
            println!("   {}", content);

            // 显示工具
            if !bullet.metadata.related_tools.is_empty() {
                println!("   Tools: {}", bullet.metadata.related_tools.join(", "));
            }

            // 显示成功率
            let total = bullet.metadata.success_count + bullet.metadata.failure_count;
            if total > 0 {
                let rate = (bullet.metadata.success_count as f32 / total as f32) * 100.0;
                println!(
                    "   Success rate: {:.0}% ({}/{})",
                    rate, bullet.metadata.success_count, total
                );
            }

            println!();
        }

        if bullets.len() > limit {
            println!("... and {} more entries", bullets.len() - limit);
            println!(
                "\nUse `codex ace show --limit {}` to see more",
                bullets.len()
            );
        }

        Ok(())
    }

    /// 处理 clear 命令
    async fn handle_clear(&self, no_archive: bool) -> Result<()> {
        // 加载配置
        let config_loader = ACEConfigLoader::new(&self.codex_home);
        let config = config_loader.load_or_create().await?;

        // 加载 storage
        let storage_path = shellexpand::tilde(&config.storage_path).to_string();
        let storage = BulletStorage::new(&PathBuf::from(storage_path), config.max_entries)?;

        // 获取当前条目数
        let playbook = storage.load_playbook().await?;
        let count = playbook.all_bullets().len();

        if count == 0 {
            println!("📭 Playbook is already empty.");
            return Ok(());
        }

        // 确认
        println!(
            "⚠️  This will {} {} learning entries.",
            if no_archive { "DELETE" } else { "ARCHIVE" },
            count
        );

        if no_archive {
            println!("   Deleted entries CANNOT be recovered!");
        } else {
            println!("   Archived entries will be saved to the archive directory.");
        }

        print!("\nAre you sure? [y/N] ");
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("❌ Cancelled.");
            return Ok(());
        }

        // 执行清空
        if no_archive {
            storage.clear_without_archive().await?;
            println!("✅ Playbook cleared (entries deleted).");
        } else {
            storage.clear().await?;
            println!("✅ Playbook cleared (entries archived).");
        }

        Ok(())
    }

    /// 处理 search 命令
    async fn handle_search(&self, query: &str) -> Result<()> {
        // 加载配置
        let config_loader = ACEConfigLoader::new(&self.codex_home);
        let config = config_loader.load_or_create().await?;

        // 加载 storage
        let storage_path = shellexpand::tilde(&config.storage_path).to_string();
        let storage = BulletStorage::new(&PathBuf::from(storage_path), config.max_entries)?;

        // 搜索
        let results = storage.query_bullets(query, 20).await?;

        if results.is_empty() {
            println!("🔍 No results found for '{}'", query);
            return Ok(());
        }

        println!(
            "🔍 Search Results for '{}' ({} matches)\n",
            query,
            results.len()
        );

        for (i, bullet) in results.iter().enumerate() {
            println!("{}. [{:?}]", i + 1, bullet.section);
            println!("   {}", bullet.content);

            if !bullet.metadata.related_tools.is_empty() {
                println!("   Tools: {}", bullet.metadata.related_tools.join(", "));
            }

            println!("   Updated: {}", bullet.updated_at.format("%Y-%m-%d %H:%M"));
            println!();
        }

        Ok(())
    }

    /// 处理 config 命令
    async fn handle_config(&self) -> Result<()> {
        let config_loader = ACEConfigLoader::new(&self.codex_home);
        let config = config_loader.load_or_create().await?;

        println!("📝 ACE Configuration\n");
        println!("Config file: {}", config_loader.config_path().display());
        println!();

        println!("[ace]");
        println!("enabled = {}", config.enabled);
        println!("storage_path = {:?}", config.storage_path);
        println!("max_entries = {}", config.max_entries);
        println!();

        println!("[ace.reflector]");
        println!("extract_patterns = {}", config.reflector.extract_patterns);
        println!("extract_tools = {}", config.reflector.extract_tools);
        println!("extract_errors = {}", config.reflector.extract_errors);
        println!();

        println!("[ace.context]");
        println!("max_recent_entries = {}", config.context.max_recent_entries);
        println!(
            "include_all_successes = {}",
            config.context.include_all_successes
        );
        println!("max_context_chars = {}", config.context.max_context_chars);
        println!();

        println!("To edit: {}", config_loader.config_path().display());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_cli_handler_creation() {
        let temp_dir = TempDir::new().unwrap();
        let handler = AceCliHandler::new(temp_dir.path());

        // 测试 config 命令（会自动创建配置）
        let result = handler.handle_config().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_status_empty() {
        let temp_dir = TempDir::new().unwrap();
        let handler = AceCliHandler::new(temp_dir.path());

        // Status 命令应该能处理空 playbook
        let result = handler.handle_status().await;
        assert!(result.is_ok());
    }
}
