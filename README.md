# CodeACE - Agentic Coding Environment for Claude Code

> 为 Claude Code 添加智能学习能力，让 AI 从对话中学习并持续改进

[![Status](https://img.shields.io/badge/Status-Phase_1_MVP-green.svg)](https://github.com/UU114/codeACE)
[![Tests](https://img.shields.io/badge/Tests-100%25-brightgreen.svg)](https://github.com/UU114/codeACE)
[![Rust](https://img.shields.io/badge/Rust-1.82+-orange.svg)](https://www.rust-lang.org)

---

## 📖 关于本项目

**CodeACE** 是基于 [Anthropic Claude Code](https://github.com/anthropics/claude-code) 的智能学习扩展项目。

### ⚠️ 重要说明

- 本项目**不是** Claude Code 的替代品，而是在其基础上添加了 ACE (Agentic Coding Environment) 学习框架
- 本文档**仅介绍 ACE 功能**，不包含 Claude Code 的基础使用说明
- 如需 Claude Code 的使用文档，请访问：[Claude Code 官方文档](https://docs.claude.com/en/docs/claude-code)

---

## 🎯 什么是 ACE？

ACE (Agentic Coding Environment) 是一个智能学习框架，它通过分析你与 Claude 的对话历史，自动提取知识并在后续对话中提供相关上下文。

### 核心理念

传统的 AI 助手每次对话都是"健忘"的，而 ACE 让 AI 能够：
- 🧠 **记住**你的编程习惯和常用模式
- 📚 **积累**项目相关的知识和经验
- 🔍 **检索**历史中相关的解决方案
- 🚀 **改进**每次对话的质量和效率

---

## ✨ 与原版 Claude Code 的区别

| 特性 | 原版 Claude Code | CodeACE (启用 ACE) |
|------|-----------------|-------------------|
| 基础对话 | ✅ | ✅ |
| 代码编辑 | ✅ | ✅ |
| 工具调用 | ✅ | ✅ |
| **智能学习** | ❌ | ✅ 自动从对话中学习 |
| **上下文注入** | ❌ | ✅ 加载历史相关经验 |
| **知识积累** | ❌ | ✅ 构建个性化 Playbook |
| **模式识别** | ❌ | ✅ 识别工具使用模式 |
| **错误记忆** | ❌ | ✅ 记住错误和解决方案 |

### ACE 新增功能

1. **自动学习** (`post_execute` Hook)
   - 对话结束后自动提取关键信息
   - 识别成功/失败的模式
   - 记录工具使用情况

2. **智能上下文** (`pre_execute` Hook)
   - 对话开始前检索相关历史
   - 注入相关的知识和经验
   - 避免重复犯错

3. **CLI 管理工具**
   ```bash
   codex ace status   # 查看学习状态
   codex ace show     # 显示学习内容
   codex ace search   # 搜索知识库
   codex ace config   # 查看配置
   codex ace clear    # 清空知识库
   ```

---

## 🚀 安装和使用

### 前置要求

- **Rust**: 1.82+ (安装: https://rustup.rs)
- **操作系统**: Linux, macOS, Windows (WSL2)
- **Claude Code 账户**: 需要有 Claude API 访问权限

### 1️⃣ 克隆项目

```bash
git clone https://github.com/UU114/codeACE.git
cd codeACE
```

### 2️⃣ 编译（启用 ACE 功能）

```bash
cd codex-rs

# 编译 release 版本（启用 ACE）
cargo build --release --features ace

# 或者编译 debug 版本用于开发
cargo build --features ace
```

**重要**：必须使用 `--features ace` 标志来启用 ACE 功能！

### 3️⃣ 安装到系统

```bash
# 方式1: 使用 cargo install
cargo install --path cli --features ace

# 方式2: 手动复制二进制文件
cp target/release/codex ~/.local/bin/
# 或其他在你的 PATH 中的目录
```

### 4️⃣ 配置（可选）

首次运行时，ACE 会自动创建配置文件：

```
~/.codeACE/codeACE-config.toml
```

**默认配置**已经可以直接使用，无需修改。如需自定义：

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

### 5️⃣ 验证安装

```bash
# 检查 ACE 状态
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

## 💡 使用示例

### 基础使用

使用方式与原版 Claude Code **完全相同**：

```bash
# 启动 TUI 界面
codex tui

# 或使用命令行模式
codex exec "帮我创建一个 Rust 项目"
```

**区别**：ACE 会在后台**自动**：
1. 在对话前加载相关历史上下文
2. 在对话后学习和提取知识

### 查看学习内容

```bash
# 查看最近的学习内容
codex ace show --limit 5

# 搜索特定主题
codex ace search "rust async"

# 查看详细统计
codex ace status
```

### 管理知识库

```bash
# 清空知识库（保留归档）
codex ace clear

# 清空知识库（不归档）
codex ace clear --no-archive

# 查看配置
codex ace config
```

---

## 🔧 配置说明

### 配置文件位置

ACE 使用**独立的配置文件**（与 Claude Code 主配置分离）：

```
~/.codeACE/codeACE-config.toml
```

### 主要配置项

#### 核心设置 `[ace]`

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `enabled` | `true` | 启用/禁用 ACE 功能 |
| `storage_path` | `"~/.codeACE/ace"` | 知识库存储路径 |
| `max_entries` | `500` | 自动归档阈值 |

#### 反思器设置 `[ace.reflector]`

控制从对话中提取哪些类型的知识：

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `extract_patterns` | `true` | 提取代码模式 |
| `extract_tools` | `true` | 提取工具使用 |
| `extract_errors` | `true` | 提取错误处理 |

#### 上下文设置 `[ace.context]`

控制如何加载和使用历史上下文：

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `max_recent_entries` | `10` | 每次最多加载的条目数 |
| `include_all_successes` | `true` | 包含所有成功案例 |
| `max_context_chars` | `4000` | 上下文最大字符数 |

### 禁用 ACE

如果需要临时禁用 ACE 功能：

```toml
[ace]
enabled = false
```

或者编译时不使用 `--features ace` 标志。

---

## 📊 工作原理

### ACE 架构

```
┌─────────────────────────────────────────────────────────────┐
│                      Claude Code 核心                        │
└─────────────────────────────────────────────────────────────┘
                           │
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                      Hook 系统                               │
│  ┌──────────────────┐           ┌──────────────────┐        │
│  │  pre_execute     │           │  post_execute    │        │
│  │  (上下文加载)     │           │  (学习过程)       │        │
│  └──────────────────┘           └──────────────────┘        │
└─────────────────────────────────────────────────────────────┘
           ↓                                 ↓
┌──────────────────────┐        ┌──────────────────────┐
│   Context Loader     │        │     Reflector        │
│   (检索相关知识)       │        │   (提取洞察)          │
└──────────────────────┘        └──────────────────────┘
           ↓                                 ↓
┌──────────────────────┐        ┌──────────────────────┐
│  Playbook Storage    │←───────│      Curator         │
│  (JSONL 知识库)       │        │   (生成 Bullets)     │
└──────────────────────┘        └──────────────────────┘
```

### 工作流程

1. **对话开始时** (pre_execute)
   - 检索与当前查询相关的历史知识
   - 格式化为上下文并注入到 prompt
   - Claude 可以参考历史经验

2. **对话结束后** (post_execute)
   - Reflector 分析对话内容
   - 提取模式、工具使用、错误处理
   - Curator 生成结构化的 Bullets
   - 存储到 Playbook

3. **持续优化**
   - 知识库随对话增长
   - 相关性检索越来越准确
   - AI 助手越来越"懂你"

---

## 🧪 测试和验证

### 运行测试

```bash
# 运行所有 ACE 测试
cargo test --features ace

# 运行特定测试
cargo test --features ace ace_e2e
cargo test --features ace ace_learning_test
```

### 测试覆盖

- ✅ E2E 集成测试: 10/10 通过
- ✅ 运行时集成测试: 1/1 通过
- ✅ 配置系统测试: 100%
- ✅ Hook 系统测试: 100%
- ✅ CLI 命令测试: 100%

---

## 🗂️ 项目结构

```
codeACE/
├── codex-rs/                    # Rust 实现（主要代码）
│   ├── core/
│   │   └── src/ace/            # ACE 核心模块
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
```

---

## 📈 开发状态

### Phase 1: 基础设施 ✅ (已完成)

- ✅ 配置系统（自动创建）
- ✅ Hook 系统（pre/post execute）
- ✅ 存储系统（JSONL + Playbook）
- ✅ CLI 命令（5 个命令）
- ✅ 测试覆盖（11/11 通过）

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

### 代码规范

- 遵循 Rust 代码规范
- 运行 `cargo fmt` 格式化代码
- 运行 `cargo clippy` 检查警告
- 添加测试覆盖新功能

---

## 🐛 问题反馈

如遇到问题，请：

1. 检查 [Issues](https://github.com/UU114/codeACE/issues) 看是否已有相关问题
2. 如果是新问题，创建新的 issue 并提供：
   - 问题描述
   - 重现步骤
   - 环境信息（OS、Rust 版本等）
   - 相关日志

---

## 📚 相关资源

### Claude Code 官方资源

- [Claude Code 官方文档](https://docs.claude.com/en/docs/claude-code)
- [Claude Code GitHub](https://github.com/anthropics/claude-code)
- [Claude API 文档](https://docs.anthropic.com/)

### ACE 相关

- [ACE 配置指南](docs/ACE_Configuration_Guide.md)
- 本项目基于 Agentic Context Engineering 论文理念实现

---

## 📄 许可证

本项目基于 MIT 许可证开源。详见 [LICENSE](LICENSE) 文件。

**注意**：本项目是 Claude Code 的扩展，Claude Code 的许可证仍然适用于其原始代码部分。

---

## 🙏 致谢

- [Anthropic](https://www.anthropic.com/) - 提供 Claude Code 和 Claude API
- ACE 论文作者 - 提供理论基础
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
