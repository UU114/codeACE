# GitHub 推送指南

本指南帮助你将 CodeACE 项目推送到 GitHub，只包含必要的代码和文档。

---

## 📋 已配置的排除规则

已在 `.gitignore` 中添加以下排除规则：

### 排除的文件/目录
```
req/                      # 需求文档目录
test1111/                 # 测试框架目录
DEVELOPMENT_LOG.md        # 开发日志
ACE_TEST_LOG.md          # 测试日志
readme-codex.md          # 原Codex README
ACE_INTEGRATION.md       # 集成文档
ACE_MVP_TEST_PLAN.md     # 测试计划
BUILD-SUCCESS.md         # 构建记录
INSTALL-SUMMARY.md       # 安装记录
ace-config-example.toml  # 配置示例
setup-env.sh             # 环境设置脚本
```

### 保留的文件
```
README.md                 # GitHub主README（新建）
readme-codeACE.md        # 完整文档
codex-rs/                # Rust代码
  └── codex-ace/         # ACE核心代码
  └── core/src/hooks.rs  # Hook机制
.gitignore               # Git配置
Cargo.toml               # 项目配置
其他必要的代码文件
```

---

## 🚀 推送步骤

### 方法1: 首次推送到新仓库

#### 1. 在GitHub上创建新仓库

访问 https://github.com/new 创建新仓库：
- 仓库名: `codeACE` (或你想要的名字)
- 描述: "Agentic Coding Environment - AI Learning Framework"
- 类型: Public 或 Private
- **不要**勾选 "Initialize with README"（我们已有README）

#### 2. 检查当前git状态

```bash
cd /home/com/codeACE

# 查看哪些文件会被提交
git status

# 查看哪些文件被忽略
git status --ignored
```

#### 3. 添加和提交更改

```bash
# 添加所有新文件（会自动排除.gitignore中的文件）
git add .

# 查看将要提交的文件
git status

# 提交（确认没有test1111/、req/等目录）
git commit -m "feat: Add ACE framework MVP

- Implement Hook mechanism for minimal intrusion
- Add Reflector for intelligent extraction
- Add Storage system with JSONL format
- Add Context Loader for smart matching
- Clean code warnings
- Add comprehensive documentation
"
```

#### 4. 关联远程仓库并推送

```bash
# 添加远程仓库（替换YOUR_USERNAME为你的GitHub用户名）
git remote add origin https://github.com/YOUR_USERNAME/codeACE.git

# 推送到main分支
git push -u origin main
```

---

### 方法2: 推送到已存在的仓库

如果你已经有仓库：

```bash
# 查看当前远程仓库
git remote -v

# 如果需要更改远程仓库地址
git remote set-url origin https://github.com/YOUR_USERNAME/codeACE.git

# 拉取远程更新（如果有）
git pull origin main --rebase

# 添加和提交
git add .
git commit -m "feat: Add ACE framework MVP"

# 推送
git push origin main
```

---

## ✅ 验证检查清单

推送前请确认：

### 1. 检查.gitignore生效

```bash
# 应该看到排除的文件
git status --ignored | grep -E "(req|test1111|DEVELOPMENT_LOG)"

# 应该显示: Ignored
```

### 2. 查看将要提交的文件

```bash
git status

# 应该只包含：
# - codex-rs/ 目录下的代码
# - README.md
# - readme-codeACE.md
# - .gitignore
# - 其他必要配置文件

# 不应该包含：
# - req/ 目录
# - test1111/ 目录
# - DEVELOPMENT_LOG.md
# - ACE_TEST_LOG.md
```

### 3. 检查暂存的文件

```bash
# 查看即将提交的具体文件列表
git diff --cached --name-only

# 或查看详细差异
git diff --cached --stat
```

---

## 🔍 故障排查

### 问题1: test1111目录还在git中

```bash
# 如果之前已经提交过test1111，需要移除
git rm -r --cached test1111/
git commit -m "Remove test files from git"
```

### 问题2: 需求文档还在git中

