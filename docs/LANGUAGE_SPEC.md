# BoxLang 语言规范

## 概述

BoxLang 是一种系统级编程语言，专为 Box Ecosystem 设计。它融合了 Rust、Go、Zig 等现代语言的优点，专注于 Windows 平台开发和嵌入式系统编程。

## 设计哲学

1. **安全性优先**: 内存安全，避免空指针和缓冲区溢出
2. **性能至上**: 零成本抽象，AOT 编译，接近 C 的性能
3. **简洁易学**: 比 Rust 更简洁的语法，降低学习曲线
4. **现代特性**: 支持泛型、模式匹配、async/await 等现代特性
5. **跨平台**: 一套代码，Host + Target 双模式

## 词法结构

### 关键字

```
module, import, pub, fn, let, mut, const, static
struct, enum, impl, trait, for, while, loop
if, else, match, return, break, continue
async, await, spawn, defer, unsafe, extern
as, in, ref, self, Self, true, false, void, null
use, mod, type, where, move, box
```

### 类型关键字

```
i8, i16, i32, i64, u8, u16, u32, u64
f32, f64, bool, char, str, String
Vec, Option, Result
```

### 标识符

- 以字母或下划线开头
- 后续字符可以是字母、数字或下划线
- 区分大小写

### 注释

```boxlang
// 单行注释

/*
 * 多行注释
 */

/// 文档注释（用于函数、结构体等）
```

## 类型系统

### 基本类型

| 类型 | 描述 | 大小 |
|------|------|------|
| `bool` | 布尔值 | 1 byte |
| `i8` | 有符号 8 位整数 | 1 byte |
| `i16` | 有符号 16 位整数 | 2 bytes |
| `i32` | 有符号 32 位整数 | 4 bytes |
| `i64` | 有符号 64 位整数 | 8 bytes |
| `u8` | 无符号 8 位整数 | 1 byte |
| `u16` | 无符号 16 位整数 | 2 bytes |
| `u32` | 无符号 32 位整数 | 4 bytes |
| `u64` | 无符号 64 位整数 | 8 bytes |
| `f32` | 32 位浮点数 | 4 bytes |
| `f64` | 64 位浮点数 | 8 bytes |
| `char` | Unicode 字符 | 4 bytes |
| `str` | 字符串切片 | - |

### 复合类型

#### 数组

```boxlang
let arr: [i32; 5] = [1, 2, 3, 4, 5];
let zeros = [0; 10]; // 10 个 0
```

#### 切片

```boxlang
let slice: &[i32] = &arr[0..3]; // 引用数组的一部分
```

#### 元组

```boxlang
let tuple: (i32, f64, bool) = (1, 2.0, true);
let first = tuple.0; // 访问第一个元素
```

#### 结构体

```boxlang
struct Point {
    x: f64,
    y: f64,
}

let p = Point { x: 1.0, y: 2.0 };
```

#### 枚举

```boxlang
enum Option<T> {
    Some(T),
    None,
}

enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

### 引用和指针

```boxlang
let ref1: &i32 = &x;          // 不可变引用
let ref2: &mut i32 = &mut y;  // 可变引用
let ptr: *mut u8 = ...;       // 裸指针
```

### 函数类型

```boxlang
let f: fn(i32, i32) -> i32 = add;
```

## 表达式

### 字面量

```boxlang
let integer = 42;           // 整数
let hex = 0xFF;             // 十六进制
let binary = 0b1010;        // 二进制
let float = 3.14;           // 浮点数
let string = "hello";       // 字符串
let character = 'a';        // 字符
let boolean = true;         // 布尔值
```

### 运算符

#### 算术运算符

| 运算符 | 描述 |
|--------|------|
| `+` | 加法 |
| `-` | 减法 |
| `*` | 乘法 |
| `/` | 除法 |
| `%` | 取模 |

#### 比较运算符

| 运算符 | 描述 |
|--------|------|
| `==` | 等于 |
| `!=` | 不等于 |
| `<` | 小于 |
| `<=` | 小于等于 |
| `>` | 大于 |
| `>=` | 大于等于 |

#### 逻辑运算符

| 运算符 | 描述 |
|--------|------|
| `&&` | 逻辑与 |
| `||` | 逻辑或 |
| `!` | 逻辑非 |

#### 位运算符

| 运算符 | 描述 |
|--------|------|
| `&` | 按位与 |
| `|` | 按位或 |
| `^` | 按位异或 |
| `<<` | 左移 |
| `>>` | 右移 |
| `~` | 按位取反 |

#### 赋值运算符

| 运算符 | 描述 |
|--------|------|
| `=` | 赋值 |
| `+=` | 加并赋值 |
| `-=` | 减并赋值 |
| `*=` | 乘并赋值 |
| `/=` | 除并赋值 |
| `%=` | 取模并赋值 |
| `&=` | 按位与并赋值 |
| `|=` | 按位或并赋值 |
| `^=` | 按位异或并赋值 |
| `<<=` | 左移并赋值 |
| `>>=` | 右移并赋值 |

### 控制流表达式

#### if 表达式

```boxlang
let result = if x > 0 {
    "positive"
} else if x < 0 {
    "negative"
} else {
    "zero"
};
```

#### match 表达式

```boxlang
match value {
    0 => println("zero"),
    1..=9 => println("single digit"),
    10 | 20 | 30 => println("ten, twenty, or thirty"),
    _ => println("other"),
}
```

#### 循环表达式

```boxlang
// while 循环
while i < 10 {
    i += 1;
}

