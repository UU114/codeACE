// 跨领域知识图谱 - 智能管理跨项目、跨语言、跨行业的知识
use serde::{Deserialize, Serialize};

/// 领域分类
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Domain {
    Generic,        // 通用知识
    WebDev,         // Web 开发
    SystemsProg,    // 系统编程
    DataScience,    // 数据科学
    DevOps,         // 运维
    Mobile,         // 移动开发
    GameDev,        // 游戏开发
    Blockchain,     // 区块链
    AI,             // 人工智能
    Custom(String), // 自定义领域
}

/// 编程语言分类
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Generic, // 语言无关
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    CSharp,
    Cpp,
    Multi(Vec<String>), // 多语言（使用 String 避免递归）
}

/// 知识范围 - 定义知识的适用范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeScope {
    pub domain: Domain,
    pub language: Language,
    pub project: Option<String>,
    pub tags: Vec<String>, // 额外标签
}

/// 上下文 - 当前查询的上下文信息
#[derive(Debug, Clone)]
pub struct Context {
    pub domain: Domain,
    pub language: Language,
    pub project: Option<String>,
    pub query: String,
}

impl Default for KnowledgeScope {
    fn default() -> Self {
        Self::new(Domain::Generic, Language::Generic)
    }
}

impl KnowledgeScope {
    /// 创建新的知识范围
    pub fn new(domain: Domain, language: Language) -> Self {
        Self {
            domain,
            language,
            project: None,
            tags: Vec::new(),
        }
    }

    /// 添加标签
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// 设置项目
    pub fn with_project(mut self, project: String) -> Self {
        self.project = Some(project);
        self
    }

    /// 自动检测领域
    pub fn detect_domain(content: &str) -> Domain {
        let content_lower = content.to_lowercase();

        // Web 开发相关
        if content_lower.contains("web")
            || content_lower.contains("http")
            || content_lower.contains("api")
            || content_lower.contains("rest")
            || content_lower.contains("graphql")
        {
            return Domain::WebDev;
        }

        // 系统编程相关
        if content_lower.contains("kernel")
            || content_lower.contains("memory")
            || content_lower.contains("thread")
            || content_lower.contains("async")
            || content_lower.contains("concurrency")
        {
            return Domain::SystemsProg;
        }

        // 数据科学相关
        if content_lower.contains("model")
            || content_lower.contains("train")
            || content_lower.contains("dataset")
            || content_lower.contains("pandas")
            || content_lower.contains("numpy")
        {
            return Domain::DataScience;
        }

        // DevOps 相关
        if content_lower.contains("docker")
            || content_lower.contains("k8s")
            || content_lower.contains("kubernetes")
            || content_lower.contains("ci/cd")
            || content_lower.contains("deploy")
        {
            return Domain::DevOps;
        }

        // 移动开发
        if content_lower.contains("android")
            || content_lower.contains("ios")
            || content_lower.contains("mobile")
            || content_lower.contains("flutter")
        {
            return Domain::Mobile;
        }

        // 游戏开发
        if content_lower.contains("game")
            || content_lower.contains("unity")
            || content_lower.contains("unreal")
            || content_lower.contains("bevy")
        {
            return Domain::GameDev;
        }

        // 区块链
        if content_lower.contains("blockchain")
            || content_lower.contains("smart contract")
            || content_lower.contains("solidity")
            || content_lower.contains("web3")
        {
            return Domain::Blockchain;
        }

        // AI 相关
        if content_lower.contains("neural")
            || content_lower.contains("deep learning")
            || content_lower.contains("machine learning")
            || content_lower.contains("tensorflow")
            || content_lower.contains("pytorch")
        {
            return Domain::AI;
        }

        Domain::Generic
    }

