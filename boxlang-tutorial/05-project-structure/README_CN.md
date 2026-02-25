# BoxLang 项目结构

本章节介绍 BoxLang 项目的标准结构和配置。

## 标准项目结构

一个典型的 BoxLang 项目结构如下：

```
myproject/
├── box.toml              # 项目配置文件（必需）
├── README.md             # 项目说明文档
├── LICENSE               # 许可证文件
├── .gitignore            # Git 忽略文件
├── .boxlang/             # BoxLang 内部目录
│   └── cache/            # 编译缓存
├── src/                  # 源代码目录
│   ├── main.box          # 主程序入口（可执行项目）
│   ├── lib.box           # 库入口（库项目）
│   └── utils/            # 子模块目录
│       └── helper.box
├── tests/                # 测试代码
│   └── integration_test.box
├── examples/             # 示例代码
│   └── basic_usage.box
├── docs/                 # 项目文档
│   └── api.md
└── target/               # 编译输出目录
    ├── debug/            # 调试构建输出
    └── release/          # 发布构建输出
```

## box.toml 配置详解

### 基本配置

```toml
[package]
name = "myproject"           # 项目名称
version = "1.0.0"            # 版本号（遵循语义化版本）
authors = ["Your Name <you@example.com>"]
edition = "2024"             # BoxLang 版本
license = "MIT"
description = "项目描述"
repository = "https://github.com/username/myproject"

[dependencies]
# 依赖项
std = { version = "1.0" }
serde = { version = "0.8", features = ["derive"] }

[dependencies.mylib]
path = "../mylib"            # 本地路径依赖

[dependencies.remote-lib]
git = "https://github.com/user/repo.git"
branch = "main"

[build]
target = "x86_64-pc-windows-msvc"  # 目标平台
opt-level = 3                      # 优化级别 (0-3)
debug = false                      # 是否包含调试信息

[features]
default = ["std"]
std = []
no_std = []
embedded = ["no_std"]
```

### 多目标配置

```toml
# Windows 桌面应用
[[target]]
name = "windows-app"
target = "x86_64-pc-windows-msvc"
output-type = "exe"

# zetboxos 嵌入式应用
[[target]]
name = "zetboxos-app"
target = "thumbv7em-none-eabihf"
output-type = "bin"
```

## 源代码组织

### 可执行项目

```boxlang
// src/main.box
module myproject;

use std::io;
use utils::helper;

pub fn main() {
    println("程序启动");
    helper::do_something();
}
```

### 库项目

```boxlang
// src/lib.box
module mylib;

pub mod core;
pub mod utils;

// 公开 API
pub use core::engine::Engine;
pub use utils::helpers::format_data;
```

### 子模块

```boxlang
// src/utils/helper.box
module myproject::utils::helper;

pub fn do_something() {
    println("Doing something...");
}

pub struct Helper {
    name: str,
}

impl Helper {
    pub fn new(name: str) -> Helper {
        Helper { name }
    }
}
```

## 模块系统

### 模块声明

```boxlang
// 声明当前模块
module myproject::core::engine;

// 导入标准库
use std::collections::HashMap;
use std::io::{File, Read};

// 导入本地模块
use crate::utils::helper;
use super::config::Config;

// 公开导入
pub use self::types::EngineType;
```

### 模块可见性

```boxlang
// 默认私有
fn private_function() {}
struct PrivateStruct {}

// 公开
pub fn public_function() {}
pub struct PublicStruct {}

// 仅 crate 内可见
pub(crate) fn crate_function() {}

// 仅父模块可见
pub(super) fn parent_visible() {}
```

## 测试组织

### 单元测试

```boxlang
// src/calculator.box
module myproject::calculator;

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

// 内置测试
#[test]
fn test_add() {
    assert_eq!(add(2, 3), 5);
    assert_eq!(add(-1, 1), 0);
}

#[test]
#[should_panic]
fn test_overflow() {
    // 测试溢出情况
}
```

### 集成测试

```boxlang
// tests/integration_test.box
use myproject::calculator;

#[test]
fn test_calculator_integration() {
    let result = calculator::add(10, 20);
    assert_eq!(result, 30);
}
```

## 工作区（Workspace）

### 工作区配置

```toml
# box.toml (workspace root)
[workspace]
members = [
    "mylib",
    "myapp",
    "utils",
]

[workspace.dependencies]
serde = "1.0"
```

### 工作区结构

```
workspace/
├── box.toml              # 工作区配置
├── mylib/
│   ├── box.toml
│   └── src/
├── myapp/
│   ├── box.toml
│   └── src/
└── utils/
    ├── box.toml
    └── src/
```

## 构建配置

### 条件编译

```boxlang
// 平台特定代码
#[cfg(target_os = "windows")]
fn platform_specific() {
    // Windows 代码
}

#[cfg(target_os = "zetboxos")]
fn platform_specific() {
    // zetboxos 代码
}

// 特性开关
#[cfg(feature = "std")]
use std::collections::HashMap;

#[cfg(not(feature = "std"))]
use alloc::collections::HashMap;
```

### 构建脚本

```toml
# box.toml
[package]
build = "build.box"
```

```boxlang
// build.box
use std::process::Command;

fn main() {
    // 生成代码
    println!("cargo:rerun-if-changed=src/schema.json");
    
    // 设置环境变量
    println!("cargo:rustc-env=BUILD_TIME=2024-01-01");
}
```

## 下一步

- [AppBox 打包](../06-appbox-packaging/README_CN.md) - 学习如何打包和发布应用
