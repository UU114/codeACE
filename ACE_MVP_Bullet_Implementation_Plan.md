# ACE MVP Bullet-based 实现计划

> 基于论文对比分析，聚焦现阶段核心功能

## 📋 目标范围

### ✅ 现阶段实现（MVP）
1. **Bullet-based 数据结构**（细粒度 metadata）
2. **Reflector 输出 insights**（而非直接生成 Entry）
3. **Curator MVP**（将 insights 组织成 structured bullets）
4. **Incremental Delta Updates**（增量更新机制）
5. **Storage 支持 bullet 操作**（append/update/query）

### ⏸️ 推迟到第二阶段
- ❌ 去重（semantic embedding）
- ❌ 高级检索（向量搜索）
- ❌ Grow-and-Refine（语义去重）
- ❌ Generator 反馈标记

---

## 🏗️ 架构设计

### 数据流向
```
用户对话
    ↓
Reflector (规则提取)
    ↓
RawInsights (未结构化的洞察)
    ↓
Curator (组织整理)
    ↓
DeltaContext (增量 bullets)
    ↓
Storage (合并到 playbook)
    ↓
Playbook (bullet 集合)
    ↓
Context Loader (检索 bullets)
    ↓
增强的 prompt
```

### 核心组件关系
```
┌─────────────┐
│  Reflector  │ 提取 RawInsights
└──────┬──────┘
       │
       ↓
┌─────────────┐
│   Curator   │ 生成 DeltaContext
└──────┬──────┘
       │
       ↓
┌─────────────┐
│   Storage   │ 合并 bullets
└─────────────┘
```

---

## 📐 数据结构设计

### 1. Bullet 数据结构（核心单元）

```rust
/// 一条可执行的规则/策略/知识点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bullet {
    /// 唯一标识符（uuid）
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

    /// 元数据（细粒度跟踪）
    pub metadata: BulletMetadata,

    /// 关联的标签（用于检索）
    pub tags: Vec<String>,
}

/// 分类（参考论文 Figure 3）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    pub fn new(
        section: BulletSection,
        content: String,
        source_session_id: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            updated_at: now,
            source_session_id,
            section,
            content,
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
```

### 2. Playbook 结构（bullet 集合）

```rust
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
        self.bullets.values()
            .flatten()
            .find(|b| b.id == id)
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
}
```

### 3. RawInsight（Reflector 输出）

```rust
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

/// 洞察类别（与原来一致）
#[derive(Debug, Clone, PartialEq)]
pub enum InsightCategory {
    ToolUsage,
    Pattern,
    Solution,
    Knowledge,
    ErrorHandling,
}
```

### 4. DeltaContext（Curator 输出）

```rust
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
```

---

## 🔧 组件实现

### Phase 1: 重构 Reflector（1-2天）

**文件**: `codex-rs/core/src/ace/reflector.rs`

#### 修改要点
1. **输出类型变更**：从 `PlaybookEntry` 改为 `Vec<RawInsight>`
2. **保留规则引擎**：继续使用正则表达式提取
3. **增强上下文**：为每个 insight 附加完整 context

#### 核心代码结构
```rust
pub struct ReflectorMVP {
    config: ReflectorConfig,
    patterns: HashMap<String, Regex>,
}

impl ReflectorMVP {
    /// 分析对话，返回原始洞察
    pub async fn analyze_conversation(
        &self,
        user_query: &str,
        assistant_response: &str,
        execution_result: &ExecutionResult,
        session_id: String,
    ) -> Result<Vec<RawInsight>> {
        let mut insights = Vec::new();

        // 构建上下文
        let context = InsightContext {
            user_query: user_query.to_string(),
            assistant_response_snippet: truncate(assistant_response, 500),
            execution_success: execution_result.success,
            tools_used: execution_result.tools_used.clone(),
            error_message: execution_result.error.clone(),
            session_id,
        };

        // 提取各类洞察
        if self.config.extract_tools {
            insights.extend(self.extract_tool_insights(
                assistant_response,
                &context,
            )?);
        }

        if self.config.extract_errors && !execution_result.success {
            insights.extend(self.extract_error_insights(
                execution_result,
                &context,
            )?);
        }

        if self.config.extract_patterns {
            insights.extend(self.extract_pattern_insights(
                assistant_response,
                &context,
            )?);
        }

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
                        content: format!(
                            "文件操作: {} {}",
                            action.as_str(),
                            path.as_str()
                        ),
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
                content: format!("错误: {}", truncate(error, 200)),
                category: InsightCategory::ErrorHandling,
                importance: 0.9,
                context: context.clone(),
            });

            // 如果后续成功，记录解决方案
            if result.retry_success {
                insights.push(RawInsight {
                    content: format!(
                        "解决方案: 针对错误 '{}' 的成功处理",
                        truncate(error, 100)
                    ),
                    category: InsightCategory::Solution,
                    importance: 0.95,
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

        Ok(insights)
    }
}
```

