//! ACE框架 - Agentic Coding Environment (Bullet-based)
//!
//! 通过智能学习和上下文管理提升编程效率的插件框架。
//!
//! 基于 Agentic Context Engineering 论文实现，采用 Bullet-based 架构。

pub mod cli;
pub mod code_analyzer;
pub mod config_loader;
pub mod context;
pub mod curator;
pub mod reflector;
pub mod storage;
pub mod types;

use crate::hooks::ExecutorHook;
use anyhow::Result;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

pub use cli::AceCliHandler;
pub use cli::AceCommand;
pub use config_loader::ACEConfigLoader;
pub use config_loader::load_ace_config;
pub use curator::CuratorMVP;
pub use reflector::ReflectorMVP;
pub use storage::BulletStorage;
pub use types::ACEConfig;
pub use types::Bullet;
pub use types::BulletSection;
pub use types::ContextConfig;
pub use types::CuratorConfig;
pub use types::DeltaContext;
pub use types::ExecutionResult;
pub use types::Playbook;
pub use types::RawInsight;

/// ACE插件 - Bullet-based 架构
///
/// 数据流:
/// 1. pre_execute: 从 Storage 检索相关 bullets，注入到 context
/// 2. post_execute: Reflector 提取 insights → Curator 生成 delta → Storage 合并
pub struct ACEPlugin {
    /// 是否启用
    enabled: bool,

    /// Reflector - 智能提取器（生成 RawInsights）
    reflector: Arc<ReflectorMVP>,

    /// Curator - 组织器（RawInsights → Bullets）
    curator: Arc<CuratorMVP>,

    /// Storage - 存储管理（增量更新）
    storage: Arc<BulletStorage>,

    /// 配置（保留用于未来扩展）
    config: ACEConfig,
}

impl ACEPlugin {
    /// 创建新的ACE插件
    pub fn new(config: ACEConfig) -> Result<Self> {
        // 展开路径中的~
        let storage_path = shellexpand::tilde(&config.storage_path).to_string();
        let storage_path = PathBuf::from(storage_path);

        // 创建 Storage
        let storage = Arc::new(BulletStorage::new(&storage_path, config.max_entries)?);

        // 创建 Reflector
        let reflector_config = reflector::ReflectorConfig {
            extract_patterns: config.reflector.extract_patterns,
            extract_tools: config.reflector.extract_tools,
            extract_errors: config.reflector.extract_errors,
        };
        let reflector = Arc::new(ReflectorMVP::new(reflector_config));

        // 创建 Curator
        let curator = Arc::new(CuratorMVP::new(CuratorConfig::default()));

        Ok(Self {
            enabled: config.enabled,
            reflector,
            curator,
            storage,
            config,
        })
    }

    /// 从配置创建（便捷方法）
    pub fn from_config(config: Option<ACEConfig>) -> Result<Option<Self>> {
        match config {
            Some(cfg) if cfg.enabled => {
                tracing::info!("Initializing ACE plugin (Bullet-based)...");
                Ok(Some(Self::new(cfg)?))
            }
            Some(_) => {
                tracing::info!("ACE plugin is disabled");
                Ok(None)
            }
            None => {
                tracing::debug!("No ACE configuration found");
                Ok(None)
            }
        }
    }

    /// 从 codex_home 自动加载配置并创建插件（推荐使用）
    ///
    /// 此方法会：
    /// 1. 从 `~/.codeACE/codeACE-config.toml` 加载配置
    /// 2. 如果配置文件不存在，自动创建默认配置
    /// 3. 如果 enabled=false，返回 None
    ///
    /// # 参数
    /// - `codex_home`: Codex home 目录路径
    pub async fn from_codex_home(codex_home: &Path) -> Result<Option<Self>> {
        tracing::debug!("Loading ACE config from {:?}", codex_home);

        // 加载配置（自动创建如果不存在）
        let config = match load_ace_config(codex_home).await {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!("Failed to load ACE config: {}, ACE disabled", e);
                return Ok(None);
            }
        };

