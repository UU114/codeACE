# CodeACE - Agentic Context Engineering for Codex

> Adding intelligent learning capabilities to OpenAI Codex CLI, enabling AI to learn from conversations and continuously improve

**[中文文档 / Chinese Documentation](docs/readme-zh.md)**

[![Status](https://img.shields.io/badge/Status-Phase_1_MVP-green.svg)](https://github.com/UU114/codeACE)
[![Tests](https://img.shields.io/badge/Tests-100%25-brightgreen.svg)](https://github.com/UU114/codeACE)
[![Rust](https://img.shields.io/badge/Rust-1.82+-orange.svg)](https://www.rust-lang.org)

---

## ⚠️ Important Notice

**This project is a fork and enhancement of OpenAI Codex CLI**, adding the ACE (Agentic Context Engineering) intelligent learning framework on top of the original foundation.

- ✅ Retains all original Codex CLI functionality
- ✅ Adds intelligent learning and context memory capabilities
- ❌ This documentation **only covers ACE extension features**
- ❌ Does not include basic Codex CLI usage instructions

**Need Codex CLI documentation?** Visit [OpenAI Codex CLI Official Repository](https://github.com/openai/codex)

---

## 🎯 What is ACE?

ACE (Agentic Context Engineering) is an intelligent context engineering framework that enables AI assistants to learn from conversation history, build an evolving knowledge base (Playbook), and provide relevant experience in subsequent conversations.

### Core Principles of ACE

Based on the paper *"Agentic Context Engineering: Evolving Contexts for Self-Improving Language Models"*, ACE achieves intelligent learning through the following mechanisms:

1. **Context Adaptation**: Improves performance by modifying input context rather than model weights
2. **Avoiding Brevity Bias**: Retains detailed domain-specific knowledge instead of compressing into brief summaries
3. **Preventing Context Collapse**: Uses incremental updates rather than complete rewrites to avoid information loss
4. **Playbook Evolution**: Treats context as an evolving knowledge base that continuously accumulates and organizes strategies

### Core Capabilities

- 🧠 **Automatic Learning (Reflector)** - Extracts tool usage, error handling, and development patterns from conversations
- 📚 **Knowledge Accumulation (Playbook)** - Builds an evolving structured knowledge base
- 🎯 **Incremental Updates (Delta Updates)** - Local updates instead of complete rewrites, preventing information loss
- 🔄 **Grow-and-Refine** - Balances knowledge expansion with redundancy control
- 🔍 **Intelligent Retrieval** - Context matching based on keywords and semantics
- ⚡ **High Performance** - Extremely fast learning and retrieval (< 100ms)
- 🔌 **Minimal Intrusion** - Integrated via Hook mechanism without polluting original codebase
- 🚀 **Ready to Use** - Automatically creates configuration, works out of the box

---

## 🚀 Quick Start

### 1️⃣ Clone the Project

```bash
git clone https://github.com/UU114/codeACE.git
cd codeACE
```

### 2️⃣ Build

> **💡 Windows Users Note**
> It's recommended to use **Git Bash** instead of PowerShell for building to avoid path handling and command compatibility issues.

```bash
cd codex-rs

# Build release version
cargo build --release

# Or build debug version for development
cargo build
```

**✨ Starting from v1.0, ACE features are enabled by default during compilation**, no additional feature flags needed!

To disable ACE features, you can use:
```bash
cargo build --release --no-default-features
```

### 3️⃣ Install to System

```bash
# Method 1: Using cargo install (recommended)
cargo install --path cli

# Method 2: Manually copy binary
cp target/release/codex ~/.local/bin/
# Or any other directory in your PATH
```

### 4️⃣ Usage

```bash
# Usage is identical to Codex CLI
codex tui                          # Launch TUI interface
codex exec "your question"         # Command line mode

# ACE works automatically in the background:
# - Before conversation (pre_execute Hook): Load relevant historical context
# - After conversation (post_execute Hook): Asynchronously learn and extract knowledge
```

### 5️⃣ Verify ACE Features

```bash
# Check ACE status
codex ace status

# You should see output similar to:
# 📚 ACE (Agentic Coding Environment) Status
#
# Configuration:
#   Enabled: ✅ Yes
#   Storage: ~/.codeACE/ace
#   Max entries: 500
```

---

## 💡 How Does ACE Work?

### Three Key Innovations

According to the paper, ACE introduces three key innovations to address limitations of existing methods:

#### 1️⃣ Independent Reflector Module
- **Problem**: Previous methods had a single model handling all responsibilities, leading to quality degradation
- **Solution**: Separate evaluation and insight extraction into an independent Reflector role
- **Effect**: Significantly improves context quality and downstream performance (proven in §4.5 ablation study)

#### 2️⃣ Incremental Delta Updates
- **Problem**: Monolithic rewrites are expensive and prone to causing context collapse
- **Solution**: Use local, incremental delta updates that only modify relevant parts
- **Effect**: Reduces adaptation latency and computational cost by 82-92% (§4.6)

#### 3️⃣ Grow-and-Refine Mechanism
- **Problem**: Brevity bias leads to loss of domain-specific knowledge
- **Solution**: Balance stable context expansion with redundancy control
- **Effect**: Maintains detailed, task-specific knowledge, preventing information compression

### Workflow

```
User Query
  ↓
[pre_execute Hook] Load relevant historical context
  ↓
Generator: Generate reasoning trace and execution
  ↓
[post_execute Hook] Asynchronous learning process:
  ├─ Reflector: Analyze trace, extract insights (can iterate multiple times)
  ├─ Curator: Generate delta context items
  └─ Storage: Incrementally merge into Playbook
  ↓
Complete (transparent to user)
```

### Usage Example

```bash
# First query
$ codex "How do I run tests?"
> You can run tests using: cargo test

# ACE automatically learns:
✓ Extracted: Tool usage "cargo test"
✓ Tags: testing, tools
✓ Saved to playbook

# Second similar query
$ codex "Run unit tests"
> Based on previous experience, use: cargo test
> (Context automatically loaded ✨)
```

---

## 🔧 Configuration

### Configuration File Location

ACE uses **a separate configuration file** (isolated from Codex CLI main configuration):

```
~/.codeACE/codeACE-config.toml
```

### Automatic Creation

On first run, ACE automatically creates the configuration file, **no manual configuration needed**.

### Custom Configuration (Optional)

```toml
[ace]
enabled = true                    # Enable/disable ACE
storage_path = "~/.codeACE/ace"  # Knowledge base storage path
max_entries = 500                 # Maximum number of entries

[ace.reflector]
extract_patterns = true           # Extract code patterns
extract_tools = true              # Extract tool usage
extract_errors = true             # Extract error information

[ace.context]
max_recent_entries = 10           # Maximum context entries per load
include_all_successes = true      # Include all successful cases
max_context_chars = 4000          # Maximum context characters
```

### Disabling ACE

Method 1: Temporarily disable via config (keeps ACE code)
```toml
[ace]
enabled = false
```

Method 2: Completely remove ACE features at compile time (reduce binary size)
```bash
cd codex-rs
cargo build --release --no-default-features
```

---

## 📊 ACE Playbook Management

### What is Playbook?

Playbook is the core knowledge base of the ACE system, used to store actionable knowledge extracted from conversations. Unlike traditional conversation history, Playbook is a **structured, deduplicated, evolving** long-term memory system.

#### Playbook vs Conversation History

| Feature | Playbook (Long-term Memory) | History Message (Short-term Memory) |
|---------|----------------------------|-------------------------------------|
| **Purpose** | Store reusable knowledge and patterns | Maintain current conversation context continuity |
| **Lifecycle** | Persists across sessions | Limited to current session |
| **Content** | Refined insights, patterns, best practices | Complete user-AI conversation sequences |
| **Information Density** | High (compressed essence) | Low (includes all details) |
| **Storage Efficiency** | Saves **76%** space vs raw conversations | Full conversation storage |
| **Retrieval Method** | Semantic + keyword intelligent matching | Sequential loading |
| **Update Mechanism** | Incremental Delta updates | Append new messages |

**Key Conclusion**: Both **work together**, cannot replace each other
- **History Message** provides fluency and context of current conversation
- **Playbook** provides accumulated knowledge and experience from the past

### Playbook Data Structure

Each Playbook entry includes:

```rust
PlaybookEntry {
    id: String,              // Unique identifier (UUID v4)
    timestamp: DateTime,     // Creation time
    context: String,         // Execution context (user question, task description)
    insights: Vec<String>,   // List of extracted insights
    tags: Vec<String>,       // Classification tags (tools, testing, error_handling, etc.)
    metadata: {
        session_id: String,  // Session identifier
        success: bool,       // Whether execution was successful
        relevance_score: f32 // Relevance score (for retrieval)
    }
}
```

**Storage Format**: JSONL (JSON Lines)
- One complete JSON object per line
- Append-only writes, excellent performance (< 1ms)
- Easy for streaming and incremental parsing

### Playbook Management Mechanisms

#### 1️⃣ **Incremental Delta Updates**
- Only update relevant parts, no complete Playbook rewrite
- Reduces update cost by 82-92% (compared to complete rewrites)
- Prevents "Context Collapse"

#### 2️⃣ **Deduplication and Merging**
- Automatically detects similar entries (based on semantics and keywords)
- Merges redundant information, keeps knowledge base compact
- Retains detailed domain-specific knowledge (avoids brevity bias)

#### 3️⃣ **Intelligent Retrieval**
- **Keyword Matching**: Fast filtering based on tags and context
- **Semantic Search** (planned): Relevance ranking based on embeddings
- **Hybrid Strategy**: Combines temporal recency and relevance scoring

#### 4️⃣ **Automatic Archiving**
- Triggered when entry count exceeds configured limit (default 500)
- Old data automatically moved to `archive/` directory
- Archive files named by timestamp for easy tracing

### CLI Commands

ACE provides a suite of management tools to view and manage learning content:

```bash
codex ace status   # View learning status and statistics
codex ace show     # Display learning content (default 10 items)
codex ace search   # Search knowledge base
codex ace config   # View configuration
codex ace clear    # Clear knowledge base (auto-archive)
```

### TUI Slash Commands 🆕

In Codex TUI interactive interface, you can use the following slash commands to quickly access playbook:

```bash
/playbook         # Display playbook status (alias: /pb)
/playbook-show    # Show recent learning entries (alias: /pbs)
/playbook-clear   # Clear playbook (alias: /pbc)
/playbook-search  # Search playbook (alias: /pbsearch, /pbq)
```

#### Command Aliases

For faster access, the following short aliases are supported:

| Full Command | Alias | Description |
|--------------|-------|-------------|
| `/playbook` | `/pb` | View status |
| `/playbook-show` | `/pbs` | Display entries |
| `/playbook-clear` | `/pbc` | Clear data |
| `/playbook-search` | `/pbsearch`, `/pbq` | Search content |

### Usage Examples

```bash
# CLI commands
codex ace show --limit 5
codex ace search "rust async"
codex ace status

# TUI slash commands (in Codex conversation)
/pb              # Quick playbook status view
/pbs             # Show recent learning entries
/pbq error       # Search entries containing "error"
```

---

## 🚀 LAPS (Lightweight Adaptive Playbook System)

### What is LAPS?

LAPS (Lightweight Adaptive Playbook System) is CodeACE's innovative implementation approach for Playbook management, optimized and simplified for **real-world engineering applications** while maintaining the core principles of the ACE paper.

### Core Design Principles

#### 1️⃣ **Lightweight**

**Problem**: Traditional knowledge base management systems typically require complex databases, indexing systems, and query engines.

**LAPS Solution**:
- ✅ **Zero Database Dependencies**: Uses JSONL plain text format
- ✅ **Minimal Storage**: Single file `playbook.jsonl`, human-readable
- ✅ **Fast Startup**: No database initialization needed, auto-creates on first run
- ✅ **Easy Backup**: Simple file copy for backup
- ✅ **Portability**: Seamless migration across platforms and systems

**Performance Metrics**:
```
Write Latency: < 1ms   (append-only writes)
Read Performance: < 10ms  (100 entries full load)
Retrieval Speed: < 50ms  (keyword filter + relevance sort)
Storage Overhead: ~500KB  (500 typical entries)
```

#### 2️⃣ **Adaptive**

**Problem**: Fixed knowledge extraction strategies cannot adapt to different scenario requirements.

**LAPS Solution**:

##### 🎯 Intelligent Essence Extraction

Traditional method problems:
- ❌ Record all details → rapid context bloat
- ❌ Over-compression → loss of critical information (brevity bias)
- ❌ Undifferentiated recording → noise drowns valuable information

LAPS adaptive strategy:

**A. Compressed Essence Principle**
```
One conversation → typically 1 refined insight (200-800 characters)
Complex tasks → can generate 2-3 insights (different aspects)
Simple operations → may generate none (trivial operations filtered)
```

**B. 7 Core Information Dimensions**
Each insight should include:
1. **User Requirements** - Clear task objectives
2. **What Was Done** - Specific operations executed
3. **Why** - Rationale for choosing this approach
4. **Outcomes** - Final achieved results
5. **Problems Solved** - Obstacles encountered and resolved
6. **Unresolved Issues** - Remaining problems or limitations
7. **Future Plans** - Suggested improvement directions

**C. Intelligent Filtering Rules**
```rust
// Content NOT recorded
- Trivial operations: ls, cat, pwd and other read-only commands
- Temporary attempts: unsuccessful intermediate steps
- Repeated operations: already recorded patterns

// Content MUST be recorded
- Successful solutions and final code
- Error handling and debugging experience
- Tool usage best practices
- Unresolved issues and failed attempts (with reasons)
```

**Effect**: Context bloat rate reduced by **80%** (from 2000 chars/conversation → 400 chars/conversation)

##### 🔄 Dynamic Weight Adjustment

Automatically adjust entry weights based on usage feedback:
```
Successfully applied → relevance_score += 0.1
Marked misleading → relevance_score -= 0.2
Long-term unused → relevance_score *= 0.9 (decay)
```

##### 📊 Adaptive Context Window

Dynamically adjust loaded context amount based on query complexity:
```
Simple query → Load Top 5 relevant entries
Medium query → Load Top 10 relevant entries
Complex task → Load Top 20 + all successful cases
```

#### 3️⃣ **Playbook-Centric**

**Core Innovation**: Organize knowledge as "executable playbooks" rather than passive documents

| Traditional Knowledge Base | LAPS Playbook |
|---------------------------|---------------|
| Static document collection | Dynamically evolving action guide |
| "Know what" (What) | "How to do" (How) + "Why do" (Why) |
| Requires manual interpretation | AI can directly apply |
| Fragmented information | Structured + contextual association |
| Passive query | Proactive recommendation |

**Playbook Entry Example**:
```json
{
  "id": "pb-2024-001",
  "timestamp": "2024-11-19T10:30:00Z",
  "context": "User requests to optimize Rust project compilation performance",
  "insights": [
    "Using cargo build --timings visualizes compilation bottlenecks, found codex-core compilation takes 45% of total time",
    "By adding incremental = true and parallel = true to Cargo.toml, compilation time reduced by 30%",
    "Key optimization: Split large mod.rs into multiple small files to improve incremental compilation efficiency"
  ],
  "tags": ["rust", "performance", "compilation", "cargo"],
  "metadata": {
    "session_id": "session-123",
    "success": true,
    "relevance_score": 0.95
  }
}
```

### LAPS vs Traditional Methods Comparison

#### Comparison with Full Conversation History

| Metric | Full Conversation History | LAPS Playbook | Advantage |
|--------|--------------------------|---------------|-----------|
| **Space Efficiency** | Baseline (100%) | **24%** | **Saves 76%** |
| **Information Density** | Baseline (1x) | **4.18x** | **318% increase** |
| **Retrieval Speed** | Traverse all messages | Keyword+relevance | **10-50x faster** |
| **Cross-session** | ❌ Not supported | ✅ Supported | Long-term memory |
| **Deduplication** | ❌ None | ✅ Automatic | Avoid redundancy |

#### Comparison with Vector Database Solutions

| Feature | Vector DB (Pinecone/Weaviate) | LAPS | LAPS Advantage |
|---------|------------------------------|------|----------------|
| **Dependencies** | Requires external service/process | Zero dependencies | ✅ Simple |
| **Startup Time** | Seconds to minutes | < 10ms | ✅ Fast |
| **Storage Cost** | Cloud service fees or local resources | Local files | ✅ Free |
| **Readability** | Binary/proprietary format | Plain text JSON | ✅ Transparent |
| **Semantic Search** | ✅ Native support | 📋 Planned | ⚠️ Future addition |
| **Exact Match** | ⚠️ May be inaccurate | ✅ Keyword precise | ✅ Reliable |

### Key Advantages of LAPS

#### ✅ **Engineering Practicality**
- Zero-configuration startup: Auto-creates required files on first run
- No external dependencies: No database, vector engine, etc. needed
- Low resource usage: Memory < 10MB, Storage < 1MB
- Cross-platform compatible: Windows/macOS/Linux identical

#### ✅ **High Performance**
- Non-blocking writes: Asynchronous append-only writes
- Efficient reads: Incremental JSONL parsing
- Fast retrieval: Two-tier indexing (tags + relevance)
- Scalable: Supports 10,000+ entries (tested)

#### ✅ **Intelligence**
- Auto-deduplication: Prevents knowledge base bloat
- Relevance learning: Adjusts based on usage feedback
- Context adaptation: Dynamically adjusts load amount
- Essence extraction: 80% compression while retaining key information

#### ✅ **Maintainability**
- Human-readable: Standard JSON format
- Easy debugging: Directly view/edit JSONL files
- Version control: Can be managed with Git
- Auto-archiving: Prevents infinite growth

### LAPS Technology Stack

```
Storage Layer:    JSONL (plain text)
Index Layer:      HashMap (tags) + BTreeMap (time)
Retrieval Layer:  Keyword matching + TF-IDF relevance
Learning Layer:   Incremental Delta updates + weight adjustment
Interface Layer:  CLI commands + TUI slash commands + Hook integration
```

### Future Roadmap

LAPS evolution roadmap:

**Phase 1** ✅ (Completed)
- Basic JSONL storage
- Keyword retrieval
- CLI/TUI management commands

**Phase 2** 🚧 (In Progress)
- Complete incremental Delta update implementation
- Reflector insight extraction optimization
- Relevance scoring algorithm improvements

**Phase 3** 📋 (Planned)
- Hybrid retrieval: Keyword + semantic vectors
- Multi-project knowledge isolation
- Knowledge graph associations
- Visual management interface

---

## 📁 Project Structure

```
codeACE/
├── codex-rs/                    # Rust implementation (main code)
│   ├── core/
│   │   └── src/ace/            # ACE core modules ⭐
│   │       ├── mod.rs          # Main plugin
│   │       ├── config_loader.rs # Configuration loading
│   │       ├── storage.rs      # Storage system
│   │       ├── reflector.rs    # Knowledge extraction
│   │       ├── curator.rs      # Bullet generation
│   │       ├── cli.rs          # CLI commands
│   │       └── types.rs        # Data types
│   ├── cli/                    # CLI entry point
│   └── tui/                    # TUI interface
├── docs/
│   ├── readme-zh.md            # Chinese documentation
│   └── ACE_Configuration_Guide.md # Detailed configuration guide
└── README.md                   # This file

⭐ = ACE core files
```

---

## 🧠 Core Components

ACE adopts a modular agentic architecture, decomposing tasks into three specialized roles:

### 1. Generator

Generates reasoning traces and executes tasks:
- Receives user queries and relevant Playbook context
- Executes multi-turn reasoning and tool calls
- Marks which bullets are useful or misleading
- Provides feedback to Reflector

### 2. Reflector

**Core Innovation**: Independent evaluation and insight extraction module
- 🔍 Analyzes execution traces, identifies successful strategies and failure patterns
- 💡 Extracts actionable insights
- 🔄 Supports Iterative Refinement
- ⚖️ Avoids brevity bias, retains detailed domain knowledge

**Essence Extraction Strategy** ✨ (v1.0 new)
- 🎯 **Compressed Essence**: One conversation typically generates 1 refined insight (200-800 characters)
- 📝 **Final Results Only**: For code modified multiple times, only record the final successful version
- 🧹 **Intelligent Filtering**: Trivial operations (ls, cat) not recorded, unresolved issues must be recorded
- 📊 **80% Context Bloat Reduction**: From average 2000 chars/conversation down to 400 chars
- 📋 **7 Core Information Points**: User requirements, what was done, why, outcomes, problems solved, unresolved issues, future plans

### 3. Curator

Integrates insights into structured delta updates:
- 📝 Generates compact delta context items (candidate bullets)
- 🔗 Uses lightweight non-LLM logic to merge into existing Playbook
- 🆔 Manages bullet metadata (ID, counters, etc.)
- 🚫 Deduplication and redundancy control

### 4. Storage

Efficient JSONL format storage:
- ⚡ Append-only writes (< 1ms)
- 📖 Fast reads (100 entries < 10ms)
- 🔍 Embedding-based semantic search
- 📦 Auto-archiving (when limit exceeded)

**Storage Location**: `~/.codeACE/ace/playbook.jsonl`

### 5. Hook Mechanism

Minimally intrusive integration into Codex CLI:
- `pre_execute`: Load relevant context before execution
- `post_execute`: Asynchronously learn after execution (non-blocking to user)

---

## 🧪 Testing and Verification

### Running Tests

```bash
# Run all ACE tests (ACE enabled by default)
cargo test

# Run specific tests
cargo test ace_e2e
cargo test ace_learning_test

# Run core package tests
cargo test -p codex-core
```

### Test Coverage

- ✅ E2E integration tests: 10/10 passed
- ✅ Runtime integration tests: 1/1 passed
- ✅ Configuration system: 100%
- ✅ Hook system: 100%
- ✅ CLI commands: 100%
- ✅ Playbook context tests: 5/5 passed 🆕

### 📋 Playbook vs History Message Tests 🆕

**Test Date**: 2025-11-19

**Core Question**: Can Playbook replace History Message?

**Test Results**: ✅ All tests passed (5/5)

```bash
# Run Playbook context tests
cd codex-rs
cargo test --test playbook_context_test --features ace -- --nocapture
```

**Key Findings**:

| Metric | Result |
|--------|--------|
| Information Density | Playbook **4.18x** higher than full conversation |
| Space Savings | **76.1%** |
| Retrieval Accuracy | ✅ Successfully retrieves relevant domain knowledge |
| Long-term Memory | ✅ Achieves cross-session knowledge reuse |

**Core Conclusion**: ❌ **Playbook cannot and should not completely replace History Message**

- **History Message**: Provides current conversation context and continuity (short-term memory)
- **Playbook**: Provides past learned knowledge and best practices (long-term memory)
- **Correct Approach**: Both work together, complementing each other

Detailed test report: [codex-rs/test20251119/测试结果.md](codex-rs/test20251119/测试结果.md)

---

## 📈 Development Status

### Phase 1: Infrastructure ✅ (Completed)

- ✅ Configuration system (auto-creation)
- ✅ Hook system (pre/post execute)
- ✅ Storage system (JSONL + Playbook)
- ✅ CLI commands (5 commands)
- ✅ Test coverage (11/11 passed)
- ✅ ACE module compiled by default (simplified build process)

### Phase 2: Core Learning 🚧 (In Progress)

- ⏳ Reflector implementation (pattern extraction)
- ⏳ Curator implementation (Bullet generation)
- ⏳ Relevance retrieval optimization

### Phase 3: Advanced Features 📋 (Planned)

- 📋 Semantic vector retrieval
- 📋 Multi-project knowledge isolation
- 📋 Knowledge export/import
- 📋 Visualization interface

---

## 🤝 Contributing

Contributions welcome! Whether bug reports, feature suggestions, or code submissions.

### Development Guide

1. Fork this repository
2. Create feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to branch (`git push origin feature/AmazingFeature`)
5. Open Pull Request

---

## 🐛 Bug Reports

If you encounter issues, please submit on [Issues](https://github.com/UU114/codeACE/issues) page.

---

## 📚 Related Resources

### Codex CLI Official Resources

- [OpenAI Codex CLI GitHub](https://github.com/openai/codex)
- [OpenAI API Documentation](https://platform.openai.com/docs)
- [OpenAI Official Website](https://www.openai.com/)

### ACE Related

- [ACE Configuration Guide](docs/ACE_Configuration_Guide.md)
- [ACE Paper](2510.04618v1.pdf) - *Agentic Context Engineering: Evolving Contexts for Self-Improving Language Models*
- Paper Authors: Qizheng Zhang et al. (Stanford University, SambaNova Systems, UC Berkeley)
- Paper Link: [arXiv:2510.04618](https://arxiv.org/abs/2510.04618)

---

## 📄 License

This project is based on OpenAI Codex CLI and follows the original project's license.

The ACE framework extension is independently developed and uses MIT License.

---

## 🙏 Acknowledgments

- [OpenAI](https://www.openai.com/) - Providing Codex CLI foundation
- [ACE Paper Authors](https://arxiv.org/abs/2510.04618) - Providing Agentic Context Engineering theoretical foundation
  - Qizheng Zhang, Changran Hu, Shubhangi Upasani, Boyuan Ma, Fenglu Hong, et al.
  - Stanford University, SambaNova Systems, UC Berkeley
- All contributors and users

---

## 💬 Contact

- **Project Homepage**: https://github.com/UU114/codeACE
- **Bug Reports**: https://github.com/UU114/codeACE/issues

---

<p align="center">
  <b>Let AI learn from conversations, make programming more intelligent!</b>
</p>

<p align="center">
  Made with ❤️ by the CodeACE Community
</p>