---

### Phase 2: 实现 Curator MVP（2-3天）

**文件**: `codex-rs/core/src/ace/curator.rs`（新建）

#### 职责
1. 接收 `Vec<RawInsight>`
2. 组织成 structured bullets
3. 决定 section 分类
4. 生成细粒度 metadata
5. 输出 `DeltaContext`

#### 核心代码
```rust
/// Curator MVP - 将洞察组织成结构化 bullets
pub struct CuratorMVP {
    config: CuratorConfig,
}

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

impl CuratorMVP {
    pub fn new(config: CuratorConfig) -> Self {
        Self { config }
    }

    /// 处理 insights，生成 delta
    pub async fn process_insights(
        &self,
        insights: Vec<RawInsight>,
        session_id: String,
    ) -> Result<DeltaContext> {
        let start = std::time::Instant::now();
        let mut delta = DeltaContext::new(session_id.clone());

        // 过滤低重要性的 insights
        let valuable_insights: Vec<_> = insights
            .into_iter()
            .filter(|i| i.importance >= self.config.min_importance)
            .collect();

        delta.metadata.insights_processed = valuable_insights.len();

        // 为每个 insight 生成 bullet
        for insight in valuable_insights {
            let bullet = self.create_bullet_from_insight(insight, &session_id)?;
            delta.new_bullets.push(bullet);
        }

        delta.metadata.new_bullets_count = delta.new_bullets.len();
        delta.metadata.processing_time_ms = start.elapsed().as_millis() as u64;

        Ok(delta)
    }

    /// 从 insight 创建 bullet
    fn create_bullet_from_insight(
        &self,
        insight: RawInsight,
        session_id: &str,
    ) -> Result<Bullet> {
        // 决定 section
        let section = if self.config.auto_categorize {
            self.categorize_insight(&insight)
        } else {
            BulletSection::General
        };

        // 创建 bullet
        let mut bullet = Bullet::new(
            section,
            insight.content.clone(),
            session_id.to_string(),
        );

        // 填充 metadata
        bullet.metadata = self.create_metadata(&insight)?;

        // 生成标签
        if self.config.generate_tags {
            bullet.tags = self.generate_tags(&insight);
        }

        Ok(bullet)
    }

    /// 分类逻辑（规则based）
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
            InsightCategory::ErrorHandling => {
                BulletSection::TroubleshootingAndPitfalls
            }
            InsightCategory::Solution => {
                BulletSection::TroubleshootingAndPitfalls
            }
            InsightCategory::Pattern => {
                BulletSection::StrategiesAndRules
            }
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
        let mut metadata = BulletMetadata {
            importance: insight.importance,
            source_type: self.determine_source_type(insight),
            applicability: self.extract_applicability(insight),
            reference_count: 0,
            success_count: if insight.context.execution_success { 1 } else { 0 },
            failure_count: if !insight.context.execution_success { 1 } else { 0 },
            related_tools: insight.context.tools_used.clone(),
            related_file_patterns: Vec::new(), // MVP 阶段留空
            confidence: 1.0,
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
        } else {
            if insight.category == InsightCategory::Pattern {
                SourceType::PatternRecognition
            } else {
                SourceType::ErrorResolution
            }
        }
    }

    /// 提取适用性范围
    fn extract_applicability(&self, insight: &RawInsight) -> Applicability {
        let mut applicability = Applicability::default();

        // 从内容中提取编程语言
        let content_lower = insight.content.to_lowercase();
        for lang in &["rust", "python", "javascript", "typescript", "go", "java"] {
            if content_lower.contains(lang) {
                applicability.languages.push(lang.to_string());
            }
        }

        // 工具
        applicability.tools = insight.context.tools_used.clone();

        // 平台（从上下文推断）
        // MVP 阶段简化，留空

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
            tags.push(format!("tool:{}", tool));
        }

        // 成功/失败标签
        if insight.context.execution_success {
            tags.push("success".to_string());
        } else {
            tags.push("failed".to_string());
        }

        // 去重排序
        tags.sort();
        tags.dedup();

        tags
    }
}
```

