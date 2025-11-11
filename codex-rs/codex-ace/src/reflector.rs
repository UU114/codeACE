//! Reflector - 智能提取器（MVP版本）
//!
//! 基于规则的模式提取，不依赖LLM，快速高效。

use crate::types::{ExecutionResult, Insight, InsightCategory, PlaybookEntry};
use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use uuid::Uuid;

/// Reflector配置
#[derive(Debug, Clone)]
pub struct ReflectorConfig {
    pub extract_patterns: bool,
    pub extract_tools: bool,
    pub extract_errors: bool,
}

impl Default for ReflectorConfig {
    fn default() -> Self {
        Self {
            extract_patterns: true,
            extract_tools: true,
            extract_errors: true,
        }
    }
}

/// MVP版Reflector - 专注于规则提取
pub struct ReflectorMVP {
    config: ReflectorConfig,
    patterns: HashMap<String, Regex>,
}

impl ReflectorMVP {
    /// 创建新的Reflector
    pub fn new(config: ReflectorConfig) -> Self {
        Self {
            config,
            patterns: Self::init_patterns(),
        }
    }

    /// 初始化正则表达式模式
    fn init_patterns() -> HashMap<String, Regex> {
        let mut patterns = HashMap::new();

        // 工具使用模式
        patterns.insert(
            "tool_bash".to_string(),
            Regex::new(r"(?i)(bash|shell|command|execute|run)\s+`([^`]+)`").unwrap(),
        );

        patterns.insert(
            "tool_file".to_string(),
            Regex::new(r"(?i)(read|write|edit|create|modify)\s+(file|path)?\s*[:\s]+([^\s\n]+)").unwrap(),
        );

        // 代码块模式
        patterns.insert(
            "code_block".to_string(),
            Regex::new(r"```(\w+)?\n([\s\S]+?)```").unwrap(),
        );

        // 错误模式
        patterns.insert(
            "error_pattern".to_string(),
            Regex::new(r"(?i)(error|错误|failed|失败|exception):\s*([^\n]+)").unwrap(),
        );

        // 测试模式
        patterns.insert(
            "test_pattern".to_string(),
            Regex::new(r"(?i)(test|pytest|cargo test|npm test)").unwrap(),
        );

        // 构建模式
        patterns.insert(
            "build_pattern".to_string(),
            Regex::new(r"(?i)(build|compile|cargo build|npm build|make)").unwrap(),
        );

        // Git操作
        patterns.insert(
            "git_pattern".to_string(),
            Regex::new(r"(?i)(git\s+(add|commit|push|pull|clone|checkout))").unwrap(),
        );

        patterns
    }

    /// 分析对话并提取知识
    pub async fn analyze_conversation(
        &self,
        user_query: &str,
        assistant_response: &str,
        execution_result: &ExecutionResult,
    ) -> Result<PlaybookEntry> {
        let mut entry = PlaybookEntry::new(
            user_query.to_string(),
            assistant_response.to_string(),
        );

        entry.session_id = Uuid::new_v4().to_string();
        entry.execution_success = execution_result.success;
        entry.tools_used = execution_result.tools_used.clone();
        entry.error_messages = execution_result.errors.clone();

        // 提取各种洞察
        if self.config.extract_tools {
            self.extract_tool_insights(&mut entry, assistant_response)?;
        }

        if self.config.extract_errors && !execution_result.success {
            self.extract_error_solutions(&mut entry, execution_result)?;
        }

        if self.config.extract_patterns {
            self.extract_patterns(&mut entry, assistant_response)?;
        }

        // 生成标签
        self.generate_tags(&mut entry);

        // 过滤出有价值的条目
        if !entry.is_valuable() {
            tracing::debug!("Entry is not valuable, skipping");
        }

        Ok(entry)
    }

    /// 提取工具使用洞察
    fn extract_tool_insights(
        &self,
        entry: &mut PlaybookEntry,
        response: &str,
    ) -> Result<()> {
        // Bash/Shell命令提取
        if let Some(regex) = self.patterns.get("tool_bash") {
            for cap in regex.captures_iter(response) {
                if let Some(command) = cap.get(2) {
                    let insight = Insight {
                        content: format!("使用命令: {}", command.as_str()),
                        category: InsightCategory::ToolUsage,
                        importance: 0.7,
                    };
                    entry.insights.push(insight);

                    if !entry.tools_used.contains(&"bash".to_string()) {
                        entry.tools_used.push("bash".to_string());
                    }
                }
            }
        }

        // 文件操作提取
        if let Some(regex) = self.patterns.get("tool_file") {
            for cap in regex.captures_iter(response) {
                if let (Some(action), Some(path)) = (cap.get(1), cap.get(3)) {
                    let insight = Insight {
                        content: format!("文件操作: {} {}", action.as_str(), path.as_str()),
                        category: InsightCategory::ToolUsage,
                        importance: 0.6,
                    };
                    entry.insights.push(insight);

                    let tool_name = action.as_str().to_lowercase();
                    if !entry.tools_used.contains(&tool_name) {
                        entry.tools_used.push(tool_name);
                    }
                }
            }
        }

        Ok(())
    }