// for 循环
for i in 0..10 {
    println(i);
}

// loop 循环（无限循环）
loop {
    // 无限循环
    if condition {
        break;
    }
}
```

## 语句

### let 语句

```boxlang
let x = 10;           // 不可变绑定
let mut y = 20;       // 可变绑定
let z: i32 = 30;      // 显式类型注解
```

### 表达式语句

```boxlang
foo();        // 函数调用语句
x + 1;        // 表达式语句
```

### return 语句

```boxlang
return;       // 返回 ()
return 42;    // 返回值
```

## 函数

### 函数定义

```boxlang
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// 简写形式（省略 return）
fn add(a: i32, b: i32) -> i32 {
    a + b
}

// 无返回值
fn print_hello() {
    println("Hello");
}
```

### 参数

```boxlang
// 不可变参数
fn foo(x: i32) {}

// 可变参数
fn bar(mut x: i32) {
    x += 1;
}

// 引用参数
fn baz(x: &i32) {}
fn qux(x: &mut i32) {}
```

### 泛型函数

```boxlang
fn identity<T>(value: T) -> T {
    value
}

fn swap<T>(a: &mut T, b: &mut T) {
    let temp = *a;
    *a = *b;
    *b = temp;
}
```

## 模块系统

### 模块声明

```boxlang
module mymodule;
```

### 导入

```boxlang
import std::io;
import std::io::println as print;
import std::io::*;  // 导入所有
```

### 可见性

```boxlang
pub fn public_function() {}     // 公开
fn private_function() {}         // 私有（默认）

pub struct PublicStruct {}       // 公开结构体
struct PrivateStruct {}          // 私有结构体

pub enum PublicEnum {}           // 公开枚举
```

## 实现块

### impl 块

```boxlang
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    // 构造函数
    pub fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }
    
    // 方法
    pub fn distance(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
    
    // 可变方法
    pub fn translate(&mut self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
    }
    
    // 关联函数
    pub fn origin() -> Point {
        Point { x: 0.0, y: 0.0 }
    }
}
```

## Trait（特性）

### 定义 Trait

```boxlang
trait Drawable {
    fn draw(&self);
    fn bounds(&self) -> Rectangle;
}

// 带默认实现的 Trait
trait Printable {
    fn print(&self);
    
    fn println(&self) {
        self.print();
        println("");
    }
}
```

### 实现 Trait

```boxlang
impl Drawable for Circle {
    fn draw(&self) {
        // 绘制圆形
    }
    
    fn bounds(&self) -> Rectangle {
        // 返回边界矩形
    }
}
```

### Trait 约束

```boxlang
fn print_all<T: Printable>(items: &[T]) {
    for item in items {
        item.print();
    }
}

// 多个约束
fn process<T: Drawable + Printable>(item: T) {
    item.draw();
    item.print();
}
```

## 错误处理

### Result 类型

```boxlang
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

### 使用 Result

```boxlang
fn may_fail() -> Result<i32, Error> {
    if some_condition {
        Ok(42)
    } else {
        Err(Error::new("something went wrong"))
    }
}

// ? 运算符
fn caller() -> Result<i32, Error> {
    let value = may_fail()?;
    let doubled = value * 2;
    Ok(doubled)
}

// match 处理
match may_fail() {
    Ok(value) => println(value),
    Err(e) => println("Error: {}", e),
}
```

### Option 类型

