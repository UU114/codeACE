//! Curator MVP - 将洞察组织成结构化 bullets
//!
//! Curator 接收 Reflector 输出的 RawInsights，将它们组织成
//! 结构化的 Bullets，决定分类，生成元数据，并输出 DeltaContext。

use super::code_analyzer::CodeAnalyzer;
use super::content_classifier::ContentClassifier;
use super::types::Applicability;
use super::types::Bullet;
use super::types::BulletCodeContent;
use super::types::BulletMetadata;
use super::types::BulletSection;
use super::types::CuratorConfig;
use super::types::DeltaContext;
use super::types::InsightCategory;
use super::types::RawInsight;
use super::types::SourceType;
use anyhow::Result;
use regex::Regex;

/// Curator MVP - 将洞察组织成结构化 bullets
#[derive(Default)]
pub struct CuratorMVP {
    config: CuratorConfig,
    code_analyzer: CodeAnalyzer,
}

impl CuratorMVP {
    pub fn new(config: CuratorConfig) -> Self {
        Self {
            config,
            code_analyzer: CodeAnalyzer::new(),
        }
    }

    /// 处理 insights，生成 delta
    ///
    /// 这是 Curator 的核心方法，它接收一组 RawInsights，
    /// 为每个 insight 生成对应的 Bullet，并返回 DeltaContext。
    pub async fn process_insights(
        &self,
        insights: Vec<RawInsight>,
        session_id: String,
    ) -> Result<DeltaContext> {
        let start = std::time::Instant::now();
        let mut delta = DeltaContext::new(session_id.clone());

        // 1. 过滤低重要性的 insights
        let valuable_insights: Vec<_> = insights
            .into_iter()
            .filter(|i| i.importance >= self.config.min_importance)
            .collect();

        // 2. 【LAPS 新增】内容质量和长度验证
        let mut validated_insights = Vec::new();
        let mut rejected_count = 0;

        for insight in valuable_insights {
            let (valid, reason) = ContentClassifier::validate_content(&insight.content);

            if valid {
                validated_insights.push(insight);
                tracing::debug!("接受 insight: {}", reason);
            } else {
                rejected_count += 1;
                tracing::warn!("拒绝 insight: {}", reason);
            }
        }

        tracing::info!(
            "内容验证: {} 通过, {} 被拒绝",
            validated_insights.len(),
            rejected_count
        );

        delta.metadata.insights_processed = validated_insights.len();

        // 3. 为每个验证通过的 insight 生成 bullet
        for insight in validated_insights {
            let bullet = self.create_bullet_from_insight(insight, &session_id)?;
            delta.new_bullets.push(bullet);
        }

        delta.metadata.new_bullets_count = delta.new_bullets.len();
        delta.metadata.processing_time_ms = start.elapsed().as_millis() as u64;

        Ok(delta)
    }

    /// 从 insight 创建 bullet
    fn create_bullet_from_insight(&self, insight: RawInsight, session_id: &str) -> Result<Bullet> {
        // 决定 section
        let section = if self.config.auto_categorize {
            self.categorize_insight(&insight)
        } else {
            BulletSection::General
        };

        // 创建 bullet
        let mut bullet = Bullet::new(section, insight.content.clone(), session_id.to_string());

        // 提取并分析代码（如果有）
        if let Some(code_content) = self.extract_and_analyze_code(&insight.content) {
            bullet.code_content = Some(code_content);
        }

        // 填充 metadata
        bullet.metadata = self.create_metadata(&insight)?;

        // 生成标签
        if self.config.generate_tags {
            bullet.tags = self.generate_tags(&insight);
        }

        Ok(bullet)
    }

    /// 提取并分析代码内容
    ///
    /// 从 insight 内容中提取代码块，并使用 CodeAnalyzer 决定保存策略
    fn extract_and_analyze_code(&self, content: &str) -> Option<BulletCodeContent> {
        // 代码块正则
        let code_block_regex = Regex::new(r"```(\w+)?\n([\s\S]+?)\n```").ok()?;

        // 查找第一个代码块
        if let Some(cap) = code_block_regex.captures(content) {
            let language = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
            let code = cap.get(2)?.as_str();

            // 使用 CodeAnalyzer 分析代码
            let analyzed = self.code_analyzer.analyze_code(&language, code, None);

            return Some(analyzed);
        }

        None
    }

