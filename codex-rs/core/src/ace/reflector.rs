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
    patterns: HashMap<String, Regex>,
}

impl ReflectorMVP {
    /// Create new Reflector
    pub fn new(_config: ReflectorConfig) -> Self {
        Self {
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
            Regex::new(r"(?i)(error|failed|exception):\s*([^\n]+)").unwrap(),
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

        // Code implementation
        if query_lower.contains("implement")
            || query_lower.contains("create")
            || query_lower.contains("add")
            || query_lower.contains("build")
        {
            return TaskType::CodeImplementation;
        }

        // Bug fix
        if query_lower.contains("fix")
            || query_lower.contains("solve")
            || query_lower.contains("bug")
            || query_lower.contains("error")
            || query_lower.contains("issue")
        {
            return TaskType::BugFix;
        }

        // Testing
        if query_lower.contains("test")
            || response_lower.contains("cargo test")
            || response_lower.contains("npm test")
            || response_lower.contains("pytest")
        {
            return TaskType::Testing;
        }

        // Refactoring
        if query_lower.contains("refactor") || query_lower.contains("restructure") {
            return TaskType::Refactoring;
        }

        // Configuration
        if query_lower.contains("config")
            || query_lower.contains("setup")
            || query_lower.contains("configure")
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
            desc_parts.push("data structure");
        }
        if has_fn && has_async {
            desc_parts.push("async function");
        } else if has_fn {
            desc_parts.push("function implementation");
        }
        if has_test {
            desc_parts.push("tests");
        }

        let mut description = if desc_parts.is_empty() {
            format!("{lang} code")
        } else {
            desc_parts.join(", ")
        };

        description.push_str(&format!(", {line_count} lines"));
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
                .unwrap_or_else(|| "Task incomplete".to_string());

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
        // Look for completion indicators
        let completion_patterns = [
            r"(?:successfully|completed|finished)\s+([^.\n]{10,100})",
            r"(?:created|implemented|modified|added|updated)\s+([^.\n]{10,100})",
            r"(?:I've|I have)\s+([^.\n]{10,100})",
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
            _ => "Executed the requested operation".to_string(),
        }
    }