    /// 自动检测编程语言
    pub fn detect_language(content: &str) -> Language {
        let content_lower = content.to_lowercase();

        // Rust 相关
        if content_lower.contains("cargo")
            || content_lower.contains("rustc")
            || content_lower.contains("fn ")
            || content_lower.contains("impl ")
            || content_lower.contains("trait ")
        {
            return Language::Rust;
        }

        // Python 相关
        if content_lower.contains("pip")
            || content_lower.contains("python")
            || content_lower.contains("def ")
            || content_lower.contains("__init__")
        {
            return Language::Python;
        }

        // JavaScript 相关
        if content_lower.contains("npm")
            || content_lower.contains("node")
            || content_lower.contains("const ")
            || content_lower.contains("let ")
            || content_lower.contains("=>")
        {
            return Language::JavaScript;
        }

        // TypeScript 相关
        if content_lower.contains("typescript")
            || content_lower.contains("interface ")
            || content_lower.contains(": string")
            || content_lower.contains(": number")
        {
            return Language::TypeScript;
        }

        // Go 相关
        if content_lower.contains("go mod")
            || content_lower.contains("golang")
            || content_lower.contains("func ")
            || content_lower.contains("package main")
        {
            return Language::Go;
        }

        // Java 相关
        if content_lower.contains("java")
            || content_lower.contains("public class")
            || content_lower.contains("maven")
            || content_lower.contains("gradle")
        {
            return Language::Java;
        }

        // C# 相关
        if content_lower.contains("csharp")
            || content_lower.contains("c#")
            || content_lower.contains("dotnet")
            || content_lower.contains("namespace ")
        {
            return Language::CSharp;
        }

        // C++ 相关
        if content_lower.contains("c++")
            || content_lower.contains("cpp")
            || content_lower.contains("#include")
            || content_lower.contains("std::")
        {
            return Language::Cpp;
        }

        Language::Generic
    }

    /// 计算与当前上下文的匹配分数
    /// 返回值范围: 0.0 - 4.5
    pub fn match_score(&self, context: &Context) -> f32 {
        let mut score = 0.0;

        // 1. 领域匹配 (0.0 - 1.0)
        score += self.domain_match_score(&context.domain);

        // 2. 语言匹配 (0.0 - 1.0)
        score += self.language_match_score(&context.language);

        // 3. 项目匹配 (0.0 - 2.0)
        score += self.project_match_score(&context.project);

        // 4. 标签匹配 (0.0 - 0.5)
        score += self.tag_match_score(&context.query);

        score
    }

    /// 领域匹配分数
    fn domain_match_score(&self, context_domain: &Domain) -> f32 {
        match (&self.domain, context_domain) {
            // 完全匹配
            (a, b) if a == b => 1.0,
            // Generic 部分匹配
            (Domain::Generic, _) | (_, Domain::Generic) => 0.5,
            // 不匹配
            _ => 0.0,
        }
    }

    /// 语言匹配分数
    fn language_match_score(&self, context_lang: &Language) -> f32 {
        match (&self.language, context_lang) {
            // 完全匹配
            (a, b) if a == b => 1.0,
            // Generic 部分匹配
            (Language::Generic, _) | (_, Language::Generic) => 0.5,
            // Multi 语言匹配
            (Language::Multi(langs), target) => {
                let target_str = format!("{target:?}");
                if langs.iter().any(|l| l == &target_str) {
                    1.0
                } else {
                    0.2
                }
            }
            // 不匹配
            _ => 0.0,
        }
    }

    /// 项目匹配分数
    fn project_match_score(&self, context_project: &Option<String>) -> f32 {
        match (&self.project, context_project) {
            // 项目完全匹配 - 最高优先级
            (Some(p1), Some(p2)) if p1 == p2 => 2.0,
            // 都没有项目限制
            (None, None) => 0.5,
            // 一方有项目限制，一方没有
            _ => 0.0,
        }
    }

