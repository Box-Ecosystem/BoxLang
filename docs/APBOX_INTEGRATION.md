# BoxLang 与 AppBox 生态集成

本文档描述了 BoxLang 编译器与 AppBox 应用容器格式的集成，以及如何打包和分发 BoxLang 应用程序。

## 概述

AppBox 是一个轻量级的应用容器格式，专为 Box 生态系统设计。BoxLang 编译器现在原生支持将应用程序打包为 AppBox 格式，实现以下目标：

1. **简化部署** - 一键打包应用程序及其依赖
2. **资源隔离** - 定义内存、CPU 等资源限制
3. **权限管理** - 声明式权限控制
4. **跨平台兼容** - 支持多种目标架构

## 使用方法

### 1. 创建 BoxLang 项目

```bash
mkdir myapp
cd myapp
```

创建 `box.toml`:
```toml
[package]
name = "myapp"
version = "0.1.0"
edition = "2024"
description = "My BoxLang application"
```

创建 `src/main.box`:
```rust
module myapp;

pub fn main() {
    println("Hello from BoxLang!");
}
```

### 2. 打包为 AppBox

使用 `boxlang package` 命令：

```bash
# 基本打包
boxlang package

# 指定输出目录
boxlang package -o ./dist

# 指定应用名称和版本
boxlang package -n "myapp" -v "1.0.0"

# 指定作者和优化级别
boxlang package -a "Your Name" -O 2
```

### 3. 完整的 box.toml 配置

```toml
[package]
name = "myapp"
version = "1.0.0"
edition = "2024"
description = "My BoxLang application"
author = "Your Name"

[dependencies]
# 依赖项

[appbox]
# 权限声明
permissions = ["network", "filesystem", "camera"]

# 资源限制
memory = "128MB"
cpu = "50%"
disk = "100MB"
network = true
```

## 已修复的 Bug

### 1. Parser 安全性修复

**问题**: Parser 中的数组索引操作可能导致 panic

**修复**:
- 使用 `saturating_add` 防止整数溢出
- `advance()` 方法现在返回 `Option` 而不是直接解引用
- 添加了边界检查，避免访问越界

```rust
// 修复前
fn advance(&mut self) -> &SpannedToken {
    if !self.is_at_end() {
        self.current += 1;
    }
    &self.tokens[self.current - 1]  // 可能越界
}

// 修复后
fn advance(&mut self) -> Option<&SpannedToken> {
    if !self.is_at_end() {
        self.current += 1;
        self.tokens.get(self.current - 1)
    } else {
        None
    }
}
```

### 2. 错误报告改进

**问题**: `error_unexpected` 和 `error_syntax` 方法可能访问无效索引

**修复**:
- 添加了安全的索引访问
- 改进错误位置报告

```rust
fn error_unexpected(&self, expected: &str, found: &str) -> ParseError {
    let (line, column) = if let Some(token) = self.tokens.get(self.current) {
        (token.line, token.column)
    } else if let Some(token) = self.tokens.last() {
        (token.line, token.column + 1)
    } else {
        (1, 1)
    };
    // ...
}
```

### 3. C 代码生成器安全修复

**问题**: Method call 生成可能存在注入风险

**修复**:
- 添加方法名验证
- 防止以数字开头的标识符
- 改进参数处理

```rust
// 验证方法名
let safe_method_name: String = method_name
    .chars()
    .filter(|c| c.is_alphanumeric() || *c == '_')
    .collect();

if safe_method_name.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
    return Err("Invalid method name: cannot start with a digit".to_string());
}
```

## 新功能

### 1. AppBox 打包支持

新增 `boxlang package` 命令，支持：
- 自动检测项目配置
- 生成 AppBox manifest
- 创建 ZIP 格式的包文件

### 2. 集成模块

新增 `integration` 模块，提供：
- `AppBoxBuilder` - 构建器模式创建包
- `AppBoxManifest` - 清单结构定义
- 配置转换工具

### 3. 改进的 CLI

- 彩色输出支持
- 进度指示器
- 详细的错误信息

## 示例

### 示例 1: 基本打包

```bash
cd box-ecosystem/boxlang/examples/appbox-demo
boxlang package
```

输出:
```
╔══════════════════════════════════════════╗
║    Packaging AppBox Application          ║
╚══════════════════════════════════════════╝

Build Configuration
  Project    : appbox-demo
  Version    : 0.1.0
  Author     : BoxLang Team
  Opt Level  : -O2
  Output     : dist

Collecting source files...
  ✓ Found src/main.box

Building AppBox package...
  ✓ Package created successfully
  Package    : dist/appbox-demo.box

✓ Successfully packaged appbox-demo
```

### 示例 2: 编程方式使用

```rust
use boxlang_compiler::integration::appbox::{AppBoxBuilder, generate_default_main};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let builder = AppBoxBuilder::new("myapp", "./dist")
        .version("1.0.0")
        .author("Developer")
        .opt_level(2)
        .add_source("src/main.box")
        .add_permission("network", "Network access", true);
    
    let package_path = builder.build()?;
    println!("Created: {}", package_path.display());
    
    Ok(())
}
```

## 未来计划

1. **签名验证** - 支持包签名和验证
2. **依赖管理** - 自动处理依赖项
3. **沙箱执行** - 运行时资源隔离
4. **应用商店** - 集成 Box 应用商店

## 相关文档

- [BoxLang 完整教程](./TUTORIAL.md)
- [BoxLang 编译器修复计划](../../.trae/documents/BoxLang%20编译器深度修复计划.md)
- [AppBox 规范](../host/README.md)
