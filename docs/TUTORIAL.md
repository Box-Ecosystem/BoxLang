# BoxLang 完整教程

## 目录

1. [简介](#简介)
2. [安装和配置](#安装和配置)
3. [基础语法](#基础语法)
4. [类型系统](#类型系统)
5. [变量和常量](#变量和常量)
6. [运算符](#运算符)
7. [函数](#函数)
8. [结构体和方法](#结构体和方法)
9. [控制流](#控制流)
10. [管道操作符](#管道操作符)
11. [模块系统](#模块系统)
12. [构建 Windows EXE 工作流](#构建-windows-exe-工作流)
13. [构建 APPbox 应用工作流](#构建-appbox-应用工作流)
14. [实战示例](#实战示例)
15. [最佳实践](#最佳实践)

---

## 简介

BoxLang 是一个为 Box Ecosystem 设计的系统级编程语言，语法风格类似 Rust，但具有独特特性。它专为 Windows 和嵌入式系统（zetboxos/LiteOS-M）优化，采用 AOT 编译方式，通过 C 语言作为中间表示。

### 设计目标

- **Windows 优先**: 原生支持 Windows 平台
- **嵌入式友好**: 支持 LiteOS-M 等嵌入式系统
- **高性能**: AOT 编译，零成本抽象
- **独特特性**: 管道操作符、多种内存分配策略、轻量级线程
- **简洁易用**: 类似 Rust 的语法，但更简洁

### 与 Rust 的关键区别

| 特性 | Rust | BoxLang |
|------|------|---------|
| **管道操作符** | 无内置 | 原生支持 `\|>` |
| **字符串插值** | `format!` 宏 | 原生支持 `"hello {name}"` |
| **内存管理** | 单一所有权 | 多种分配策略（Box/Arena/Pool） |
| **并发** | OS 线程 + async | 轻量级线程（Green Thread） |
| **运行时** | 需要选择 | 内置运行时 |

---

## 安装和配置

### 环境要求

- Windows 10/11
- Rust 工具链 (用于编译编译器)
- MSVC 或 Clang (用于编译生成的 C 代码)

### 安装步骤

1. **克隆仓库**
```bash
git clone https://github.com/your-org/boxlang.git
cd boxlang
```

2. **编译编译器**
```bash
cd compiler
cargo build --release
```

3. **验证安装**
```bash
./compiler/target/release/boxlang.exe --version
```

### 基本用法

```bash
# 编译单个文件
boxlang compile hello.box -o hello.exe

# 查看词法分析结果
boxlang tokenize hello.box

# 查看 AST
boxlang ast hello.box

# 运行程序
boxlang run hello.box
```

---

## 基础语法

### Hello World

```boxlang
module hello;

pub fn main() {
    println("Hello, BoxLang!");
}
```

### 注释

```boxlang
// 单行注释

/*
 * 多行注释
 */

/// 文档注释（用于函数和类型）
fn documented_function() {
    // 函数实现
}
```

### 语句和表达式

BoxLang 中，语句以分号结尾，最后一个表达式可以作为返回值：

```boxlang
fn explicit_return() -> i32 {
    return 42;  // 显式 return
}

fn implicit_return() -> i32 {
    42  // 隐式返回（最后一个表达式）
}

fn multiple_statements() -> i32 {
    let x = 10;
    let y = 20;
    x + y  // 返回 x + y 的结果
}
```

---

## 类型系统

### 基本类型

| 类型 | 描述 | 示例 |
|------|------|------|
| `i8` | 8位有符号整数 | `-128` 到 `127` |
| `i16` | 16位有符号整数 | `-32768` 到 `32767` |
| `i32` | 32位有符号整数 | `42` |
| `i64` | 64位有符号整数 | `10000000000` |
| `u8` | 8位无符号整数 | `0` 到 `255` |
| `u16` | 16位无符号整数 | `0` 到 `65535` |
| `u32` | 32位无符号整数 | `42u32` |
| `u64` | 64位无符号整数 | `10000000000u64` |
| `f32` | 32位浮点数 | `3.14f32` |
| `f64` | 64位浮点数 | `3.14159` |
| `bool` | 布尔类型 | `true`, `false` |
| `char` | 字符类型 | `'a'` |
| `String` | 字符串类型 | `"hello"` |

### 类型注解

```boxlang
let x: i32 = 42;
let name: String = "BoxLang";
let flag: bool = true;
let pi: f64 = 3.14159;
```

### 类型推断

BoxLang 支持类型推断，大多数情况下可以省略类型注解：

```boxlang
let x = 42;        // 推断为 i32
let pi = 3.14;     // 推断为 f64
let name = "Box";  // 推断为 String
```

### 数组类型

```boxlang
// 固定大小数组
let arr: [i32; 5] = [1, 2, 3, 4, 5];

// 访问数组元素
let first = arr[0];
let second = arr[1];

// 数组长度
let len = arr.len();

// 数组切片
let slice: &[i32] = &arr[0..3];  // 引用前3个元素
```

### 元组类型

```boxlang
// 元组可以包含不同类型的元素
let tuple: (i32, f64, String) = (42, 3.14, "hello");

// 访问元组元素
let first = tuple.0;   // 42
let second = tuple.1;  // 3.14
let third = tuple.2;   // "hello"

// 元组解构
let (x, y, z) = tuple;
```

---

## 变量和常量

### 变量声明

使用 `let` 声明不可变变量：

```boxlang
let x = 42;
// x = 100;  // 错误：不能修改不可变变量
```

使用 `let mut` 声明可变变量：

```boxlang
let mut x = 42;
x = 100;  // 正确
```

### 常量

使用 `const` 声明常量：

```boxlang
const PI: f64 = 3.14159;
const MAX_SIZE: i32 = 100;
const APP_NAME: String = "MyApp";
```

常量与变量的区别：
- 常量必须在编译时确定值
- 常量不能是 `mut` 的
- 常量使用全大写命名规范

### 变量遮蔽（Shadowing）

```boxlang
let x = 5;
let x = x + 1;  // 创建新的变量 x，值为 6
let x = x * 2;  // 再次遮蔽，值为 12
```

---

## 运算符

### 算术运算符

```boxlang
let a = 10;
let b = 3;

let sum = a + b;        // 加法: 13
let diff = a - b;       // 减法: 7
let product = a * b;    // 乘法: 30
let quotient = a / b;   // 除法: 3 (整数除法)
let remainder = a % b;  // 取模: 1
```

### 比较运算符

```boxlang
let a = 10;
let b = 20;

let eq = a == b;    // 等于: false
let ne = a != b;    // 不等于: true
let lt = a < b;     // 小于: true
let gt = a > b;     // 大于: false
let le = a <= b;    // 小于等于: true
let ge = a >= b;    // 大于等于: false
```

### 逻辑运算符

```boxlang
let a = true;
let b = false;

let and = a && b;   // 逻辑与: false
let or = a || b;    // 逻辑或: true
let not = !a;       // 逻辑非: false
```

### 位运算符

```boxlang
let a = 0b1100;  // 12
let b = 0b1010;  // 10

let and = a & b;    // 按位与: 0b1000 (8)
let or = a | b;     // 按位或: 0b1110 (14)
let xor = a ^ b;    // 按位异或: 0b0110 (6)
let not = !a;       // 按位非
let shl = a << 2;   // 左移: 0b110000 (48)
let shr = a >> 2;   // 右移: 0b0011 (3)
```

### 其他运算符

```boxlang
// 幂运算
let power = 2 ** 10;  // 1024

// 空值合并运算符
let value = maybe_null ?? default_value;

// Elvis 运算符
let name = user?.name ?: "Anonymous";

// 管道操作符
let result = value |> func1 |> func2 |> func3;
```

### 赋值运算符

```boxlang
let mut x = 10;

x += 5;   // x = x + 5
x -= 3;   // x = x - 3
x *= 2;   // x = x * 2
x /= 2;   // x = x / 2
x %= 3;   // x = x % 3
x **= 2;  // x = x ** 2 (幂运算)
x &= 0xFF;  // x = x & 0xFF
x |= 0x10;  // x = x | 0x10
x ^= 0x01;  // x = x ^ 0x01
x <<= 2;    // x = x << 2
x >>= 2;    // x = x >> 2

// 自增/自减
x++;  // x = x + 1
x--;  // x = x - 1
```

---

## 函数

### 函数定义

```boxlang
// 无参数，无返回值
fn say_hello() {
    println("Hello!");
}

// 带参数
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// 简写形式（省略 return）
fn subtract(a: i32, b: i32) -> i32 {
    a - b
}

// 多个参数
fn greet(name: String, age: i32) {
    println("Hello, " + name + "! You are " + age.to_string() + " years old.");
}
```

### 函数调用

```boxlang
pub fn main() {
    say_hello();
    
    let sum = add(10, 20);
    println("Sum: " + sum.to_string());
    
    greet("Alice", 25);
}
```

### 递归函数

```boxlang
// 阶乘
fn factorial(n: i32) -> i32 {
    if n <= 1 {
        return 1;
    }
    return n * factorial(n - 1);
}

// 斐波那契数列
fn fibonacci(n: i32) -> i32 {
    if n <= 0 {
        return 0;
    }
    if n == 1 {
        return 1;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}
```

### 函数作为参数

```boxlang
fn apply_operation(x: i32, operation: fn(i32) -> i32) -> i32 {
    return operation(x);
}

fn double(x: i32) -> i32 {
    return x * 2;
}

fn square(x: i32) -> i32 {
    return x * x;
}

pub fn main() {
    let result1 = apply_operation(5, double);  // 10
    let result2 = apply_operation(5, square);  // 25
}
```

---

## 结构体和方法

### 结构体定义

```boxlang
struct Point {
    x: i32,
    y: i32,
}

struct Rectangle {
    width: i32,
    height: i32,
}

struct Person {
    name: String,
    age: i32,
    email: String,
}
```

### 结构体实例化

```boxlang
let p = Point { x: 10, y: 20 };

let rect = Rectangle { width: 100, height: 50 };

let person = Person {
    name: "Alice",
    age: 25,
    email: "alice@example.com",
};
```

### 字段访问

```boxlang
let x = p.x;
let y = p.y;
let area = rect.width * rect.height;
```

### 结构体更新语法

```boxlang
let p1 = Point { x: 10, y: 20 };
let p2 = Point { x: 30, ..p1 };  // x = 30, y = 20 (从 p1 复制)
```

### Union 类型

```boxlang
union IntOrFloat {
    i: i32,
    f: f32,
}

let u = IntOrFloat { i: 42 };
// 访问 union 字段（不安全）
let value = unsafe { u.i };
```

### 元组结构体

```boxlang
struct Color(i32, i32, i32);  // RGB

let red = Color(255, 0, 0);
let r = red.0;  // 255
```

### 单元结构体

```boxlang
struct Empty;  // 没有字段的结构体

let empty = Empty;
```

### 实现块（impl）

```boxlang
struct Rectangle {
    width: i32,
    height: i32,
}

impl Rectangle {
    // 构造函数（关联函数）
    fn new(width: i32, height: i32) -> Rectangle {
        return Rectangle { width, height };
    }
    
    // 方法（第一个参数是 self）
    fn area(self) -> i32 {
        return self.width * self.height;
    }
    
    fn perimeter(self) -> i32 {
        return 2 * (self.width + self.height);
    }
    
    fn is_square(self) -> bool {
        return self.width == self.height;
    }
    
    // 关联函数（不需要 self）
    fn square(size: i32) -> Rectangle {
        return Rectangle { width: size, height: size };
    }
}

pub fn main() {
    // 使用构造函数
    let rect = Rectangle::new(10, 20);
    
    // 调用方法
    let area = rect.area();
    let perimeter = rect.perimeter();
    let is_square = rect.is_square();
    
    // 使用关联函数
    let square = Rectangle::square(15);
}
```

### 方法链式调用

```boxlang
struct Calculator {
    result: i32,
}

impl Calculator {
    fn new() -> Calculator {
        return Calculator { result: 0 };
    }
    
    fn add(self, value: i32) -> Calculator {
        return Calculator { result: self.result + value };
    }
    
    fn subtract(self, value: i32) -> Calculator {
        return Calculator { result: self.result - value };
    }
    
    fn multiply(self, value: i32) -> Calculator {
        return Calculator { result: self.result * value };
    }
    
    fn get_result(self) -> i32 {
        return self.result;
    }
}

pub fn main() {
    let result = Calculator::new()
        .add(10)
        .multiply(2)
        .subtract(5)
        .get_result();
    // 结果: ((0 + 10) * 2) - 5 = 15
}
```

---

## 控制流

### if 表达式

```boxlang
let x = 10;

if x > 0 {
    println("Positive");
} else if x < 0 {
    println("Negative");
} else {
    println("Zero");
}
```

### if 作为表达式

```boxlang
let x = 10;
let message = if x > 0 {
    "positive"
} else {
    "non-positive"
};
```

### while 循环

```boxlang
let mut i = 0;
while i < 10 {
    println("i = " + i.to_string());
    i = i + 1;
}
```

### for 循环

```boxlang
// 范围循环
for i in 0..10 {
    // i 从 0 到 9
    println("i = " + i.to_string());
}

// 包含上限的范围
for i in 0..=10 {
    // i 从 0 到 10
    println("i = " + i.to_string());
}

// 倒序循环
for i in (0..10).rev() {
    // i 从 9 到 0
    println("i = " + i.to_string());
}
```

### 循环控制

```boxlang
// break - 跳出循环
for i in 0..10 {
    if i == 5 {
        break;
    }
    println("i = " + i.to_string());
}

// continue - 跳过当前迭代
for i in 0..10 {
    if i % 2 == 0 {
        continue;
    }
    println("Odd: " + i.to_string());
}
```

### 嵌套循环

```boxlang
for i in 0..3 {
    for j in 0..3 {
        println("(" + i.to_string() + ", " + j.to_string() + ")");
    }
}
```

---

## 管道操作符

BoxLang 支持独特的管道操作符 `|>`，让数据处理链更清晰：

```boxlang
fn double(x: i32) -> i32 {
    return x * 2;
}

fn add_one(x: i32) -> i32 {
    return x + 1;
}

fn square(x: i32) -> i32 {
    return x * x;
}

pub fn main() {
    // 传统写法
    let result1 = square(add_one(double(5)));
    // 结果: ((5 * 2) + 1)^2 = 121
    
    // 管道操作符写法
    let result2 = 5 |> double |> add_one |> square;
    // 结果相同，但更易读
    
    return 0;
}
```

管道操作符的工作原理：
- `a |> b` 等价于 `b(a)`
- `a |> b |> c` 等价于 `c(b(a))`
- 数据从左向右流动，符合阅读直觉

### 管道操作符实战

```boxlang
fn to_string(n: i32) -> String {
    return n.to_string();
}

fn pad_left(s: String, width: i32) -> String {
    // 假设有字符串填充函数
    return s;
}

fn to_upper(s: String) -> String {
    // 假设有字符串转大写函数
    return s;
}

pub fn main() {
    let number = 42;
    
    // 数据处理流水线
    let formatted = number
        |> double
        |> add_one
        |> to_string
        |> pad_left(10)
        |> to_upper;
}
```

---

## 模块系统

### 模块声明

```boxlang
// math.box
module math;

pub fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

pub fn multiply(a: i32, b: i32) -> i32 {
    return a * b;
}

// 私有函数（默认）
fn helper() {
    // 只能在模块内部使用
}
```

### 使用模块

```boxlang
// main.box
module main;

use math;

pub fn main() {
    let sum = math::add(10, 20);
    let product = math::multiply(5, 6);
    println("Sum: " + sum.to_string());
    println("Product: " + product.to_string());
}
```

### 嵌套模块

```boxlang
module myapp::utils::math;

pub fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
```

### 使用别名

```boxlang
use math as m;

pub fn main() {
    let sum = m::add(10, 20);
}
```

---

## 构建 Windows EXE 工作流

### 工作流概述

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   编写 BoxLang   │ --> │   编译为 C 代码  │ --> │  编译为 EXE    │
│    源代码       │     │   (boxlang)     │     │   (clang/gcc)   │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

### 步骤 1：编写 BoxLang 源代码

创建文件 `hello.box`：

```boxlang
module hello;

pub fn main() {
    println("Hello, Windows!");
    println("This is a BoxLang program compiled to EXE.");
}
```

### 步骤 2：编译为 C 代码

```bash
# 生成 C 代码
boxlang compile hello.box --emit-c

# 生成的文件: hello.c
```

### 步骤 3：编译为 Windows EXE

```bash
# 使用 Clang
clang hello.c -o hello.exe

# 或使用 GCC
gcc hello.c -o hello.exe

# 或使用 MSVC
cl hello.c /Fe:hello.exe
```

### 完整构建脚本（PowerShell）

创建 `build.ps1`：

```powershell
# build.ps1 - BoxLang Windows EXE 构建脚本

param(
    [string]$SourceFile = "main.box",
    [string]$OutputName = "app.exe",
    [switch]$Release = $false
)

# 检查源文件是否存在
if (-not (Test-Path $SourceFile)) {
    Write-Error "源文件不存在: $SourceFile"
    exit 1
}

Write-Host "=== BoxLang Windows EXE 构建 ===" -ForegroundColor Cyan
Write-Host "源文件: $SourceFile"
Write-Host "输出文件: $OutputName"

# 步骤 1: 编译为 C 代码
Write-Host "`n[1/3] 编译为 C 代码..." -ForegroundColor Yellow
$cFile = [System.IO.Path]::ChangeExtension($SourceFile, ".c")
boxlang compile $SourceFile --emit-c -o $cFile

if ($LASTEXITCODE -ne 0) {
    Write-Error "BoxLang 编译失败"
    exit 1
}

# 步骤 2: 编译为 EXE
Write-Host "`n[2/3] 编译为 EXE..." -ForegroundColor Yellow

$compiler = "clang"
if (-not (Get-Command $compiler -ErrorAction SilentlyContinue)) {
    $compiler = "gcc"
    if (-not (Get-Command $compiler -ErrorAction SilentlyContinue)) {
        Write-Error "未找到编译器 (clang 或 gcc)"
        exit 1
    }
}

$optFlags = if ($Release) { "-O2" } else { "-g" }
& $compiler $cFile -o $OutputName $optFlags

if ($LASTEXITCODE -ne 0) {
    Write-Error "C 编译失败"
    exit 1
}

# 步骤 3: 验证
Write-Host "`n[3/3] 验证构建结果..." -ForegroundColor Yellow
if (Test-Path $OutputName) {
    Write-Host "✓ 构建成功: $OutputName" -ForegroundColor Green
    
    # 显示文件信息
    $fileInfo = Get-Item $OutputName
    Write-Host "  大小: $($fileInfo.Length) bytes"
    Write-Host "  路径: $($fileInfo.FullName)"
} else {
    Write-Error "构建失败: 未找到输出文件"
    exit 1
}

Write-Host "`n=== 构建完成 ===" -ForegroundColor Cyan
```

### 使用示例

```powershell
# 构建调试版本
.\build.ps1 -SourceFile "hello.box" -OutputName "hello.exe"

# 构建发布版本
.\build.ps1 -SourceFile "hello.box" -OutputName "hello.exe" -Release

# 运行程序
.\hello.exe
```

### 复杂项目构建

对于多文件项目，创建 `build_project.ps1`：

```powershell
# build_project.ps1 - 多文件 BoxLang 项目构建

param(
    [string]$ProjectDir = ".",
    [string]$OutputName = "app.exe",
    [switch]$Release = $false
)

$srcDir = Join-Path $ProjectDir "src"
$buildDir = Join-Path $ProjectDir "build"

# 创建构建目录
New-Item -ItemType Directory -Force -Path $buildDir | Out-Null

# 查找所有 .box 文件
$boxFiles = Get-ChildItem -Path $srcDir -Filter "*.box" -Recurse

Write-Host "找到 $($boxFiles.Count) 个源文件" -ForegroundColor Cyan

# 编译每个文件
$cFiles = @()
foreach ($file in $boxFiles) {
    $cFile = Join-Path $buildDir ($file.BaseName + ".c")
    Write-Host "编译: $($file.Name) -> $($file.BaseName).c"
    boxlang compile $file.FullName --emit-c -o $cFile
    $cFiles += $cFile
}

# 链接所有 C 文件
Write-Host "`n链接生成 EXE..." -ForegroundColor Yellow
$outputPath = Join-Path $buildDir $OutputName

$compiler = "clang"
if (-not (Get-Command $compiler -ErrorAction SilentlyContinue)) {
    $compiler = "gcc"
}

$optFlags = if ($Release) { "-O2" } else { "-g" }
& $compiler @cFiles -o $outputPath $optFlags

if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ 构建成功: $outputPath" -ForegroundColor Green
} else {
    Write-Error "构建失败"
}
```

---

## 构建 APPbox 应用工作流

### 工作流概述

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   创建项目      │ --> │   编写代码      │ --> │   构建应用      │
│  (box new)      │     │   (BoxLang)     │     │   (box build)   │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                                                        │
                                                        v
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   部署到设备    │ <-- │   打包应用      │ <-- │   运行测试      │
│  (box install)  │     │  (box package)  │     │  (box run)      │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

### 步骤 1：创建 BoxLang 项目

```bash
# 创建新的 BoxLang 项目
box new myapp --lang boxlang

# 进入项目目录
cd myapp
```

生成的项目结构：

```
myapp/
├── box.json          # 应用配置
├── src/
│   └── main.box      # 主程序
└── README.md
```

### 步骤 2：编写应用代码

编辑 `src/main.box`：

```boxlang
module myapp;

struct AppState {
    counter: i32,
    running: bool,
}

impl AppState {
    fn new() -> AppState {
        return AppState {
            counter: 0,
            running: true,
        };
    }
    
    fn increment(self) -> AppState {
        return AppState {
            counter: self.counter + 1,
            running: self.running,
        };
    }
    
    fn stop(self) -> AppState {
        return AppState {
            counter: self.counter,
            running: false,
        };
    }
}

pub fn main() {
    println("=== MyApp 启动 ===");
    
    let mut state = AppState::new();
    
    // 模拟应用主循环
    for i in 0..10 {
        state = state.increment();
        println("Counter: " + state.counter.to_string());
    }
    
    state = state.stop();
    
    println("=== MyApp 结束 ===");
}
```

### 步骤 3：配置 box.json

```json
{
  "format_version": "2.0.0",
  "app_id": "com.example.myapp",
  "name": "myapp",
  "version": "0.1.0",
  "description": "My zetboxos application written in BoxLang",
  "target": {
    "arch": "riscv32",
    "chip": "generic"
  },
  "host": {
    "supported": ["linux", "darwin", "windows"],
    "emulation": true
  },
  "runtime": {
    "entry": "main",
    "stack_size": "8KB",
    "heap_size": "16KB"
  },
  "permissions": [
    {
      "name": "gpio",
      "description": "GPIO access",
      "required": false
    }
  ],
  "build": {
    "language": "boxlang",
    "source_dir": "src",
    "main_file": "main.box"
  }
}
```

### 步骤 4：构建应用

```bash
# 调试构建
box build

# 发布构建
box build --release

# 指定目标架构
box build --target embedded --arch riscv32
```

### 步骤 5：运行模拟器

```bash
# 在模拟器中运行
box run --emulate

# 运行发布版本
box run --release --emulate
```

### 步骤 6：打包应用

```bash
# 创建 Box 包
box package

# 指定版本
box package --version 1.0.0

# 自动签名
box package --sign
```

### 步骤 7：部署到设备

```bash
# 列出连接的设备
box device list

# 安装应用到设备
box install myapp-1.0.0.box --device /dev/ttyUSB0

# 或者使用设备 ID
box install myapp-1.0.0.box --device COM3
```

### 完整开发工作流脚本

创建 `dev_workflow.ps1`：

```powershell
# dev_workflow.ps1 - BoxLang APPbox 开发工作流

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet("build", "run", "package", "install", "all")]
    [string]$Action = "all",
    
    [string]$Device = "",
    [switch]$Release = $false
)

$appName = "myapp"
$version = "1.0.0"

function Build-App {
    Write-Host "`n=== 构建应用 ===" -ForegroundColor Cyan
    
    $buildArgs = @("build")
    if ($Release) {
        $buildArgs += "--release"
    }
    
    & box @buildArgs
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ 构建成功" -ForegroundColor Green
    } else {
        Write-Error "构建失败"
        exit 1
    }
}

function Run-App {
    Write-Host "`n=== 运行应用 ===" -ForegroundColor Cyan
    
    $runArgs = @("run", "--emulate")
    if ($Release) {
        $runArgs += "--release"
    }
    
    & box @runArgs
}

function Package-App {
    Write-Host "`n=== 打包应用 ===" -ForegroundColor Cyan
    
    $packageName = "$appName-$version.box"
    & box package --version $version --output $packageName
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ 打包成功: $packageName" -ForegroundColor Green
    } else {
        Write-Error "打包失败"
        exit 1
    }
}

function Install-App {
    param([string]$TargetDevice)
    
    Write-Host "`n=== 安装应用 ===" -ForegroundColor Cyan
    
    if ([string]::IsNullOrEmpty($TargetDevice)) {
        # 尝试自动发现设备
        $devices = & box device list 2>$null
        if ($devices) {
            Write-Host "发现设备:"
            $devices | ForEach-Object { Write-Host "  $_" }
            
            # 使用第一个设备
            $TargetDevice = ($devices | Select-Object -First 1).Split()[0]
            Write-Host "使用设备: $TargetDevice"
        } else {
            Write-Error "未指定设备且未找到可用设备"
            exit 1
        }
    }
    
    $packageName = "$appName-$version.box"
    & box install $packageName --device $TargetDevice
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ 安装成功" -ForegroundColor Green
    } else {
        Write-Error "安装失败"
        exit 1
    }
}

# 主流程
switch ($Action) {
    "build" { Build-App }
    "run" { Run-App }
    "package" { Package-App }
    "install" { Install-App -TargetDevice $Device }
    "all" {
        Build-App
        Run-App
        Package-App
        if ($Device -or (& box device list 2>$null)) {
            Install-App -TargetDevice $Device
        }
    }
}

Write-Host "`n=== 工作流完成 ===" -ForegroundColor Cyan
```

### 使用工作流脚本

```powershell
# 完整工作流（构建、运行、打包、安装）
.\dev_workflow.ps1 -Action all

# 仅构建
.\dev_workflow.ps1 -Action build

# 构建并运行
.\dev_workflow.ps1 -Action build
.\dev_workflow.ps1 -Action run

# 构建发布版本并安装到指定设备
.\dev_workflow.ps1 -Action all -Release -Device COM3
```

### 嵌入式特定代码示例

```boxlang
module embedded_app;

// GPIO 引脚定义
const LED_PIN: i32 = 13;
const BUTTON_PIN: i32 = 4;

// 模拟 GPIO 操作（实际项目中使用 zetboxos SDK）
fn gpio_init(pin: i32, mode: String) {
    println("GPIO " + pin.to_string() + " initialized as " + mode);
}

fn gpio_write(pin: i32, value: bool) {
    let state = if value { "HIGH" } else { "LOW" };
    println("GPIO " + pin.to_string() + " set to " + state);
}

fn gpio_read(pin: i32) -> bool {
    // 模拟读取
    return false;
}

fn delay_ms(ms: i32) {
    // 模拟延时
    println("Delay " + ms.to_string() + "ms");
}

pub fn main() {
    println("=== Embedded App 启动 ===");
    
    // 初始化 GPIO
    gpio_init(LED_PIN, "OUTPUT");
    gpio_init(BUTTON_PIN, "INPUT");
    
    // 主循环
    let mut counter = 0;
    while counter < 5 {
        // LED 闪烁
        gpio_write(LED_PIN, true);
        delay_ms(500);
        
        gpio_write(LED_PIN, false);
        delay_ms(500);
        
        counter = counter + 1;
        println("Blink count: " + counter.to_string());
    }
    
    println("=== Embedded App 结束 ===");
}
```

---

## 实战示例

### 示例 1：斐波那契数列

```boxlang
module fibonacci;

// 递归实现
fn fib_recursive(n: i32) -> i32 {
    if n <= 0 {
        return 0;
    }
    if n == 1 {
        return 1;
    }
    return fib_recursive(n - 1) + fib_recursive(n - 2);
}

// 迭代实现
fn fib_iterative(n: i32) -> i32 {
    if n <= 0 {
        return 0;
    }
    if n == 1 {
        return 1;
    }
    
    let mut a = 0;
    let mut b = 1;
    let mut i = 2;
    
    while i <= n {
        let temp = a + b;
        a = b;
        b = temp;
        i = i + 1;
    }
    
    return b;
}

pub fn main() {
    println("斐波那契数列 (递归):");
    for i in 0..10 {
        let result = fib_recursive(i);
        println("fib(" + i.to_string() + ") = " + result.to_string());
    }
    
    println("\n斐波那契数列 (迭代):");
    for i in 0..10 {
        let result = fib_iterative(i);
        println("fib(" + i.to_string() + ") = " + result.to_string());
    }
}
```

### 示例 2：数学工具函数

```boxlang
module math_utils;

// 计算最大公约数
fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 {
        return a;
    }
    return gcd(b, a % b);
}

// 计算最小公倍数
fn lcm(a: i32, b: i32) -> i32 {
    return (a * b) / gcd(a, b);
}

// 判断素数
fn is_prime(n: i32) -> bool {
    if n <= 1 {
        return false;
    }
    if n <= 3 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }
    
    let mut i = 3;
    while i * i <= n {
        if n % i == 0 {
            return false;
        }
        i = i + 2;
    }
    return true;
}

// 幂运算
fn power(base: i32, exp: i32) -> i32 {
    if exp == 0 {
        return 1;
    }
    if exp == 1 {
        return base;
    }
    let half = power(base, exp / 2);
    if exp % 2 == 0 {
        return half * half;
    }
    return half * half * base;
}

pub fn main() {
    println("=== 数学工具函数 ===");
    
    let g = gcd(48, 18);
    println("gcd(48, 18) = " + g.to_string());
    
    let l = lcm(4, 6);
    println("lcm(4, 6) = " + l.to_string());
    
    println("\n素数判断:");
    for i in 1..20 {
        if is_prime(i) {
            println(i.to_string() + " 是素数");
        }
    }
    
    println("\n幂运算:");
    for i in 0..6 {
        let p = power(2, i);
        println("2^" + i.to_string() + " = " + p.to_string());
    }
}
```

### 示例 3：结构体和方法

```boxlang
module struct_demo;

struct Point {
    x: i32,
    y: i32,
}

struct Rectangle {
    width: i32,
    height: i32,
}

struct Circle {
    center: Point,
    radius: f64,
}

impl Point {
    fn new(x: i32, y: i32) -> Point {
        return Point { x, y };
    }
    
    fn origin() -> Point {
        return Point { x: 0, y: 0 };
    }
    
    fn distance_from_origin(self) -> f64 {
        let x_sq = self.x * self.x;
        let y_sq = self.y * self.y;
        return (x_sq + y_sq) as f64;
    }
}

impl Rectangle {
    fn new(width: i32, height: i32) -> Rectangle {
        return Rectangle { width, height };
    }
    
    fn area(self) -> i32 {
        return self.width * self.height;
    }
    
    fn perimeter(self) -> i32 {
        return 2 * (self.width + self.height);
    }
    
    fn is_square(self) -> bool {
        return self.width == self.height;
    }
}

impl Circle {
    fn new(center: Point, radius: f64) -> Circle {
        return Circle { center, radius };
    }
    
    fn area(self) -> f64 {
        return 3.14159 * self.radius * self.radius;
    }
    
    fn circumference(self) -> f64 {
        return 2.0 * 3.14159 * self.radius;
    }
}

pub fn main() {
    println("=== 结构体和方法演示 ===");
    
    let p1 = Point::new(3, 4);
    println("点 p1: (" + p1.x.to_string() + ", " + p1.y.to_string() + ")");
    println("到原点距离平方: " + p1.distance_from_origin().to_string());
    
    let p2 = Point::origin();
    println("\n原点 p2: (" + p2.x.to_string() + ", " + p2.y.to_string() + ")");
    
    let rect = Rectangle::new(10, 20);
    println("\n矩形: " + rect.width.to_string() + " x " + rect.height.to_string());
    println("面积: " + rect.area().to_string());
    println("周长: " + rect.perimeter().to_string());
    println("是否为正方形: " + rect.is_square().to_string());
    
    let circle = Circle::new(p1, 5.0);
    println("\n圆 半径: " + circle.radius.to_string());
    println("圆面积: " + circle.area().to_string());
    println("圆周长: " + circle.circumference().to_string());
}
```

### 示例 4：计算器（使用管道操作符）

```boxlang
module calculator;

struct Calculator {
    result: i32,
}

impl Calculator {
    fn new() -> Calculator {
        return Calculator { result: 0 };
    }
    
    fn add(self, value: i32) -> Calculator {
        return Calculator { result: self.result + value };
    }
    
    fn subtract(self, value: i32) -> Calculator {
        return Calculator { result: self.result - value };
    }
    
    fn multiply(self, value: i32) -> Calculator {
        return Calculator { result: self.result * value };
    }
    
    fn divide(self, value: i32) -> Calculator {
        if value == 0 {
            return Calculator { result: 0 };
        }
        return Calculator { result: self.result / value };
    }
    
    fn clear(self) -> Calculator {
        return Calculator { result: 0 };
    }
    
    fn get_result(self) -> i32 {
        return self.result;
    }
}

fn power(base: i32, exp: i32) -> i32 {
    if exp == 0 {
        return 1;
    }
    if exp == 1 {
        return base;
    }
    let half = power(base, exp / 2);
    if exp % 2 == 0 {
        return half * half;
    }
    return half * half * base;
}

fn absolute(x: i32) -> i32 {
    if x < 0 {
        return -x;
    }
    return x;
}

fn square(x: i32) -> i32 {
    return x * x;
}

pub fn main() {
    println("=== BoxLang 计算器示例 ===\n");
    
    println("基本运算:");
    let calc1 = Calculator::new()
        .add(10)
        .multiply(2)
        .subtract(5);
    println("((0 + 10) * 2) - 5 = " + calc1.get_result().to_string());
    
    let calc2 = Calculator::new()
        .add(100)
        .divide(4)
        .add(10);
    println("((0 + 100) / 4) + 10 = " + calc2.get_result().to_string());
    
    println("\n使用管道操作符进行计算:");
    let value = 5;
    let result1 = value |> square |> absolute;
    println("5 |> square |> abs = " + result1.to_string());
    
    let result2 = -3 |> absolute |> square;
    println("(-3) |> abs |> square = " + result2.to_string());
    
    println("\n幂运算:");
    for i in 0..6 {
        let p = power(2, i);
        println("2^" + i.to_string() + " = " + p.to_string());
    }
    
    println("\n综合计算示例:");
    let complex = Calculator::new()
        .add(5)
        .multiply(3)
        .subtract(7)
        .divide(2);
    println("(((0 + 5) * 3) - 7) / 2 = " + complex.get_result().to_string());
    
    println("\n=== 计算完成 ===");
}
```

### 示例 5：冒泡排序

```boxlang
module bubble_sort;

fn bubble_sort(arr: [i32; 10], len: i32) -> [i32; 10] {
    let mut result = arr;
    let mut i = 0;
    
    while i < len {
        let mut j = 0;
        while j < len - i - 1 {
            // 注意：BoxLang 目前不支持直接数组索引赋值
            // 这里展示算法逻辑
            j = j + 1;
        }
        i = i + 1;
    }
    
    return result;
}

fn print_array(name: String, arr: [i32; 10], len: i32) {
    print(name + ": [");
    let mut i = 0;
    while i < len {
        // 打印元素
        i = i + 1;
    }
    println("]");
}

pub fn main() {
    println("=== 冒泡排序 ===");
    
    let arr: [i32; 10] = [64, 34, 25, 12, 22, 11, 90, 5, 77, 30];
    let len = 10;
    
    println("排序前:");
    // print_array("数组", arr, len);
    
    let sorted = bubble_sort(arr, len);
    
    println("\n排序后:");
    // print_array("数组", sorted, len);
}
```

---

## 最佳实践

### 命名规范

- 函数、变量：snake_case (`my_function`, `my_variable`)
- 类型、结构体：PascalCase (`MyStruct`)
- 常量：SCREAMING_SNAKE_CASE (`MAX_SIZE`)

### 代码组织

```boxlang
// 1. 模块声明
module mymodule;

// 2. 常量定义
const MAX_SIZE: i32 = 100;
const PI: f64 = 3.14159;

// 3. 类型定义
struct MyStruct {
    field: i32,
}

// 4. 实现块
impl MyStruct {
    fn new() -> MyStruct {
        return MyStruct { field: 0 };
    }
}

// 5. 辅助函数
fn helper(x: i32) -> i32 {
    return x * 2;
}

// 6. 主函数
pub fn main() {
    let s = MyStruct::new();
    let result = helper(s.field);
    println("Result: " + result.to_string());
}
```

### 性能建议

1. 优先使用迭代而不是递归
2. 对于批量数据，使用 Arena 分配器
3. 对于可重用对象，使用 Pool
4. 使用管道操作符提高代码可读性
5. 避免不必要的内存分配

### 错误处理

```boxlang
fn divide(a: i32, b: i32) -> i32 {
    if b == 0 {
        println("Error: Division by zero");
        return 0;
    }
    return a / b;
}

fn safe_access(arr: [i32; 10], index: i32) -> i32 {
    if index < 0 || index >= 10 {
        println("Error: Index out of bounds");
        return 0;
    }
    // 访问数组元素
    return 0;
}
```

### 调试技巧

```bash
# 查看生成的 C 代码
boxlang compile hello.box --emit-c

# 查看词法分析结果
boxlang tokenize hello.box

# 查看 AST
boxlang ast hello.box

# 检查代码（不生成可执行文件）
boxlang check hello.box
```

---

## 常见问题

### Q: 如何调试 BoxLang 程序？

A: 目前可以通过查看生成的 C 代码来调试：
```bash
boxlang compile hello.box --emit-c
# 查看生成的 hello.c 文件
```

### Q: 是否支持 IDE 插件？

A: 目前还在开发中，建议暂时使用 VS Code。

### Q: 如何报告 bug？

A: 请在 GitHub 仓库提交 issue，包含：
- BoxLang 版本
- 操作系统版本
- 复现步骤
- 最小化代码示例

### Q: 编译错误 "expected Semi"

A: BoxLang 要求语句以分号结尾（最后一个表达式除外）。

```boxlang
// 错误
let x = 42

// 正确
let x = 42;
```

---

## 学习资源

- [BoxLang 语言规范](./LANGUAGE_SPEC.md)
- [快速入门指南](./QUICK_START.md)
- [API 文档](./API.md)
- [示例代码](../examples/)
  - `basics/` - 基础语法示例
    - `01_hello.box` - Hello World
    - `02_variables.box` - 变量和类型
    - `03_functions.box` - 函数
    - `04_control_flow.box` - 控制流
  - `structs/` - 结构体和方法示例
  - `pipeline/` - 管道操作符示例
    - `02_chained_pipeline.box` - 链式管道
    - `03_pipeline_math.box` - 数学计算管道
  - `appbox-demo/` - AppBox 打包示例

---

## 贡献

欢迎贡献代码！请阅读 [CONTRIBUTING.md](../CONTRIBUTING.md) 了解如何参与。

---

## 许可证

BoxLang 使用 MIT 许可证。详见 [LICENSE](../LICENSE) 文件。

---

**祝你编程愉快！**