    /// 标签匹配分数
    fn tag_match_score(&self, query: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let mut score: f32 = 0.0;

        for tag in &self.tags {
            if query_lower.contains(&tag.to_lowercase()) {
                score += 0.1;
            }
        }

        // 最多 0.5 分
        score.min(0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_detection() {
        // Web 开发
        let web_content = "使用 HTTP API 构建 RESTful 服务";
        assert_eq!(KnowledgeScope::detect_domain(web_content), Domain::WebDev);

        // 系统编程
        let sys_content = "使用 async/await 处理并发";
        assert_eq!(
            KnowledgeScope::detect_domain(sys_content),
            Domain::SystemsProg
        );

        // 数据科学
        let ds_content = "训练机器学习 model";
        assert_eq!(
            KnowledgeScope::detect_domain(ds_content),
            Domain::DataScience
        );

        // DevOps
        let devops_content = "使用 Docker 部署应用";
        assert_eq!(
            KnowledgeScope::detect_domain(devops_content),
            Domain::DevOps
        );

        // 通用
        let generic_content = "这是一些通用的编程建议";
        assert_eq!(
            KnowledgeScope::detect_domain(generic_content),
            Domain::Generic
        );
    }

    #[test]
    fn test_language_detection() {
        // Rust
        let rust_content = "使用 cargo build 构建项目";
        assert_eq!(
            KnowledgeScope::detect_language(rust_content),
            Language::Rust
        );

        // Python
        let py_content = "使用 pip install 安装包";
        assert_eq!(
            KnowledgeScope::detect_language(py_content),
            Language::Python
        );

        // JavaScript
        let js_content = "使用 npm install 安装依赖";
        assert_eq!(
            KnowledgeScope::detect_language(js_content),
            Language::JavaScript
        );

        // Go
        let go_content = "使用 go mod init 初始化项目";
        assert_eq!(KnowledgeScope::detect_language(go_content), Language::Go);

        // 通用
        let generic_content = "这是语言无关的建议";
        assert_eq!(
            KnowledgeScope::detect_language(generic_content),
            Language::Generic
        );
    }

    #[test]
    fn test_match_score_perfect_match() {
        let scope = KnowledgeScope::new(Domain::WebDev, Language::Rust)
            .with_project("my-project".to_string());

        let context = Context {
            domain: Domain::WebDev,
            language: Language::Rust,
            project: Some("my-project".to_string()),
            query: "如何实现 API".to_string(),
        };

        let score = scope.match_score(&context);
        // 领域(1.0) + 语言(1.0) + 项目(2.0) = 4.0
        assert!(score >= 4.0);
    }

    #[test]
    fn test_match_score_partial_match() {
        let scope = KnowledgeScope::new(Domain::Generic, Language::Rust);

        let context = Context {
            domain: Domain::WebDev,
            language: Language::Rust,
            project: None,
            query: "如何处理错误".to_string(),
        };

        let score = scope.match_score(&context);
        // 领域(0.5) + 语言(1.0) + 项目(0.5) = 2.0
        assert!(score >= 2.0 && score < 3.0);
    }

    #[test]
    fn test_match_score_no_match() {
        let scope = KnowledgeScope::new(Domain::WebDev, Language::Rust);

        let context = Context {
            domain: Domain::DataScience,
            language: Language::Python,
            project: None,
            query: "训练模型".to_string(),
        };

        let score = scope.match_score(&context);
        // 领域(0.0) + 语言(0.0) + 项目(0.5) = 0.5
        assert!(score < 1.0);
    }

    #[test]
    fn test_tag_matching() {
        let scope = KnowledgeScope::new(Domain::Generic, Language::Generic)
            .with_tags(vec!["async".to_string(), "tokio".to_string()]);

        let context = Context {
            domain: Domain::Generic,
            language: Language::Generic,
            project: None,
            query: "如何使用 tokio 处理 async 任务".to_string(),
        };

        let score = scope.match_score(&context);
        // 应该包含标签匹配分数
        assert!(score > 1.0);
    }

    #[test]
    fn test_project_priority() {
        let scope1 = KnowledgeScope::new(Domain::Generic, Language::Generic)
            .with_project("my-project".to_string());

        let scope2 = KnowledgeScope::new(Domain::WebDev, Language::Rust);

        let context = Context {
            domain: Domain::WebDev,
            language: Language::Rust,
            project: Some("my-project".to_string()),
            query: "实现功能".to_string(),
        };

        let score1 = scope1.match_score(&context);
        let score2 = scope2.match_score(&context);

        // 项目特定知识应该优先级更高
        // scope1: 领域(0.5) + 语言(0.5) + 项目(2.0) = 3.0
        // scope2: 领域(1.0) + 语言(1.0) + 项目(0.0) = 2.0
        assert!(score1 > score2);
    }

    #[test]
    fn test_multi_language_matching() {
        let scope = KnowledgeScope {
            domain: Domain::Generic,
            language: Language::Multi(vec!["Rust".to_string(), "Python".to_string()]),
            project: None,
            tags: vec![],
        };

        let context_rust = Context {
            domain: Domain::Generic,
            language: Language::Rust,
            project: None,
            query: "测试".to_string(),
        };

        let context_python = Context {
            domain: Domain::Generic,
            language: Language::Python,
            project: None,
            query: "测试".to_string(),
        };

        let score_rust = scope.match_score(&context_rust);
        let score_python = scope.match_score(&context_python);

        // 多语言应该匹配 Rust 和 Python
        assert!(score_rust > 1.0);
        assert!(score_python > 1.0);
    }
}
