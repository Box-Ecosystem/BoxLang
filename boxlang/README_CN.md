# BoxLang - 盒子语言

BoxLang 是一个适用于各种环境的通用系统级编程语言，吸收了 Rust、Go、Zig 等现代语言的优点，同时提供了独特的特性和语法。

## 版本

当前版本：0.1.0-alpha

## 关键特性

- **Windows 优先**: 原生支持 Windows 开发
- **嵌入式友好**: 设计用于在嵌入式环境中无缝运行
- **高性能**: AOT 编译，零成本抽象
- **易于学习**: 比 Rust 更简洁的语法
- **现代特性**: 支持 async/await、泛型、模式匹配
- **打包支持**: 原生支持应用程序打包

## 商标

"BoxLang" 是用于标识此编程语言的项目特定名称。除非另有说明，否则此名称不是注册商标。

## 文档 / Dokumentation / Dokumentation

本模块支持多种语言。请选择您喜欢的语言：

- [中文文档 (Chinese)](README_CN.md)
- [English Documentation](README_EN.md)
- [Deutsche Dokumentation](README_DE.md)

## 教程

提供多语言的综合教程：

- [中文教程 (Chinese Tutorials)](../boxlang-tutorial/01-introduction/README_CN.md)
- [English Tutorials](../boxlang-tutorial/01-introduction/README_EN.md)
- [Deutsche Tutorials](../boxlang-tutorial/01-introduction/README_DE.md)

## 快速开始

### 安装

```bash
git clone https://github.com/NaAIO27/boxlang.git
cd boxlang/compiler
cargo build --release
```

### 创建新项目

```bash
boxlang new myproject
cd myproject
```

### 编写第一个程序

创建 `src/main.box`:

```boxlang
module myproject;

pub fn main() {
    println("Hello, BoxLang!");
}
```

### 编译和运行

```bash
boxlang compile src/main.box -o hello
boxlang build
boxlang run
```

### 打包应用程序

```bash
boxlang package
boxlang package -o ./dist -n "myapp" -v "1.0.0"
```

## 项目结构

典型的 BoxLang 项目具有以下结构：

```
myproject/
├── box.toml          # 项目配置
├── README.md         # 项目说明
├── .gitignore        # Git 忽略文件
└── src/
    ├── main.box      # 主程序
    └── lib.box       # 库代码
```

## 基础语法

以下是一些基本的 BoxLang 语法示例：

```boxlang
// 变量声明
let x = 10;           // 不可变
let mut y = 20;       // 可变
const PI = 3.14159;   // 常量

// 函数定义
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// 结构体
pub struct Point {
    x: f64,
    y: f64,
}

// 枚举
pub enum Option<T> {
    Some(T),
    None,
}

// 模式匹配
fn process_option<T>(opt: Option<T>) {
    match opt {
        Option::Some(value) => println("Got value: {}", value),
        Option::None => println("Got nothing"),
    }
}
```

*本文档中的代码示例使用 MIT License 许可证。*

## 命令行工具

BoxLang 提供了用于项目管理和编译的综合命令行工具：

```bash
# 创建新项目
boxlang new <project_name>

# 构建项目
boxlang build

# 运行项目
boxlang run

# 编译单个文件
boxlang compile <input_file> -o <output_file>

# 打包应用程序
boxlang package
# 使用自定义选项打包
boxlang package -o ./dist -n "myapp" -v "1.0.0"

# 显示帮助
boxlang help
# 显示特定命令的帮助
boxlang help <command>
```

## 许可证

BoxLang 使用 MIT License 许可证。完整的许可证文本，请参见 [LICENSE](LICENSE) 文件。

## 贡献

我们欢迎对 BoxLang 的贡献！要贡献，请：

1. Fork 仓库
2. 为您的功能或错误修复创建一个新分支
3. 进行更改
4. 提交拉取请求

### 贡献者许可协议 (CLA)

通过为此项目做出贡献，您同意贡献者许可协议 (CLA) 的条款。本协议确保您的贡献已获得适当的许可，并且项目维护者拥有将您的工作包含在项目中的必要权利。

请确保您的代码遵循项目的编码指南并通过所有测试。

## 常见问题

### Q: BoxLang 和 Rust 有什么区别？
A: BoxLang 设计为比 Rust 更易于学习，同时保留了许多其安全特性。它还为各种编程环境提供了更灵活的方法。

### Q: 我可以在嵌入式开发中使用 BoxLang 吗？
A: 是的，BoxLang 设计为嵌入式友好，可以在嵌入式环境中无缝运行。

### Q: BoxLang 如何实现高性能？
A: BoxLang 使用 AOT（预先）编译和零成本抽象，类似于 Rust。

## 社区

加入我们的社区，与其他 BoxLang 用户和贡献者联系：

- [BoxLang GitHub Issues](https://github.com/Box-Ecosystem/BoxLang/issues)

## 免责声明

BoxLang 是一个正在积极开发的开源项目。虽然我们努力提供可靠和安全的编程语言，但请自行承担使用风险。项目维护者不对 BoxLang 的功能或适用性做出任何明示或暗示的保证。

---

由 BoxLang 社区制作
