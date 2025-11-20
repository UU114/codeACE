# Codex CLI (Rust Implementation)

We provide Codex CLI as a standalone, native executable to ensure a zero-dependency install.

## Installing Codex

Today, the easiest way to install Codex is via `npm`:

```shell
npm i -g @openai/codex
codex
```

You can also install via Homebrew (`brew install --cask codex`) or download a platform-specific release directly from our [GitHub Releases](https://github.com/openai/codex/releases).

## Documentation quickstart

- First run with Codex? Follow the walkthrough in [`docs/getting-started.md`](../docs/getting-started.md) for prompts, keyboard shortcuts, and session management.
- Already shipping with Codex and want deeper control? Jump to [`docs/advanced.md`](../docs/advanced.md) and the configuration reference at [`docs/config.md`](../docs/config.md).

## What's new in the Rust CLI

The Rust implementation is now the maintained Codex CLI and serves as the default experience. It includes a number of features that the legacy TypeScript CLI never supported.

### Config

Codex supports a rich set of configuration options. Note that the Rust CLI uses `config.toml` instead of `config.json` (stored in `~/.codeACE/`). See [`docs/config.md`](../docs/config.md) for details.

### ACE (Agentic Context Engineering) - Automatic Knowledge Capture

The Rust CLI includes an experimental ACE system with **LAPS (Lightweight Adaptive Playbook System)** that automatically captures and organizes knowledge from your development sessions into a personal "Playbook". **ACE is now compiled by default** - no need to manually specify `--features ace` when building.

#### Core Features

- **Mission → TodoList → Tasks workflow**: Tracks high-level missions and their associated todos
- **Automatic learning**: When you complete a todo, the system analyzes the conversation and extracts valuable insights:
  - Code snippets and templates
  - Technical decisions (why you chose a particular approach)
  - Workflow patterns (testing, building, deployment)
  - Error resolutions and troubleshooting tips
- **Smart filtering**: Only captures meaningful operations, filtering out read-only commands like `ls`, `cat`, `grep`
- **Code grading**: Small code blocks (<200 lines) are saved in full; larger blocks are intelligently summarized
- **Structured storage**: Knowledge is organized into categories (Strategies, Tools, Troubleshooting, etc.) and saved to `~/.codeACE/ace/playbook.json`

#### LAPS System Enhancements (New!)

LAPS extends ACE with intelligent, adaptive optimizations that run transparently in the background:

- **智能内容管理** (Intelligent Content Management)
  - 自动分类 6 种内容类型（代码片段、错误解决、策略规则、工具使用、API 指南、项目特定）
  - 自适应长度验证：根据内容类型动态调整长度要求（代码片段 100-3000 字符，策略规则 30-400 字符等）
  - 内容质量评分系统，自动过滤低质量信息

- **动态优先级系统** (Dynamic Priority System)
  - 基于实际使用情况的动态权重：`weight = importance × ln(1 + recall_count) × success_rate × recency`
  - 召回频率追踪：记录每个 bullet 被使用的次数和成功率
  - 时效性考量：最近使用的知识获得更高权重

- **高效检索** (High-Performance Retrieval)
  - 纯内存轻量级索引（HashMap + BTreeMap），毫秒级响应
  - 关键词倒排索引，支持快速全文搜索
  - LRU 热度缓存（100 项），优化高频访问

- **跨领域知识图谱** (Cross-Domain Knowledge Graph)
  - 支持 10 种领域分类（Web 开发、系统编程、数据科学、DevOps 等）
  - 识别 9 种编程语言（Rust、Python、JavaScript、Go 等）
  - 智能上下文匹配：根据当前项目自动推荐相关知识

- **无感知后台优化** (Background Optimization)
  - 智能去重：使用高级相似度算法（Levenshtein + N-gram）检测重复内容，相似度 ≥ 85% 自动合并
  - 低价值清理：自动删除 30 天未使用、失败率高或内容过短的 bullets
  - 智能保护：保留最近使用和高价值的知识
  - 定期权重重算：确保权重反映最新使用情况

**Technical Highlights:**
- ✅ 零数据库依赖：纯内存索引 + JSON 文件存储
- ✅ 轻量级：除 lru 缓存（50KB），无其他外部依赖
- ✅ 高性能：< 10ms 查询响应，< 50MB 内存占用
- ✅ 100% 测试覆盖：110+ 单元测试和集成测试全部通过

**Build & Run:**
```bash
# ACE with LAPS is included by default - just build and run
cargo build
cargo run --bin codex
```

Enable ACE at runtime in your `config.toml` to start building your personal development knowledge base. See implementation details in `ref/FINAL-IMPLEMENTATION-REPORT.md` and LAPS technical documentation in `ref/v2/`.

### Model Context Protocol Support

#### MCP client

Codex CLI functions as an MCP client that allows the Codex CLI and IDE extension to connect to MCP servers on startup. See the [`configuration documentation`](../docs/config.md#mcp_servers) for details.

#### MCP server (experimental)

Codex can be launched as an MCP _server_ by running `codex mcp-server`. This allows _other_ MCP clients to use Codex as a tool for another agent.

Use the [`@modelcontextprotocol/inspector`](https://github.com/modelcontextprotocol/inspector) to try it out:

```shell
npx @modelcontextprotocol/inspector codex mcp-server
```

Use `codex mcp` to add/list/get/remove MCP server launchers defined in `config.toml`, and `codex mcp-server` to run the MCP server directly.

### Notifications

You can enable notifications by configuring a script that is run whenever the agent finishes a turn. The [notify documentation](../docs/config.md#notify) includes a detailed example that explains how to get desktop notifications via [terminal-notifier](https://github.com/julienXX/terminal-notifier) on macOS.

### `codex exec` to run Codex programmatically/non-interactively

To run Codex non-interactively, run `codex exec PROMPT` (you can also pass the prompt via `stdin`) and Codex will work on your task until it decides that it is done and exits. Output is printed to the terminal directly. You can set the `RUST_LOG` environment variable to see more about what's going on.

### Experimenting with the Codex Sandbox

To test to see what happens when a command is run under the sandbox provided by Codex, we provide the following subcommands in Codex CLI:

```
# macOS
codex sandbox macos [--full-auto] [COMMAND]...

# Linux
codex sandbox linux [--full-auto] [COMMAND]...

# Windows
codex sandbox windows [--full-auto] [COMMAND]...

# Legacy aliases
codex debug seatbelt [--full-auto] [COMMAND]...
codex debug landlock [--full-auto] [COMMAND]...
```

### Selecting a sandbox policy via `--sandbox`

The Rust CLI exposes a dedicated `--sandbox` (`-s`) flag that lets you pick the sandbox policy **without** having to reach for the generic `-c/--config` option:

```shell
# Run Codex with the default, read-only sandbox
codex --sandbox read-only

# Allow the agent to write within the current workspace while still blocking network access
codex --sandbox workspace-write

# Danger! Disable sandboxing entirely (only do this if you are already running in a container or other isolated env)
codex --sandbox danger-full-access
```

The same setting can be persisted in `~/.codeACE/config.toml` via the top-level `sandbox_mode = "MODE"` key, e.g. `sandbox_mode = "workspace-write"`.

## Code Organization

This folder is the root of a Cargo workspace. It contains quite a bit of experimental code, but here are the key crates:

- [`core/`](./core) contains the business logic for Codex. Ultimately, we hope this to be a library crate that is generally useful for building other Rust/native applications that use Codex.
- [`exec/`](./exec) "headless" CLI for use in automation.
- [`tui/`](./tui) CLI that launches a fullscreen TUI built with [Ratatui](https://ratatui.rs/).
- [`cli/`](./cli) CLI multitool that provides the aforementioned CLIs via subcommands.