---

### Phase 3: 实现 Incremental Updates Storage（2-3天）

**文件**: `codex-rs/core/src/ace/storage.rs`（重构）

#### 修改要点
1. 存储格式：从 `PlaybookEntry` 改为 `Playbook`（含 bullets）
2. 支持增量合并：`merge_delta`
3. 支持 bullet 查询/更新

#### 核心代码
```rust
/// Storage for bullet-based playbook
pub struct BulletStorage {
    playbook_path: PathBuf,
    archive_dir: PathBuf,
    max_bullets: usize,
}

impl BulletStorage {
    pub fn new(base_path: &str, max_bullets: usize) -> Result<Self> {
        let base = expand_path(base_path);
        let playbook_path = base.join("playbook.json");
        let archive_dir = base.join("archive");

        // 创建目录
        std::fs::create_dir_all(&base)?;
        std::fs::create_dir_all(&archive_dir)?;

        Ok(Self {
            playbook_path,
            archive_dir,
            max_bullets,
        })
    }

    /// 加载 playbook
    pub async fn load_playbook(&self) -> Result<Playbook> {
        if !self.playbook_path.exists() {
            return Ok(Playbook::new());
        }

        let content = tokio::fs::read_to_string(&self.playbook_path).await?;
        let playbook: Playbook = serde_json::from_str(&content)?;
        Ok(playbook)
    }

    /// 保存 playbook
    pub async fn save_playbook(&self, playbook: &Playbook) -> Result<()> {
        let json = serde_json::to_string_pretty(playbook)?;
        tokio::fs::write(&self.playbook_path, json).await?;
        Ok(())
    }

    /// **核心方法**: 合并 delta（增量更新）
    pub async fn merge_delta(&self, delta: DeltaContext) -> Result<()> {
        if delta.is_empty() {
            tracing::debug!("Delta is empty, skipping merge");
            return Ok(());
        }

        // 加载现有 playbook
        let mut playbook = self.load_playbook().await?;

        tracing::info!(
            "Merging delta: {} new bullets, {} updated bullets",
            delta.new_bullets.len(),
            delta.updated_bullets.len()
        );

        // 1. 添加新 bullets
        for bullet in delta.new_bullets {
            playbook.add_bullet(bullet);
        }

        // 2. 更新现有 bullets
        for bullet in delta.updated_bullets {
            if !playbook.update_bullet(bullet) {
                tracing::warn!("Failed to update bullet (not found)");
            }
        }

        // 3. 检查是否需要归档
        if playbook.metadata.total_bullets > self.max_bullets {
            self.auto_archive(&mut playbook).await?;
        }

        // 4. 保存
        self.save_playbook(&playbook).await?;

        tracing::info!(
            "Delta merged successfully. Total bullets: {}",
            playbook.metadata.total_bullets
        );

        Ok(())
    }

    /// 查询 bullets（用于 context loading）
    pub async fn query_bullets(
        &self,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<Bullet>> {
        let playbook = self.load_playbook().await?;
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        // 简单的关键词匹配（MVP）
        for bullets in playbook.bullets.values() {
            for bullet in bullets {
                let content_lower = bullet.content.to_lowercase();
                let tags_str = bullet.tags.join(" ").to_lowercase();

                // 计算相关性分数
                let mut score = 0;

                // 内容匹配
                if content_lower.contains(&query_lower) {
                    score += 3;
                }

                // 标签匹配
                for keyword in query_lower.split_whitespace() {
                    if tags_str.contains(keyword) {
                        score += 2;
                    }
                }

                // 工具匹配
                for tool in &bullet.metadata.related_tools {
                    if query_lower.contains(&tool.to_lowercase()) {
                        score += 2;
                    }
                }

                if score > 0 {
                    results.push((bullet.clone(), score));
                }
            }
        }

        // 按分数排序
        results.sort_by(|a, b| b.1.cmp(&a.1));

        // 返回前 N 个
        Ok(results
            .into_iter()
            .take(max_results)
            .map(|(bullet, _)| bullet)
            .collect())
    }

    /// 自动归档旧 bullets
    async fn auto_archive(&self, playbook: &mut Playbook) -> Result<()> {
        tracing::info!(
            "Auto-archiving: {} bullets exceed limit {}",
            playbook.metadata.total_bullets,
            self.max_bullets
        );

        // 生成归档文件名
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let archive_path = self.archive_dir.join(format!(
            "playbook_{}.json",
            timestamp
        ));

        // 保存当前 playbook 到归档
        let json = serde_json::to_string_pretty(playbook)?;
        tokio::fs::write(&archive_path, json).await?;

        // 清空当前 playbook（保留最近的一部分）
        // MVP: 简单截断策略
        let keep_ratio = 0.7; // 保留 70%
        let keep_count = (self.max_bullets as f32 * keep_ratio) as usize;

        // 按更新时间排序，保留最新的
        let mut all_bullets: Vec<_> = playbook.bullets
            .values()
            .flatten()
            .cloned()
            .collect();
        all_bullets.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        // 重建 playbook
        *playbook = Playbook::new();
        for bullet in all_bullets.into_iter().take(keep_count) {
            playbook.add_bullet(bullet);
        }

        tracing::info!(
            "Archive completed: {} bullets retained",
            playbook.metadata.total_bullets
        );

        Ok(())
    }

    /// 清空 playbook
    pub async fn clear(&self) -> Result<()> {
        let playbook = Playbook::new();
        self.save_playbook(&playbook).await?;
        Ok(())
    }
}
```

