# BoxLang 快速开始

本指南将帮助你在 5 分钟内创建并运行第一个 BoxLang 程序。

## 创建新项目

### 使用 boxlang new 命令

```bash
# 创建名为 hello_boxlang 的新项目
boxlang new hello_boxlang

# 进入项目目录
cd hello_boxlang
```

### 项目结构

创建完成后，你会看到以下目录结构：

```
hello_boxlang/
├── box.toml          # 项目配置文件
├── README.md         # 项目说明
├── .gitignore        # Git 忽略文件
└── src/
    └── main.box      # 主程序入口
```

## 编写第一个程序

### 默认生成的 main.box

```boxlang
module hello_boxlang;

pub fn main() {
    println("Hello, BoxLang!");
}
```

### 自定义程序

让我们修改程序，添加一些交互功能：

```boxlang
module hello_boxlang;

pub fn main() {
    // 打印欢迎信息
    println("欢迎使用 BoxLang!");
    println("==================");
    
    // 变量声明
    let name = "BoxLang 开发者";
    let version = 1.0;
    
    // 字符串格式化输出
    println("你好, {name}!");
    println("当前版本: {version}");
    
    // 调用函数
    let result = add(10, 20);
    println("10 + 20 = {result}");
}

// 定义加法函数
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
```

## 编译和运行

### 方式一：编译后运行

```bash
# 编译项目
boxlang build

# 运行编译后的程序
boxlang run
```

### 方式二：直接运行（开发模式）

```bash
# 编译并立即运行
boxlang run --dev
```

### 方式三：指定输出文件

```bash
# 编译为指定名称的可执行文件
boxlang compile src/main.box -o myprogram

# 直接运行
./myprogram
```

## 开发工作流

### 1. 编辑代码
使用你喜欢的编辑器修改 `src/main.box` 文件。

### 2. 检查语法
```bash
# 检查代码语法而不编译
boxlang check
```

### 3. 编译项目
```bash
# 开发模式（快速编译，无优化）
boxlang build --dev

# 发布模式（优化编译）
boxlang build --release
```

### 4. 运行测试
```bash
# 运行项目中的测试
boxlang test
```

## 常用命令速查

| 命令 | 说明 |
|------|------|
| `boxlang new <name>` | 创建新项目 |
| `boxlang build` | 编译项目 |
| `boxlang run` | 运行项目 |
| `boxlang check` | 语法检查 |
| `boxlang test` | 运行测试 |
| `boxlang clean` | 清理构建文件 |
| `boxlang fmt` | 格式化代码 |

## 下一步

- [基础语法](../04-basic-syntax/README_CN.md) - 学习 BoxLang 的核心语法
- [项目结构](../05-project-structure/README_CN.md) - 了解项目配置和结构
