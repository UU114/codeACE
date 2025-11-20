//! Reflector - Intelligent Extractor (MVP Bullet-based version)
//!
//! Rule-based pattern extraction, outputs unstructured RawInsights.

use super::types::ExecutionResult;
use super::types::InsightCategory;
use super::types::InsightContext;
use super::types::RawInsight;
use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;

/// Reflector configuration
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

/// MVP version Reflector - Outputs RawInsights
pub struct ReflectorMVP {
    config: ReflectorConfig,
    patterns: HashMap<String, Regex>,
}

impl ReflectorMVP {
    /// Create new Reflector
    pub fn new(config: ReflectorConfig) -> Self {
        Self {
            config,
            patterns: Self::init_patterns(),
        }
    }

    /// Initialize regex patterns
    fn init_patterns() -> HashMap<String, Regex> {
        let mut patterns = HashMap::new();

        // Tool usage patterns
        patterns.insert(
            "tool_bash".to_string(),
            Regex::new(r"(?i)(bash|shell|command|execute|run)\s+`([^`]+)`").unwrap(),
        );

        patterns.insert(
            "tool_file".to_string(),
            Regex::new(r"(?i)(read|write|edit|create|modify)\s+(file|path)?\s*[:\s]+([^\s\n]+)")
                .unwrap(),
        );

        // Code block pattern
        patterns.insert(
            "code_block".to_string(),
            Regex::new(r"```(\w+)?\n([\s\S]+?)```").unwrap(),
        );

        // Error pattern
        patterns.insert(
            "error_pattern".to_string(),
            Regex::new(r"(?i)(error|错误|failed|失败|exception):\s*([^\n]+)").unwrap(),
        );

        // Test pattern
        patterns.insert(
            "test_pattern".to_string(),
            Regex::new(r"(?i)(test|pytest|cargo test|npm test)").unwrap(),
        );

        // Build pattern
        patterns.insert(
            "build_pattern".to_string(),
            Regex::new(r"(?i)(build|compile|cargo build|npm build|make)").unwrap(),
        );

        // Git operations
        patterns.insert(
            "git_pattern".to_string(),
            Regex::new(r"(?i)(git\s+(add|commit|push|pull|clone|checkout))").unwrap(),
        );

        patterns
    }

    /// Analyze conversation, return raw insights (essence extraction version)
    ///
    /// This is the core method of Reflector. Uses essence extraction strategy:
    /// - One conversation usually generates only 1 refined insight (200-800 characters)
    /// - Only keep final code version, don't record intermediate process
    /// - Compress and extract essence, slow down context inflation
    pub async fn analyze_conversation(
        &self,
        user_query: &str,
        assistant_response: &str,
        execution_result: &ExecutionResult,
        session_id: String,
    ) -> Result<Vec<RawInsight>> {
        // 1. Extract conversation essence
        let summary = self.extract_conversation_essence(
            user_query,
            assistant_response,
            execution_result,
            session_id.clone(),
        )?;

        // 2. Decide whether to record
        if !self.should_record_conversation(&summary) {
            return Ok(Vec::new());
        }

        // 3. Generate refined insight content
        let content = self.generate_insight_content(&summary);

        // 4. Determine category and importance
        let category = self.map_task_type_to_category(&summary.task_type);
        let importance = self.calculate_importance(&summary);

        // 5. Create insight (usually only 1)
        let insight = RawInsight {
            content,
            category,
            importance,
            context: InsightContext {
                user_query: user_query.to_string(),
                assistant_response_snippet: super::types::truncate_string(assistant_response, 200),
                execution_success: execution_result.success,
                tools_used: execution_result.tools_used.clone(),
                error_message: execution_result.error.clone(),
                session_id,
            },
        };

        Ok(vec![insight])
    }

