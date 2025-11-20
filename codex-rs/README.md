# codeACE - 智能上下文自适应的代码助手

codeACE 是一个增强版的代码智能助手，在原有基础上集成了 **ACE (Agentic Context Engineering)** 模块，能够自动构建和优化个人知识库，实现自我改进的智能编程辅助。

## 核心理念

传统的大语言模型通过修改权重来优化性能，而 codeACE 采用**上下文适应**（Context Adaptation）策略——通过动态构建和优化输入上下文来提升模型表现。这种方法具有以下优势：

- **可解释性强**：上下文内容可直接查看和修改
- **即时更新**：无需重新训练，运行时即可整合新知识
- **跨模型共享**：构建的知识库可在不同模型间复用

### ACE 架构设计

ACE 将上下文视为**不断演化的 Playbook**（策略手册），而非简单的提示词。系统采用三角色协作架构：

```
┌─────────────┐
│  Generator  │ ← 生成推理轨迹，执行任务
└──────┬──────┘
       │
       ↓ 执行轨迹
┌─────────────┐
│  Reflector  │ ← 分析成功/失败，提取洞察
└──────┬──────┘
       │
       ↓ 结构化洞察
┌─────────────┐
│  Curator    │ ← 组织知识，增量更新 Playbook
└──────┬──────┘
       │
       ↓ Delta 更新
┌─────────────┐
│  Playbook   │ ← 持久化知识库
└─────────────┘
```

**工作流程：**
1. **Generator** 执行任务并产生推理轨迹
2. **Reflector** 分析轨迹，识别有效策略和常见错误
3. **Curator** 将洞察组织成结构化条目（bullets），增量更新 Playbook
4. 后续任务复用 Playbook 中的知识，形成正向循环

### 关键技术特性

#### 1. 增量 Delta 更新
传统方法容易出现"上下文崩溃"——大模型在重写长上下文时会丢失细节。codeACE 采用**增量更新**策略：

- Playbook 由独立的结构化条目（bullets）组成
- 每次只生成小的 delta（新增/修改的条目）
- 使用轻量级非 LLM 逻辑合并 delta，避免信息丢失
- 支持并行处理多个 delta，大幅提升效率

#### 2. Grow-and-Refine 机制
平衡知识积累与去重：

- **Grow（增长）**：新条目持续追加，保留详细信息
- **Refine（精炼）**：使用语义去重（基于嵌入）移除冗余
- 支持懒惰精炼（上下文溢出时触发）或主动精炼（每次更新后）

#### 3. 结构化条目设计
每个 bullet 包含：
- **元数据**：唯一标识符、成功/失败计数器、召回统计
- **内容**：可复用的策略、领域概念、常见错误模式

这种设计支持：
- 局部更新（只修改相关条目）
- 细粒度检索（按相关性筛选）
- 增量适应（合并、修剪、去重）

## LAPS 系统 - 轻量级自适应 Playbook 系统

LAPS (Lightweight Adaptive Playbook System) 是 codeACE 的核心创新，在后台透明运行，提供智能优化：

### 技术架构

```
┌──────────────────────────────────────────────────────────┐
│                    LAPS 系统架构                          │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐   │
│  │   内容分类   │→│  召回追踪   │→│  动态权重     │   │
│  │ ContentType │  │ RecallCount │  │ Weight=f(...)│   │
│  └─────────────┘  └─────────────┘  └──────────────┘   │
│         ↓                                                │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐   │
│  │ 轻量级索引   │  │ 知识图谱    │  │ 后台优化器    │   │
│  │ HashMap+BTree│←│ 10领域+9语言│→│ 去重+清理     │   │
│  └─────────────┘  └─────────────┘  └──────────────┘   │
│         ↓                 ↓                 ↓           │
│  ┌──────────────────────────────────────────────────┐  │
│  │              Playbook (JSON 存储)                 │  │
│  └──────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
```

### 核心模块说明

#### 1. 智能内容管理（Content Classifier）
**位置**：`core/src/ace/content_classifier.rs`

- **自动分类**：识别 6 种内容类型
  - CodeSnippet（代码片段）
  - ErrorSolution（错误解决方案）
  - StrategyRule（策略规则）
  - ToolUsage（工具使用）
  - ApiGuide（API 指南）
  - ProjectSpecific（项目特定知识）

- **自适应验证**：根据类型动态调整长度要求
  ```rust
  CodeSnippet:     100-3000 字符
  ErrorSolution:   50-1000 字符
  StrategyRule:    30-400 字符
  ToolUsage:       30-500 字符
  ApiGuide:        80-2000 字符
  ProjectSpecific: 50-1000 字符
  ```