        // 根据配置创建插件
        Self::from_config(Some(config))
    }

    /// 格式化 bullets 为上下文字符串
    fn format_bullets_as_context(&self, bullets: Vec<Bullet>) -> String {
        let mut output = String::from("# 📚 ACE Playbook Context\n\n");
        output.push_str(&format!("Found {} relevant strategies:\n\n", bullets.len()));

        // 按 section 分组
        let mut by_section: std::collections::HashMap<BulletSection, Vec<&Bullet>> =
            std::collections::HashMap::new();
        for bullet in &bullets {
            by_section
                .entry(bullet.section.clone())
                .or_insert_with(Vec::new)
                .push(bullet);
        }

        // 格式化输出
        for (section, bullets) in by_section {
            output.push_str(&format!("## {}\n\n", self.section_title(&section)));

            for bullet in bullets {
                output.push_str(&format!("- {}\n", bullet.content));

                // 显示相关工具
                if !bullet.metadata.related_tools.is_empty() {
                    output.push_str(&format!(
                        "  - Tools: {}\n",
                        bullet.metadata.related_tools.join(", ")
                    ));
                }

                // 显示成功率
                let total = bullet.metadata.success_count + bullet.metadata.failure_count;
                if total > 0 {
                    let success_rate =
                        (bullet.metadata.success_count as f32 / total as f32) * 100.0;
                    output.push_str(&format!("  - Success rate: {:.0}%\n", success_rate));
                }

                output.push('\n');
            }
        }

        output
    }

    fn section_title(&self, section: &BulletSection) -> &str {
        match section {
            BulletSection::StrategiesAndRules => "Strategies and Rules",
            BulletSection::CodeSnippetsAndTemplates => "Code Snippets and Templates",
            BulletSection::TroubleshootingAndPitfalls => "Troubleshooting and Pitfalls",
            BulletSection::ApiUsageGuides => "API Usage Guides",
            BulletSection::ErrorHandlingPatterns => "Error Handling Patterns",
            BulletSection::ToolUsageTips => "Tool Usage Tips",
            BulletSection::General => "General Knowledge",
        }
    }

    /// Todo 完成时触发 Reflector
    ///
    /// 这个方法在 plan handler 检测到 Todo 完成时被调用，
    /// 用于生成和存储相关的 Bullets。
    ///
    /// # 参数
    /// - `todo_step`: Todo 的描述
    /// - `conversation_context`: 完成该 Todo 的对话上下文
    /// - `session_id`: 会话 ID
    pub fn on_todo_completed(
        &self,
        todo_step: String,
        conversation_context: String,
        session_id: String,
    ) {
        if !self.enabled {
            return;
        }

        let reflector = Arc::clone(&self.reflector);
        let curator = Arc::clone(&self.curator);
        let storage = Arc::clone(&self.storage);

        // 异步执行学习过程（不阻塞主流程）
        tokio::spawn(async move {
            tracing::info!("🎯 Todo completed, triggering Reflector: {}", todo_step);

            // 构造执行结果（Todo 完成场景）
            let execution_result = ExecutionResult {
                success: true,
                output: Some(format!("Completed todo: {}", todo_step)),
                error: None,
                tools_used: Vec::new(),
                errors: Vec::new(),
                retry_success: false,
            };

            // 1. Reflector 分析
            let insights = match reflector
                .analyze_conversation(
                    &format!("Todo: {}", todo_step),
                    &conversation_context,
                    &execution_result,
                    session_id.clone(),
                )
                .await
            {
                Ok(insights) => insights,
                Err(e) => {
                    tracing::error!("Reflector failed for todo: {}", e);
                    return;
                }
            };

            if insights.is_empty() {
                tracing::debug!("No insights extracted from todo completion");
                return;
            }

            tracing::info!("Extracted {} insights from todo", insights.len());

            // 2. Curator 生成 delta
            let delta = match curator.process_insights(insights, session_id).await {
                Ok(delta) => delta,
                Err(e) => {
                    tracing::error!("Curator failed for todo: {}", e);
                    return;
                }
            };

            if delta.is_empty() {
                tracing::debug!("Delta is empty for todo");
                return;
            }

            tracing::info!(
                "Generated {} bullets from todo completion",
                delta.new_bullets.len()
            );

            // 3. Storage 合并 delta
            if let Err(e) = storage.merge_delta(delta).await {
                tracing::error!("Failed to merge delta for todo: {}", e);
            } else {
                tracing::info!("✅ Todo completion learning completed");
            }
        });
    }
}