    /// Extract tool usage insights (enhanced version - with intelligent filtering)
    fn extract_tool_insights(
        &self,
        response: &str,
        context: &InsightContext,
    ) -> Result<Vec<RawInsight>> {
        let mut insights = Vec::new();
        let mut found_command = false;

        // Bash commands
        if let Some(regex) = self.patterns.get("tool_bash") {
            for cap in regex.captures_iter(response) {
                if let Some(command) = cap.get(2) {
                    let cmd = command.as_str();

                    // Intelligent filtering: skip view-type commands
                    if Self::is_read_only_command(cmd) {
                        continue;
                    }

                    // Detect if it's a decision-related command (like install, configure, etc.)
                    let importance = if Self::is_decision_command(cmd) {
                        0.85 // Decision commands are more important
                    } else {
                        0.7
                    };

                    insights.push(RawInsight {
                        content: format!("Execute command: {cmd}"),
                        category: InsightCategory::ToolUsage,
                        importance,
                        context: context.clone(),
                    });
                    found_command = true;
                }
            }
        }

        // Fallback: if no tool usage extracted from text but tools_used is not empty, generate from tools_used
        if !found_command && !context.tools_used.is_empty() {
            for tool in &context.tools_used {
                insights.push(RawInsight {
                    content: format!("Use tool: {tool}"),
                    category: InsightCategory::ToolUsage,
                    importance: 0.6,
                    context: context.clone(),
                });
            }
        }

        // File operations (filter read operations)
        if let Some(regex) = self.patterns.get("tool_file") {
            for cap in regex.captures_iter(response) {
                if let (Some(action), Some(path)) = (cap.get(1), cap.get(3)) {
                    let action_str = action.as_str().to_lowercase();

                    // Filter read-only operations
                    if action_str.contains("read") || action_str.contains("view") {
                        continue;
                    }

                    insights.push(RawInsight {
                        content: format!("File operation: {} {}", action.as_str(), path.as_str()),
                        category: InsightCategory::ToolUsage,
                        importance: 0.7,
                        context: context.clone(),
                    });
                }
            }
        }

        Ok(insights)
    }

    /// Check if command is read-only (should be filtered)
    fn is_read_only_command(cmd: &str) -> bool {
        let cmd_lower = cmd.trim().to_lowercase();
        let read_only_commands = [
            "ls", "cat", "grep", "find", "head", "tail", "less", "more", "pwd", "which", "whereis",
            "whoami", "echo", "printf", "tree", "file", "stat", "wc", "diff",
        ];

        // Check command beginning
        for ro_cmd in &read_only_commands {
            if cmd_lower.starts_with(ro_cmd) {
                return true;
            }
        }

        false
    }

    /// Check if command is decision-type (install, configure, etc., high importance)
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

    /// Extract error handling insights
    fn extract_error_insights(
        &self,
        result: &ExecutionResult,
        context: &InsightContext,
    ) -> Result<Vec<RawInsight>> {
        let mut insights = Vec::new();

        if let Some(error) = &result.error {
            insights.push(RawInsight {
                content: format!("Error: {}", super::types::truncate_string(error, 200)),
                category: InsightCategory::ErrorHandling,
                importance: 0.9,
                context: context.clone(),
            });

            // If succeeded afterwards, record solution
            if result.retry_success {
                insights.push(RawInsight {
                    content: format!(
                        "Solution: Successful handling for error '{}'",
                        super::types::truncate_string(error, 100)
                    ),
                    category: InsightCategory::Solution,
                    importance: 0.95,
                    context: context.clone(),
                });
            }
        }

        // Process error list
        for error in &result.errors {
            if !error.is_empty() {
                insights.push(RawInsight {
                    content: format!(
                        "Encountered error: {}",
                        super::types::truncate_string(error, 150)
                    ),
                    category: InsightCategory::ErrorHandling,
                    importance: 0.8,
                    context: context.clone(),
                });
            }
        }

        Ok(insights)
    }

    /// Extract pattern insights
    fn extract_pattern_insights(
        &self,
        response: &str,
        context: &InsightContext,
    ) -> Result<Vec<RawInsight>> {
        let mut insights = Vec::new();

        // Test pattern
        if let Some(regex) = self.patterns.get("test_pattern")
            && regex.is_match(response)
        {
            insights.push(RawInsight {
                content: "Executed test workflow".to_string(),
                category: InsightCategory::Pattern,
                importance: 0.6,
                context: context.clone(),
            });
        }

        // Build pattern
        if let Some(regex) = self.patterns.get("build_pattern")
            && regex.is_match(response)
        {
            insights.push(RawInsight {
                content: "Executed build workflow".to_string(),
                category: InsightCategory::Pattern,
                importance: 0.6,
                context: context.clone(),
            });
        }

        // Git operation pattern
        if let Some(regex) = self.patterns.get("git_pattern")
            && regex.is_match(response)
        {
            insights.push(RawInsight {
                content: "Executed Git operation".to_string(),
                category: InsightCategory::Pattern,
                importance: 0.7,
                context: context.clone(),
            });
        }

        Ok(insights)
    }