    /// 分类逻辑（规则based）
    ///
    /// 根据 insight 的类别和内容，决定 bullet 应该归属于哪个 section。
    fn categorize_insight(&self, insight: &RawInsight) -> BulletSection {
        match insight.category {
            InsightCategory::ToolUsage => {
                // 判断是否为代码片段
                if insight.content.contains("```") || insight.content.contains("代码") {
                    BulletSection::CodeSnippetsAndTemplates
                } else {
                    BulletSection::ToolUsageTips
                }
            }
            InsightCategory::ErrorHandling => BulletSection::TroubleshootingAndPitfalls,
            InsightCategory::Solution => BulletSection::TroubleshootingAndPitfalls,
            InsightCategory::Pattern => BulletSection::StrategiesAndRules,
            InsightCategory::Knowledge => {
                // 检查是否为 API 相关
                if insight.content.to_lowercase().contains("api") {
                    BulletSection::ApiUsageGuides
                } else {
                    BulletSection::General
                }
            }
        }
    }

    /// 创建细粒度 metadata
    fn create_metadata(&self, insight: &RawInsight) -> Result<BulletMetadata> {
        let success_count = if insight.context.execution_success {
            1
        } else {
            0
        };
        let failure_count = if !insight.context.execution_success {
            1
        } else {
            0
        };
        let total = success_count + failure_count;
        let success_rate = if total > 0 {
            success_count as f32 / total as f32
        } else {
            0.0
        };

        let metadata = BulletMetadata {
            importance: insight.importance,
            source_type: self.determine_source_type(insight),
            applicability: self.extract_applicability(insight),
            reference_count: 0,
            success_count,
            failure_count,
            related_tools: insight.context.tools_used.clone(),
            related_file_patterns: Vec::new(), // MVP 阶段留空
            confidence: 1.0,
            // LAPS 新增字段
            recall_count: 0,
            last_recall: None,
            recall_contexts: Vec::new(),
            success_rate,
        };

        Ok(metadata)
    }

    /// 判断来源类型
    fn determine_source_type(&self, insight: &RawInsight) -> SourceType {
        if insight.context.execution_success {
            if insight.category == InsightCategory::ErrorHandling {
                SourceType::ErrorResolution
            } else {
                SourceType::SuccessExecution
            }
        } else if insight.category == InsightCategory::Pattern {
            SourceType::PatternRecognition
        } else {
            SourceType::ErrorResolution
        }
    }

    /// 提取适用性范围
    fn extract_applicability(&self, insight: &RawInsight) -> Applicability {
        let mut applicability = Applicability::default();

        // 从内容中提取编程语言
        let content_lower = insight.content.to_lowercase();
        for lang in &[
            "rust",
            "python",
            "javascript",
            "typescript",
            "go",
            "java",
            "c++",
            "c",
            "ruby",
            "php",
        ] {
            if content_lower.contains(lang) {
                applicability.languages.push(lang.to_string());
            }
        }

        // 工具
        applicability.tools = insight.context.tools_used.clone();

        // 平台（从上下文推断）
        // MVP 阶段简化，可以从环境变量或配置中获取
        // 这里暂时留空

        applicability
    }

