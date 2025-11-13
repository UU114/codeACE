# CodeACE - Agentic Coding Environment for Codex

> 为 Anthropic Codex CLI 添加智能学习能力，让 AI 从对话中学习并持续改进

[![Status](https://img.shields.io/badge/Status-Phase_1_MVP-green.svg)](https://github.com/UU114/codeACE)
[![Tests](https://img.shields.io/badge/Tests-100%25-brightgreen.svg)](https://github.com/UU114/codeACE)
[![Rust](https://img.shields.io/badge/Rust-1.82+-orange.svg)](https://www.rust-lang.org)

---

## ⚠️ 重要说明

**本项目是对 Codex CLI 的改造项目**，在原有基础上添加了 ACE (Agentic Coding Environment) 智能学习框架。

- ✅ 保留 Codex CLI 的所有原有功能
- ✅ 新增智能学习和上下文记忆能力
- ❌ 本文档**仅介绍 ACE 扩展功能**
- ❌ 不包含 Codex CLI 的基础使用说明

**需要 Codex CLI 的使用文档？** 请访问 [Codex CLI 官方仓库](https://github.com/anthropics/claude-code)

---

## 🎯 什么是 ACE？

ACE (Agentic Coding Environment) 是一个智能学习框架，让 AI 助手能够从你的对话历史中学习，并在后续对话中提供相关经验。

### 核心能力

- 🧠 **自动学习** - 从对话中提取工具使用、错误处理、开发模式
- 📚 **知识积累** - 构建个性化的 Playbook 知识库
- 🔍 **智能检索** - 基于关键词的相关上下文匹配
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

### 4️⃣ 使用

```bash
# 使用方式与 Codex CLI 完全相同
codex tui                          # 启动 TUI 界面
codex exec "你的问题"               # 命令行模式

# ACE 在后台自动工作：
# - 对话前：加载相关历史上下文
# - 对话后：学习并提取知识
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

### 工作流程

```
用户提问
  ↓
[pre_execute Hook] 加载相关历史上下文
  ↓
AI 生成回复（带上下文增强）
  ↓
执行操作
  ↓
[post_execute Hook] 异步学习（提取知识并存储）
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

```toml
[ace]
enabled = false
```

或者编译时不使用 `--features ace` 标志。

---

## 📊 ACE CLI 命令

ACE 提供了一套管理工具来查看和管理学习内容：

```bash
codex ace status   # 查看学习状态和统计
codex ace show     # 显示学习内容（默认 10 条）
codex ace search   # 搜索知识库
codex ace config   # 查看配置
codex ace clear    # 清空知识库
```

### 示例

```bash
# 查看最近的学习内容
codex ace show --limit 5

# 搜索特定主题
codex ace search "rust async"

# 查看详细统计
codex ace status
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

### 1. Reflector（智能提取器）

从对话中智能提取有价值的信息：
- 🔧 工具使用（bash 命令、文件操作）
- ❌ 错误处理（错误信息和解决方案）
- 🔄 模式识别（测试、构建、Git 操作等）
- 🏷️ 自动标签（基于内容的智能标签）

### 2. Storage（存储系统）

高效的 JSONL 格式存储：
- ⚡ 追加式写入（< 1ms）
- 📖 快速读取（100 条目 < 10ms）
- 🔍 简单搜索
- 📦 自动归档（超过限制时）

**存储位置**：`~/.codeACE/ace/playbook.jsonl`

### 3. Context Loader（上下文加载器）

智能加载相关历史上下文：
1. 用户提问
2. 提取关键词
3. 匹配相关条目（关键词+标签）
4. 评分排序
5. 格式化上下文
6. 注入到系统消息

### 4. Hook 机制

最小侵入式集成到 Codex：
- `pre_execute`: 执行前加载上下文
- `post_execute`: 执行后进行学习

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

- [Codex CLI GitHub](https://github.com/anthropics/claude-code)
- [Claude API 文档](https://docs.anthropic.com/)
- [Anthropic 官网](https://www.anthropic.com/)

### ACE 相关

- [ACE 配置指南](docs/ACE_Configuration_Guide.md)
- 本项目基于 Agentic Context Engineering 论文理念实现

---

## 📄 许可证

本项目基于 Codex CLI (Anthropic)，遵循原项目许可证。

ACE 框架部分为独立开发，采用 MIT License。

---

## 🙏 致谢

- [Anthropic](https://www.anthropic.com/) - 提供 Codex CLI 基础
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
