//! 内容分类器 - LAPS 系统的核心组件
//!
//! 根据内容类型自动分类并应用相应的长度策略，避免死板限制。

/// 内容类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum ContentType {
    /// 代码片段 - 可以很长
    CodeSnippet,
    /// 错误解决方案 - 中等长度
    ErrorSolution,
    /// 策略和规则 - 精简
    Strategy,
    /// 项目特定知识 - 灵活
    ProjectSpecific,
    /// 工具使用技巧 - 中等
    ToolUsage,
    /// API 使用指南 - 中等偏长
    ApiGuide,
}

/// 内容长度策略
#[derive(Debug, Clone)]
pub struct ContentLengthStrategy {
    /// 最小长度（字符数）
    pub min: usize,
    /// 理想长度（字符数）
    pub ideal: usize,
    /// 最大长度（字符数）
    pub max: usize,
}

impl ContentType {
    /// 获取该内容类型的长度策略
    pub fn get_length_strategy(&self) -> ContentLengthStrategy {
        match self {
            ContentType::CodeSnippet => ContentLengthStrategy {
                min: 100,
                ideal: 500,
                max: 3000,
            },
            ContentType::ErrorSolution => ContentLengthStrategy {
                min: 50,
                ideal: 300,
                max: 800,
            },
            ContentType::Strategy => ContentLengthStrategy {
                min: 30,
                ideal: 150,
                max: 400,
            },
            ContentType::ProjectSpecific => ContentLengthStrategy {
                min: 50,
                ideal: 400,
                max: 1500,
            },
            ContentType::ToolUsage => ContentLengthStrategy {
                min: 40,
                ideal: 200,
                max: 600,
            },
            ContentType::ApiGuide => ContentLengthStrategy {
                min: 80,
                ideal: 400,
                max: 1000,
            },
        }
    }
}

/// 内容分类器
pub struct ContentClassifier;

impl ContentClassifier {
    /// 对内容进行自动分类
    ///
    /// # 参数
    /// - `content`: 待分类的内容
    ///
    /// # 返回
    /// 内容类型
    pub fn classify(content: &str) -> ContentType {
        let content_lower = content.to_lowercase();

        // 代码检测 - 检测代码块或函数定义
        if content.contains("```")
            || content.contains("fn ")
            || content.contains("def ")
            || content.contains("function ")
            || content.contains("class ")
            || content.contains("impl ")
            || content.contains("struct ")
        {
            return ContentType::CodeSnippet;
        }

        // 错误解决检测
        if content_lower.contains("error")
            || content_lower.contains("fix")
            || content_lower.contains("解决")
            || content_lower.contains("错误")
            || content_lower.contains("failed")
            || content_lower.contains("issue")
        {
            return ContentType::ErrorSolution;
        }

        // API 指南检测
        if content_lower.contains("api")
            || content_lower.contains("endpoint")
            || content_lower.contains("接口")
            || content_lower.contains("request")
            || content_lower.contains("response")
        {
            return ContentType::ApiGuide;
        }

        // 工具使用检测
        if content_lower.contains("cargo")
            || content_lower.contains("npm")
            || content_lower.contains("git")
            || content_lower.contains("docker")
            || content_lower.contains("kubectl")
        {
            return ContentType::ToolUsage;
        }

        // 项目特定检测（包含文件路径）
        if content.contains("src/")
            || content.contains("./")
            || content.contains("~/")
            || content.contains("codex-rs/")
        {
            return ContentType::ProjectSpecific;
        }

        // 默认为策略
        ContentType::Strategy
    }

    /// 验证内容质量和长度
    ///
    /// # 参数
    /// - `content`: 待验证的内容
    ///
    /// # 返回
    /// (是否有效, 验证信息)
    pub fn validate_content(content: &str) -> (bool, String) {
        let content_type = Self::classify(content);
        let strategy = content_type.get_length_strategy();
        let length = content.len();

        // 检查长度下限
        if length < strategy.min {
            return (
                false,
                format!(
                    "内容太短，{:?} 类型至少需要 {} 字符，当前 {} 字符",
                    content_type, strategy.min, length
                ),
            );
        }

        // 检查长度上限
        if length > strategy.max {
            return (
                false,
                format!(
                    "内容太长，{:?} 类型最多 {} 字符，当前 {} 字符",
                    content_type, strategy.max, length
                ),
            );
        }

        // 检查实质内容（放宽限制，只检查最基本的要求）
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return (false, "内容为空".to_string());
        }

        // 只拒绝极短的内容（< 10 个字符）
        if trimmed.len() < 10 {
            return (false, format!("内容太短，只有 {} 个字符", trimmed.len()));
        }

        // 检查通用错误信息
        let generic_errors = vec![
            "execution failed",
            "error occurred",
            "something went wrong",
            "failed to execute",
            "an error happened",
        ];

        let trimmed_lower = trimmed.to_lowercase();
        for error in generic_errors {
            // 当内容完全等于通用错误时拒绝
            if trimmed_lower == error {
                return (false, "通用错误信息，缺乏具体内容".to_string());
            }
            // 当内容主要由通用错误组成时也拒绝（超过80%）
            if trimmed_lower.contains(error)
                && error.len() as f32 / trimmed_lower.len() as f32 > 0.8
            {
                return (false, "通用错误信息，缺乏具体内容".to_string());
            }
        }