---

### Phase 4: 集成到主流程（1天）

**文件**: `codex-rs/core/src/ace/mod.rs`

#### 修改 ACE Plugin 主流程

```rust
/// ACE Plugin - 协调所有组件
pub struct ACEPlugin {
    reflector: ReflectorMVP,
    curator: CuratorMVP,
    storage: BulletStorage,
    context_loader: SimpleContextLoader, // 复用现有
    config: ACEConfig,
}

impl ACEPlugin {
    pub fn new(config: ACEConfig) -> Result<Self> {
        let reflector = ReflectorMVP::new(config.reflector.clone().into());
        let curator = CuratorMVP::new(CuratorConfig::default());
        let storage = BulletStorage::new(&config.storage_path, config.max_entries)?;
        let context_loader = SimpleContextLoader::new(
            Arc::new(storage.clone()), // 需要实现共享
            config.context.max_context_chars,
        );

        Ok(Self {
            reflector,
            curator,
            storage,
            context_loader,
            config,
        })
    }
}

#[async_trait]
impl ExecutorHook for ACEPlugin {
    async fn pre_execute(&self, user_input: &str) -> Option<String> {
        if !self.config.enabled {
            return None;
        }

        // 从 storage 查询相关 bullets
        match self.storage.query_bullets(user_input, 10).await {
            Ok(bullets) => {
                if bullets.is_empty() {
                    None
                } else {
                    Some(self.format_bullets_as_context(bullets))
                }
            }
            Err(e) => {
                tracing::error!("Failed to query bullets: {}", e);
                None
            }
        }
    }

    fn post_execute(&self, query: &str, response: &str, success: bool) {
        if !self.config.enabled {
            return;
        }

        // 异步处理（不阻塞主流程）
        let reflector = self.reflector.clone();
        let curator = self.curator.clone();
        let storage = self.storage.clone();
        let query = query.to_string();
        let response = response.to_string();
        let session_id = Uuid::new_v4().to_string();

        tokio::spawn(async move {
            // 1. Reflector 分析
            let execution_result = ExecutionResult {
                success,
                ..Default::default()
            };

            let insights = match reflector.analyze_conversation(
                &query,
                &response,
                &execution_result,
                session_id.clone(),
            ).await {
                Ok(insights) => insights,
                Err(e) => {
                    tracing::error!("Reflector failed: {}", e);
                    return;
                }
            };

            if insights.is_empty() {
                tracing::debug!("No valuable insights extracted");
                return;
            }

            tracing::info!("Extracted {} insights", insights.len());

            // 2. Curator 生成 delta
            let delta = match curator.process_insights(insights, session_id).await {
                Ok(delta) => delta,
                Err(e) => {
                    tracing::error!("Curator failed: {}", e);
                    return;
                }
            };

            if delta.is_empty() {
                tracing::debug!("Delta is empty, nothing to merge");
                return;
            }

            tracing::info!(
                "Generated delta: {} new bullets, {} updated",
                delta.new_bullets.len(),
                delta.updated_bullets.len()
            );

            // 3. Storage 合并 delta
            if let Err(e) = storage.merge_delta(delta).await {
                tracing::error!("Failed to merge delta: {}", e);
            } else {
                tracing::info!("Delta merged successfully");
            }
        });
    }
}

impl ACEPlugin {
    /// 格式化 bullets 为上下文字符串
    fn format_bullets_as_context(&self, bullets: Vec<Bullet>) -> String {
        let mut output = String::from("# 📚 ACE Playbook Context\n\n");
        output.push_str(&format!("Found {} relevant strategies:\n\n", bullets.len()));

        // 按 section 分组
        let mut by_section: HashMap<BulletSection, Vec<&Bullet>> = HashMap::new();
        for bullet in &bullets {
            by_section
                .entry(bullet.section.clone())
                .or_insert_with(Vec::new)
                .push(bullet);
        }

        // 格式化输出
        for (section, bullets) in by_section {
            output.push_str(&format!("## {}\n\n", self.section_title(&section)));

            for bullet in bullets {
                output.push_str(&format!("- {}\n", bullet.content));

                // 显示相关工具
                if !bullet.metadata.related_tools.is_empty() {
                    output.push_str(&format!(
                        "  - Tools: {}\n",
                        bullet.metadata.related_tools.join(", ")
                    ));
                }

                // 显示成功率
                let total = bullet.metadata.success_count + bullet.metadata.failure_count;
                if total > 0 {
                    let success_rate = (bullet.metadata.success_count as f32 / total as f32) * 100.0;
                    output.push_str(&format!("  - Success rate: {:.0}%\n", success_rate));
                }

                output.push('\n');
            }
        }

        output
    }

    fn section_title(&self, section: &BulletSection) -> &str {
        match section {
            BulletSection::StrategiesAndRules => "Strategies and Rules",
            BulletSection::CodeSnippetsAndTemplates => "Code Snippets and Templates",
            BulletSection::TroubleshootingAndPitfalls => "Troubleshooting and Pitfalls",
            BulletSection::ApiUsageGuides => "API Usage Guides",
            BulletSection::ErrorHandlingPatterns => "Error Handling Patterns",
            BulletSection::ToolUsageTips => "Tool Usage Tips",
            BulletSection::General => "General Knowledge",
        }
    }
}
```

