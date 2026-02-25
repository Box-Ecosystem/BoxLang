# BoxLang 基础语法

本章节基于 BoxLang 编译器实现，详细介绍语言的核心语法概念。

## 1. 词法元素

### 1.1 关键字

BoxLang 定义了以下关键字（区分大小写）：

**模块与可见性：**
- `module` - 模块声明
- `import` - 导入声明
- `pub` - 公开可见性
- `use` - 使用/导入
- `mod` - 子模块

**函数与类型：**
- `fn` - 函数定义
- `struct` - 结构体
- `enum` - 枚举
- `impl` - 实现块
- `trait` - 特质/接口
- `type` - 类型别名
- `const` - 常量
- `static` - 静态变量

**变量与内存：**
- `let` - 变量声明
- `mut` - 可变修饰符
- `ref` - 引用
- `move` - 移动语义
- `box` - 堆分配

**控制流：**
- `if` / `else` - 条件分支
- `match` - 模式匹配
- `for` - 循环
- `while` - 条件循环
- `loop` - 无限循环
- `return` - 返回
- `break` - 跳出循环
- `continue` - 继续循环

**异步编程：**
- `async` - 异步
- `await` - 等待
- `spawn` - 生成任务

**其他：**
- `self` / `Self` - 自身引用
- `true` / `false` - 布尔值
- `null` - 空值
- `void` - 空类型
- `unsafe` - 不安全代码块
- `extern` - 外部接口
- `defer` - 延迟执行

### 1.2 标识符规则

```boxlang
// 有效的标识符
name
_name
name123
NameWithCaps

// 无效的标识符
123name      // 不能以数字开头
name-with-dash  // 不能包含连字符
name with space // 不能包含空格
```

标识符规则：
- 以字母或下划线开头
- 后续字符可以是字母、数字或下划线
- 区分大小写

### 1.3 注释

```boxlang
// 单行注释

/*
 * 多行注释
 * 可以跨越多行
 */

/// 文档注释（用于函数、结构体等）
pub fn documented_function() {
    //! 内部文档注释
}
```

## 2. 字面量

### 2.1 整数

```boxlang
// 十进制整数
let dec = 42;
let negative = -17;

// 十六进制整数
let hex = 0xFF;        // 255
let hex_large = 0x1A3B; // 6715

// 二进制整数
let bin = 0b1010;      // 10
let bin_flags = 0b11110000;

// 整数类型后缀（可选）
let explicit_i32 = 42i32;
let explicit_u64 = 100u64;
```

### 2.2 浮点数

```boxlang
// 基本浮点数
let pi = 3.14159;
let negative = -2.5;

// 科学计数法
let large = 1.5e10;    // 1.5 × 10^10
let small = 2.5e-3;    // 0.0025

// 类型后缀
let f32_val = 3.14f32;
let f64_val = 3.14159265358979f64;
```

### 2.3 字符串

```boxlang
// 简单字符串
let simple = "Hello, World!";

// 支持转义字符
let escaped = "Line 1\nLine 2\tTabbed";
let quote = "He said \"Hello\"";

// 字符串插值（Interpolated）
let name = "BoxLang";
let version = 1.0;
let message = "Welcome to {name} version {version}!";

// 格式化插值
let pi = 3.14159;
let formatted = "Pi = {pi:.2}";  // Pi = 3.14
```

### 2.4 字符

```boxlang
// 单字符
let ch = 'A';
let digit = '9';

// 转义字符
let newline = '\n';
let tab = '\t';
let backslash = '\\';
let single_quote = '\'';
```

### 2.5 布尔值与空值

```boxlang
let flag = true;
let disabled = false;
let empty = null;
```

## 3. 变量与常量

### 3.1 变量声明

```boxlang
// 不可变变量（默认）
let x = 10;
// x = 20;  // 错误！不可变变量不能重新赋值

// 可变变量
let mut y = 20;
y = 30;  // 正确

// 类型注解（可选）
let count: i32 = 100;
let price: f64 = 19.99;
let name: str = "BoxLang";

// 延迟初始化（需要类型注解）
let value: i32;
value = 42;  // 首次赋值
// value = 100;  // 错误！不可变变量只能赋值一次
```

### 3.2 常量

