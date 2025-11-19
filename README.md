# CodeACE - Agentic Context Engineering for Codex

> 为 OpenAI Codex CLI 添加智能学习能力，让 AI 从对话中学习并持续改进

[![Status](https://img.shields.io/badge/Status-Phase_1_MVP-green.svg)](https://github.com/UU114/codeACE)
[![Tests](https://img.shields.io/badge/Tests-100%25-brightgreen.svg)](https://github.com/UU114/codeACE)
[![Rust](https://img.shields.io/badge/Rust-1.82+-orange.svg)](https://www.rust-lang.org)

---

## ⚠️ 重要说明

**本项目是对 OpenAI Codex CLI 的改造项目**，在原有基础上添加了 ACE (Agentic Context Engineering) 智能学习框架。

- ✅ 保留 Codex CLI 的所有原有功能
- ✅ 新增智能学习和上下文记忆能力
- ❌ 本文档**仅介绍 ACE 扩展功能**
- ❌ 不包含 Codex CLI 的基础使用说明

**需要 Codex CLI 的使用文档？** 请访问 [OpenAI Codex CLI 官方仓库](https://github.com/openai/codex)

---

## 🎯 什么是 ACE？

ACE (Agentic Context Engineering) 是一个智能上下文工程框架，让 AI 助手能够从对话历史中学习，构建可演化的知识库（Playbook），并在后续对话中提供相关经验。

### ACE 的核心原理

根据论文 *"Agentic Context Engineering: Evolving Contexts for Self-Improving Language Models"*，ACE 通过以下机制实现智能学习：

1. **上下文适应（Context Adaptation）**：通过修改输入上下文而非模型权重来改进性能
2. **避免简洁偏差（Brevity Bias）**：保留详细的领域特定知识，而非压缩成简短摘要
3. **防止上下文崩溃（Context Collapse）**：使用增量更新而非整体重写，避免信息丢失
4. **Playbook 演化**：将上下文视为持续积累和组织策略的演化知识库

### 核心能力

- 🧠 **自动学习（Reflector）** - 从对话中提取工具使用、错误处理、开发模式
- 📚 **知识积累（Playbook）** - 构建可演化的结构化知识库
- 🎯 **增量更新（Delta Updates）** - 局部更新而非整体重写，防止信息丢失
- 🔄 **生长和精炼（Grow-and-Refine）** - 平衡知识扩展与冗余控制
- 🔍 **智能检索** - 基于关键词和语义的相关上下文匹配
- ⚡ **高性能** - 极快的学习和检索（< 100ms）
- 🔌 **最小侵入** - 通过 Hook 机制集成，不污染原有代码
- 🚀 **即用即学** - 自动创建配置，开箱即用

---

## 🚀 快速开始

### 1️⃣ 克隆项目

```bash
git clone https://github.com/UU114/codeACE.git
cd codeACE
```

### 2️⃣ 编译

```bash
cd codex-rs

# 编译 release 版本
cargo build --release

# 或者编译 debug 版本用于开发
cargo build
```

**✨ 从 v1.0 开始，ACE 功能已默认编译启用**，无需额外添加 feature 标志！

如果需要禁用 ACE 功能，可以使用：
```bash
cargo build --release --no-default-features
```

### 3️⃣ 安装到系统

```bash
# 方式1: 使用 cargo install（推荐）
cargo install --path cli

# 方式2: 手动复制二进制文件
cp target/release/codex ~/.local/bin/
# 或其他在你的 PATH 中的目录
```

### 4️⃣ 使用

```bash
# 使用方式与 Codex CLI 完全相同
codex tui                          # 启动 TUI 界面
codex exec "你的问题"               # 命令行模式

# ACE 在后台自动工作：
# - 对话前（pre_execute Hook）：加载相关历史上下文
# - 对话后（post_execute Hook）：异步学习并提取知识
```

### 5️⃣ 验证 ACE 功能

```bash
# 查看 ACE 状态
codex ace status

# 应该看到类似输出：
# 📚 ACE (Agentic Coding Environment) Status
#
# Configuration:
#   Enabled: ✅ Yes
#   Storage: ~/.codeACE/ace
#   Max entries: 500
```

---

## 💡 ACE 如何工作？

### 三大关键创新

根据论文，ACE 引入三个关键创新来解决现有方法的局限性：

#### 1️⃣ 独立的 Reflector 模块
- **问题**：以往方法由单一模型承担所有职责，导致质量下降
- **解决**：将评估和洞察提取分离为独立的 Reflector 角色
- **效果**：显著提高上下文质量和下游性能（§4.5 消融实验证明）

#### 2️⃣ 增量 Delta 更新
- **问题**：整体重写（monolithic rewrite）代价高且容易导致上下文崩溃
- **解决**：使用局部的、增量的 delta 更新，只修改相关部分
- **效果**：降低 82-92% 的适应延迟和计算成本（§4.6）

#### 3️⃣ Grow-and-Refine 机制
- **问题**：简洁偏差导致丢失领域特定知识
- **解决**：平衡稳定的上下文扩展与冗余控制
- **效果**：保持详细的、任务特定的知识，防止信息压缩

### 工作流程

```
用户提问
  ↓
[pre_execute Hook] 加载相关历史上下文
  ↓
Generator: 生成推理轨迹和执行
  ↓
[post_execute Hook] 异步学习流程：
  ├─ Reflector: 分析轨迹，提取洞察（可多轮迭代）
  ├─ Curator: 生成 delta context items
  └─ Storage: 增量合并到 Playbook
  ↓
完成（用户无感知）
```

### 使用示例

```bash
# 第一次询问
$ codex "How do I run tests?"
> You can run tests using: cargo test

# ACE 自动学习:
✓ 提取: 工具使用 "cargo test"
✓ 标签: testing, tools
✓ 保存到 playbook

# 第二次类似询问
$ codex "Run unit tests"
> Based on previous experience, use: cargo test
> (上下文已自动加载 ✨)
```

---

## 🔧 配置

### 配置文件位置

ACE 使用**独立的配置文件**（与 Codex CLI 主配置分离）：

```
~/.codeACE/codeACE-config.toml
```

### 自动创建

首次运行时，ACE 会自动创建配置文件，**无需手动配置**。

### 自定义配置（可选）

```toml
[ace]
enabled = true                    # 启用/禁用 ACE
storage_path = "~/.codeACE/ace"  # 知识库存储路径
max_entries = 500                 # 最大条目数

[ace.reflector]
extract_patterns = true           # 提取代码模式
extract_tools = true              # 提取工具使用
extract_errors = true             # 提取错误信息

[ace.context]
max_recent_entries = 10           # 每次加载的最大上下文数
include_all_successes = true      # 包含所有成功案例
max_context_chars = 4000          # 上下文最大字符数
```

### 禁用 ACE

方式1：通过配置文件临时禁用（保留 ACE 代码）
```toml
[ace]
enabled = false
```

方式2：编译时完全移除 ACE 功能（减小二进制体积）
```bash
cd codex-rs
cargo build --release --no-default-features
```

---

## 📊 ACE Playbook 管理

### CLI 命令

ACE 提供了一套管理工具来查看和管理学习内容：

```bash
codex ace status   # 查看学习状态和统计
codex ace show     # 显示学习内容（默认 10 条）
codex ace search   # 搜索知识库
codex ace config   # 查看配置
codex ace clear    # 清空知识库（自动归档）
```

### TUI 斜杠命令 🆕

在 Codex TUI 交互界面中，可以使用以下斜杠命令快速访问 playbook：

```bash
/playbook         # 显示 playbook 状态（别名：/pb）
/playbook-show    # 显示最近学习条目（别名：/pbs）
/playbook-clear   # 清空 playbook（别名：/pbc）
/playbook-search  # 搜索 playbook（别名：/pbsearch, /pbq）
```

#### 命令别名

为了更快速地访问，支持以下短别名：

| 完整命令 | 别名 | 说明 |
|---------|------|------|
| `/playbook` | `/pb` | 查看状态 |
| `/playbook-show` | `/pbs` | 显示条目 |
| `/playbook-clear` | `/pbc` | 清空数据 |
| `/playbook-search` | `/pbsearch`, `/pbq` | 搜索内容 |

### 使用示例

```bash
# CLI 命令
codex ace show --limit 5
codex ace search "rust async"
codex ace status

# TUI 斜杠命令（在 Codex 对话中）
/pb              # 快速查看 playbook 状态
/pbs             # 显示最近的学习条目
/pbq error       # 搜索包含 "error" 的条目
```

---

## 📁 项目结构

```
codeACE/
├── codex-rs/                    # Rust 实现（主要代码）
│   ├── core/
│   │   └── src/ace/            # ACE 核心模块 ⭐
│   │       ├── mod.rs          # 主插件
│   │       ├── config_loader.rs # 配置加载
│   │       ├── storage.rs      # 存储系统
│   │       ├── reflector.rs    # 知识提取
│   │       ├── curator.rs      # Bullet 生成
│   │       ├── cli.rs          # CLI 命令
│   │       └── types.rs        # 数据类型
│   ├── cli/                    # CLI 入口
│   └── tui/                    # TUI 界面
├── docs/
│   └── ACE_Configuration_Guide.md # 配置详细指南
└── README.md                   # 本文件

⭐ = ACE 核心文件
```

---

## 🧠 核心组件

ACE 采用模块化的代理架构（Agentic Architecture），将任务分解为三个专门角色：

### 1. Generator（生成器）

生成推理轨迹和执行任务：
- 接收用户查询和相关 Playbook 上下文
- 执行多轮推理和工具调用
- 标记哪些 bullets 有用或误导性
- 为 Reflector 提供反馈

### 2. Reflector（反思器）

**核心创新**：独立的评估和洞察提取模块
- 🔍 分析执行轨迹，识别成功策略和失败模式
- 💡 提取可操作的洞察（insights）
- 🔄 支持迭代精炼（Iterative Refinement）
- ⚖️ 避免简洁偏差，保留详细领域知识

**精华提取策略** ✨ (v1.0 新增)
- 🎯 **压缩精华**：一次对话通常只生成 1 条精炼的 insight (200-800 字符)
- 📝 **只保留最终结果**：多次修改的代码只记录最后成功的版本
- 🧹 **智能过滤**：琐碎操作（ls、cat）不记录，未解决的问题必须记录
- 📊 **减缓上下文膨胀 80%**：从平均 2000 字符/对话降到 400 字符
- 📋 **7 个核心信息**：用户要求、做了什么、为什么、成果、解决的问题、未解决的、后续计划

### 3. Curator（策展器）

将洞察整合为结构化的 delta 更新：
- 📝 生成紧凑的 delta context items（候选 bullets）
- 🔗 使用轻量级非 LLM 逻辑合并到现有 Playbook
- 🆔 管理 bullets 的元数据（ID、计数器等）
- 🚫 去重和冗余控制

### 4. Storage（存储系统）

高效的 JSONL 格式存储：
- ⚡ 追加式写入（< 1ms）
- 📖 快速读取（100 条目 < 10ms）
- 🔍 基于嵌入的语义搜索
- 📦 自动归档（超过限制时）

**存储位置**：`~/.codeACE/ace/playbook.jsonl`

### 5. Hook 机制

最小侵入式集成到 Codex CLI：
- `pre_execute`: 执行前加载相关上下文
- `post_execute`: 执行后异步学习（不阻塞用户）

---

## 🧪 测试和验证

### 运行测试

```bash
# 运行所有 ACE 测试（ACE 默认启用）
cargo test

# 运行特定测试
cargo test ace_e2e
cargo test ace_learning_test

# 运行 core 包的测试
cargo test -p codex-core
```

### 测试覆盖

- ✅ E2E 集成测试: 10/10 通过
- ✅ 运行时集成测试: 1/1 通过
- ✅ 配置系统: 100%
- ✅ Hook 系统: 100%
- ✅ CLI 命令: 100%

---

## 📈 开发状态

### Phase 1: 基础设施 ✅ (已完成)

- ✅ 配置系统（自动创建）
- ✅ Hook 系统（pre/post execute）
- ✅ 存储系统（JSONL + Playbook）
- ✅ CLI 命令（5 个命令）
- ✅ 测试覆盖（11/11 通过）
- ✅ ACE 模块默认编译（简化构建流程）

### Phase 2: 核心学习 🚧 (进行中)

- ⏳ Reflector 实现（模式提取）
- ⏳ Curator 实现（Bullet 生成）
- ⏳ 相关性检索优化

### Phase 3: 高级功能 📋 (计划中)

- 📋 语义向量检索
- 📋 多项目知识隔离
- 📋 知识导出/导入
- 📋 可视化界面

---

## 🤝 参与贡献

欢迎贡献！无论是 bug 报告、功能建议还是代码提交。

### 开发指南

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

---

## 🐛 问题反馈

如遇到问题，请在 [Issues](https://github.com/UU114/codeACE/issues) 页面提交。

---

## 📚 相关资源

### Codex CLI 官方资源

- [OpenAI Codex CLI GitHub](https://github.com/openai/codex)
- [OpenAI API 文档](https://platform.openai.com/docs)
- [OpenAI 官网](https://www.openai.com/)

### ACE 相关

- [ACE 配置指南](docs/ACE_Configuration_Guide.md)
- [ACE 论文](2510.04618v1.pdf) - *Agentic Context Engineering: Evolving Contexts for Self-Improving Language Models*
- 论文作者：Qizheng Zhang et al. (Stanford University, SambaNova Systems, UC Berkeley)
- 论文链接：[arXiv:2510.04618](https://arxiv.org/abs/2510.04618)

---

## 📄 许可证

本项目基于 OpenAI Codex CLI，遵循原项目许可证。

ACE 框架扩展部分为独立开发，采用 MIT License。

---

## 🙏 致谢

- [OpenAI](https://www.openai.com/) - 提供 Codex CLI 基础框架
- [ACE 论文作者](https://arxiv.org/abs/2510.04618) - 提供 Agentic Context Engineering 理论基础
  - Qizheng Zhang, Changran Hu, Shubhangi Upasani, Boyuan Ma, Fenglu Hong, et al.
  - Stanford University, SambaNova Systems, UC Berkeley
- 所有贡献者和使用者

---

## 💬 联系方式

- **项目主页**: https://github.com/UU114/codeACE
- **问题反馈**: https://github.com/UU114/codeACE/issues

---

<p align="center">
  <b>让 AI 从对话中学习，让编程更加智能！</b>
</p>

<p align="center">
  Made with ❤️ by the CodeACE Community
</p>