```bash
# 移除req目录
git rm -r --cached req/
git commit -m "Remove requirement docs from git"
```

### 问题3: 开发日志还在git中

```bash
# 移除开发日志
git rm --cached DEVELOPMENT_LOG.md ACE_TEST_LOG.md
git commit -m "Remove development logs from git"
```

### 问题4: 想重新开始

```bash
# 取消所有暂存的更改
git reset HEAD

# 重新添加（会遵循.gitignore）
git add .
git status
```

---

## 📦 推荐的提交信息格式

### 首次提交

```bash
git commit -m "feat: Add ACE framework MVP

- Implement Hook mechanism for minimal intrusion
- Add Reflector for intelligent extraction
- Add Storage system with JSONL format
- Add Context Loader for smart matching
- 19 tests with 100% pass rate
- Performance: <100ms for all operations
- Documentation and examples included
"
```

### 后续提交

```bash
# 功能添加
git commit -m "feat: Add semantic deduplication"

# Bug修复
git commit -m "fix: Correct context loading logic"

# 文档更新
git commit -m "docs: Update installation guide"

# 性能优化
git commit -m "perf: Improve search performance"
```

---

## 🌿 分支管理建议

### 主分支

```bash
main          # 稳定版本
```

### 开发分支

```bash
develop       # 开发分支
feature/*     # 功能分支
fix/*         # 修复分支
```

### 创建功能分支

```bash
# 创建并切换到功能分支
git checkout -b feature/semantic-search

# 开发完成后合并到main
git checkout main
git merge feature/semantic-search
git push origin main
```

---

## 📝 .gitignore说明

当前的.gitignore会排除：

### 开发文件
- `req/` - 你的需求文档
- `test1111/` - 测试框架和测试数据
- `DEVELOPMENT_LOG.md` - 开发过程记录
- `ACE_TEST_LOG.md` - 测试过程记录

### 构建和缓存
- `node_modules/` - Node依赖
- `target/` - Rust编译产物（Cargo.toml中配置）
- `.cache/` - 各种缓存

### 编辑器和系统
- `.vscode/` - VS Code配置
- `.idea/` - JetBrains配置
- `.DS_Store` - macOS系统文件

---

## 🎯 最小推送内容

如果你想要**最小**推送，只包含核心代码：

```bash
# 只添加核心文件
git add codex-rs/codex-ace/
git add codex-rs/core/src/hooks.rs
git add README.md
git add .gitignore

# 提交
git commit -m "feat: Add ACE core functionality"

# 推送
git push origin main
```

---

## 🔐 私密信息检查

推送前确保没有：

```bash
# 检查是否有API密钥
git grep -i "api_key\|secret\|password\|token"

# 检查环境变量文件
git status | grep .env

# 检查配置文件
git status | grep config
```

---

## 📊 推送后验证

推送成功后，访问你的GitHub仓库页面验证：

### ✅ 应该看到
- README.md 显示正确
- codex-rs/ 目录完整
- .gitignore 文件存在
- 必要的配置文件

### ❌ 不应该看到
- req/ 目录
- test1111/ 目录
- DEVELOPMENT_LOG.md
- ACE_TEST_LOG.md
- 其他开发文档

---

## 🚀 快速命令参考

```bash
# 检查状态
git status
git status --ignored

# 添加更改
git add .

# 提交
git commit -m "Your message"

# 推送
git push origin main

# 查看远程
git remote -v

# 查看历史
git log --oneline

# 移除已跟踪但现在被忽略的文件
git rm -r --cached test1111/
git rm --cached DEVELOPMENT_LOG.md
```

---

## 💡 提示

1. **首次推送前**，先用 `git status` 仔细检查
2. **确认.gitignore生效**，可以用 `git check-ignore -v test1111/`
3. **测试推送**，可以先推送到test分支
4. **保留本地备份**，推送不会删除本地文件
5. **推送后**，test1111/和req/等目录仍然在你的本地，只是不会被推送到GitHub

---

**准备好了吗？按照上面的步骤操作即可！** 🚀