/// 实现ExecutorHook trait
impl ExecutorHook for ACEPlugin {
    /// 在执行前加载相关上下文
    fn pre_execute(&self, query: &str) -> Option<String> {
        if !self.enabled {
            return None;
        }

        let storage = Arc::clone(&self.storage);
        let query_content = query.to_string();

        // 使用新的运行时来避免嵌套 block_on 的问题
        // 这是因为 Hook trait 不是 async 的，但我们需要执行异步操作
        let context = std::thread::spawn(move || {
            // 创建新的运行时
            let rt = tokio::runtime::Runtime::new().ok()?;
            rt.block_on(async move {
                match storage.query_bullets(&query_content, 10).await {
                    Ok(bullets) if !bullets.is_empty() => {
                        tracing::debug!("Found {} relevant bullets", bullets.len());
                        Some(bullets)
                    }
                    Ok(_) => {
                        tracing::debug!("No relevant bullets found");
                        None
                    }
                    Err(e) => {
                        tracing::warn!("Failed to query bullets: {}", e);
                        None
                    }
                }
            })
        })
        .join()
        .ok()
        .flatten();

        context.map(|bullets| self.format_bullets_as_context(bullets))
    }

    /// 在执行后进行学习
    fn post_execute(&self, query: &str, response: &str, success: bool) {
        if !self.enabled {
            return;
        }

        // 克隆必要的组件
        let reflector = Arc::clone(&self.reflector);
        let curator = Arc::clone(&self.curator);
        let storage = Arc::clone(&self.storage);
        let query_content = query.to_string();
        let response_content = response.to_string();

        // 创建执行结果（简化版）
        let execution_result = ExecutionResult {
            success,
            output: if success {
                Some(response.to_string())
            } else {
                None
            },
            error: if !success {
                Some("Execution failed".to_string())
            } else {
                None
            },
            tools_used: Vec::new(), // TODO: 从响应中提取
            errors: Vec::new(),
            retry_success: false,
        };

        // 异步执行学习过程
        tokio::spawn(async move {
            tracing::debug!("Starting ACE learning process (Bullet-based)...");

            // 1. Reflector 分析
            let session_id = uuid::Uuid::new_v4().to_string();

            let insights = match reflector
                .analyze_conversation(
                    &query_content,
                    &response_content,
                    &execution_result,
                    session_id.clone(),
                )
                .await
            {
                Ok(insights) => insights,
                Err(e) => {
                    tracing::error!("Reflector failed: {}", e);
                    return;
                }
            };

            if insights.is_empty() {
                tracing::debug!("No valuable insights extracted");
                return;
            }

            tracing::info!("Extracted {} insights", insights.len());

            // 2. Curator 生成 delta
            let delta = match curator.process_insights(insights, session_id).await {
                Ok(delta) => delta,
                Err(e) => {
                    tracing::error!("Curator failed: {}", e);
                    return;
                }
            };

            if delta.is_empty() {
                tracing::debug!("Delta is empty, nothing to merge");
                return;
            }

            tracing::info!(
                "Generated delta: {} new bullets, {} updated",
                delta.new_bullets.len(),
                delta.updated_bullets.len()
            );

            // 3. Storage 合并 delta
            if let Err(e) = storage.merge_delta(delta).await {
                tracing::error!("Failed to merge delta: {}", e);
            } else {
                tracing::info!("Delta merged successfully");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let config = ACEConfig {
            enabled: true,
            storage_path: "/tmp/test-ace-bullet".to_string(),
            max_entries: 100,
            ..Default::default()
        };

        let plugin = ACEPlugin::new(config).unwrap();
        assert!(plugin.enabled);
    }

    #[test]
    fn test_plugin_from_config_disabled() {
        let config = ACEConfig {
            enabled: false,
            ..Default::default()
        };

        let result = ACEPlugin::from_config(Some(config)).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_plugin_from_config_none() {
        let result = ACEPlugin::from_config(None).unwrap();
        assert!(result.is_none());
    }
}