```boxlang
// 常量必须在编译时确定
const PI: f64 = 3.14159;
const MAX_SIZE: i32 = 100;
const GREETING: str = "Hello";

// 常量表达式
const DOUBLE_MAX: i32 = MAX_SIZE * 2;
```

### 3.3 静态变量

```boxlang
// 不可变静态变量
static VERSION: str = "1.0.0";

// 可变静态变量（需要 unsafe 访问）
static mut COUNTER: i32 = 0;
```

## 4. 数据类型

### 4.1 基本类型

```boxlang
// 有符号整数
let a: i8 = 127;              // 8位: -128 ~ 127
let b: i16 = 32767;           // 16位: -32768 ~ 32767
let c: i32 = 2147483647;      // 32位（默认）
let d: i64 = 9223372036854775807;  // 64位

// 无符号整数
let e: u8 = 255;              // 8位: 0 ~ 255
let f: u16 = 65535;
let g: u32 = 4294967295;
let h: u64 = 18446744073709551615;

// 浮点数
let f32_val: f32 = 3.14;      // 32位单精度
let f64_val: f64 = 3.14159;   // 64位双精度（默认）

// 布尔值
let flag: bool = true;

// 字符（Unicode标量值）
let ch: char = 'A';
let emoji: char = '😀';

// 字符串切片
let s: str = "Hello";

// 空类型
let unit = ();                // 类似于 ()
```

### 4.2 复合类型

```boxlang
// 数组（固定长度，同类型）
let arr = [1, 2, 3, 4, 5];
let first = arr[0];           // 索引访问
let nested = [[1, 2], [3, 4]];

// 指定类型和长度的数组
let typed_arr: [i32; 5] = [1, 2, 3, 4, 5];

// 重复初始化
let zeros = [0; 10];          // 10个0

// 元组（固定长度，可不同类型）
let tuple = (1, "hello", 3.14);
let num = tuple.0;            // 访问第一个元素
let text = tuple.1;           // 访问第二个元素

// 具名元组（结构体元组）
let point = (x: 10, y: 20);
let x_coord = point.x;
```

### 4.3 引用与指针

```boxlang
// 不可变引用
let x = 10;
let r = &x;                   // r 是 &i32

// 可变引用
let mut y = 20;
let r_mut = &mut y;           // r_mut 是 &mut i32
*r_mut = 30;                  // 通过引用修改

// 原始指针（unsafe）
let raw = &x as *const i32;
let raw_mut = &mut y as *mut i32;
```

## 5. 运算符

### 5.1 算术运算符

```boxlang
let a = 10;
let b = 3;

let sum = a + b;          // 13
let diff = a - b;         // 7
let product = a * b;      // 30
let quotient = a / b;     // 3（整数除法）
let remainder = a % b;    // 1

// 自增/自减（作为表达式）
let mut x = 5;
let y = x++;              // y = 5, x = 6
let z = ++x;              // z = 7, x = 7
```

### 5.2 比较运算符

```boxlang
let a = 10;
let b = 20;

let eq = a == b;          // false
let ne = a != b;          // true
let lt = a < b;           // true
let le = a <= b;          // true
let gt = a > b;           // false
let ge = a >= b;          // false
```

### 5.3 逻辑运算符

```boxlang
let a = true;
let b = false;

let and = a && b;         // false（逻辑与）
let or = a || b;          // true（逻辑或）
let not = !a;             // false（逻辑非）
```

### 5.4 位运算符

```boxlang
let a = 0b1100;           // 12
let b = 0b1010;           // 10

let and = a & b;          // 0b1000 = 8（按位与）
let or = a | b;           // 0b1110 = 14（按位或）
let xor = a ^ b;          // 0b0110 = 6（按位异或）
let not = !a;             // 按位取反
let shl = a << 2;         // 0b110000 = 48（左移）
let shr = a >> 2;         // 0b0011 = 3（右移）
```

### 5.5 赋值与复合赋值

```boxlang
let mut x = 10;

// 简单赋值
x = 20;

// 复合赋值
x += 5;                   // x = x + 5
x -= 3;                   // x = x - 3
x *= 2;                   // x = x * 2
x /= 4;                   // x = x / 4
x %= 3;                   // x = x % 3

// 位运算复合赋值
x &= 0xFF;
x |= 0x10;
x ^= 0x0F;
x <<= 2;
x >>= 1;
```

