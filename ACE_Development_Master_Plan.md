# ACE开发总计划

## 项目概述

### 目标
为Codex CLI实现ACE（Agentic Coding Environment）框架，通过智能学习和上下文管理提升编程效率。

### 核心策略
- **两阶段开发**：MVP先行（3-4周） + 优化提升（4-6周）
- **最小侵入**：Hook机制，对原代码修改<20行
- **插件化架构**：ACE作为独立crate，100%代码隔离
- **配置驱动**：通过feature flag和配置文件控制

## 技术方案

### 架构设计
```
用户输入 → Executor(with hooks) → 执行结果
              ↓                        ↓
         [pre_hook]               [post_hook]
              ↓                        ↓
      加载ACE上下文            Reflector分析学习
              ↓                        ↓
        增强的prompt            存储到playbook.jsonl
```

### 集成策略
1. **Hook机制**：在Executor中添加扩展点
2. **Feature Flag**：`#[cfg(feature = "ace")]`控制
3. **独立Crate**：`codex-ace`完全独立开发
4. **异步处理**：学习过程不阻塞主流程

## 第一阶段：MVP实现（3-4周）

### 目标
快速实现可运行的核心功能，验证ACE价值。

### 核心组件
1. **Hook System**：最小化的扩展机制
2. **Simple Storage**：JSONL格式追加存储
3. **Reflector MVP**：基于规则的知识提取
4. **Context Loader**：简单的上下文加载
5. **CLI Commands**：基础管理命令

### 技术选型
- 存储：JSONL文件（无数据库）
- 提取：正则表达式（无LLM）
- 检索：关键词匹配（无向量化）
- 格式：JSON序列化（无二进制）

### 关键简化
- ✅ 不去重（直接追加）
- ✅ 不排序（按时间倒序）
- ✅ 不优化（够用即可）
- ✅ 不依赖（纯Rust实现）

## 第二阶段：优化提升（4-6周）

### 目标
提升效率和智能化程度。

### 优化内容
1. **智能去重**：语义哈希 + TF-IDF
2. **高效检索**：全文索引 + 相关性排序
3. **知识管理**：Curator + 关系发现
4. **性能优化**：缓存 + 增量更新
5. **LLM增强**：可选的智能提取

### 技术升级
- sled数据库
- tantivy全文检索
- 多级缓存
- 向量检索（可选）

## 实施步骤

### Week 1: 基础架构
- [ ] 创建codex-ace crate
- [ ] 实现Hook机制
- [ ] 基础数据结构
- [ ] 存储系统

### Week 2: 核心功能
- [ ] Reflector规则引擎
- [ ] 模式提取
- [ ] 工具使用识别
- [ ] 错误解决提取

### Week 3: 集成测试
- [ ] Executor集成
- [ ] 上下文加载
- [ ] CLI命令
- [ ] 端到端测试

### Week 4: 优化发布
- [ ] 性能测试
- [ ] Bug修复
- [ ] 文档编写
- [ ] MVP发布

## 文件结构

```
codex-rs/
├── codex-core/           # 最小修改
│   ├── src/
│   │   ├── hooks.rs      # [新增] Hook trait定义
│   │   └── executor.rs   # [修改] 添加hook调用点
│   └── Cargo.toml        # [修改] 添加ace feature
│
├── codex-cli/            # 最小修改
│   ├── src/
│   │   └── main.rs       # [修改] 注册ACE插件
│   └── Cargo.toml        # [修改] 添加ace依赖
│
└── codex-ace/            # ACE独立实现
    ├── src/
    │   ├── lib.rs        # ACE主入口和Hook实现
    │   ├── reflector.rs  # 智能提取器
    │   ├── storage.rs    # 存储管理
    │   ├── context.rs    # 上下文管理
    │   ├── types.rs      # 数据结构定义
    │   └── cli.rs        # CLI命令
    ├── Cargo.toml
    └── README.md
```

## 配置设计

```toml
# ~/.codex/config.toml
[ace]
enabled = false                    # 默认关闭
storage_path = "~/.codex/ace"     # 存储路径
max_entries = 500                  # 最大条目数

[ace.reflector]
extract_tools = true               # 提取工具使用
extract_patterns = true            # 提取模式
extract_errors = true              # 提取错误处理

[ace.context]
max_recent = 10                    # 加载最近N条
max_chars = 4000                   # 最大字符数
```

## 开发原则

1. **最小侵入**：对原代码改动最少
2. **渐进增强**：先能用，后好用
3. **配置驱动**：行为可配置
4. **错误安全**：失败不影响主功能
5. **异步非阻塞**：学习过程后台进行
6. **日志完善**：关键步骤记录日志

## 成功标准

### MVP阶段
- [ ] 2周内可运行
- [ ] 能够提取和存储知识
- [ ] 能够加载相关上下文
- [ ] 不影响原有功能

### 优化阶段
- [ ] 检索速度<100ms
- [ ] 去重准确率>90%
- [ ] 相关性召回率>70%
- [ ] 内存占用<100MB

## 风险管理

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 上游大改 | 高 | Hook机制最小化接触面 |
| 性能影响 | 中 | 异步处理+特性开关 |
| 存储增长 | 低 | 自动归档+容量限制 |
| 兼容性 | 低 | 充分测试+版本控制 |

## 项目管理

### 沟通计划
- 每周更新进度
- 问题及时反馈
- 文档同步更新

### 质量保证
- 单元测试覆盖率>80%
- 集成测试必须通过
- 代码审查和重构

### 发布策略
- MVP内部测试
- Beta版本收集反馈
- 正式版本公开发布

## 长期规划

### 3个月目标
- 完整的ACE系统
- 1000+用户使用
- 社区贡献者参与

### 6个月目标
- 高级AI功能
- 插件生态系统
- 企业级支持

### 1年目标
- 行业标准方案
- 多语言支持
- 云端同步

---

# 实施进度（2025-11-10更新）

## Phase 1: MVP实现状态 ✅ 90%完成

### 已完成 ✅
1. **基础Hook机制** - 在codex-core中实现了ExecutorHook trait和HookManager
2. **ACE独立Crate** - 创建了完全独立的codex-ace库
3. **存储系统** - 实现了JSONL格式的SimpleStorage
4. **Reflector** - 基于规则的模式提取（无需LLM）
5. **Context Loader** - 关键词匹配的上下文加载
6. **编译集成** - 成功集成到codex-core，通过ace feature控制
7. **文档** - 创建了配置示例和集成说明

### 进行中 🚧
- Hook调用点的完善（需要在合适位置调用pre_execute和post_execute）

### 待完成 📋
- CLI命令支持（ace status, ace clear等）
- 端到端功能测试
- 从配置文件加载ACE设置

## 关键成果
- **最小侵入**：对codex-core的修改少于20行
- **完全隔离**：ACE功能全部在独立crate中
- **易于维护**：可以独立升级ACE而不影响主程序
- **性能友好**：异步学习，不阻塞主流程

## 下一步行动
1. 完善Hook调用点
2. 实现配置加载
3. 添加CLI命令
4. 进行端到端测试