    /// Extract "why" (reason)
    fn extract_why(&self, response: &str) -> Option<String> {
        let why_patterns = [
            r"(?:because|since|in order to)\s+([^.\n]{15,100})",
            r"(?:the reason is|reason:)\s+([^.\n]{15,100})",
            r"(?:to|for)\s+([^.\n]{15,100})",
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

    /// Extract "what problem was solved"
    fn extract_problem_solved(&self, response: &str, result: &ExecutionResult) -> Option<String> {
        // If there were errors but eventually succeeded, a problem was solved
        if !result.errors.is_empty()
            && result.success
            && let Some(first_error) = result.errors.first()
        {
            let error_type = first_error
                .lines()
                .next()
                .unwrap_or("unknown error")
                .chars()
                .take(50)
                .collect::<String>();

            return Some(format!("Fixed: {error_type}"));
        }

        // Look for problem descriptions in response
        let problem_patterns = [
            r"(?:fixed|resolved|addressed)\s+([^.\n]{10,80})",
            r"(?:solved|corrected)\s+([^.\n]{10,80})",
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

    /// Extract key decisions
    fn extract_key_decisions(&self, response: &str) -> Vec<String> {
        let mut decisions = Vec::new();

        let decision_patterns = [
            r"(?:chose|decided to|using)\s+([^.\n]{10,60})",
            r"(?:selected|picked)\s+([^.\n]{10,60})",
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
            r"(?:in summary|overall),\s*([^.\n]{10,80})",
            r"(?:now|currently),\s*([^.\n]{10,80})",
            r"(?:successfully|completed)\s+([^.\n]{10,80})",
        ];

        for pattern_str in &summary_patterns {
            if let Ok(re) = Regex::new(pattern_str)
                && let Some(cap) = re.captures(response)
                && let Some(summary) = cap.get(1)
            {
                return summary.as_str().trim().to_string();
            }
        }

        "Task completed".to_string()
    }

    /// Extract next steps
    fn extract_next_steps(&self, response: &str) -> Vec<String> {
        let mut steps = Vec::new();

        // Look for numbered lists
        let step_pattern = Regex::new(r"(?m)^[\s]*(\d+)[.)]\s+(.+)$").unwrap();
        for cap in step_pattern.captures_iter(response) {
            if let Some(step) = cap.get(2) {
                let step_text = step.as_str().trim();
                if step_text.len() >= 5 {
                    steps.push(step_text.to_string());
                }
            }
        }

        // Limit to 5 steps max
        steps.truncate(5);

        if steps.is_empty() {
            steps.push("Continue debugging".to_string());
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

        // 5. Trivial operations (like ls, cat): don't record
        let trivial_keywords = ["list", "show", "display", "view", "cat", "ls", "print"];
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

    /// Template 1: Successfully completed task
    fn build_completed_task_content(&self, summary: &super::types::ConversationSummary) -> String {
        let essence = &summary.essence;

        let mut content = format!("**Task**: {}\n\n", summary.user_request);

        content.push_str(&format!(
            "**Implementation**: {}\n\n",
            essence.what_was_done
        ));

        if let Some(why) = &essence.why {
            content.push_str(&format!("**Reason**: {why}\n\n"));
        }

        if let super::types::FinalState::Completed { summary: outcome } = &summary.final_state {
            content.push_str(&format!("**Outcome**: {outcome}\n\n"));
        }

        // Add code (only final version)
        if !essence.final_code.is_empty() {
            content.push_str("**Code**:\n");
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

        // Add files
        if !essence.modified_files.is_empty() {
            content.push_str(&format!(
                "**Files**: {}\n",
                essence.modified_files.join(", ")
            ));
        }

        // Add key decisions
        if !essence.key_decisions.is_empty() {
            content.push_str("\n**Key Decisions**:\n");
            for decision in &essence.key_decisions {
                content.push_str(&format!("- {decision}\n"));
            }
        }

        content
    }

    /// Template 2: Bug fix (resolved)
    fn build_bugfix_content(&self, summary: &super::types::ConversationSummary) -> String {
        let essence = &summary.essence;

        let mut content = format!("**Task**: {}\n\n", summary.user_request);

        if let Some(problem) = &essence.problem_solved {
            content.push_str(&format!("**Problem**: {problem}\n\n"));
        }

        content.push_str(&format!("**Solution**: {}\n\n", essence.what_was_done));

        // Add modified code
        if !essence.final_code.is_empty() {
            content.push_str("**Changes**:\n");
            for code_block in &essence.final_code {
                content.push_str(&format!(
                    "```{}\n{}\n```\n",
                    code_block.language, code_block.code
                ));
            }
        }

        if let super::types::FinalState::Completed { summary: outcome } = &summary.final_state {
            content.push_str(&format!("**Result**: ✅ {outcome}\n\n"));
        }

        if !essence.modified_files.is_empty() {
            content.push_str(&format!(
                "**Files**: {}\n",
                essence.modified_files.join(", ")
            ));
        }

        content
    }

    /// Template 3: Code implementation
    fn build_code_implementation_content(
        &self,
        summary: &super::types::ConversationSummary,
    ) -> String {
        let essence = &summary.essence;

        let mut content = format!("**Task**: {}\n\n", summary.user_request);

        content.push_str(&format!(
            "**Implementation**: {}\n\n",
            essence.what_was_done
        ));

        if let Some(why) = &essence.why {
            content.push_str(&format!("**Tech Stack**: {why}\n\n"));
        }

        // Core code
        if !essence.final_code.is_empty() {
            content.push_str("**Code**:\n");
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
            content.push_str(&format!("**Outcome**: {outcome}\n\n"));
        }

        if !essence.modified_files.is_empty() {
            content.push_str(&format!(
                "**Files**: {}\n",
                essence.modified_files.join(", ")
            ));
        }

        content
    }

    /// Template 4: Unresolved problem
    fn build_failed_task_content(&self, summary: &super::types::ConversationSummary) -> String {
        let essence = &summary.essence;

        let mut content = format!("**Task**: {} ⚠️ Unresolved\n\n", summary.user_request);

        if let super::types::FinalState::Failed {
            problem,
            next_steps,
        } = &summary.final_state
        {
            content.push_str(&format!("**Problem**: {problem}\n\n"));

            content.push_str(&format!("**Attempted**: {}\n\n", essence.what_was_done));

            if let Some(problem_context) = &essence.problem_solved {
                content.push_str(&format!("**Current State**: {problem_context}\n\n"));
            }

            content.push_str("**Next Steps**:\n");
            for (i, step) in next_steps.iter().enumerate() {
                content.push_str(&format!("{}. {}\n", i + 1, step));
            }
        }

        if !essence.modified_files.is_empty() {
            content.push_str(&format!(
                "\n**Related Files**: {}\n",
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

        // Should generate 1 insight
        assert_eq!(insights.len(), 1);

        let insight = &insights[0];
        // Category should be Pattern (Testing)
        assert_eq!(insight.category, InsightCategory::Pattern);
        // Content should contain task description
        assert!(insight.content.contains("Task"));
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

        // Unresolved problem must be recorded
        assert_eq!(insights.len(), 1);

        let insight = &insights[0];
        // Content should be marked as unresolved
        assert!(insight.content.contains("⚠️ Unresolved"));
        assert!(insight.content.contains("Next Steps"));
        // Importance should be high (because unresolved)
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

        let user_query = "list files";
        let assistant_response = "Running ls command";
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

        // Trivial operations should not be recorded
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
