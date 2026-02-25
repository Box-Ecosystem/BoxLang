# BoxLang 快速入门指南

本指南将帮助你在 10 分钟内快速上手 BoxLang 编程语言。

## 目录

- [安装](#安装)
- [第一个程序](#第一个程序)
- [基础概念](#基础概念)
- [下一步](#下一步)

---

## 安装

### 1. 环境准备

确保你已经安装了：
- Windows 10/11
- Rust 工具链 ([rustup.rs](https://rustup.rs))
- LLVM/Clang

### 2. 安装 LLVM/Clang

```powershell
winget install LLVM.LLVM
```

### 3. 编译 BoxLang 编译器

```bash
git clone https://github.com/your-org/boxlang.git
cd boxlang/compiler
cargo build --release
```

### 4. 添加到 PATH

将 `boxlang/compiler/target/release` 目录添加到你的系统 PATH 中。

验证安装：
```bash
boxlang --version
```

---

## 第一个程序

### 创建 Hello World

创建文件 `hello.box`：

```boxlang
module hello;

pub fn main() {
    println("Hello, World!");
}
```

### 编译和运行

```bash
# 编译
boxlang compile hello.box -o hello.exe

# 运行
./hello.exe
```

输出：
```
Hello, World!
```

---

## 基础概念

### 1. 变量

```boxlang
// 不可变变量
let x = 42;

// 可变变量
let mut y = 10;
y = 20;

// 类型注解
let z: f64 = 3.14;
```

### 2. 函数

```boxlang
// 基本函数
fn greet(name: String) {
    println("Hello, " + name + "!");
}

// 带返回值的函数
fn add(a: i32, b: i32) -> i32 {
    a + b  // 最后一个表达式作为返回值
}

pub fn main() {
    greet("BoxLang");
    let sum = add(10, 20);
    println("Sum: " + sum.to_string());
}
```

### 3. 结构体

```boxlang
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    // 构造函数
    fn new(x: i32, y: i32) -> Point {
        Point { x, y }
    }
    
    // 方法
    fn distance_from_origin(self) -> f64 {
        let x = self.x as f64;
        let y = self.y as f64;
        (x * x + y * y).sqrt()
    }
}

pub fn main() {
    let p = Point::new(3, 4);
    println("Distance: " + p.distance_from_origin().to_string());
}
```

### 4. 控制流

```boxlang
pub fn main() {
    // if 表达式
    let x = 10;
    if x > 0 {
        println("Positive");
    } else {
        println("Non-positive");
    }
    
    // while 循环
    let mut i = 0;
    while i < 5 {
        println("i = " + i.to_string());
        i = i + 1;
    }
    
    // for 循环
    for j in 0..5 {
        println("j = " + j.to_string());
    }
}
```

### 5. 模块

**math.box**:
```boxlang
module math;

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
```

**main.box**:
```boxlang
module main;

use math;

pub fn main() {
    let sum = math::add(10, 20);
    let product = math::multiply(5, 6);
    println("Sum: " + sum.to_string());
    println("Product: " + product.to_string());
}
```

---

## 完整示例

### 计算器程序

```boxlang
module calculator;

struct Calculator {
    result: f64,
}

impl Calculator {
    fn new() -> Calculator {
        Calculator { result: 0.0 }
    }
    
    fn add(&mut self, value: f64) -> &mut Calculator {
        self.result = self.result + value;
        self
    }
    
    fn subtract(&mut self, value: f64) -> &mut Calculator {
        self.result = self.result - value;
        self
    }
    
    fn multiply(&mut self, value: f64) -> &mut Calculator {
        self.result = self.result * value;
        self
    }
    
    fn divide(&mut self, value: f64) -> &mut Calculator {
        if value != 0.0 {
            self.result = self.result / value;
        }
        self
    }
    
    fn get_result(self) -> f64 {
        self.result
    }
    
    fn clear(&mut self) -> &mut Calculator {
        self.result = 0.0;
        self
    }
}

pub fn main() {
    let mut calc = Calculator::new();
    
    // 链式调用
    let result = calc
        .add(10.0)
        .multiply(2.0)
        .subtract(5.0)
        .divide(3.0)
        .get_result();
    
    println("Result: " + result.to_string());
    
    // 重新开始
    calc.clear().add(100.0);
    println("New result: " + calc.get_result().to_string());
}
```

编译和运行：
```bash
boxlang compile calculator.box -o calculator.exe
./calculator.exe
```

---

## 常用命令

```bash
# 编译
boxlang compile file.box -o output.exe

# 运行（编译并执行）
boxlang run file.box

# 查看词法分析结果
boxlang tokenize file.box

# 查看 AST
boxlang ast file.box

# 生成 C 代码
boxlang compile file.box --emit-c

# 检查代码（不生成可执行文件）
boxlang check file.box
```

---

## 故障排除

### 问题：找不到 clang

**解决方案**：
1. 确保 LLVM 已正确安装
2. 将 LLVM 的 bin 目录添加到 PATH
3. 重启终端

### 问题：编译错误 "expected Semi"

**原因**：BoxLang 要求语句以分号结尾（最后一个表达式除外）。

**解决方案**：
```boxlang
// 错误
let x = 42

// 正确
let x = 42;
```

### 问题：链接错误

**解决方案**：确保已安装 Visual Studio 的 C++ 工具链或 MinGW。

---

## 下一步

- 阅读完整的 [BoxLang 教程](./TUTORIAL.md)
- 查看 [语言规范](./LANGUAGE_SPEC.md)
- 探索 [示例代码](../examples/)
- 加入社区讨论

---

## 获取帮助

- GitHub Issues: [报告问题](https://github.com/your-org/boxlang/issues)
- 文档: [完整文档](./README.md)
- 示例: [示例代码](../examples/)

---

**开始你的 BoxLang 之旅吧！**
