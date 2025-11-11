#!/bin/bash
# 检查Git状态 - 用于推送前验证

echo "========================================"
echo "  CodeACE Git 状态检查"
echo "========================================"
echo ""

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}[1] 检查.gitignore规则...${NC}"
echo ""

# 检查排除的目录
EXCLUDED_DIRS=("req/" "test1111/")
EXCLUDED_FILES=("DEVELOPMENT_LOG.md" "ACE_TEST_LOG.md" "readme-codex.md")

for dir in "${EXCLUDED_DIRS[@]}"; do
    if git check-ignore -q "$dir"; then
        echo -e "  ${GREEN}✓${NC} $dir - 已排除"
    else
        echo -e "  ${RED}✗${NC} $dir - 未排除！"
    fi
done

for file in "${EXCLUDED_FILES[@]}"; do
    if [ -f "$file" ]; then
        if git check-ignore -q "$file"; then
            echo -e "  ${GREEN}✓${NC} $file - 已排除"
        else
            echo -e "  ${RED}✗${NC} $file - 未排除！"
        fi
    fi
done

echo ""
echo -e "${BLUE}[2] 将要提交的文件（新增/修改）...${NC}"
echo ""

# 统计
STAGED_COUNT=$(git diff --cached --name-only 2>/dev/null | wc -l)
MODIFIED_COUNT=$(git diff --name-only 2>/dev/null | wc -l)
UNTRACKED_COUNT=$(git ls-files --others --exclude-standard 2>/dev/null | wc -l)

if [ $STAGED_COUNT -gt 0 ]; then
    echo -e "${YELLOW}已暂存的文件 ($STAGED_COUNT):${NC}"
    git diff --cached --name-only | head -20
    if [ $STAGED_COUNT -gt 20 ]; then
        echo "  ... 和其他 $((STAGED_COUNT - 20)) 个文件"
    fi
    echo ""
fi

if [ $MODIFIED_COUNT -gt 0 ]; then
    echo -e "${YELLOW}已修改但未暂存 ($MODIFIED_COUNT):${NC}"
    git diff --name-only | head -10
    if [ $MODIFIED_COUNT -gt 10 ]; then
        echo "  ... 和其他 $((MODIFIED_COUNT - 10)) 个文件"
    fi
    echo ""
fi

if [ $UNTRACKED_COUNT -gt 0 ]; then
    echo -e "${YELLOW}未跟踪的文件 ($UNTRACKED_COUNT):${NC}"
    git ls-files --others --exclude-standard | head -10
    if [ $UNTRACKED_COUNT -gt 10 ]; then
        echo "  ... 和其他 $((UNTRACKED_COUNT - 10)) 个文件"
    fi
    echo ""
fi

echo -e "${BLUE}[3] 检查是否有不应该推送的文件...${NC}"
echo ""

# 检查暂存区是否有排除的文件
SHOULD_NOT_PUSH=""
for dir in "${EXCLUDED_DIRS[@]}"; do
    if git diff --cached --name-only 2>/dev/null | grep -q "^${dir}"; then
        echo -e "  ${RED}⚠ 警告: ${dir} 在暂存区中！${NC}"
        SHOULD_NOT_PUSH="yes"
    fi
done

for file in "${EXCLUDED_FILES[@]}"; do
    if git diff --cached --name-only 2>/dev/null | grep -q "^${file}$"; then
        echo -e "  ${RED}⚠ 警告: ${file} 在暂存区中！${NC}"
        SHOULD_NOT_PUSH="yes"
    fi
done

if [ -z "$SHOULD_NOT_PUSH" ]; then
    echo -e "  ${GREEN}✓ 没有不应该推送的文件${NC}"
fi

echo ""
echo -e "${BLUE}[4] 核心ACE文件检查...${NC}"
echo ""

# 检查核心文件
CORE_FILES=(
    "codex-rs/codex-ace/src/lib.rs"
    "codex-rs/codex-ace/src/reflector.rs"
    "codex-rs/codex-ace/src/storage.rs"
    "codex-rs/codex-ace/src/context.rs"
    "codex-rs/codex-ace/Cargo.toml"
    "codex-rs/core/src/hooks.rs"
    "README.md"
)

for file in "${CORE_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo -e "  ${GREEN}✓${NC} $file"
    else
        echo -e "  ${RED}✗${NC} $file - 缺失！"
    fi
done

echo ""
echo -e "${BLUE}[5] 远程仓库信息...${NC}"
echo ""

if git remote -v &>/dev/null; then
    REMOTE_COUNT=$(git remote | wc -l)
    if [ $REMOTE_COUNT -gt 0 ]; then
        git remote -v
    else
        echo -e "  ${YELLOW}⚠ 未配置远程仓库${NC}"
        echo "  使用: git remote add origin https://github.com/YOUR_USERNAME/codeACE.git"
    fi
else
    echo -e "  ${YELLOW}⚠ 未配置远程仓库${NC}"
fi

echo ""
echo "========================================"
echo -e "${GREEN}检查完成！${NC}"
echo "========================================"
echo ""

if [ -z "$SHOULD_NOT_PUSH" ]; then
    echo -e "${GREEN}✓ 准备就绪！可以执行推送操作${NC}"
    echo ""
    echo "推送命令:"
    echo "  git add ."
    echo "  git commit -m 'feat: Add ACE framework MVP'"
    echo "  git push origin main"
else
    echo -e "${RED}⚠ 发现问题！请先解决上述警告${NC}"
    echo ""
    echo "移除不需要的文件:"
    echo "  git rm -r --cached req/ test1111/"
    echo "  git rm --cached DEVELOPMENT_LOG.md ACE_TEST_LOG.md"
fi

echo ""