---

## 🧪 测试计划

### 单元测试

#### 1. Reflector 测试
```rust
#[tokio::test]
async fn test_reflector_extracts_insights() {
    let reflector = ReflectorMVP::new(ReflectorConfig::default());

    let query = "Run tests for the project";
    let response = "I'll run `cargo test`";
    let result = ExecutionResult {
        success: true,
        tools_used: vec!["bash".to_string()],
        ..Default::default()
    };

    let insights = reflector
        .analyze_conversation(query, response, &result, "test-session".to_string())
        .await
        .unwrap();

    assert!(!insights.is_empty());
    assert!(insights.iter().any(|i| i.category == InsightCategory::ToolUsage));
}
```

#### 2. Curator 测试
```rust
#[tokio::test]
async fn test_curator_generates_bullets() {
    let curator = CuratorMVP::new(CuratorConfig::default());

    let insight = RawInsight {
        content: "使用命令: cargo test".to_string(),
        category: InsightCategory::ToolUsage,
        importance: 0.7,
        context: InsightContext {
            user_query: "Run tests".to_string(),
            assistant_response_snippet: "...".to_string(),
            execution_success: true,
            tools_used: vec!["bash".to_string()],
            error_message: None,
            session_id: "test".to_string(),
        },
    };

    let delta = curator
        .process_insights(vec![insight], "test-session".to_string())
        .await
        .unwrap();

    assert_eq!(delta.new_bullets.len(), 1);
    assert_eq!(delta.new_bullets[0].section, BulletSection::ToolUsageTips);
}
```

#### 3. Storage 测试
```rust
#[tokio::test]
async fn test_storage_merge_delta() {
    let temp_dir = tempfile::tempdir().unwrap();
    let storage = BulletStorage::new(
        temp_dir.path().to_str().unwrap(),
        100,
    ).unwrap();

    // 创建 delta
    let bullet = Bullet::new(
        BulletSection::StrategiesAndRules,
        "Test strategy".to_string(),
        "session-1".to_string(),
    );

    let mut delta = DeltaContext::new("session-1".to_string());
    delta.new_bullets.push(bullet);

    // 合并
    storage.merge_delta(delta).await.unwrap();

    // 验证
    let playbook = storage.load_playbook().await.unwrap();
    assert_eq!(playbook.metadata.total_bullets, 1);
}
```

### 集成测试

