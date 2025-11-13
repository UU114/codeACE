//! Reflector - 智能提取器（MVP Bullet-based 版本）
//!
//! 基于规则的模式提取，输出未结构化的 RawInsights。

use super::types::{ExecutionResult, InsightCategory, InsightContext, RawInsight};
use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;

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

impl From<super::types::ReflectorConfig> for ReflectorConfig {
    fn from(config: super::types::ReflectorConfig) -> Self {
        Self {
            extract_patterns: config.extract_patterns,
            extract_tools: config.extract_tools,
            extract_errors: config.extract_errors,
        }
    }
}

/// MVP版Reflector - 输出 RawInsights
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
            Regex::new(r"(?i)(read|write|edit|create|modify)\s+(file|path)?\s*[:\s]+([^\s\n]+)")
                .unwrap(),
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

    /// 分析对话，返回原始洞察
    ///
    /// 这是 Reflector 的核心方法，它不再生成 PlaybookEntry，
    /// 而是返回未结构化的 RawInsight 列表。
    pub async fn analyze_conversation(
        &self,
        user_query: &str,
        assistant_response: &str,
        execution_result: &ExecutionResult,
        session_id: String,
    ) -> Result<Vec<RawInsight>> {
        let mut insights = Vec::new();

        // 构建上下文（为 Curator 提供足够信息）
        let context = InsightContext {
            user_query: user_query.to_string(),
            assistant_response_snippet: super::types::truncate_string(assistant_response, 500),
            execution_success: execution_result.success,
            tools_used: execution_result.tools_used.clone(),
            error_message: execution_result.error.clone(),
            session_id,
        };

        // 提取各类洞察
        if self.config.extract_tools {
            insights.extend(self.extract_tool_insights(assistant_response, &context)?);
        }

        if self.config.extract_errors && !execution_result.success {
            insights.extend(self.extract_error_insights(execution_result, &context)?);
        }

        if self.config.extract_patterns {
            insights.extend(self.extract_pattern_insights(assistant_response, &context)?);
        }

        // 提取代码片段 insights
        insights.extend(self.extract_code_insights(assistant_response, &context)?);

        Ok(insights)
    }

    /// 提取工具使用洞察
    fn extract_tool_insights(
        &self,
        response: &str,
        context: &InsightContext,
    ) -> Result<Vec<RawInsight>> {
        let mut insights = Vec::new();

        // Bash 命令
        if let Some(regex) = self.patterns.get("tool_bash") {
            for cap in regex.captures_iter(response) {
                if let Some(command) = cap.get(2) {
                    insights.push(RawInsight {
                        content: format!("使用命令: {}", command.as_str()),
                        category: InsightCategory::ToolUsage,
                        importance: 0.7,
                        context: context.clone(),
                    });
                }
            }
        }

        // 文件操作
        if let Some(regex) = self.patterns.get("tool_file") {
            for cap in regex.captures_iter(response) {
                if let (Some(action), Some(path)) = (cap.get(1), cap.get(3)) {
                    insights.push(RawInsight {
                        content: format!("文件操作: {} {}", action.as_str(), path.as_str()),
                        category: InsightCategory::ToolUsage,
                        importance: 0.6,
                        context: context.clone(),
                    });
                }
            }
        }

        Ok(insights)
    }

    /// 提取错误处理洞察
    fn extract_error_insights(
        &self,
        result: &ExecutionResult,
        context: &InsightContext,
    ) -> Result<Vec<RawInsight>> {
        let mut insights = Vec::new();

        if let Some(error) = &result.error {
            insights.push(RawInsight {
                content: format!("错误: {}", super::types::truncate_string(error, 200)),
                category: InsightCategory::ErrorHandling,
                importance: 0.9,
                context: context.clone(),
            });

            // 如果后续成功，记录解决方案
            if result.retry_success {
                insights.push(RawInsight {
                    content: format!(
                        "解决方案: 针对错误 '{}' 的成功处理",
                        super::types::truncate_string(error, 100)
                    ),
                    category: InsightCategory::Solution,
                    importance: 0.95,
                    context: context.clone(),
                });
            }
        }

        // 处理错误列表
        for error in &result.errors {
            if !error.is_empty() {
                insights.push(RawInsight {
                    content: format!("遇到错误: {}", super::types::truncate_string(error, 150)),
                    category: InsightCategory::ErrorHandling,
                    importance: 0.8,
                    context: context.clone(),
                });
            }
        }

        Ok(insights)
    }

    /// 提取模式洞察
    fn extract_pattern_insights(
        &self,
        response: &str,
        context: &InsightContext,
    ) -> Result<Vec<RawInsight>> {
        let mut insights = Vec::new();

        // 测试模式
        if let Some(regex) = self.patterns.get("test_pattern") {
            if regex.is_match(response) {
                insights.push(RawInsight {
                    content: "执行了测试流程".to_string(),
                    category: InsightCategory::Pattern,
                    importance: 0.6,
                    context: context.clone(),
                });
            }
        }

        // 构建模式
        if let Some(regex) = self.patterns.get("build_pattern") {
            if regex.is_match(response) {
                insights.push(RawInsight {
                    content: "执行了构建流程".to_string(),
                    category: InsightCategory::Pattern,
                    importance: 0.6,
                    context: context.clone(),
                });
            }
        }

        // Git操作模式
        if let Some(regex) = self.patterns.get("git_pattern") {
            if regex.is_match(response) {
                insights.push(RawInsight {
                    content: "执行了 Git 操作".to_string(),
                    category: InsightCategory::Pattern,
                    importance: 0.7,
                    context: context.clone(),
                });
            }
        }

        Ok(insights)
    }

    /// 提取代码片段洞察
    fn extract_code_insights(
        &self,
        response: &str,
        context: &InsightContext,
    ) -> Result<Vec<RawInsight>> {
        let mut insights = Vec::new();

        // 代码块模式
        if let Some(regex) = self.patterns.get("code_block") {
            for cap in regex.captures_iter(response) {
                if let Some(lang) = cap.get(1) {
                    let lang_str = lang.as_str();
                    if !lang_str.is_empty() {
                        insights.push(RawInsight {
                            content: format!("包含 {} 代码片段", lang_str),
                            category: InsightCategory::Knowledge,
                            importance: 0.5,
                            context: context.clone(),
                        });
                    }
                }
            }
        }

        Ok(insights)
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

        let insights = reflector
            .analyze_conversation(
                user_query,
                assistant_response,
                &execution_result,
                "test-session".to_string(),
            )
            .await
            .unwrap();

        assert!(!insights.is_empty());
        assert!(
            insights
                .iter()
                .any(|i| i.category == InsightCategory::ToolUsage)
        );
        assert!(
            insights
                .iter()
                .any(|i| i.category == InsightCategory::Pattern)
        );
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

        let insights = reflector
            .analyze_conversation(
                user_query,
                assistant_response,
                &execution_result,
                "test-session".to_string(),
            )
            .await
            .unwrap();

        assert!(!insights.is_empty());
        assert!(
            insights
                .iter()
                .any(|i| i.category == InsightCategory::ErrorHandling)
        );

        // 验证有多个错误 insight
        let error_insights: Vec<_> = insights
            .iter()
            .filter(|i| i.category == InsightCategory::ErrorHandling)
            .collect();
        assert!(error_insights.len() >= 2); // 至少包含 error 和 errors 列表
    }

    #[tokio::test]
    async fn test_context_propagation() {
        let reflector = ReflectorMVP::new(ReflectorConfig::default());

        let user_query = "Test context propagation";
        let assistant_response = "Running command `echo test`";
        let execution_result = ExecutionResult {
            success: true,
            tools_used: vec!["bash".to_string()],
            ..Default::default()
        };

        let session_id = "test-session-123".to_string();
        let insights = reflector
            .analyze_conversation(
                user_query,
                assistant_response,
                &execution_result,
                session_id.clone(),
            )
            .await
            .unwrap();

        // 验证 context 正确传播
        for insight in &insights {
            assert_eq!(insight.context.session_id, session_id);
            assert_eq!(insight.context.user_query, user_query);
            assert!(insight.context.execution_success);
            assert_eq!(insight.context.tools_used, vec!["bash".to_string()]);
        }
    }

    #[tokio::test]
    async fn test_retry_success_solution() {
        let reflector = ReflectorMVP::new(ReflectorConfig::default());

        let user_query = "Fix the error";
        let assistant_response = "Fixed the issue";
        let execution_result = ExecutionResult {
            success: false,
            error: Some("Initial error".to_string()),
            retry_success: true,
            ..Default::default()
        };

        let insights = reflector
            .analyze_conversation(
                user_query,
                assistant_response,
                &execution_result,
                "test-session".to_string(),
            )
            .await
            .unwrap();

        // 应该同时有错误 insight 和解决方案 insight
        assert!(
            insights
                .iter()
                .any(|i| i.category == InsightCategory::ErrorHandling)
        );
        assert!(
            insights
                .iter()
                .any(|i| i.category == InsightCategory::Solution)
        );
    }
}