- **质量评分**：自动过滤通用错误信息、过短内容等低质量数据

#### 2. 动态权重系统（Recall Tracker）
**位置**：`core/src/ace/recall_tracker.rs`

**权重公式**：
```rust
weight = importance × ln(1 + recall_count) × success_rate × recency_factor
```

- `importance`：内容基础重要性
- `ln(1 + recall_count)`：对数增长的召回频率（避免线性膨胀）
- `success_rate`：成功率（成功次数 / 总召回次数）
- `recency_factor`：时效性衰减因子

**追踪数据**：
- 召回次数和时间戳
- 最近 10 次召回的上下文
- 成功/失败计数

#### 3. 高效检索（Lightweight Index）
**位置**：`core/src/ace/lightweight_index.rs`

**索引结构**：
```rust
pub struct LightweightIndex {
    // 主索引：bullet_id → Bullet
    bullet_map: HashMap<String, Arc<Bullet>>,

    // 倒排索引：keyword → [bullet_ids]
    inverted_index: HashMap<String, Vec<String>>,

    // 权重排序：weight → bullet_id
    weight_index: BTreeMap<OrderedFloat<f32>, String>,

    // LRU 热度缓存（100 项）
    hot_cache: LruCache<String, Arc<Bullet>>,
}
```

**性能指标**：
- 构建索引：O(n × m)，n=bullets数，m=平均关键词数
- 关键词查询：O(k × log n)，k=匹配的bullets数
- 权重排序：O(log n)
- 缓存命中：O(1)

**查询响应时间**：< 10ms（万级 bullets）

#### 4. 跨领域知识图谱（Knowledge Scope）
**位置**：`core/src/ace/knowledge_scope.rs`

**领域分类**（10 种）：
- WebDevelopment（Web 开发）
- SystemProgramming（系统编程）
- DataScience（数据科学）
- DevOps（运维）
- MachineLearning（机器学习）
- DatabaseManagement（数据库）
- Security（安全）
- MobileApp（移动应用）
- GameDevelopment（游戏开发）
- General（通用）

**语言识别**（9 种）：
Rust, Python, JavaScript, TypeScript, Go, Java, C++, Shell, SQL

**上下文匹配算法**：
```rust
score = domain_match × 0.4
      + language_match × 0.3
      + project_match × 0.2
      + tag_overlap × 0.1
```

根据当前项目上下文自动推荐相关知识。

#### 5. 后台自动优化（Background Optimizer）
**位置**：`core/src/ace/background_optimizer.rs`

**优化策略**：

**a) 智能去重**
- 使用高级相似度算法（详见 similarity.rs）
- 阈值：85% 相似度自动合并
- 权重优先：保留高权重 bullet，删除低权重重复项

**b) 低价值清理**
自动删除满足以下条件的 bullets：
- 30 天未使用
- 失败率 > 70%
- 内容长度不足（低于类型最低要求）

**c) 智能保护**
永不删除：
- 最近 7 天使用过的 bullets
- 成功率 > 80% 的高质量 bullets
- 重要性标记 > 0.8 的 bullets

**d) 定期权重重算**
每次优化时重新计算所有 bullets 的动态权重，确保反映最新使用情况。

#### 6. 高级相似度计算（Similarity Calculator）
**位置**：`core/src/ace/similarity.rs`

**算法组合**：
```rust
combined_similarity =
    0.4 × levenshtein_similarity +
    0.3 × bigram_similarity +
    0.3 × trigram_similarity
```

**Levenshtein 距离**：
- 基于动态规划的编辑距离
- 时间复杂度：O(m × n)
- 精确计算字符级差异

**N-gram 相似度**：
- 提取 2-gram 和 3-gram 特征
- 计算 Jaccard 相似度
- 捕捉局部模式相似性

**文本归一化**：
```rust
pub fn normalize_text(text: &str, remove_punctuation: bool) -> String {
    // 1. 转小写
    // 2. 保留中日韩字符（is_cjk 检测）
    // 3. 可选去除标点
    // 4. 压缩多余空白
}
```

**性能**：
- 短文本（< 100 字符）：< 1ms
- 中等文本（100-500 字符）：< 5ms
- 长文本（500-3000 字符）：< 20ms

### LAPS 系统优势

1. **零数据库依赖**
   - 纯内存索引 + JSON 文件存储
   - 无需安装和维护额外数据库
   - 数据文件可直接查看和备份

2. **轻量级设计**
   - 核心依赖仅 `lru` 缓存库（50KB）
   - 其余功能纯 Rust 实现
   - 内存占用 < 50MB（万级 bullets）