### 5.6 管道操作符

BoxLang 支持管道操作符 `|>`，让数据处理链更加清晰和易读。

#### 基本用法

管道操作符将左侧表达式的结果作为参数传递给右侧的函数：

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
    // 传统写法（嵌套调用）
    let result1 = square(add_one(double(5)));
    // 计算顺序: double(5) -> add_one(10) -> square(11) = 121
    
    // 管道操作符写法（更清晰）
    let result2 = 5 |> double |> add_one |> square;
    // 结果相同: 121，但阅读顺序与执行顺序一致
}
```

#### 工作原理

- `a |> f` 等价于 `f(a)`
- `a |> f |> g` 等价于 `g(f(a))`
- 数据从左向右流动，符合阅读直觉

#### 链式管道

```boxlang
fn increment(x: i32) -> i32 {
    return x + 1;
}

fn double(x: i32) -> i32 {
    return x * 2;
}

fn square(x: i32) -> i32 {
    return x * x;
}

fn negate(x: i32) -> i32 {
    return -x;
}

pub fn main() {
    let x = 3;
    
    // 多步管道
    let result1 = x |> increment |> double;
    // 3 -> 4 -> 8
    
    let result2 = x |> double |> square;
    // 3 -> 6 -> 36
    
    let result3 = x |> increment |> double |> square;
    // 3 -> 4 -> 8 -> 64
    
    let result4 = x |> double |> square |> negate;
    // 3 -> 6 -> 36 -> -36
}
```

#### 数学计算管道示例

```boxlang
fn absolute(x: i32) -> i32 {
    if x < 0 {
        return -x;
    }
    return x;
}

fn power_of_two(x: i32) -> i32 {
    return x * x;
}

pub fn main() {
    let value1 = 5;
    let result1 = value1 |> power_of_two |> absolute;
    // 5 -> 25 -> 25
    
    let value2 = -3;
    let result2 = value2 |> absolute |> power_of_two;
    // -3 -> 3 -> 9
    
    let result3 = 2 |> power_of_two |> power_of_two;
    // 2 -> 4 -> 16
}
```

#### 管道操作符的优势

1. **可读性**：代码阅读顺序与执行顺序一致
2. **可维护性**：易于添加、删除或重排处理步骤
3. **减少嵌套**：避免深层嵌套的函数调用
4. **数据流清晰**：明确展示数据的转换过程

```boxlang
// 对比：传统嵌套 vs 管道操作符

// 传统写法（从内到外阅读）
let result = format(process(validate(parse(data))));

// 管道写法（从左到右阅读）
let result = data |> parse |> validate |> process |> format;
```

### 5.7 运算符优先级

从高到低：

| 优先级 | 运算符 | 说明 |
|--------|--------|------|
| 1 | `()` `[]` `.` `::` | 函数调用、索引、字段访问 |
| 2 | `++` `--` | 自增、自减 |
| 3 | `!` `~` `-` `*` `&` `&mut` | 一元运算符 |
| 4 | `*` `/` `%` | 乘法、除法、取模 |
| 5 | `+` `-` | 加法、减法 |
| 6 | `<<` `>>` | 位移 |
| 7 | `&` | 按位与 |
| 8 | `^` | 按位异或 |
| 9 | `\|` | 按位或 |
| 10 | `==` `!=` `<` `<=` `>` `>=` | 比较 |
| 11 | `&&` | 逻辑与 |
| 12 | `\|\|` | 逻辑或 |
| 13 | `..` `...` | 范围 |
| 14 | `\|>` | 管道 |
| 15 | `=` `+=` `-=` 等 | 赋值 |

## 6. 函数

### 6.1 基本函数定义

```boxlang
// 无参数、无返回值
fn greet() {
    println("Hello!");
}

// 有参数
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// 表达式返回值（省略 return）
fn multiply(a: i32, b: i32) -> i32 {
    a * b  // 最后一行表达式作为返回值
}