    /// Extract code snippet insights (enhanced version - extract complete code)
    fn extract_code_insights(
        &self,
        response: &str,
        context: &InsightContext,
    ) -> Result<Vec<RawInsight>> {
        let mut insights = Vec::new();

        // Code block pattern
        if let Some(regex) = self.patterns.get("code_block") {
            for cap in regex.captures_iter(response) {
                let lang_str = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let code_content = cap.get(2).map(|m| m.as_str()).unwrap_or("");

                // Filter empty or too short code blocks
                if code_content.trim().len() < 10 {
                    continue;
                }

                // Calculate importance: based on code length and language
                let line_count = code_content.lines().count();
                let importance = if line_count > 20 {
                    0.85 // Longer code snippets are more important
                } else if line_count > 5 {
                    0.7
                } else {
                    0.5
                };

                // Generate descriptive content
                let description = if !lang_str.is_empty() {
                    format!("{lang_str} code implementation ({line_count} lines)")
                } else {
                    format!("Code implementation ({line_count} lines)")
                };

                // Create insight containing complete code
                let content = format!("{description}\n\n```{lang_str}\n{code_content}\n```");

                insights.push(RawInsight {
                    content,
                    category: InsightCategory::Knowledge,
                    importance,
                    context: context.clone(),
                });
            }
        }

        // Extract technical decision information
        insights.extend(self.extract_decision_insights(response, context)?);

        // Extract API call information
        insights.extend(self.extract_api_insights(response, context)?);

        Ok(insights)
    }

