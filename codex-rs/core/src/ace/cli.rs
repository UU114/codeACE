//! ACE CLI command processing
//!
//! Provides command-line interface for users to manage ACE playbook.

use anyhow::Context;
use anyhow::Result;
use std::path::Path;
use std::path::PathBuf;

use super::config_loader::ACEConfigLoader;
use super::storage::BulletStorage;

/// ACE CLI commands
#[derive(Debug, Clone)]
pub enum AceCommand {
    /// Display ACE status and statistics
    Status,

    /// Display recent learning entries
    Show { limit: usize },

    /// Clear playbook
    Clear {
        /// Skip archiving and delete directly
        no_archive: bool,
    },

    /// Search playbook
    Search { query: String },

    /// Display configuration information
    Config,
}

/// CLI command handler
pub struct AceCliHandler {
    codex_home: std::path::PathBuf,
}

impl AceCliHandler {
    /// Create new CLI handler
    pub fn new(codex_home: &Path) -> Self {
        Self {
            codex_home: codex_home.to_path_buf(),
        }
    }

    /// Execute command
    pub async fn execute(&self, command: AceCommand) -> Result<()> {
        match command {
            AceCommand::Status => self.handle_status().await,
            AceCommand::Show { limit } => self.handle_show(limit).await,
            AceCommand::Clear { no_archive } => self.handle_clear(no_archive).await,
            AceCommand::Search { query } => self.handle_search(&query).await,
            AceCommand::Config => self.handle_config().await,
        }
    }

    /// Handle status command
    pub async fn handle_status(&self) -> Result<()> {
        // Load configuration
        let config_loader = ACEConfigLoader::new(&self.codex_home);
        let config = config_loader
            .load_or_create()
            .await
            .context("Failed to load ACE config")?;

        // Load storage
        let storage_path = shellexpand::tilde(&config.storage_path).to_string();
        let storage = BulletStorage::new(PathBuf::from(storage_path), config.max_entries)
            .context("Failed to open ACE storage")?;

        // Get statistics
        let stats = storage.get_stats().await?;

        // Display status
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
                println!("  {section:?}: {count}");
            }
            println!();
        }

        if !stats.tool_usage.is_empty() {
            println!("Top Tools (by usage):");
            let mut tools: Vec<_> = stats.tool_usage.iter().collect();
            tools.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

            for (tool, count) in tools.iter().take(10) {
                println!("  {tool}: {count} times");
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

    /// Handle show command
    pub async fn handle_show(&self, limit: usize) -> Result<()> {
        // Load configuration
        let config_loader = ACEConfigLoader::new(&self.codex_home);
        let config = config_loader.load_or_create().await?;

        // Load storage
        let storage_path = shellexpand::tilde(&config.storage_path).to_string();
        let storage = BulletStorage::new(PathBuf::from(storage_path), config.max_entries)?;

        // Load all bullets
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

        // Display in reverse chronological order
        let mut sorted_bullets = bullets.clone();
        sorted_bullets.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        for (i, bullet) in sorted_bullets.iter().take(limit).enumerate() {
            println!(
                "{}. [{:?}] {}",
                i + 1,
                bullet.section,
                bullet.updated_at.format("%Y-%m-%d %H:%M")
            );

            // Display content (truncated)
            let content = if bullet.content.len() > 80 {
                format!("{}...", &bullet.content[..80])
            } else {
                bullet.content.clone()
            };
            println!("   {content}");

            // Display tools
            if !bullet.metadata.related_tools.is_empty() {
                println!("   Tools: {}", bullet.metadata.related_tools.join(", "));
            }

            // Display success rate
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

    /// Handle clear command
    pub async fn handle_clear(&self, no_archive: bool) -> Result<()> {
        // Load configuration
        let config_loader = ACEConfigLoader::new(&self.codex_home);
        let config = config_loader.load_or_create().await?;

        // Load storage
        let storage_path = shellexpand::tilde(&config.storage_path).to_string();
        let storage = BulletStorage::new(PathBuf::from(storage_path), config.max_entries)?;

        // Get current entry count
        let playbook = storage.load_playbook().await?;
        let count = playbook.all_bullets().len();

        if count == 0 {
            println!("📭 Playbook is already empty.");
            return Ok(());
        }

        // Confirm
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
        use std::io::Write;
        use std::io::{self};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("❌ Cancelled.");
            return Ok(());
        }

        // Execute clear
        if no_archive {
            storage.clear_without_archive().await?;
            println!("✅ Playbook cleared (entries deleted).");
        } else {
            storage.clear().await?;
            println!("✅ Playbook cleared (entries archived).");
        }

        Ok(())
    }

    /// Handle search command
    pub async fn handle_search(&self, query: &str) -> Result<()> {
        // Load configuration
        let config_loader = ACEConfigLoader::new(&self.codex_home);
        let config = config_loader.load_or_create().await?;

        // Load storage
        let storage_path = shellexpand::tilde(&config.storage_path).to_string();
        let storage = BulletStorage::new(PathBuf::from(storage_path), config.max_entries)?;

        // Search
        let results = storage.query_bullets(query, 20).await?;

        if results.is_empty() {
            println!("🔍 No results found for '{query}'");
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

    /// Handle config command
    pub async fn handle_config(&self) -> Result<()> {
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

        // Test config command (will auto-create config)
        let result = handler.handle_config().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_status_empty() {
        let temp_dir = TempDir::new().unwrap();
        let handler = AceCliHandler::new(temp_dir.path());

        // Status command should handle empty playbook
        let result = handler.handle_status().await;
        assert!(result.is_ok());
    }
}
