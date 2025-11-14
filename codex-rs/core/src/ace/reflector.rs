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

    /// 提取工具使用洞察（增强版 - 带智能过滤）
    fn extract_tool_insights(
        &self,
        response: &str,
        context: &InsightContext,
    ) -> Result<Vec<RawInsight>> {
        let mut insights = Vec::new();
        let mut found_command = false;

        // Bash 命令
        if let Some(regex) = self.patterns.get("tool_bash") {
            for cap in regex.captures_iter(response) {
                if let Some(command) = cap.get(2) {
                    let cmd = command.as_str();

                    // 智能过滤：跳过查看类命令
                    if Self::is_read_only_command(cmd) {
                        continue;
                    }

                    // 检测是否为决策相关的命令（如安装、配置等）
                    let importance = if Self::is_decision_command(cmd) {
                        0.85 // 决策类命令更重要
                    } else {
                        0.7
                    };

                    insights.push(RawInsight {
                        content: format!("执行命令: {}", cmd),
                        category: InsightCategory::ToolUsage,
                        importance,
                        context: context.clone(),
                    });
                    found_command = true;
                }
            }
        }

        // 回退：如果没有从文本提取到工具使用，但 tools_used 不为空，则从 tools_used 生成
        if !found_command && !context.tools_used.is_empty() {
            for tool in &context.tools_used {
                insights.push(RawInsight {
                    content: format!("使用工具: {}", tool),
                    category: InsightCategory::ToolUsage,
                    importance: 0.6,
                    context: context.clone(),
                });
            }
        }

        // 文件操作（过滤读操作）
        if let Some(regex) = self.patterns.get("tool_file") {
            for cap in regex.captures_iter(response) {
                if let (Some(action), Some(path)) = (cap.get(1), cap.get(3)) {
                    let action_str = action.as_str().to_lowercase();

                    // 过滤只读操作
                    if action_str.contains("read") || action_str.contains("view") {
                        continue;
                    }

                    insights.push(RawInsight {
                        content: format!("文件操作: {} {}", action.as_str(), path.as_str()),
                        category: InsightCategory::ToolUsage,
                        importance: 0.7,
                        context: context.clone(),
                    });
                }
            }
        }

        Ok(insights)
    }

    /// 判断是否为只读命令（应该过滤）
    fn is_read_only_command(cmd: &str) -> bool {
        let cmd_lower = cmd.trim().to_lowercase();
        let read_only_commands = [
            "ls", "cat", "grep", "find", "head", "tail", "less", "more", "pwd", "which",
            "whereis", "whoami", "echo", "printf", "tree", "file", "stat", "wc", "diff",
        ];

        // 检查命令开头
        for ro_cmd in &read_only_commands {
            if cmd_lower.starts_with(ro_cmd) {
                return true;
            }
        }

        false
    }

    /// 判断是否为决策类命令（安装、配置等，重要性高）
    fn is_decision_command(cmd: &str) -> bool {
        let cmd_lower = cmd.to_lowercase();
        cmd_lower.contains("install")
            || cmd_lower.contains("npm init")
            || cmd_lower.contains("cargo new")
            || cmd_lower.contains("git init")
            || cmd_lower.contains("create-react-app")
            || cmd_lower.contains("vue create")
            || cmd_lower.starts_with("npm create")
            || cmd_lower.starts_with("npx create")
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

    /// 提取代码片段洞察（增强版 - 提取完整代码）
    fn extract_code_insights(
        &self,
        response: &str,
        context: &InsightContext,
    ) -> Result<Vec<RawInsight>> {
        let mut insights = Vec::new();

        // 代码块模式
        if let Some(regex) = self.patterns.get("code_block") {
            for cap in regex.captures_iter(response) {
                let lang_str = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let code_content = cap.get(2).map(|m| m.as_str()).unwrap_or("");

                // 过滤空代码块或太短的代码块
                if code_content.trim().len() < 10 {
                    continue;
                }

                // 计算重要性：基于代码长度和语言
                let line_count = code_content.lines().count();
                let importance = if line_count > 20 {
                    0.85 // 长代码片段更重要
                } else if line_count > 5 {
                    0.7
                } else {
                    0.5
                };

                // 生成描述性内容
                let description = if !lang_str.is_empty() {
                    format!(
                        "{} 代码实现 ({} 行)",
                        lang_str,
                        line_count
                    )
                } else {
                    format!("代码实现 ({} 行)", line_count)
                };

                // 创建包含完整代码的 insight
                let content = format!(
                    "{}\n\n```{}\n{}\n```",
                    description, lang_str, code_content
                );

                insights.push(RawInsight {
                    content,
                    category: InsightCategory::Knowledge,
                    importance,
                    context: context.clone(),
                });
            }
        }

        // 提取技术决策信息
        insights.extend(self.extract_decision_insights(response, context)?);

        // 提取 API 调用信息
        insights.extend(self.extract_api_insights(response, context)?);

        Ok(insights)
    }

    /// 提取技术决策信息
    ///
    /// 识别"为什么选择 X"、"理由是"、"因为"等决策性描述
    fn extract_decision_insights(
        &self,
        response: &str,
        context: &InsightContext,
    ) -> Result<Vec<RawInsight>> {
        let mut insights = Vec::new();

        // 技术决策关键词模式
        let decision_patterns = [
            (r"(?i)(选择|chose|using)\s+([a-zA-Z0-9\+\-\.]+).*?(因为|because|since|理由是|reason)[^\n]{10,200}", 0.9),
            (r"(?i)(技术栈|tech stack|framework)[:：]\s*([^\n]{10,150})", 0.85),
            (r"(?i)(决定|decided to|选用)\s+([^\n]{10,150})", 0.8),
            (r"(?i)(推荐|recommend|建议)\s+(使用|use|用)\s+([a-zA-Z0-9\+\-\.]+).*?([^\n]{10,150})", 0.75),
        ];

        for (pattern_str, importance) in &decision_patterns {
            if let Ok(pattern) = Regex::new(pattern_str) {
                for cap in pattern.captures_iter(response) {
                    if let Some(full_match) = cap.get(0) {
                        let decision_text = full_match.as_str().trim();

                        // 过滤太短的匹配
                        if decision_text.len() < 15 {
                            continue;
                        }

                        insights.push(RawInsight {
                            content: format!("技术决策: {}", decision_text),
                            category: InsightCategory::Knowledge,
                            importance: *importance,
                            context: context.clone(),
                        });
                    }
                }
            }
        }

        Ok(insights)
    }

    /// 提取 API 调用信息
    ///
    /// 识别常见的 API 调用模式
    fn extract_api_insights(
        &self,
        response: &str,
        context: &InsightContext,
    ) -> Result<Vec<RawInsight>> {
        let mut insights = Vec::new();

        // API 调用模式
        let api_patterns = [
            // fetch/axios 调用
            r#"(?:fetch|axios)\s*\(\s*['"]([^'"]+)['"]"#,
            // REST API 端点
            r"(?:GET|POST|PUT|DELETE|PATCH)\s+(/[^\s\)]+)",
            // GraphQL
            r"(?:query|mutation)\s+(\w+)",
        ];

        for pattern_str in &api_patterns {
            if let Ok(pattern) = Regex::new(pattern_str) {
                for cap in pattern.captures_iter(response) {
                    if let Some(api_match) = cap.get(1) {
                        let api_info = api_match.as_str();

                        insights.push(RawInsight {
                            content: format!("API 调用: {}", api_info),
                            category: InsightCategory::ToolUsage,
                            importance: 0.75,
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