// 多参数、多返回值（元组）
fn divide(dividend: i32, divisor: i32) -> (i32, i32) {
    (dividend / divisor, dividend % divisor)
}
```

### 6.2 函数参数

```boxlang
// 不可变参数（默认）
fn print_value(x: i32) {
    // x = 10;  // 错误！参数默认不可变
    println("{x}");
}

// 可变参数
fn increment(mut x: i32) -> i32 {
    x = x + 1;
    x
}

// 引用参数
fn modify_value(x: &mut i32) {
    *x = *x + 10;
}

// 使用
let mut val = 5;
modify_value(&mut val);
// val 现在是 15
```

### 6.3 泛型函数

```boxlang
// 简单泛型
fn identity<T>(x: T) -> T {
    x
}

// 带约束的泛型
fn max<T: Ord>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

// 多类型参数
fn pair<T, U>(a: T, b: U) -> (T, U) {
    (a, b)
}
```

### 6.4 函数重载

```boxlang
fn process(x: i32) -> i32 {
    x * 2
}

fn process(x: str) -> str {
    x + " processed"
}

// 使用
let num_result = process(10);       // 20
let str_result = process("hello");  // "hello processed"
```

## 7. 控制流

### 7.1 条件语句

```boxlang
// if-else
let number = 10;

if number > 0 {
    println("正数");
} else if number < 0 {
    println("负数");
} else {
    println("零");
}

// if 作为表达式
let result = if number > 5 { "大" } else { "小" };

// 带条件的 let
let value = if let Some(x) = maybe_value {
    x
} else {
    0
};
```

### 7.2 循环

```boxlang
// while 循环
let mut i = 0;
while i < 5 {
    println("i = {i}");
    i = i + 1;
}

// loop 无限循环（需要 break）
let mut counter = 0;
loop {
    counter = counter + 1;
    if counter >= 10 {
        break;
    }
}

// 带标签的循环
'outer: for i in 0..5 {
    for j in 0..5 {
        if i * j > 10 {
            break 'outer;
        }
    }
}

// for 循环（范围）
for i in 0..5 {           // 0, 1, 2, 3, 4
    println("{i}");
}

// for 循环（包含结束）
for i in 0...5 {          // 0, 1, 2, 3, 4, 5
    println("{i}");
}

// for 循环（数组）
let arr = [10, 20, 30];
for item in arr {
    println("item = {item}");
}

// for 循环（带索引）
for (index, value) in arr.iter().enumerate() {
    println("[{index}] = {value}");
}
```

### 7.3 模式匹配

```boxlang
let value = 5;

match value {
    1 => println("一"),
    2 => println("二"),
    3 | 4 | 5 => println("三到五"),
    6...10 => println("六到十"),
    _ => println("其他"),
}

// 带守卫的匹配
match age {
    n if n < 0 => println("无效年龄"),
    n if n < 18 => println("未成年"),
    n if n < 60 => println("成年人"),
    _ => println("老年人"),
}

// if let 简化匹配
if let Some(x) = maybe_value {
    println("值是 {x}");
}

// while let
while let Some(value) = iter.next() {
    println("{value}");
}
```

## 8. 结构体与枚举

### 8.1 结构体定义

```boxlang
// 命名字段结构体
pub struct Point {
    x: f64,
    y: f64,
}

// 元组结构体
pub struct Color(u8, u8, u8);

// 单元结构体
pub struct Empty;

// 泛型结构体
pub struct Container<T> {
    value: T,
}
```

### 8.2 结构体使用

```boxlang
// 创建实例
let p = Point { x: 10.0, y: 20.0 };

// 字段访问
println("x = {p.x}, y = {p.y}");

// 结构体更新语法
let p2 = Point { x: 5.0, ..p };

// 可变结构体
let mut p3 = Point { x: 0.0, y: 0.0 };
p3.x = 10.0;

// 元组结构体
let red = Color(255, 0, 0);
let r = red.0;
```

### 8.3 方法实现

```boxlang
impl Point {
    // 构造函数（关联函数）
    fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }
    
    // 方法（第一个参数是 self）
    fn distance_from_origin(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    
    // 可变方法
    fn move_by(&mut self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
    }
    
    // 消费方法
    fn destroy(self) {
        // self 被移动到这里
    }
}