    /// 提取错误解决方案
    fn extract_error_solutions(
        &self,
        entry: &mut PlaybookEntry,
        result: &ExecutionResult,
    ) -> Result<()> {
        if let Some(error) = &result.error {
            let insight = Insight {
                content: format!("错误处理: {}", truncate_string(error, 200)),
                category: InsightCategory::ErrorHandling,
                importance: 0.9,
            };
            entry.insights.push(insight);

            // 如果后续成功了，记录为成功的解决策略
            if result.retry_success {
                entry.learned_strategies.push(
                    format!("解决错误 '{}' 的方法: 查看助手响应", truncate_string(error, 100))
                );
            }
        }

        Ok(())
    }

    /// 提取模式
    fn extract_patterns(&self, entry: &mut PlaybookEntry, response: &str) -> Result<()> {
        // 测试模式
        if let Some(regex) = self.patterns.get("test_pattern") {
            if regex.is_match(response) {
                entry.patterns.push("测试执行".to_string());
            }
        }

        // 构建模式
        if let Some(regex) = self.patterns.get("build_pattern") {
            if regex.is_match(response) {
                entry.patterns.push("构建流程".to_string());
            }
        }

        // Git操作模式
        if let Some(regex) = self.patterns.get("git_pattern") {
            if regex.is_match(response) {
                entry.patterns.push("Git操作".to_string());
            }
        }

        // 代码块模式
        if let Some(regex) = self.patterns.get("code_block") {
            let code_blocks = regex.captures_iter(response).count();
            if code_blocks > 0 {
                entry.patterns.push(format!("包含{}个代码块", code_blocks));

                // 提取编程语言
                for cap in regex.captures_iter(response) {
                    if let Some(lang) = cap.get(1) {
                        let lang_str = lang.as_str();
                        if !lang_str.is_empty() {
                            entry.tags.push(format!("lang:{}", lang_str));
                        }
                    }
                }
            }
        }

        // 组合模式识别
        if entry.patterns.contains(&"测试执行".to_string()) &&
           entry.patterns.contains(&"构建流程".to_string()) {
            entry.learned_strategies.push(
                "测试驱动开发流程：先测试后构建".to_string()
            );
        }

        Ok(())
    }

    /// 生成标签
    fn generate_tags(&self, entry: &mut PlaybookEntry) {
        // 工具标签
        if !entry.tools_used.is_empty() {
            entry.tags.push("tools".to_string());
            for tool in &entry.tools_used {
                entry.tags.push(format!("tool:{}", tool));
            }
        }

        // 错误处理标签
        if !entry.error_messages.is_empty() {
            entry.tags.push("error-handling".to_string());
        }

        // 成功标签
        if entry.execution_success {
            entry.tags.push("success".to_string());
        } else {
            entry.tags.push("failed".to_string());
        }

        // 基于用户查询的标签
        let query_lower = entry.user_query.to_lowercase();

        // 操作类型标签
        if query_lower.contains("test") {
            entry.tags.push("testing".to_string());
        }
        if query_lower.contains("build") || query_lower.contains("compile") {
            entry.tags.push("building".to_string());
        }
        if query_lower.contains("fix") || query_lower.contains("debug") {
            entry.tags.push("debugging".to_string());
        }
        if query_lower.contains("install") || query_lower.contains("setup") {
            entry.tags.push("setup".to_string());
        }
        if query_lower.contains("deploy") {
            entry.tags.push("deployment".to_string());
        }

        // 去重标签
        entry.tags.sort();
        entry.tags.dedup();
    }
}

/// 辅助函数：截断字符串
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_extraction() {
        let reflector = ReflectorMVP::new(ReflectorConfig::default());

        let user_query = "Run tests for the project";
        let assistant_response = "I'll run the tests using `cargo test`";
        let execution_result = ExecutionResult {
            success: true,
            tools_used: vec!["bash".to_string()],
            ..Default::default()
        };

        let entry = reflector
            .analyze_conversation(user_query, assistant_response, &execution_result)
            .await
            .unwrap();

        assert!(entry.tools_used.contains(&"bash".to_string()));
        assert!(entry.tags.contains(&"testing".to_string()));
        assert!(entry.patterns.contains(&"测试执行".to_string()));
    }

    #[tokio::test]
    async fn test_error_extraction() {
        let reflector = ReflectorMVP::new(ReflectorConfig::default());

        let user_query = "Fix the compilation error";
        let assistant_response = "Let me fix the error";
        let execution_result = ExecutionResult {
            success: false,
            error: Some("Compilation failed: missing semicolon".to_string()),
            errors: vec!["Compilation failed".to_string()],
            ..Default::default()
        };

        let entry = reflector
            .analyze_conversation(user_query, assistant_response, &execution_result)
            .await
            .unwrap();

        assert!(!entry.insights.is_empty());
        assert!(entry.tags.contains(&"debugging".to_string()));
        assert!(entry.tags.contains(&"error-handling".to_string()));
    }
}