3. **高性能表现**
   - 查询响应：< 10ms
   - 增量更新：< 5ms
   - 去重处理：1000 bullets < 500ms

4. **完整测试覆盖**
   - 110+ 单元测试和集成测试
   - 覆盖所有核心功能和边界情况
   - 测试文件：`core/tests/laps_integration_test.rs`

## 安装 codeACE

### 通过源码构建

```bash
cd codex-rs
cargo build --release
```

ACE 模块已默认编译，无需额外配置。

### 配置文件

编辑 `~/.codeACE/config.toml` 启用 ACE：

```toml
[ace]
enabled = true
storage_path = "~/.codeACE/ace/playbook.json"
max_entries = 10000

[ace.reflector]
extract_patterns = true
extract_tools = true
extract_errors = true
```

## 快速开始

### 基本使用

```bash
# 启动交互式 TUI
cargo run --bin codex

# 非交互模式执行
cargo run --bin codex exec "你的任务描述"

# 查看详细日志
RUST_LOG=debug cargo run --bin codex
```

### ACE Playbook 管理

```bash
# 查看当前 Playbook 统计
codex ace stats

# 手动触发优化
codex ace optimize

# 导出 Playbook
codex ace export --output ./my-playbook.json

# 导入 Playbook
codex ace import --input ./my-playbook.json
```

## 项目结构

```
codex-rs/
├── core/                    # 核心业务逻辑
│   ├── src/
│   │   ├── ace/             # ACE 模块
│   │   │   ├── background_optimizer.rs   # 后台优化器
│   │   │   ├── content_classifier.rs     # 内容分类器
│   │   │   ├── curator.rs                # 策展器
│   │   │   ├── knowledge_scope.rs        # 知识图谱
│   │   │   ├── lightweight_index.rs      # 轻量级索引
│   │   │   ├── recall_tracker.rs         # 召回追踪
│   │   │   ├── reflector.rs              # 反思器
│   │   │   ├── similarity.rs             # 相似度计算
│   │   │   ├── storage.rs                # 存储管理
│   │   │   └── types.rs                  # 类型定义
│   │   └── ...
│   └── tests/
│       └── laps_integration_test.rs      # LAPS 集成测试
├── tui/                     # 终端界面
├── exec/                    # 无头 CLI
└── cli/                     # 命令行工具
```

## 技术文档

- **ACE 实现报告**：`ref/FINAL-IMPLEMENTATION-REPORT.md`
- **LAPS 详细文档**：`ref/v2/`
  - `plan-detail.md` - 设计方案
  - `phase1-completion.md` - 第一阶段完成报告
  - `phase2-completion.md` - 第二阶段完成报告
  - `phase3-completion.md` - 第三阶段完成报告
  - `steps.md` - 完整实施记录

## 配置选项

详见 [`docs/config.md`](../docs/config.md)

### Model Context Protocol (MCP)

codeACE 同时支持 MCP 客户端和服务器模式：

```bash
# 作为 MCP 服务器运行
codex mcp-server

# 管理 MCP 服务器配置
codex mcp list
codex mcp add <name> <command>
codex mcp remove <name>
```

### 沙箱模式

```bash
# 只读模式（默认）
codex --sandbox read-only

# 允许工作区写入
codex --sandbox workspace-write

# 完全访问（需在隔离环境使用）
codex --sandbox danger-full-access
```

配置文件持久化：

```toml
sandbox_mode = "workspace-write"
```

## 平台支持

- **macOS 12+**：Seatbelt 沙箱
- **Linux**：Ubuntu 20.04+ / Debian 10+（Landlock 或 Docker）
- **Windows 11**：仅支持 WSL2

## 性能特性

### ACE 系统性能

| 操作类型 | 响应时间 | 内存占用 |
|---------|---------|---------|
| 检索相关 bullets | < 10ms | < 50MB |
| 增量更新 | < 5ms | 临时 < 10MB |
| 智能去重（1000 项） | < 500ms | 临时 < 20MB |
| Playbook 加载 | < 100ms | 持久 < 50MB |

### 长上下文优化

得益于 KV cache 复用技术，长上下文不会带来线性成本增长：
- 上下文片段可本地或远程缓存
- 避免重复的 prefill 操作
- 适合频繁复用的 Playbook 场景

## 相关资源

- [快速入门](../docs/getting-started.md)
- [进阶使用](../docs/advanced.md)
- [配置参考](../docs/config.md)
- [GitHub 仓库](https://github.com/UU114/codeACE)

## License

查看 LICENSE 文件了解详情。
