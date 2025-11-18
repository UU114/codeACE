//! ACE框架的核心数据结构
//!
//! MVP版本，专注于简单和实用。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Playbook条目 - MVP版本
///
/// 每个条目代表一次对话的学习结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookEntry {
    /// 唯一标识符
    pub id: String,

    /// 时间戳
    pub timestamp: DateTime<Utc>,

    /// 会话ID
    pub session_id: String,

    /// 用户查询
    pub user_query: String,

    /// 助手响应（截断到合理长度）
    pub assistant_response: String,

    /// 执行是否成功
    pub execution_success: bool,

    /// 提取的洞察
    pub insights: Vec<Insight>,

    /// 识别的模式
    pub patterns: Vec<String>,

    /// 学到的策略
    pub learned_strategies: Vec<String>,

    /// 使用的工具
    pub tools_used: Vec<String>,

    /// 错误信息（如果有）
    pub error_messages: Vec<String>,

    /// 标签（用于快速过滤）
    pub tags: Vec<String>,
}

impl PlaybookEntry {
    /// 创建一个新的空条目
    pub fn new(user_query: String, assistant_response: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            session_id: Uuid::new_v4().to_string(),
            user_query,
            assistant_response: truncate_string(&assistant_response, 2000),
            execution_success: false,
            insights: Vec::new(),
            patterns: Vec::new(),
            learned_strategies: Vec::new(),
            tools_used: Vec::new(),
            error_messages: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// 判断是否为有价值的条目
    pub fn is_valuable(&self) -> bool {
        !self.insights.is_empty()
            || !self.patterns.is_empty()
            || !self.learned_strategies.is_empty()
            || self.execution_success
    }
}

/// 洞察 - 从对话中提取的知识
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    /// 洞察内容
    pub content: String,

    /// 类别
    pub category: InsightCategory,

    /// 重要性（0.0 - 1.0）
    pub importance: f32,
}

/// 洞察类别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InsightCategory {
    /// 工具使用
    ToolUsage,

    /// 模式识别
    Pattern,

    /// 问题解决
    Solution,

    /// 知识捕获
    Knowledge,

    /// 错误处理
    ErrorHandling,
}

/// 执行结果
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// 是否成功
    pub success: bool,

    /// 输出内容
    pub output: Option<String>,

    /// 错误信息
    pub error: Option<String>,

    /// 使用的工具
    pub tools_used: Vec<String>,

    /// 错误列表
    pub errors: Vec<String>,

    /// 是否重试成功
    pub retry_success: bool,
}

impl Default for ExecutionResult {
    fn default() -> Self {
        Self {
            success: false,
            output: None,
            error: None,
            tools_used: Vec::new(),
            errors: Vec::new(),
            retry_success: false,
        }
    }
}

/// ACE配置
#[derive(Debug, Clone, Deserialize)]
pub struct ACEConfig {
    /// 是否启用
    pub enabled: bool,

    /// 存储路径
    pub storage_path: String,

    /// 最大条目数
    pub max_entries: usize,

    /// Reflector配置
    pub reflector: ReflectorConfig,

    /// Context配置
    pub context: ContextConfig,
}

impl Default for ACEConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            storage_path: "~/.codeACE/ace".to_string(),
            max_entries: 500,
            reflector: ReflectorConfig::default(),
            context: ContextConfig::default(),
        }
    }
}

/// Reflector配置
#[derive(Debug, Clone, Deserialize)]
pub struct ReflectorConfig {
    /// 提取模式
    pub extract_patterns: bool,

    /// 提取工具使用
    pub extract_tools: bool,

    /// 提取错误处理
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

/// Context配置
#[derive(Debug, Clone, Deserialize)]
pub struct ContextConfig {
    /// 最近条目数
    pub max_recent_entries: usize,

    /// 包含所有成功案例
    pub include_all_successes: bool,

    /// 最大字符数
    pub max_context_chars: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_recent_entries: 10,
            include_all_successes: true,
            max_context_chars: 4000,
        }
    }
}

// 辅助函数

/// 截断字符串到指定长度
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
