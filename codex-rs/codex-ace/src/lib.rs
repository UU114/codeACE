//! ACE框架 - Agentic Coding Environment
//!
//! 通过智能学习和上下文管理提升编程效率的插件框架。

pub mod context;
pub mod reflector;
pub mod storage;
pub mod types;

use anyhow::Result;
use codex_core::hooks::ExecutorHook;
use std::path::PathBuf;
use std::sync::Arc;

pub use context::SimpleContextLoader;
pub use reflector::{ReflectorConfig, ReflectorMVP};
pub use storage::SimpleStorage;
pub use types::{ACEConfig, ContextConfig, ExecutionResult, PlaybookEntry};

/// ACE插件 - 实现ExecutorHook接口
pub struct ACEPlugin {
    /// 是否启用
    enabled: bool,

    /// Reflector - 智能提取器
    reflector: Arc<ReflectorMVP>,

    /// Storage - 存储管理
    storage: Arc<SimpleStorage>,

    /// Context Loader - 上下文加载器
    context_loader: Arc<SimpleContextLoader>,

    /// 配置（保留用于未来扩展）
    #[allow(dead_code)]
    config: ACEConfig,
}

impl ACEPlugin {
    /// 创建新的ACE插件
    pub fn new(config: ACEConfig) -> Result<Self> {
        // 展开路径中的~
        let storage_path = shellexpand::tilde(&config.storage_path).to_string();
        let storage_path = PathBuf::from(storage_path);

        // 创建存储管理器
        let storage = Arc::new(SimpleStorage::new(
            &storage_path,
            config.max_entries,
        ));

        // 创建Reflector
        let reflector_config = ReflectorConfig {
            extract_patterns: config.reflector.extract_patterns,
            extract_tools: config.reflector.extract_tools,
            extract_errors: config.reflector.extract_errors,
        };
        let reflector = Arc::new(ReflectorMVP::new(reflector_config));

        // 创建上下文加载器
        let context_loader = Arc::new(SimpleContextLoader::new(
            Arc::clone(&storage),
            config.context.clone(),
        ));

        Ok(Self {
            enabled: config.enabled,
            reflector,
            storage,
            context_loader,
            config,
        })
    }

    /// 从配置创建（便捷方法）
    pub fn from_config(config: Option<ACEConfig>) -> Result<Option<Self>> {
        match config {
            Some(cfg) if cfg.enabled => {
                tracing::info!("Initializing ACE plugin...");
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
}

/// 实现ExecutorHook trait
impl ExecutorHook for ACEPlugin {
    /// 在执行前加载相关上下文
    fn pre_execute(&self, query: &str) -> Option<String> {
        if !self.enabled {
            return None;
        }

        // 在后台加载上下文
        let context_loader = Arc::clone(&self.context_loader);
        let query_content = query.to_string();

        // 同步执行（因为Hook trait不是async的）
        let runtime = tokio::runtime::Handle::current();
        let context = runtime.block_on(async move {
            match context_loader.load_context(&query_content).await {
                Ok(ctx) if !ctx.is_empty() => {
                    tracing::debug!("Loaded {} chars of context", ctx.len());
                    Some(ctx)
                }
                Ok(_) => {
                    tracing::debug!("No relevant context found");
                    None
                }
                Err(e) => {
                    tracing::warn!("Failed to load context: {}", e);
                    None
                }
            }
        });

        context
    }

    /// 在执行后进行学习
    fn post_execute(&self, query: &str, response: &str, success: bool) {
        if !self.enabled {
            return;
        }

        // 克隆必要的组件
        let reflector = Arc::clone(&self.reflector);
        let storage = Arc::clone(&self.storage);
        let query_content = query.to_string();
        let response_content = response.to_string();

        // 创建执行结果（简化版）
        let execution_result = ExecutionResult {
            success,
            output: if success { Some(response.to_string()) } else { None },
            error: if !success { Some("Execution failed".to_string()) } else { None },
            tools_used: Vec::new(), // TODO: 从响应中提取
            errors: Vec::new(),
            retry_success: false,
        };

        // 异步执行学习过程
        tokio::spawn(async move {
            tracing::debug!("Starting ACE learning process...");

            // 分析对话
            match reflector
                .analyze_conversation(&query_content, &response_content, &execution_result)
                .await
            {
                Ok(entry) if entry.is_valuable() => {
                    // 保存有价值的条目
                    if let Err(e) = storage.append_entry(&entry).await {
                        tracing::warn!("Failed to save ACE entry: {}", e);
                    } else {
                        tracing::debug!(
                            "Saved ACE entry {} with {} insights",
                            entry.id,
                            entry.insights.len()
                        );
                    }
                }
                Ok(_) => {
                    tracing::debug!("Entry is not valuable, skipping");
                }
                Err(e) => {
                    tracing::warn!("ACE reflection failed: {}", e);
                }
            }
        });
    }
}

// CLI命令支持（可选）
#[cfg(feature = "cli")]
pub mod cli;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let config = ACEConfig {
            enabled: true,
            storage_path: "/tmp/test-ace".to_string(),
            max_entries: 100,
            ..Default::default()
        };

        let plugin = ACEPlugin::new(config).unwrap();
        assert!(plugin.enabled);
    }
}