//! ACE 框架的核心数据结构 - Bullet-based 架构
//!
//! 基于 Agentic Context Engineering 论文实现，采用细粒度的 bullet 管理。

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// Bullet 数据结构（核心单元）
// ============================================================================

/// 一条可执行的规则/策略/知识点
///
/// Bullet 是 ACE 系统的核心存储单元，每个 bullet 代表一条独立的、
/// 可引用的知识点或策略。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bullet {
    /// 唯一标识符（UUID）
    pub id: String,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 最后更新时间
    pub updated_at: DateTime<Utc>,

    /// 来源会话ID（首次创建时）
    pub source_session_id: String,

    /// 所属分类（structured sections）
    pub section: BulletSection,

    /// 具体内容（markdown 格式）
    pub content: String,

    /// 代码内容（如果包含代码）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_content: Option<BulletCodeContent>,

    /// 元数据（细粒度跟踪）
    pub metadata: BulletMetadata,

    /// 关联的标签（用于检索）
    pub tags: Vec<String>,
}

/// 代码内容（分级保存）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BulletCodeContent {
    /// 完整代码（用于核心代码、小文件）
    Full {
        /// 编程语言
        language: String,
        /// 完整代码
        code: String,
        /// 文件路径（如果有）
        #[serde(skip_serializing_if = "Option::is_none")]
        file_path: Option<String>,
    },

    /// 摘要+引用（用于大文件、辅助代码）
    Summary {
        /// 编程语言
        language: String,
        /// 代码摘要（函数签名、关键类型等）
        summary: String,
        /// 文件路径
        file_path: String,
        /// 关键行号范围
        #[serde(skip_serializing_if = "Option::is_none")]
        key_lines: Option<Vec<(usize, usize)>>,
    },
}

/// 分类（参考论文 Figure 3）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BulletSection {
    /// 策略和硬性规则
    StrategiesAndRules,

    /// 可用的代码片段和模板
    CodeSnippetsAndTemplates,

    /// 故障排查和陷阱
    TroubleshootingAndPitfalls,

    /// API 使用指南
    ApiUsageGuides,

    /// 错误处理模式
    ErrorHandlingPatterns,

    /// 工具使用技巧
    ToolUsageTips,

    /// 其他通用知识
    General,
}

/// 细粒度元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletMetadata {
    /// 重要性评分（0.0 - 1.0）
    pub importance: f32,

    /// 来源类型
    pub source_type: SourceType,

    /// 适用性范围
    pub applicability: Applicability,

    /// 引用次数（被 context loader 使用）
    pub reference_count: u32,

    /// 成功应用次数
    pub success_count: u32,

    /// 失败应用次数
    pub failure_count: u32,

    /// 相关工具/语言
    pub related_tools: Vec<String>,

    /// 相关文件模式（glob）
    pub related_file_patterns: Vec<String>,

    /// 置信度（0.0 - 1.0，MVP 可固定为 1.0）
    pub confidence: f32,
}

/// 来源类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SourceType {
    /// 从成功执行中提取
    SuccessExecution,

    /// 从错误解决中提取
    ErrorResolution,

    /// 从模式识别中提取
    PatternRecognition,

    /// 手动添加（预留）
    ManualEntry,
}

/// 适用性范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Applicability {
    /// 适用的编程语言（空表示通用）
    pub languages: Vec<String>,

    /// 适用的工具
    pub tools: Vec<String>,

    /// 适用的操作系统
    pub platforms: Vec<String>,

    /// 适用的项目类型
    pub project_types: Vec<String>,
}

impl Default for Applicability {
    fn default() -> Self {
        Self {
            languages: Vec::new(),
            tools: Vec::new(),
            platforms: Vec::new(),
            project_types: Vec::new(),
        }
    }
}

impl Bullet {
    /// 创建新 bullet
    pub fn new(section: BulletSection, content: String, source_session_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            updated_at: now,
            source_session_id,
            section,
            content,
            code_content: None,
            metadata: BulletMetadata::default(),
            tags: Vec::new(),
        }
    }

    /// 增加引用计数
    pub fn increment_reference(&mut self) {
        self.metadata.reference_count += 1;
        self.updated_at = Utc::now();
    }

    /// 记录成功应用
    pub fn record_success(&mut self) {
        self.metadata.success_count += 1;
        self.updated_at = Utc::now();
    }

    /// 记录失败应用
    pub fn record_failure(&mut self) {
        self.metadata.failure_count += 1;
        self.updated_at = Utc::now();
    }

    /// 计算成功率（0.0 - 1.0）
    pub fn success_rate(&self) -> f32 {
        let total = self.metadata.success_count + self.metadata.failure_count;
        if total == 0 {
            0.0
        } else {
            self.metadata.success_count as f32 / total as f32
        }
    }
}

impl Default for BulletMetadata {
    fn default() -> Self {
        Self {
            importance: 0.5,
            source_type: SourceType::PatternRecognition,
            applicability: Applicability::default(),
            reference_count: 0,
            success_count: 0,
            failure_count: 0,
            related_tools: Vec::new(),
            related_file_patterns: Vec::new(),
            confidence: 1.0,
        }
    }
}

// ============================================================================
// Playbook 结构（bullet 集合）
// ============================================================================

/// Playbook - bullet 的有序集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playbook {
    /// 版本号（用于追踪变更）
    pub version: u32,

    /// 最后更新时间
    pub last_updated: DateTime<Utc>,

    /// 所有 bullets（按 section 分组）
    pub bullets: HashMap<BulletSection, Vec<Bullet>>,

    /// 全局元数据
    pub metadata: PlaybookMetadata,
}