// 使用
let p = Point::new(3.0, 4.0);
let dist = p.distance_from_origin();  // 5.0
```

### 8.4 枚举定义

```boxlang
// 基本枚举
pub enum Direction {
    North,
    South,
    East,
    West,
}

// 带数据的枚举
pub enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(str),
    ChangeColor(i32, i32, i32),
}

// 泛型枚举
pub enum Option<T> {
    Some(T),
    None,
}

pub enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

### 8.5 枚举使用

```boxlang
let msg = Message::Move { x: 10, y: 20 };

match msg {
    Message::Quit => println("退出"),
    Message::Move { x, y } => println("移动到 ({x}, {y})"),
    Message::Write(text) => println("文本: {text}"),
    Message::ChangeColor(r, g, b) => println("RGB({r}, {g}, {b})"),
}

// Option 使用
let some_num = Option::Some(5);
let no_num: Option<i32> = Option::None;

// if let
if let Option::Some(x) = some_num {
    println("值: {x}");
}
```

## 9. 模块系统

### 9.1 模块声明

```boxlang
// 声明模块
module myproject;

// 导入其他模块
use std::io;
use std::fs::File;
use std::collections::{HashMap, Vec};

// 公开模块
pub mod utils;

// 使用模块中的内容
use utils::helper;
use crate::core::engine::Engine;
```

### 9.2 可见性

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

// 指定路径可见
pub(in crate:: submodule) fn restricted_visible() {}
```

## 10. 闭包

```boxlang
// 基本闭包
let add_one = |x: i32| -> i32 { x + 1 };
let result = add_one(5);  // 6

// 类型推断
let multiply = |x, y| x * y;

// 捕获环境
let factor = 2;
let multiply_by_factor = |x| x * factor;

// 移动闭包
let data = vec![1, 2, 3];
let consume = move || data.len();

// 作为参数
fn apply<F>(f: F, x: i32) -> i32
where
    F: Fn(i32) -> i32,
{
    f(x)
}

let result = apply(|x| x * 2, 5);  // 10
```

## 11. 异步编程

```boxlang
// 异步函数
async fn fetch_data() -> Data {
    // 异步操作
    await load_from_network();
}

// 异步块
let future = async {
    let data = await fetch_data();
    process(data);
};

// 生成任务
spawn async {
    await some_async_operation();
};

// await 表达式
let result = await some_future;
```

## 12. 类型转换

```boxlang
// 显式转换（as）
let x: i32 = 10;
let y: i64 = x as i64;

// 类型推断
let inferred = 42;  // i32
let float_inferred = 3.14;  // f64

// 字面量后缀
let explicit = 42u64;
let float_explicit = 3.14f32;
```

## 13. 属性与宏

```boxlang
// 函数属性
#[inline]
fn fast_function() {}

#[test]
fn test_addition() {
    assert_eq!(2 + 2, 4);
}

#[derive(Debug, Clone)]
struct MyStruct {
    value: i32,
}

// 条件编译
#[cfg(target_os = "windows")]
fn platform_specific() {}

#[cfg(feature = "std")]
use std::collections::HashMap;
```

## 14. 完整示例

```boxlang
module calculator;

pub struct Calculator {
    result: f64,
}

impl Calculator {
    pub fn new() -> Calculator {
        Calculator { result: 0.0 }
    }
    
    pub fn add(&mut self, value: f64) -> &mut Self {
        self.result += value;
        self
    }
    
    pub fn subtract(&mut self, value: f64) -> &mut Self {
        self.result -= value;
        self
    }
    
    pub fn result(&self) -> f64 {
        self.result
    }
    
    pub fn reset(&mut self) {
        self.result = 0.0;
    }
}

pub fn main() {
    let mut calc = Calculator::new();
    
    // 链式调用
    calc.add(10.0).subtract(3.0).add(5.0);
    
    println("结果: {calc.result()}");  // 12.0
    
    // 模式匹配
    match calc.result() as i32 {
        0 => println("零"),
        1...10 => println("一到十"),
        n if n > 10 => println("大于十: {n}"),
        _ => println("负数"),
    }
}
```

## 下一步

- [项目结构](../05-project-structure/README_CN.md) - 了解项目配置和模块组织
- [AppBox 打包](../06-appbox-packaging/README_CN.md) - 学习如何打包和发布应用
