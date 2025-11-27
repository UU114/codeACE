# 🎉 Codex v1.0.0 正式发布

## 版本信息

**版本**: v1.0.0
**发布日期**: 2025-11-26
**类型**: 正式版 (Production Release)

## 📦 下载

| 平台 | 文件 | 大小 | SHA256 |
|------|------|------|--------|
| Linux x86_64 | `codex-linux-x86_64` | 49 MB | `ef6b05464418f83f...` |
| Windows x86_64 | `codex-windows-x86_64.exe` | 39 MB | `9dabd91246fa8e21...` |

**完整校验和**: 见 `SHA256SUMS.txt`

## ✨ 主要特性

### 完整的ACE框架
- **智能学习**: 从你的编程会话中学习
- **上下文注入**: 自动注入相关的历史经验
- **模糊匹配**: 智能检索最相关的知识
- **中文支持**: 完美支持中文分词和匹配

### 零依赖部署
- **Linux**: 完全静态链接,无需任何依赖库
- **Windows**: 原生PE可执行文件,双击即用
- **跨平台**: 相同的配置和数据格式

### 企业级性能
- **LTO优化**: 全链接时优化
- **体积优化**: 剥离调试符号
- **快速响应**: Playbook查询 <25ms

## 🚀 快速开始

### Linux

```bash
# 1. 下载并安装
wget https://github.com/UU114/codeACE/releases/download/v1.0.0/codex-linux-x86_64
chmod +x codex-linux-x86_64
sudo mv codex-linux-x86_64 /usr/local/bin/codex

# 2. 验证安装
codex --version

# 3. 首次运行
codex
```

### Windows

```cmd
REM 1. 下载 codex-windows-x86_64.exe
REM 2. 重命名为 codex.exe (可选)
REM 3. 双击运行或在CMD中执行

codex.exe --version
```

## 📖 使用指南

### 基本命令

```bash
# 交互式模式
codex

# TUI模式
codex tui

# 执行单个命令
codex exec "你的问题"

# ACE管理
codex ace status      # 查看状态
codex ace list        # 查看playbook
codex ace clear       # 清空playbook
```

### 配置

首次运行自动创建配置文件:
- `~/.codeACE/config.toml` - 主配置
- `~/.codeACE/codeACE-config.toml` - ACE配置

示例配置:
```toml
# config.toml
model = "deepseek-chat"
model_provider = "deepseek"

[model_providers.deepseek]
name = "deepseek"
base_url = "https://api.deepseek.com"
env_key = "DEEPSEEK_API_KEY"
```

## 🔥 v1.0.0 新特性

### ACE框架正式版
- ✅ Playbook学习系统
- ✅ 智能上下文检索
- ✅ 模糊匹配算法
- ✅ 性能优化 (查询<25ms)
- ✅ 中文完美支持

### 技术改进
- ✅ 完全静态链接 (Linux)
- ✅ LTO全链接优化
- ✅ 二进制体积优化
- ✅ 跨平台一致性

### 测试验证
- ✅ ACE功能全面测试
- ✅ 4轮递进对话测试通过
- ✅ 模糊匹配准确度验证
- ✅ 性能基准测试通过

## 📊 性能指标

| 指标 | 数值 |
|------|------|
| Playbook查询延迟 | ~23ms |
| 上下文注入大小 | ~4148 chars |
| 模糊匹配召回率 | 30.8% |
| 关键词提取数量 | 7个/查询 |
| ACE初始化时间 | ~2ms |

## 🔒 安全性

- ✅ 静态链接消除依赖风险
- ✅ 剥离调试符号防止逆向
- ✅ SHA256校验和验证完整性
- ✅ 官方发布签名

验证文件:
```bash
sha256sum -c SHA256SUMS.txt
```

## 📚 文档

- [BUILD_INFO.md](BUILD_INFO.md) - 详细构建信息
- [SHA256SUMS.txt](SHA256SUMS.txt) - 文件校验和
- [GitHub仓库](https://github.com/UU114/codeACE) - 源代码和文档

## 🐛 已知问题

无已知重大问题。

## 💬 反馈与支持

- **Issues**: https://github.com/UU114/codeACE/issues
- **Discussions**: https://github.com/UU114/codeACE/discussions

## 📝 更新日志

### v1.0.0 (2025-11-26)

**新特性**:
- ACE框架正式发布
- Playbook学习系统
- 智能上下文注入
- 模糊匹配算法
- 完整中文支持

**改进**:
- 完全静态链接 (Linux)
- LTO全链接优化
- 二进制体积优化
- 性能优化

**测试**:
- ACE功能测试
- 模糊匹配测试
- 性能基准测试
- 中文支持测试

## 🙏 致谢

感谢所有贡献者和测试者。

---

**发布者**: codeACE Team
**发布日期**: 2025-11-26
**版本**: v1.0.0