    /// 生成标签
    fn generate_tags(&self, insight: &RawInsight) -> Vec<String> {
        let mut tags = Vec::new();

        // 基于类别的标签
        match insight.category {
            InsightCategory::ToolUsage => tags.push("tool-usage".to_string()),
            InsightCategory::ErrorHandling => tags.push("error-handling".to_string()),
            InsightCategory::Pattern => tags.push("pattern".to_string()),
            InsightCategory::Solution => tags.push("solution".to_string()),
            InsightCategory::Knowledge => tags.push("knowledge".to_string()),
        }

        // 工具标签
        for tool in &insight.context.tools_used {
            tags.push(format!("tool:{tool}"));
        }

        // 成功/失败标签
        if insight.context.execution_success {
            tags.push("success".to_string());
        } else {
            tags.push("failed".to_string());
        }

        // 从内容中提取关键词标签
        let content_lower = insight.content.to_lowercase();

        // 操作类型标签
        if content_lower.contains("test") {
            tags.push("testing".to_string());
        }
        if content_lower.contains("build") || content_lower.contains("compile") {
            tags.push("building".to_string());
        }
        if content_lower.contains("fix") || content_lower.contains("debug") {
            tags.push("debugging".to_string());
        }
        if content_lower.contains("install") || content_lower.contains("setup") {
            tags.push("setup".to_string());
        }
        if content_lower.contains("deploy") {
            tags.push("deployment".to_string());
        }
        if content_lower.contains("git") {
            tags.push("git".to_string());
        }

        // 编程语言标签
        for lang in &["rust", "python", "javascript", "typescript", "go", "java"] {
            if content_lower.contains(lang) {
                tags.push(format!("lang:{lang}"));
            }
        }

        // 去重排序
        tags.sort();
        tags.dedup();

        tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::types::InsightContext;

    fn create_test_insight(
        content: &str,
        category: InsightCategory,
        execution_success: bool,
    ) -> RawInsight {
        RawInsight {
            content: content.to_string(),
            category,
            importance: 0.7,
            context: InsightContext {
                user_query: "Test query".to_string(),
                assistant_response_snippet: "Test response".to_string(),
                execution_success,
                tools_used: vec!["bash".to_string()],
                error_message: None,
                session_id: "test-session".to_string(),
            },
        }
    }

    #[tokio::test]
    async fn test_curator_generates_bullets() {
        let curator = CuratorMVP::new(CuratorConfig::default());

        let insight = create_test_insight(
            "使用 cargo test 命令可以运行项目的所有测试",
            InsightCategory::ToolUsage,
            true,
        );

        let delta = curator
            .process_insights(vec![insight], "test-session".to_string())
            .await
            .unwrap();

        assert_eq!(delta.new_bullets.len(), 1);
        assert_eq!(delta.new_bullets[0].section, BulletSection::ToolUsageTips);
        assert!(
            delta.new_bullets[0]
                .tags
                .contains(&"tool-usage".to_string())
        );
    }

    #[tokio::test]
    async fn test_curator_categorization() {
        let curator = CuratorMVP::new(CuratorConfig::default());

        // 测试各种分类
        let test_cases = vec![
            (
                "使用 cargo build 命令可以编译 Rust 项目",
                InsightCategory::ToolUsage,
                BulletSection::ToolUsageTips,
            ),
            (
                "遇到错误: Compilation failed 时需要检查语法错误",
                InsightCategory::ErrorHandling,
                BulletSection::TroubleshootingAndPitfalls,
            ),
            (
                "执行测试流程时应该先运行单元测试再运行集成测试",
                InsightCategory::Pattern,
                BulletSection::StrategiesAndRules,
            ),
            (
                "API 使用指南: 调用 RESTful 接口前需要先进行身份认证，获取有效的 access_token 后才能访问受保护的资源",
                InsightCategory::Knowledge,
                BulletSection::ApiUsageGuides,
            ),
            (
                "包含 rust 代码片段的示例可以帮助理解概念",
                InsightCategory::Knowledge,
                BulletSection::General,
            ),
        ];

        for (content, category, expected_section) in test_cases {
            let insight = create_test_insight(content, category, true);
            let delta = curator
                .process_insights(vec![insight], "test-session".to_string())
                .await
                .unwrap();

            assert_eq!(delta.new_bullets.len(), 1);
            assert_eq!(delta.new_bullets[0].section, expected_section);
        }
    }

    #[tokio::test]
    async fn test_curator_filters_low_importance() {
        let config = CuratorConfig {
            min_importance: 0.8,
            ..Default::default()
        };
        let curator = CuratorMVP::new(config);

        let mut low_importance = create_test_insight(
            "This is a low importance insight for testing",
            InsightCategory::Knowledge,
            true,
        );
        low_importance.importance = 0.5;

        let mut high_importance = create_test_insight(
            "This is a high importance insight for testing",
            InsightCategory::Knowledge,
            true,
        );
        high_importance.importance = 0.9;

        let delta = curator
            .process_insights(
                vec![low_importance, high_importance],
                "test-session".to_string(),
            )
            .await
            .unwrap();

        // 只有高重要性的 insight 应该被转换为 bullet
        assert_eq!(delta.new_bullets.len(), 1);
        assert!(delta.new_bullets[0].content.contains("high importance"));
        assert_eq!(delta.metadata.insights_processed, 1);
    }

    #[tokio::test]
    async fn test_curator_metadata_generation() {
        let curator = CuratorMVP::new(CuratorConfig::default());

        let mut insight = create_test_insight(
            "使用 Rust 命令行工具处理文件操作",
            InsightCategory::ToolUsage,
            true,
        );
        insight.context.tools_used = vec!["bash".to_string(), "cargo".to_string()];

        let delta = curator
            .process_insights(vec![insight], "test-session".to_string())
            .await
            .unwrap();

        assert_eq!(delta.new_bullets.len(), 1);

        let bullet = &delta.new_bullets[0];

        // 检查 metadata
        assert_eq!(bullet.metadata.importance, 0.7);
        assert_eq!(bullet.metadata.source_type, SourceType::SuccessExecution);
        assert_eq!(bullet.metadata.success_count, 1);
        assert_eq!(bullet.metadata.failure_count, 0);
        assert_eq!(
            bullet.metadata.related_tools,
            vec!["bash".to_string(), "cargo".to_string()]
        );
        assert_eq!(bullet.metadata.confidence, 1.0);
    }

    #[tokio::test]
    async fn test_curator_tag_generation() {
        let curator = CuratorMVP::new(CuratorConfig::default());

        let insight = create_test_insight(
            "使用 rust 测试命令 cargo test 可以运行所有测试用例",
            InsightCategory::ToolUsage,
            true,
        );

        let delta = curator
            .process_insights(vec![insight], "test-session".to_string())
            .await
            .unwrap();

        assert_eq!(delta.new_bullets.len(), 1);

        let tags = &delta.new_bullets[0].tags;

        // 应该包含多个标签
        assert!(tags.contains(&"tool-usage".to_string()));
        assert!(tags.contains(&"success".to_string()));
        assert!(tags.contains(&"tool:bash".to_string()));
        assert!(tags.contains(&"testing".to_string()));
        assert!(tags.contains(&"lang:rust".to_string()));
    }

    #[tokio::test]
    async fn test_curator_applicability_extraction() {
        let curator = CuratorMVP::new(CuratorConfig::default());

        let insight = create_test_insight(
            "使用 python 和 rust 进行开发",
            InsightCategory::Knowledge,
            true,
        );

        let delta = curator
            .process_insights(vec![insight], "test-session".to_string())
            .await
            .unwrap();

        assert_eq!(delta.new_bullets.len(), 1);

        let applicability = &delta.new_bullets[0].metadata.applicability;

        // 应该提取出两种语言
        assert!(applicability.languages.contains(&"python".to_string()));
        assert!(applicability.languages.contains(&"rust".to_string()));
    }

    #[tokio::test]
    async fn test_curator_empty_insights() {
        let curator = CuratorMVP::new(CuratorConfig::default());

        let delta = curator
            .process_insights(vec![], "test-session".to_string())
            .await
            .unwrap();

        assert!(delta.is_empty());
        assert_eq!(delta.metadata.insights_processed, 0);
        assert_eq!(delta.metadata.new_bullets_count, 0);
    }

    #[tokio::test]
    async fn test_curator_processing_time() {
        let curator = CuratorMVP::new(CuratorConfig::default());

        let insights = vec![
            create_test_insight(
                "使用 cargo test 命令可以运行 Rust 项目中的所有测试用例",
                InsightCategory::ToolUsage,
                true,
            ),
            create_test_insight(
                "处理异步操作时应使用 async/await 模式，可以提高代码可读性",
                InsightCategory::Pattern,
                true,
            ),
            create_test_insight(
                "Rust 的所有权系统可以在编译期防止内存错误和数据竞争",
                InsightCategory::Knowledge,
                true,
            ),
        ];

        let delta = curator
            .process_insights(insights, "test-session".to_string())
            .await
            .unwrap();

        // 处理时间应该大于 0
        assert!(delta.metadata.processing_time_ms > 0);
        assert_eq!(delta.metadata.insights_processed, 3);
        assert_eq!(delta.metadata.new_bullets_count, 3);
    }
}