        (
            true,
            format!("{content_type:?} 类型内容，长度 {length} 字符，符合质量标准"),
        )
    }

    /// 获取内容质量分数（0.0 - 1.0）
    ///
    /// # 参数
    /// - `content`: 待评分的内容
    ///
    /// # 返回
    /// 质量分数
    pub fn quality_score(content: &str) -> f32 {
        let (valid, _) = Self::validate_content(content);
        if !valid {
            return 0.0;
        }

        let content_type = Self::classify(content);
        let strategy = content_type.get_length_strategy();
        let length = content.len();

        // 计算长度分数（靠近理想长度得分更高）
        let length_score = if length < strategy.ideal {
            length as f32 / strategy.ideal as f32
        } else {
            let excess = (length - strategy.ideal) as f32;
            let max_excess = (strategy.max - strategy.ideal) as f32;
            1.0 - (excess / max_excess) * 0.3 // 超出理想长度时略微降分
        };

        // 计算信息密度分数
        let word_count = content.split_whitespace().count();
        let avg_word_length = if word_count > 0 {
            content.len() as f32 / word_count as f32
        } else {
            0.0
        };

        // 平均单词长度在 4-8 之间为最佳
        let density_score = if (4.0..=8.0).contains(&avg_word_length) {
            1.0
        } else if avg_word_length < 4.0 {
            avg_word_length / 4.0
        } else {
            1.0 - ((avg_word_length - 8.0) / 12.0).min(0.5)
        };

        // 综合分数
        (length_score * 0.6 + density_score * 0.4).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_classification() {
        let code = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        assert_eq!(ContentClassifier::classify(code), ContentType::CodeSnippet);

        let code2 = "impl MyStruct { fn new() {} }";
        assert_eq!(ContentClassifier::classify(code2), ContentType::CodeSnippet);
    }

    #[test]
    fn test_error_classification() {
        let error = "Error: failed to compile, fix by adding dependency";
        assert_eq!(
            ContentClassifier::classify(error),
            ContentType::ErrorSolution
        );
    }

    #[test]
    fn test_api_classification() {
        let api = "API endpoint /users returns user list";
        assert_eq!(ContentClassifier::classify(api), ContentType::ApiGuide);
    }

    #[test]
    fn test_tool_classification() {
        let tool = "使用 cargo build --release 来构建优化版本";
        assert_eq!(ContentClassifier::classify(tool), ContentType::ToolUsage);
    }

    #[test]
    fn test_project_classification() {
        let project = "修改 src/main.rs 文件中的配置";
        assert_eq!(
            ContentClassifier::classify(project),
            ContentType::ProjectSpecific
        );
    }

    #[test]
    fn test_strategy_classification() {
        let strategy = "使用异步编程提高性能";
        assert_eq!(ContentClassifier::classify(strategy), ContentType::Strategy);
    }

    #[test]
    fn test_content_validation_too_short() {
        let short = "太短";
        let (valid, reason) = ContentClassifier::validate_content(short);
        assert!(!valid);
        assert!(reason.contains("太短"));
    }

    #[test]
    fn test_content_validation_good_strategy() {
        let good_strategy = "使用 async/await 处理异步操作可以避免回调地狱，提高代码可读性和维护性";
        let (valid, reason) = ContentClassifier::validate_content(good_strategy);
        assert!(valid);
        assert!(reason.contains("Strategy"));
    }

    #[test]
    fn test_content_validation_good_code() {
        let good_code = r#"
```rust
fn handle_error(result: Result<(), Error>) -> Result<(), Error> {
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            tracing::error!("Error occurred: {}", e);
            Err(e)
        }
    }
}
```
"#;
        let (valid, reason) = ContentClassifier::validate_content(good_code);
        assert!(valid);
        assert!(reason.contains("CodeSnippet"));
    }

    #[test]
    fn test_content_validation_too_long() {
        let too_long = "a".repeat(5000); // 超过所有类型的最大长度
        let (valid, _) = ContentClassifier::validate_content(&too_long);
        assert!(!valid);
    }

    #[test]
    fn test_content_validation_generic_error() {
        // 测试完全匹配的通用错误
        let generic = "execution failed";
        let (valid, reason) = ContentClassifier::validate_content(generic);
        assert!(!valid);
        assert!(reason.contains("通用错误") || reason.contains("太短"));

        // 测试主要由通用错误组成的内容
        let mostly_generic = "execution failed!!!";
        let (valid, reason) = ContentClassifier::validate_content(mostly_generic);
        assert!(!valid);
        assert!(reason.contains("通用错误") || reason.contains("太短"));
    }

    #[test]
    fn test_quality_score() {
        let good_content = "使用 async/await 处理异步操作可以避免回调地狱，提高代码可读性和维护性";
        let score = ContentClassifier::quality_score(good_content);
        assert!(score > 0.5); // 调整期望值，应该是高质量内容

        let poor_content = "太短";
        let score = ContentClassifier::quality_score(poor_content);
        assert_eq!(score, 0.0); // 无效内容
    }

    #[test]
    fn test_length_strategies() {
        let code_strategy = ContentType::CodeSnippet.get_length_strategy();
        assert_eq!(code_strategy.min, 100);
        assert_eq!(code_strategy.max, 3000);

        let error_strategy = ContentType::ErrorSolution.get_length_strategy();
        assert_eq!(error_strategy.min, 50);
        assert_eq!(error_strategy.max, 800);

        let strategy_strategy = ContentType::Strategy.get_length_strategy();
        assert_eq!(strategy_strategy.min, 30);
        assert_eq!(strategy_strategy.max, 400);
    }
}