/// Playbook 元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookMetadata {
    /// 总 bullet 数
    pub total_bullets: usize,

    /// 按 section 统计
    pub section_counts: HashMap<BulletSection, usize>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 来源会话数
    pub total_sessions: usize,
}

impl Playbook {
    /// 创建空 playbook
    pub fn new() -> Self {
        Self {
            version: 1,
            last_updated: Utc::now(),
            bullets: HashMap::new(),
            metadata: PlaybookMetadata {
                total_bullets: 0,
                section_counts: HashMap::new(),
                created_at: Utc::now(),
                total_sessions: 0,
            },
        }
    }

    /// 添加 bullet
    pub fn add_bullet(&mut self, bullet: Bullet) {
        let section = bullet.section.clone();
        self.bullets
            .entry(section.clone())
            .or_insert_with(Vec::new)
            .push(bullet);

        self.metadata.total_bullets += 1;
        *self.metadata.section_counts.entry(section).or_insert(0) += 1;
        self.version += 1;
        self.last_updated = Utc::now();
    }

    /// 查找 bullet
    pub fn find_bullet(&self, id: &str) -> Option<&Bullet> {
        self.bullets.values().flatten().find(|b| b.id == id)
    }

    /// 查找 bullet（可变引用）
    pub fn find_bullet_mut(&mut self, id: &str) -> Option<&mut Bullet> {
        self.bullets.values_mut().flatten().find(|b| b.id == id)
    }

    /// 更新 bullet（返回是否成功）
    pub fn update_bullet(&mut self, updated: Bullet) -> bool {
        for bullets in self.bullets.values_mut() {
            if let Some(pos) = bullets.iter().position(|b| b.id == updated.id) {
                bullets[pos] = updated;
                self.version += 1;
                self.last_updated = Utc::now();
                return true;
            }
        }
        false
    }

    /// 获取所有 bullets（扁平化）
    pub fn all_bullets(&self) -> Vec<&Bullet> {
        self.bullets.values().flatten().collect()
    }

    /// 按 section 获取 bullets
    pub fn bullets_by_section(&self, section: &BulletSection) -> Vec<&Bullet> {
        self.bullets
            .get(section)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }
}

impl Default for Playbook {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// RawInsight（Reflector 输出）
// ============================================================================

/// Reflector 的原始输出（未结构化）
#[derive(Debug, Clone)]
pub struct RawInsight {
    /// 洞察内容
    pub content: String,

    /// 类别
    pub category: InsightCategory,

    /// 重要性
    pub importance: f32,

    /// 来源上下文
    pub context: InsightContext,
}

/// 洞察上下文（帮助 Curator 生成 metadata）
#[derive(Debug, Clone)]
pub struct InsightContext {
    /// 用户查询
    pub user_query: String,

    /// 助手响应片段
    pub assistant_response_snippet: String,

    /// 执行结果
    pub execution_success: bool,

    /// 使用的工具
    pub tools_used: Vec<String>,

    /// 错误信息
    pub error_message: Option<String>,

    /// 会话ID
    pub session_id: String,
}

/// 洞察类别
#[derive(Debug, Clone, PartialEq)]
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

// ============================================================================
// DeltaContext（Curator 输出）
// ============================================================================

/// 增量上下文更新（Curator 输出）
#[derive(Debug, Clone)]
pub struct DeltaContext {
    /// 会话ID
    pub session_id: String,

    /// 新增的 bullets
    pub new_bullets: Vec<Bullet>,

    /// 需要更新的 bullets（仅 metadata 变化）
    pub updated_bullets: Vec<Bullet>,

    /// 生成时间
    pub generated_at: DateTime<Utc>,

    /// 元数据
    pub metadata: DeltaMetadata,
}

/// Delta 元数据
#[derive(Debug, Clone)]
pub struct DeltaMetadata {
    /// 处理的 insights 数量
    pub insights_processed: usize,

    /// 生成的新 bullets 数量
    pub new_bullets_count: usize,

    /// 更新的 bullets 数量
    pub updated_bullets_count: usize,

    /// 处理耗时（毫秒）
    pub processing_time_ms: u64,
}

impl DeltaContext {
    /// 创建空 delta
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            new_bullets: Vec::new(),
            updated_bullets: Vec::new(),
            generated_at: Utc::now(),
            metadata: DeltaMetadata {
                insights_processed: 0,
                new_bullets_count: 0,
                updated_bullets_count: 0,
                processing_time_ms: 0,
            },
        }
    }

    /// 是否为空（无变更）
    pub fn is_empty(&self) -> bool {
        self.new_bullets.is_empty() && self.updated_bullets.is_empty()
    }
}

// ============================================================================
// 执行结果
// ============================================================================

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

// ============================================================================
// 配置
// ============================================================================

/// ACE配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// Curator配置
#[derive(Debug, Clone)]
pub struct CuratorConfig {
    /// 最小重要性阈值
    pub min_importance: f32,

    /// 是否自动分类
    pub auto_categorize: bool,

    /// 是否生成标签
    pub generate_tags: bool,
}

impl Default for CuratorConfig {
    fn default() -> Self {
        Self {
            min_importance: 0.5,
            auto_categorize: true,
            generate_tags: true,
        }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 截断字符串到指定长度
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