```boxlang
enum Option<T> {
    Some(T),
    None,
}

fn find_item(items: &[i32], target: i32) -> Option<usize> {
    for (i, item) in items.iter().enumerate() {
        if *item == target {
            return Some(i);
        }
    }
    None
}
```

## 异步编程

### async/await

```boxlang
async fn fetch_data() -> Result<Data, Error> {
    let response = await http::get("https://api.example.com");
    response.json()
}

async fn process() {
    match await fetch_data() {
        Ok(data) => println("Got data: {}", data),
        Err(e) => println("Error: {}", e),
    }
}
```

### 并发任务

```boxlang
pub fn main() {
    let task1 = async fetch_data();
    let task2 = async fetch_data();
    
    let (result1, result2) = await (task1, task2);
}
```

### 通道

```boxlang
let (sender, receiver) = chan::new::<i32>(10);

spawn {
    sender.send(42);
}

let value = receiver.recv();
```

## 不安全代码

### unsafe 块

```boxlang
unsafe {
    // 不安全操作
    let ptr = malloc(1024);
    *ptr = 42;
    free(ptr);
}
```

### unsafe 函数

```boxlang
unsafe fn dangerous_function() {
    // 不安全操作
}

// 调用 unsafe 函数
unsafe {
    dangerous_function();
}
```

## FFI（外部函数接口）

### extern 块

```boxlang
extern "C" {
    fn printf(format: *const u8, ...) -> i32;
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
}
```

### 调用 C 函数

```boxlang
unsafe {
    printf("Hello from C\n".as_ptr());
}
```

## 属性

### 常用属性

```boxlang
// 标记为 zetboxos 应用
#[box_app(
    id = "com.example.myapp",
    version = "1.0.0",
    stack_size = 8192,
    heap_size = 16384,
)]
pub fn main() -> Result<void> {
    // 应用代码
}

// 标记为测试
#[test]
fn test_addition() {
    assert_eq!(add(2, 2), 4);
}

// 标记为内联
#[inline]
fn small_function() {}

// 标记为不安全
#[no_mangle]
pub extern "C" fn exported_function() {}
```

## 标准库

### 核心模块

```boxlang
use std::io;           // 输入输出
use std::fs;           // 文件系统
use std::net;          // 网络
use std::thread;       // 线程
use std::sync;         // 同步原语
use std::collections;  // 集合类型
use std::mem;          // 内存操作
use std::ptr;          // 指针操作
```

### zetboxos 模块

```boxlang
use zetboxos::gpio;       // GPIO 控制
use zetboxos::uart;       // UART 通信
use zetboxos::i2c;        // I2C 通信
use zetboxos::spi;        // SPI 通信
use zetboxos::timer;      // 定时器
use zetboxos::power;      // 电源管理
```

## 内存管理

### 所有权

```boxlang
let s1 = String::from("hello");  // s1 拥有字符串
let s2 = s1;                      // 所有权转移到 s2
// s1 不再有效
```

### 借用

```boxlang
let s = String::from("hello");
let len = calculate_length(&s);  // 借用 s
// s 仍然有效

fn calculate_length(s: &String) -> usize {
    s.len()
}
```

### 生命周期

```boxlang
// 显式生命周期
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}
```

## 最佳实践

1. **优先使用不可变绑定**: 使用 `let` 而不是 `let mut`，除非确实需要修改
2. **利用类型推断**: 让编译器推断类型，只在需要时显式注解
3. **处理错误**: 使用 `Result` 和 `Option` 处理可能的错误
4. **避免 unsafe**: 只在必要时使用 `unsafe` 块
5. **文档注释**: 为公共 API 添加文档注释
6. **单元测试**: 为函数编写测试

## 示例程序

### Hello World

```boxlang
module hello;

pub fn main() {
    println("Hello, BoxLang!");
}
```

### 斐波那契数列

```boxlang
module fibonacci;

fn fib(n: i32) -> i32 {
    if n <= 1 {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

pub fn main() {
    for i in 0..10 {
        println(fib(i));
    }
}
```

### 结构体和方法

```boxlang
module shapes;

pub struct Rectangle {
    width: f64,
    height: f64,
}

impl Rectangle {
    pub fn new(width: f64, height: f64) -> Rectangle {
        Rectangle { width, height }
    }
    
    pub fn area(&self) -> f64 {
        self.width * self.height
    }
}

pub fn main() {
    let rect = Rectangle::new(10.0, 20.0);
    println("Area: {}", rect.area());
}
```

---

**注意**: 本规范描述的是 BoxLang 的目标设计。当前实现可能不完整，某些特性仍在开发中。
