# BoxLang 安装指南

## 系统要求

### 最低配置
- **操作系统**: Windows 10/11 (64位)
- **内存**: 4 GB RAM
- **磁盘空间**: 2 GB 可用空间
- **网络**: 互联网连接（用于下载依赖）

### 推荐配置
- **操作系统**: Windows 11 (64位)
- **内存**: 8 GB RAM
- **磁盘空间**: 5 GB 可用空间
- **开发环境**: Visual Studio 2022 或 VS Code

## 前置依赖

在安装 BoxLang 之前，请确保已安装以下软件：

### 1. Git
```bash
# 检查是否已安装
git --version

# 下载地址: https://git-scm.com/download/win
```

### 2. Rust 工具链
```bash
# 使用 rustup 安装（推荐）
# 下载地址: https://rustup.rs/

# 安装后检查
rustc --version
cargo --version
```

### 3. LLVM（可选，用于优化编译）
```bash
# 下载地址: https://releases.llvm.org/download.html
# 建议版本: 15.0 或更高
```

## 安装步骤

### 方式一：从源码编译安装

#### 1. 克隆仓库
```bash
git clone https://github.com/yourusername/box-ecosystem.git
cd box-ecosystem
```

#### 2. 编译编译器
```bash
cd boxlang/compiler
cargo build --release
```

编译完成后，可执行文件位于：
```
target/release/boxlang.exe
```

#### 3. 添加到系统 PATH
```powershell
# PowerShell（以管理员身份运行）
[Environment]::SetEnvironmentVariable(
    "Path",
    [Environment]::GetEnvironmentVariable("Path", "User") + ";C:\path\to\box-ecosystem\boxlang\compiler\target\release",
    "User"
)
```

### 方式二：使用安装脚本（推荐）

```powershell
# PowerShell
irm https://boxlang.dev/install.ps1 | iex
```

### 方式三：手动下载预编译版本

1. 访问 [GitHub Releases](https://github.com/yourusername/box-ecosystem/releases)
2. 下载最新版本的 `boxlang-windows-x64.zip`
3. 解压到目标目录
4. 将目录添加到系统 PATH

## 验证安装

```bash
# 检查版本
boxlang --version

# 查看帮助
boxlang --help

# 测试编译器
boxlang doctor
```

## 配置开发环境

### VS Code 配置

#### 1. 安装扩展
- BoxLang Language Support
- BoxLang Debugger

#### 2. 配置 settings.json
```json
{
    "boxlang.compilerPath": "C:\\path\\to\\boxlang.exe",
    "boxlang.enableLinter": true,
    "boxlang.formatOnSave": true
}
```

### Visual Studio 配置

#### 1. 安装 BoxLang VS Extension
```bash
boxlang install-vs-extension
```

#### 2. 配置项目属性
- 右键项目 → 属性 → BoxLang
- 设置编译器路径和编译选项

## 常见问题

### Q: 编译时出现 "linker not found" 错误
**A**: 安装 Visual C++ Build Tools
```powershell
# 使用 Visual Studio Installer 安装
# 或下载独立版本
https://visualstudio.microsoft.com/visual-cpp-build-tools/
```

### Q: cargo build 失败，提示缺少依赖
**A**: 更新 Rust 工具链
```bash
rustup update
rustup component add rust-src
```

### Q: boxlang 命令无法识别
**A**: 检查 PATH 配置
```powershell
# 查看当前 PATH
$env:Path -split ";"

# 确认包含 boxlang.exe 所在目录
```

## 更新 BoxLang

```bash
# 从源码更新
cd box-ecosystem
git pull
cd boxlang/compiler
cargo build --release

# 或使用更新命令
boxlang self-update
```

## 卸载 BoxLang

```powershell
# 删除安装目录
Remove-Item -Recurse -Force "C:\path\to\box-ecosystem"

# 从 PATH 中移除
# 手动编辑系统环境变量
```

## 下一步

- [快速开始](../03-quickstart/README_CN.md) - 创建你的第一个 BoxLang 项目
