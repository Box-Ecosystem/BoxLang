# BoxLang Basic Syntax

This chapter introduces the core syntax concepts of BoxLang.

## Variables and Constants

### Variable Declaration

```boxlang
// Immutable variable (default)
let x = 10;
// x = 20;  // Error! Immutable variables cannot be reassigned

// Mutable variable
let mut y = 20;
y = 30;  // OK

// Type annotation (optional)
let count: i32 = 100;
let price: f64 = 19.99;
let name: str = "BoxLang";
```

### Constants

```boxlang
// Constants must be determined at compile time
const PI: f64 = 3.14159;
const MAX_SIZE: i32 = 100;
```

## Data Types

### Basic Types

```boxlang
// Integers
let a: i8 = 127;        // 8-bit signed integer
let b: i16 = 32767;     // 16-bit signed integer
let c: i32 = 2147483647; // 32-bit signed integer (default)
let d: i64 = 9223372036854775807; // 64-bit signed integer

// Unsigned integers
let e: u32 = 4294967295;

// Floating point
let f: f32 = 3.14;      // 32-bit float
let g: f64 = 3.14159;   // 64-bit float (default)

// Boolean
let flag: bool = true;

// Character
let ch: char = 'A';

// String
let s: str = "Hello, BoxLang!";
```

### Compound Types

```boxlang
// Array
let arr = [1, 2, 3, 4, 5];
let first = arr[0];  // Access element

// Tuple
let tuple = (1, "hello", 3.14);
let num = tuple.0;   // Access first element
let text = tuple.1;  // Access second element
```

## Functions

### Basic Function Definition

```boxlang
// Function with no return value
fn greet(name: str) {
    println("Hello, {name}!");
}

// Function with return value
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// Expression return (omit return)
fn multiply(a: i32, b: i32) -> i32 {
    a * b  // Last expression is the return value
}
```

### Function Overloading

```boxlang
fn process(x: i32) -> i32 {
    x * 2
}

fn process(x: str) -> str {
    x + " processed"
}
```

## Control Flow

### Conditional Statements

```boxlang
let number = 10;

// if-else
if number > 0 {
    println("Positive");
} else if number < 0 {
    println("Negative");
} else {
    println("Zero");
}

// if as expression
let result = if number > 5 { "Large" } else { "Small" };
```

### Loops

```boxlang
// while loop
let mut i = 0;
while i < 5 {
    println("i = {i}");
    i = i + 1;
}

// for loop (range)
for j in 0..5 {
    println("j = {j}");
}

// for loop (array)
let arr = [10, 20, 30];
for item in arr {
    println("item = {item}");
}

// loop infinite loop
let mut counter = 0;
loop {
    counter = counter + 1;
    if counter >= 10 {
        break;
    }
}
```

## Structs

### Defining Structs

```boxlang
pub struct Point {
    x: f64,
    y: f64,
}

pub struct User {
    name: str,
    age: u32,
    email: str,
}
```

### Using Structs

```boxlang
// Create instance
let p = Point { x: 10.0, y: 20.0 };
let user = User {
    name: "Alice",
    age: 30,
    email: "alice@example.com",
};

// Access fields
println("x = {p.x}, y = {p.y}");
println("Name: {user.name}");

// Struct methods
impl Point {
    // Method
    fn distance_from_origin(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    
    // Associated function (constructor)
    fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }
}
```

## Enums

### Basic Enum

```boxlang
pub enum Direction {
    North,
    South,
    East,
    West,
}

let dir = Direction::North;
```

### Enum with Data

```boxlang
pub enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(str),
    ChangeColor(i32, i32, i32),
}

let msg1 = Message::Quit;
let msg2 = Message::Move { x: 10, y: 20 };
let msg3 = Message::Write("Hello".to_string());
```

### Option Type

```boxlang
pub enum Option<T> {
    Some(T),
    None,
}

// Usage example
let some_number = Option::Some(5);
let no_number: Option<i32> = Option::None;
```

## Pattern Matching

```boxlang
let msg = Message::Move { x: 10, y: 20 };

match msg {
    Message::Quit => {
        println("Quit");
    }
    Message::Move { x, y } => {
        println("Move to ({x}, {y})");
    }
    Message::Write(text) => {
        println("Text: {text}");
    }
    Message::ChangeColor(r, g, b) => {
        println("Color: RGB({r}, {g}, {b})");
    }
}

// if let simplified matching
if let Message::Write(text) = msg {
    println("Received text: {text}");
}
```

## Module System

```boxlang
// Declare module
module myproject;

// Import other modules
use std::io;
use std::fs::File;

// Public module
pub mod utils;

// Use items from module
use utils::helper;
```

## Pipeline Operator

BoxLang supports the pipeline operator `|>`, making data processing chains clearer and more readable.

### Basic Usage

The pipeline operator passes the result of the left expression as an argument to the function on the right:

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
    // Traditional approach (nested calls)
    let result1 = square(add_one(double(5)));
    // Execution order: double(5) -> add_one(10) -> square(11) = 121
    
    // Pipeline operator approach (clearer)
    let result2 = 5 |> double |> add_one |> square;
    // Same result: 121, but reading order matches execution order
}
```

### How It Works

- `a |> f` is equivalent to `f(a)`
- `a |> f |> g` is equivalent to `g(f(a))`
- Data flows from left to right, matching natural reading direction

### Chained Pipeline

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
    
    // Multi-step pipeline
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

### Math Pipeline Example

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

### Benefits of Pipeline Operator

1. **Readability**: Code reading order matches execution order
2. **Maintainability**: Easy to add, remove, or reorder processing steps
3. **Reduced Nesting**: Avoids deeply nested function calls
4. **Clear Data Flow**: Clearly shows data transformation process

```boxlang
// Comparison: Traditional nesting vs Pipeline operator

// Traditional approach (read inside-out)
let result = format(process(validate(parse(data))));

// Pipeline approach (read left-to-right)
let result = data |> parse |> validate |> process |> format;
```

## Comments

```boxlang
// Single-line comment

/*
 * Multi-line comment
 * Can span multiple lines
 */

/// Documentation comment (for functions, structs, etc.)
pub fn documented_function() {
    //! Inner documentation comment
}
```

## Next Steps

- [Project Structure](../05-project-structure/README_EN.md) - Learn about project configuration and module organization