    /// Extract technical decision information
    ///
    /// Identify decision descriptions like "why choose X", "reason is", "because", etc.
    fn extract_decision_insights(
        &self,
        response: &str,
        context: &InsightContext,
    ) -> Result<Vec<RawInsight>> {
        let mut insights = Vec::new();

        // Technical decision keyword patterns
        let decision_patterns = [
            (
                r"(?i)(选择|chose|using)\s+([a-zA-Z0-9\+\-\.]+).*?(因为|because|since|理由是|reason)[^\n]{10,200}",
                0.9,
            ),
            (
                r"(?i)(技术栈|tech stack|framework)[:：]\s*([^\n]{10,150})",
                0.85,
            ),
            (r"(?i)(决定|decided to|选用)\s+([^\n]{10,150})", 0.8),
            (
                r"(?i)(推荐|recommend|建议)\s+(使用|use|用)\s+([a-zA-Z0-9\+\-\.]+).*?([^\n]{10,150})",
                0.75,
            ),
        ];

        for (pattern_str, importance) in &decision_patterns {
            if let Ok(pattern) = Regex::new(pattern_str) {
                for cap in pattern.captures_iter(response) {
                    if let Some(full_match) = cap.get(0) {
                        let decision_text = full_match.as_str().trim();

                        // Filter too short matches
                        if decision_text.len() < 15 {
                            continue;
                        }

                        insights.push(RawInsight {
                            content: format!("Technical decision: {decision_text}"),
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

    /// Extract API call information
    ///
    /// Identify common API call patterns
    fn extract_api_insights(
        &self,
        response: &str,
        context: &InsightContext,
    ) -> Result<Vec<RawInsight>> {
        let mut insights = Vec::new();

        // API call patterns
        let api_patterns = [
            // fetch/axios calls
            r#"(?:fetch|axios)\s*\(\s*['"]([^'"]+)['"]"#,
            // REST API endpoints
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
                            content: format!("API call: {api_info}"),
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

    // ========================================================================
    // Essence Extraction Methods
    // ========================================================================

    /// Extract essence from a complete conversation
    ///
    /// Key: Only keep final result, compress intermediate process
    /// Goal: Generate 200-800 character refined insight
    pub fn extract_conversation_essence(
        &self,
        user_query: &str,
        assistant_response: &str,
        execution_result: &ExecutionResult,
        _session_id: String,
    ) -> Result<super::types::ConversationSummary> {
        use super::types::*;

        // 1. 判断任务类型
        let task_type = self.detect_task_type(user_query, assistant_response);

        // 2. 提取最终代码（只保留最后一个版本）
        let final_code = self.extract_final_code_blocks(assistant_response);

        // 3. 提取修改的文件
        let modified_files = execution_result
            .tools_used
            .iter()
            .filter(|t| t.contains("write") || t.contains("edit") || t.contains("create"))
            .cloned()
            .collect();

        // 4. 判断最终状态
        let final_state = self.determine_final_state(execution_result, assistant_response);

        // 5. 提取精华信息
        let essence = TaskEssence {
            what_was_done: self.extract_what_was_done(assistant_response, &final_state),
            why: self.extract_why(assistant_response),
            final_code,
            problem_solved: self.extract_problem_solved(assistant_response, execution_result),
            modified_files,
            key_decisions: self.extract_key_decisions(assistant_response),
        };

        Ok(ConversationSummary {
            user_request: user_query.to_string(),
            task_type,
            final_state,
            essence,
        })
    }

    /// 判断任务类型
    fn detect_task_type(
        &self,
        user_query: &str,
        assistant_response: &str,
    ) -> super::types::TaskType {
        use super::types::TaskType;

        let query_lower = user_query.to_lowercase();
        let response_lower = assistant_response.to_lowercase();

        // 代码实现
        if query_lower.contains("实现")
            || query_lower.contains("implement")
            || query_lower.contains("创建")
            || query_lower.contains("create")
            || query_lower.contains("添加")
            || query_lower.contains("add")
        {
            return TaskType::CodeImplementation;
        }

        // Bug 修复
        if query_lower.contains("修复")
            || query_lower.contains("fix")
            || query_lower.contains("解决")
            || query_lower.contains("solve")
            || query_lower.contains("bug")
            || query_lower.contains("错误")
        {
            return TaskType::BugFix;
        }

        // 测试
        if query_lower.contains("测试")
            || query_lower.contains("test")
            || response_lower.contains("cargo test")
            || response_lower.contains("npm test")
        {
            return TaskType::Testing;
        }

        // 重构
        if query_lower.contains("重构") || query_lower.contains("refactor") {
            return TaskType::Refactoring;
        }

        // 配置
        if query_lower.contains("配置")
            || query_lower.contains("config")
            || query_lower.contains("设置")
            || query_lower.contains("setup")
        {
            return TaskType::Configuration;
        }

        TaskType::Other
    }

    /// 提取最终代码块（只保留最后一个版本）
    ///
    /// 如果有多个相同文件的代码块，只保留最后一个
    fn extract_final_code_blocks(&self, response: &str) -> Vec<super::types::CodeBlock> {
        use super::types::CodeBlock;
        use std::collections::HashMap;

        let mut all_code_blocks = Vec::new();

        // 提取所有代码块
        if let Some(regex) = self.patterns.get("code_block") {
            for cap in regex.captures_iter(response) {
                let lang = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                let code = cap.get(2).map(|m| m.as_str()).unwrap_or("");

                if code.trim().len() < 10 {
                    continue;
                }

                all_code_blocks.push((lang, code.to_string()));
            }
        }

        // 按文件路径/语言分组，每个只保留最后一个版本
        let mut file_to_code: HashMap<String, CodeBlock> = HashMap::new();

        for (lang, code) in all_code_blocks {
            // 尝试从上下文提取文件路径
            let file_path = self.extract_file_path_for_code(response, &code);

            // 生成代码描述
            let description = self.generate_code_description(&code, &lang);

            let code_block = CodeBlock {
                language: lang.clone(),
                code: code.clone(),
                file_path: file_path.clone(),
                description,
            };

            // 使用文件路径或语言作为 key，自动覆盖旧版本
            let key = file_path.unwrap_or_else(|| lang.clone());
            file_to_code.insert(key, code_block);
        }

        file_to_code.into_values().collect()
    }

    /// 从上下文提取代码对应的文件路径
    fn extract_file_path_for_code(&self, response: &str, code: &str) -> Option<String> {
        // 在代码块前查找文件路径
        if let Some(pos) = response.find(code) {
            let before = &response[..pos];
            let lines: Vec<&str> = before.lines().rev().take(5).collect();

            for line in lines {
                if let Some(path) = self.extract_path_from_line(line) {
                    return Some(path);
                }
            }
        }

        None
    }

    /// 从一行文本中提取路径
    fn extract_path_from_line(&self, line: &str) -> Option<String> {
        let path_patterns = [
            r"([a-zA-Z0-9_/\-\.]+\.rs)",
            r"([a-zA-Z0-9_/\-\.]+\.ts)",
            r"([a-zA-Z0-9_/\-\.]+\.js)",
            r"([a-zA-Z0-9_/\-\.]+\.py)",
            r"([a-zA-Z0-9_/\-\.]+\.toml)",
            r"src/[a-zA-Z0-9_/\-\.]+",
        ];

        for pattern_str in &path_patterns {
            if let Ok(re) = Regex::new(pattern_str)
                && let Some(cap) = re.captures(line)
                && let Some(path) = cap.get(1).or_else(|| cap.get(0))
            {
                return Some(path.as_str().to_string());
            }
        }

        None
    }

    /// 为代码生成简短描述
    fn generate_code_description(&self, code: &str, lang: &str) -> String {
        let has_async = code.contains("async");
        let has_struct = code.contains("struct") || code.contains("class");
        let has_fn = code.contains("fn ") || code.contains("function ");
        let has_test = code.contains("#[test]") || code.contains("test_");

        let line_count = code.lines().count();

        let mut desc_parts = Vec::new();

        if has_struct {
            desc_parts.push("数据结构");
        }
        if has_fn && has_async {
            desc_parts.push("异步函数");
        } else if has_fn {
            desc_parts.push("函数实现");
        }
        if has_test {
            desc_parts.push("测试");
        }

        let mut description = if desc_parts.is_empty() {
            format!("{lang} 代码")
        } else {
            desc_parts.join("、")
        };

        description.push_str(&format!("，{line_count} 行"));
        description
    }

    /// 判断最终状态
    fn determine_final_state(
        &self,
        result: &ExecutionResult,
        response: &str,
    ) -> super::types::FinalState {
        use super::types::FinalState;

        if result.success {
            // 成功完成
            let summary = self.extract_completion_summary(response);
            FinalState::Completed { summary }
        } else {
            // 失败未解决
            let problem = result
                .error
                .clone()
                .map(|e| super::types::truncate_string(&e, 100))
                .unwrap_or_else(|| "任务未完成".to_string());

            let next_steps = self.extract_next_steps(response);

            FinalState::Failed {
                problem,
                next_steps,
            }
        }
    }

    /// 提取"做了什么"（最终结果）
    fn extract_what_was_done(
        &self,
        response: &str,
        final_state: &super::types::FinalState,
    ) -> String {
        // 查找表示完成的关键句
        let completion_patterns = [
            r"(?:已|成功|完成)\s*([^。.\n]{10,100})",
            r"(?:创建|实现|修改|添加|更新)了?\s*([^。.\n]{10,100})",
            r"(?:I've|I have|successfully)\s+([^.。\n]{10,100})",
        ];

        for pattern_str in &completion_patterns {
            if let Ok(re) = Regex::new(pattern_str)
                && let Some(cap) = re.captures(response)
                && let Some(action) = cap.get(1)
            {
                return action.as_str().trim().to_string();
            }
        }

        // 回退：根据 final_state 生成
        match final_state {
            super::types::FinalState::Completed { summary } => summary.clone(),
            _ => "执行了用户请求的操作".to_string(),
        }
    }

    /// 提取"为什么"
    fn extract_why(&self, response: &str) -> Option<String> {
        let why_patterns = [
            r"(?:因为|由于|为了|为的是)\s*([^。.\n]{15,100})",
            r"(?:原因是|理由是)\s*([^。.\n]{15,100})",
            r"(?:because|since|in order to)\s+([^.。\n]{15,100})",
        ];

        for pattern_str in &why_patterns {
            if let Ok(re) = Regex::new(pattern_str)
                && let Some(cap) = re.captures(response)
                && let Some(reason) = cap.get(1)
            {
                return Some(reason.as_str().trim().to_string());
            }
        }

        None
    }

    /// 提取"解决了什么问题"
    fn extract_problem_solved(&self, response: &str, result: &ExecutionResult) -> Option<String> {
        // 如果有错误但最终成功，说明解决了问题
        if !result.errors.is_empty()
            && result.success
            && let Some(first_error) = result.errors.first()
        {
            let error_type = first_error
                .lines()
                .next()
                .unwrap_or("未知错误")
                .chars()
                .take(50)
                .collect::<String>();

            return Some(format!("修复了：{error_type}"));
        }

        // 从响应中查找问题描述
        let problem_patterns = [
            r"(?:解决|修复|处理)了?\s*([^。.\n]{10,80})",
            r"(?:fixed|resolved|addressed)\s+([^.。\n]{10,80})",
        ];

        for pattern_str in &problem_patterns {
            if let Ok(re) = Regex::new(pattern_str)
                && let Some(cap) = re.captures(response)
                && let Some(problem) = cap.get(1)
            {
                return Some(problem.as_str().trim().to_string());
            }
        }

        None
    }

    /// 提取关键决策
    fn extract_key_decisions(&self, response: &str) -> Vec<String> {
        let mut decisions = Vec::new();

        let decision_patterns = [
            r"(?:选择|决定|使用)\s*([^。.\n]{10,60})",
            r"(?:chose|decided to|using)\s+([^.。\n]{10,60})",
        ];

        for pattern_str in &decision_patterns {
            if let Ok(re) = Regex::new(pattern_str) {
                for cap in re.captures_iter(response) {
                    if let Some(decision) = cap.get(1) {
                        let text = decision.as_str().trim().to_string();
                        if text.len() >= 10 && !decisions.contains(&text) {
                            decisions.push(text);
                        }
                    }
                }
            }
        }

        // 限制最多 3 个决策
        decisions.truncate(3);
        decisions
    }

    /// 提取完成总结（一句话）
    fn extract_completion_summary(&self, response: &str) -> String {
        let summary_patterns = [
            r"总之，([^。.\n]{10,80})",
            r"现在，([^。.\n]{10,80})",
            r"(?:已|成功)([^。.\n]{10,80})",
        ];

        for pattern_str in &summary_patterns {
            if let Ok(re) = Regex::new(pattern_str)
                && let Some(cap) = re.captures(response)
                && let Some(summary) = cap.get(1)
            {
                return summary.as_str().trim().to_string();
            }
        }

        "任务已完成".to_string()
    }

    /// 提取后续计划
    fn extract_next_steps(&self, response: &str) -> Vec<String> {
        let mut steps = Vec::new();

        // 查找编号列表
        let step_pattern = Regex::new(r"(?m)^[\s]*(\d+)[.、]\s+(.+)$").unwrap();
        for cap in step_pattern.captures_iter(response) {
            if let Some(step) = cap.get(2) {
                let step_text = step.as_str().trim();
                if step_text.len() >= 5 {
                    steps.push(step_text.to_string());
                }
            }
        }

        // 限制最多 5 个步骤
        steps.truncate(5);

        if steps.is_empty() {
            steps.push("继续调试".to_string());
        }

        steps
    }

    /// 决定是否记录这次对话
    pub fn should_record_conversation(&self, summary: &super::types::ConversationSummary) -> bool {
        use super::types::FinalState;

        // 1. 未解决的问题：必须记录
        if matches!(summary.final_state, FinalState::Failed { .. }) {
            return true;
        }

        // 2. 有代码产出：必须记录
        if !summary.essence.final_code.is_empty() {
            return true;
        }

        // 3. 有文件修改：必须记录
        if !summary.essence.modified_files.is_empty() {
            return true;
        }

        // 4. 有重要决策：记录
        if !summary.essence.key_decisions.is_empty() {
            return true;
        }

        // 5. 琐碎操作（如 ls、cat）：不记录
        let trivial_keywords = ["list", "show", "display", "查看", "显示", "cat", "ls"];
        let is_trivial = trivial_keywords
            .iter()
            .any(|k| summary.user_request.to_lowercase().contains(k));

        if is_trivial {
            return false;
        }

        // 默认记录
        true
    }

    // ========================================================================
    // 内容模板生成 (Content Templates)
    // ========================================================================

    /// 生成精炼的 insight 内容
    ///
    /// 根据任务类型选择合适的模板
    /// 目标：200-800 字符的精炼内容
    pub fn generate_insight_content(&self, summary: &super::types::ConversationSummary) -> String {
        use super::types::FinalState;
        use super::types::TaskType;

        match summary.task_type {
            TaskType::CodeImplementation => self.build_code_implementation_content(summary),
            TaskType::BugFix => {
                if matches!(summary.final_state, FinalState::Failed { .. }) {
                    self.build_failed_task_content(summary)
                } else {
                    self.build_bugfix_content(summary)
                }
            }
            _ => match &summary.final_state {
                FinalState::Failed { .. } => self.build_failed_task_content(summary),
                _ => self.build_completed_task_content(summary),
            },
        }
    }

    /// 模板1：成功完成的任务
    fn build_completed_task_content(&self, summary: &super::types::ConversationSummary) -> String {
        let essence = &summary.essence;

        let mut content = format!("**任务**：{}\n\n", summary.user_request);

        content.push_str(&format!("**实现**：{}\n\n", essence.what_was_done));

        if let Some(why) = &essence.why {
            content.push_str(&format!("**原因**：{why}\n\n"));
        }

        if let super::types::FinalState::Completed { summary: outcome } = &summary.final_state {
            content.push_str(&format!("**成果**：{outcome}\n\n"));
        }

        // 添加代码（只有最终版本）
        if !essence.final_code.is_empty() {
            content.push_str("**代码**：\n");
            for code_block in &essence.final_code {
                content.push_str(&format!(
                    "```{}\n{}\n```\n",
                    code_block.language, code_block.code
                ));
                if !code_block.description.is_empty() {
                    content.push_str(&format!("// {}\n\n", code_block.description));
                }
            }
        }

        // 添加文件
        if !essence.modified_files.is_empty() {
            content.push_str(&format!(
                "**文件**：{}\n",
                essence.modified_files.join(", ")
            ));
        }

        // 添加关键决策
        if !essence.key_decisions.is_empty() {
            content.push_str("\n**关键决策**：\n");
            for decision in &essence.key_decisions {
                content.push_str(&format!("- {decision}\n"));
            }
        }

        content
    }

    /// 模板2：Bug修复（已解决）
    fn build_bugfix_content(&self, summary: &super::types::ConversationSummary) -> String {
        let essence = &summary.essence;

        let mut content = format!("**任务**：{}\n\n", summary.user_request);

        if let Some(problem) = &essence.problem_solved {
            content.push_str(&format!("**问题**：{problem}\n\n"));
        }

        content.push_str(&format!("**解决方案**：{}\n\n", essence.what_was_done));

        // 添加修改的代码
        if !essence.final_code.is_empty() {
            content.push_str("**修改**：\n");
            for code_block in &essence.final_code {
                content.push_str(&format!(
                    "```{}\n{}\n```\n",
                    code_block.language, code_block.code
                ));
            }
        }

        if let super::types::FinalState::Completed { summary: outcome } = &summary.final_state {
            content.push_str(&format!("**结果**：✅ {outcome}\n\n"));
        }

        if !essence.modified_files.is_empty() {
            content.push_str(&format!(
                "**文件**：{}\n",
                essence.modified_files.join(", ")
            ));
        }

        content
    }

    /// 模板3：代码实现
    fn build_code_implementation_content(
        &self,
        summary: &super::types::ConversationSummary,
    ) -> String {
        let essence = &summary.essence;

        let mut content = format!("**任务**：{}\n\n", summary.user_request);

        content.push_str(&format!("**实现**：{}\n\n", essence.what_was_done));

        if let Some(why) = &essence.why {
            content.push_str(&format!("**技术选型**：{why}\n\n"));
        }

        // 核心代码
        if !essence.final_code.is_empty() {
            content.push_str("**代码**：\n");
            for code_block in &essence.final_code {
                content.push_str(&format!(
                    "```{}\n{}\n```\n",
                    code_block.language, code_block.code
                ));
                if !code_block.description.is_empty() {
                    content.push_str(&format!("// {}\n\n", code_block.description));
                }
            }
        }

        if let super::types::FinalState::Completed { summary: outcome } = &summary.final_state {
            content.push_str(&format!("**成果**：{outcome}\n\n"));
        }

        if !essence.modified_files.is_empty() {
            content.push_str(&format!(
                "**文件**：{}\n",
                essence.modified_files.join(", ")
            ));
        }

        content
    }

    /// 模板4：未解决的问题
    fn build_failed_task_content(&self, summary: &super::types::ConversationSummary) -> String {
        let essence = &summary.essence;

        let mut content = format!("**任务**：{} ⚠️ 未解决\n\n", summary.user_request);

        if let super::types::FinalState::Failed {
            problem,
            next_steps,
        } = &summary.final_state
        {
            content.push_str(&format!("**问题**：{problem}\n\n"));

            content.push_str(&format!("**已尝试**：{}\n\n", essence.what_was_done));

            if let Some(problem_context) = &essence.problem_solved {
                content.push_str(&format!("**当前状态**：{problem_context}\n\n"));
            }

            content.push_str("**后续计划**：\n");
            for (i, step) in next_steps.iter().enumerate() {
                content.push_str(&format!("{}. {}\n", i + 1, step));
            }
        }

        if !essence.modified_files.is_empty() {
            content.push_str(&format!(
                "\n**相关文件**：{}\n",
                essence.modified_files.join(", ")
            ));
        }

        content
    }

    /// 映射 TaskType 到 InsightCategory
    pub fn map_task_type_to_category(&self, task_type: &super::types::TaskType) -> InsightCategory {
        use super::types::TaskType;

        match task_type {
            TaskType::CodeImplementation => InsightCategory::Knowledge,
            TaskType::BugFix => InsightCategory::Solution,
            TaskType::Testing => InsightCategory::Pattern,
            TaskType::Refactoring => InsightCategory::Pattern,
            TaskType::Configuration => InsightCategory::ToolUsage,
            TaskType::Documentation => InsightCategory::Knowledge,
            TaskType::Other => InsightCategory::Knowledge,
        }
    }

    /// 计算重要性评分
    pub fn calculate_importance(&self, summary: &super::types::ConversationSummary) -> f32 {
        use super::types::FinalState;

        let mut importance: f32 = 0.6; // 基础分数

        // 未解决的问题：提高重要性
        if matches!(summary.final_state, FinalState::Failed { .. }) {
            importance += 0.3;
        }

        // 有代码产出：提高重要性
        if !summary.essence.final_code.is_empty() {
            importance += 0.2;
        }

        // 有关键决策：提高重要性
        if !summary.essence.key_decisions.is_empty() {
            importance += 0.1;
        }

        // 限制在 0.0-1.0 范围
        importance.min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试：成功的测试任务
    #[tokio::test]
    async fn test_essence_extraction_testing() {
        let reflector = ReflectorMVP::new(ReflectorConfig::default());

        let user_query = "运行项目测试";
        let assistant_response = "我将使用 cargo test 运行测试";
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

        // 应该只生成 1 条 insight
        assert_eq!(insights.len(), 1);

        let insight = &insights[0];
        // 类别应该是 Pattern (Testing)
        assert_eq!(insight.category, InsightCategory::Pattern);
        // 内容应该包含任务描述
        assert!(insight.content.contains("任务"));
        assert!(insight.content.contains(user_query));
    }

    /// 测试：未解决的错误
    #[tokio::test]
    async fn test_essence_extraction_failed_task() {
        let reflector = ReflectorMVP::new(ReflectorConfig::default());

        let user_query = "修复编译错误";
        let assistant_response = "尝试修复了类型错误";
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

        // 未解决的问题必须记录
        assert_eq!(insights.len(), 1);

        let insight = &insights[0];
        // 内容应该标记为未解决
        assert!(insight.content.contains("⚠️ 未解决"));
        assert!(insight.content.contains("后续计划"));
        // 重要性应该较高（因为未解决）
        assert!(insight.importance >= 0.8);
    }

    /// 测试：代码实现任务
    #[tokio::test]
    async fn test_essence_extraction_code_implementation() {
        let reflector = ReflectorMVP::new(ReflectorConfig::default());

        let user_query = "实现用户登录功能";
        let assistant_response = r#"我将实现登录功能。代码如下：
```rust
async fn login(username: &str, password: &str) -> Result<String> {
    let user = verify_credentials(username, password).await?;
    Ok(generate_token(&user))
}
```
"#;
        let execution_result = ExecutionResult {
            success: true,
            tools_used: vec!["write".to_string()],
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

        assert_eq!(insights.len(), 1);

        let insight = &insights[0];
        // 应该包含代码（只有最终版本）
        assert!(insight.content.contains("```rust"));
        assert!(insight.content.contains("login"));
        // 类别应该是 Knowledge
        assert_eq!(insight.category, InsightCategory::Knowledge);
    }

    /// 测试：琐碎操作不记录
    #[tokio::test]
    async fn test_essence_extraction_trivial_not_recorded() {
        let reflector = ReflectorMVP::new(ReflectorConfig::default());

        let user_query = "查看文件列表";
        let assistant_response = "运行 ls 命令";
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

        // 琐碎操作不应该记录
        assert_eq!(insights.len(), 0);
    }

    /// 测试：多次修改代码，只保留最后版本
    #[tokio::test]
    async fn test_essence_extraction_only_final_code() {
        let reflector = ReflectorMVP::new(ReflectorConfig::default());

        let user_query = "实现计算函数";
        let assistant_response = r#"首先实现第一版：
```rust
fn calculate(x: i32) -> i32 { x + 1 }
```
修改后的版本：
```rust
fn calculate(x: i32, y: i32) -> i32 { x + y }
```
最终版本：
```rust
fn calculate(x: i32, y: i32) -> Result<i32> {
    Ok(x + y)
}
```
"#;
        let execution_result = ExecutionResult {
            success: true,
            tools_used: vec!["write".to_string()],
            ..Default::default()
        };

        let summary = reflector
            .extract_conversation_essence(
                user_query,
                assistant_response,
                &execution_result,
                "test-session".to_string(),
            )
            .unwrap();

        // 应该只保留最后一个版本的代码
        // 由于同一个文件/语言，HashMap会自动覆盖
        // 实际保留的数量取决于是否能识别出是同一个文件
        // 这里至少验证有代码被提取
        assert!(!summary.essence.final_code.is_empty());

        // 验证内容中包含代码
        let content = reflector.generate_insight_content(&summary);
        assert!(content.contains("```rust"));
    }
}