```rust
#[tokio::test]
async fn test_ace_end_to_end() {
    let config = ACEConfig::default();
    let ace = ACEPlugin::new(config).unwrap();

    // 模拟对话
    let query = "How do I run tests?";
    let response = "Use `cargo test` to run tests";

    // Post-execute（学习）
    ace.post_execute(query, response, true);

    // 等待异步处理
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Pre-execute（检索）
    let context = ace.pre_execute("run tests").await;
    assert!(context.is_some());
    assert!(context.unwrap().contains("cargo test"));
}
```

---

## 📅 实施时间表

### Week 1: 数据结构 + Reflector（3-4天）
- Day 1-2: 实现所有数据结构（`types.rs`）
  - `Bullet`, `Playbook`, `RawInsight`, `DeltaContext`
  - 单元测试
- Day 3-4: 重构 Reflector
  - 修改输出为 `Vec<RawInsight>`
  - 增强上下文信息
  - 单元测试

### Week 2: Curator + Storage（4-5天）
- Day 5-7: 实现 Curator MVP
  - 分类逻辑
  - Metadata 生成
  - Delta 生成
  - 单元测试
- Day 8-9: 重构 Storage
  - Bullet-based 存储
  - `merge_delta` 实现
  - 查询方法
  - 单元测试

### Week 3: 集成 + 测试（2-3天）
- Day 10-11: 集成到主流程
  - 修改 `ACEPlugin`
  - Hook 调用链路
  - 端到端测试
- Day 12: 完善和文档
  - 补充测试
  - 更新文档
  - Bug 修复

---

## 📊 验收标准

### 功能完整性
- [x] Reflector 能提取 insights（而非直接 Entry）
- [x] Curator 能将 insights 转为 bullets
- [x] Storage 支持 delta 增量合并
- [x] Playbook 以 JSON 格式存储
- [x] Context Loader 能检索 bullets

### 数据质量
- [x] Bullet 包含细粒度 metadata
- [x] 自动分类到正确 section
- [x] 标签生成准确
- [x] 适用性范围正确提取

### 性能要求
- [x] Post-execute 不阻塞主流程
- [x] Delta 合并 < 100ms
- [x] Context 加载 < 50ms
- [x] 内存占用 < 50MB

### 测试覆盖
- [x] 单元测试覆盖率 > 80%
- [x] 集成测试通过
- [x] 端到端测试验证完整流程

---

## 🔄 与现有代码的兼容性

### 保留的部分
- ✅ Hook 机制（`ExecutorHook` trait）
- ✅ 配置系统（`ACEConfig`）
- ✅ Feature flag 控制
- ✅ 异步处理模式

### 废弃的部分
- ❌ `PlaybookEntry` 数据结构（替换为 `Bullet` + `Playbook`）
- ❌ JSONL 追加式存储（改为 JSON 整体存储）
- ❌ Reflector 直接生成 Entry（改为生成 insights）

### 迁移策略
1. 保留旧代码，通过 feature flag 切换
2. 提供迁移工具（旧格式 → 新格式）
3. 文档说明兼容性变更

---

## ❓ FAQ

### Q1: 为什么不直接用 JSONL 追加？
**A**: 论文强调增量更新而非追加。Bullet 需要支持原地更新（如增加 reference_count），JSONL 追加模式无法高效支持。JSON 整体存储更适合 MVP 阶段。

### Q2: 细粒度 metadata 会不会太复杂？
**A**: MVP 阶段大部分字段可以留空或使用默认值。关键字段（importance、source_type、success_count）会在 Curator 中自动填充，不增加复杂度。

### Q3: Curator 是否需要 LLM？
**A**: MVP 阶段使用规则引擎即可。论文中的 Curator 也可以是规则based，LLM 是可选优化（第二阶段）。

### Q4: 如何保证 delta 合并的原子性？
**A**: MVP 阶段使用文件锁或简单的"读-修改-写"模式。性能足够时不需要复杂的并发控制。

### Q5: 旧数据如何迁移？
**A**: 提供工具脚本，读取旧 JSONL，为每个 Entry 生成对应的 Bullets，保存为新格式。可在第二阶段实现。

---

## 📚 参考资料

- **论文**: Agentic Context Engineering (2510.04618v1.pdf)
- **关键章节**:
  - Section 3.1: Incremental Delta Updates
  - Section 3.2: Grow-and-Refine
  - Figure 3: Example ACE-Generated Context
  - Figure 4: ACE Framework Architecture

---

**总结**: 现阶段聚焦 Bullet-based 架构 + Curator + Incremental Updates，为第二阶段的智能化优化打好基础。
