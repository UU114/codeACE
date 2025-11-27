# Codex Release Build - v0.0.0

## 构建信息

**构建日期**: 2025-11-26  
**构建环境**: Ubuntu 22.04 WSL2  
**Rust版本**: 1.90.0  
**构建类型**: Release (优化+strip)

## 文件列表

### Linux x86_64 (静态链接)

**文件名**: `codex-linux-x86_64`  
**大小**: 49 MB  
**Target**: x86_64-unknown-linux-musl  
**链接类型**: 静态链接 (statically linked)  
**兼容性**: 任何x86_64 Linux发行版 (无依赖)  

**特性**:
- ✅ 零依赖 - 无需安装任何库
- ✅ 完全可移植 - 可直接复制到任何Linux系统
- ✅ musl libc - 更小更安全的C库
- ✅ 静态链接OpenSSL

**验证**:
```bash
ldd codex-linux-x86_64
# 输出: statically linked

./codex-linux-x86_64 --version
# 输出: codex-cli 0.0.0
```

### Windows x86_64

**文件名**: `codex-windows-x86_64.exe`  
**大小**: 39 MB  
**Target**: x86_64-pc-windows-gnu  
**编译方式**: 交叉编译 (Linux -> Windows)  
**兼容性**: Windows 7+ (64位)

**特性**:
- ✅ 原生Windows可执行文件
- ✅ 包含所有必要的运行时库
- ✅ 双击即可运行

**验证** (在Windows上):
```cmd
codex-windows-x86_64.exe --version
```

## 编译选项

```toml
[profile.release]
lto = "fat"              # 完全链接时优化
strip = "symbols"        # 剥离调试符号
opt-level = 3            # 最高优化级别
```

## ACE功能

✅ **已启用** - 两个版本都包含完整的ACE框架支持:
- Playbook学习和检索
- 上下文智能注入
- 模糊匹配
- 中文支持

## 使用说明

### Linux

1. 下载 `codex-linux-x86_64`
2. 添加执行权限: `chmod +x codex-linux-x86_64`
3. 运行: `./codex-linux-x86_64 --help`

可选择移动到PATH目录:
```bash
sudo mv codex-linux-x86_64 /usr/local/bin/codex
```

### Windows

1. 下载 `codex-windows-x86_64.exe`
2. 双击运行或从CMD/PowerShell执行
3. 可重命名为 `codex.exe`

可选择添加到PATH:
- 将文件复制到 `C:\Program Files\Codex\`
- 添加该目录到系统PATH环境变量

## 配置文件

首次运行会自动创建配置:
- **Linux**: `~/.codeACE/`
- **Windows**: `C:\Users\<用户名>\.codeACE\`

配置文件:
- `config.toml` - 主配置
- `codeACE-config.toml` - ACE框架配置

## 校验和

见 `SHA256SUMS.txt` 文件

验证文件完整性:
```bash
sha256sum -c SHA256SUMS.txt
```

## 已知问题

无

## 测试状态

- ✅ Linux版本: 已测试,运行正常
- ⚠️ Windows版本: 交叉编译,建议在Windows上实际测试

## 编译时间

- Linux musl版本: ~17分钟
- Windows版本: ~20分钟
- 总计: ~37分钟

## 技术细节

### Linux Static Build

使用musl-libc实现完全静态链接:
- musl-tools提供musl-gcc编译器
- OpenSSL从源码编译并静态链接
- ring库静态编译
- 最终产物无任何动态库依赖

### Windows Cross-Compilation

使用MinGW-w64工具链:
- x86_64-w64-mingw32-gcc交叉编译器
- 从Linux编译Windows PE可执行文件
- 包含Windows运行时库

## 下一步

如需创建GitHub Release:
1. 创建git tag: `git tag v0.0.0-20251126`
2. 推送tag: `git push origin v0.0.0-20251126`
3. 在GitHub创建Release
4. 上传这两个文件和SHA256SUMS.txt

---

**构建者**: codeACE AI Assistant  
**构建日期**: 2025-11-26 08:02 UTC
