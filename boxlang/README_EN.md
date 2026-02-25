# BoxLang

BoxLang is a versatile system-level programming language designed for various environments, drawing inspiration from modern languages like Rust, Go, and Zig while providing its own unique features and syntax.

## Design Goals

- **Windows First**: Native support for Windows development
- **Embedded Friendly**: Designed for seamless operation in embedded environments
- **High Performance**: AOT compilation, zero-cost abstraction
- **Easy to Learn**: Simpler syntax than Rust
- **Modern Features**: Support for async/await, generics, pattern matching
- **Packaging Support**: Native support for application packaging

## Quick Start

### Installation

```bash
git clone https://github.com/NaAIO27/boxlang.git
cd boxlang/compiler
cargo build --release
```

### Create New Project

```bash
boxlang new myproject
cd myproject
```

### Write First Program

Create `src/main.box`:

```boxlang
module myproject;

pub fn main() {
    println("Hello, BoxLang!");
}
```

### Compile and Run

```bash
boxlang compile src/main.box -o hello
boxlang build
boxlang run
```

### Package Application

```bash
boxlang package
boxlang package -o ./dist -n "myapp" -v "1.0.0"
```

## Basic Syntax

```boxlang
// Variable declaration
let x = 10;           // Immutable
let mut y = 20;       // Mutable
const PI = 3.14159;   // Constant

// Function definition
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// Struct
pub struct Point {
    x: f64,
    y: f64,
}

// Enum
pub enum Option<T> {
    Some(T),
    None,
}
```

## Project Structure

```
myproject/
├── box.toml          # Project configuration
├── README.md         # Project description
├── .gitignore        # Git ignore file
└── src/
    ├── main.box      # Main program
    └── lib.box       # Library code
```

## License

BoxLang is licensed under the MIT License. For the full license text, see the [LICENSE](LICENSE) file.

## Trademarks

"BoxLang" is a project-specific name used to identify this programming language. This name is not a registered trademark unless otherwise indicated.

## Contributing

We welcome contributions to BoxLang! To contribute, please:

1. Fork the repository
2. Create a new branch for your feature or bug fix
3. Make your changes
4. Submit a pull request

### Contributor License Agreement (CLA)

By contributing to this project, you agree to the terms of the Contributor License Agreement (CLA). This agreement ensures that your contributions are properly licensed and that the project maintainers have the necessary rights to include your work in the project.

Please ensure your code follows the project's coding guidelines and passes all tests.

## Disclaimer

BoxLang is an open-source project under active development. While we strive to provide a reliable and secure programming language, please use it at your own risk. The project maintainers make no warranties, express or implied, regarding the functionality or suitability of BoxLang for any particular purpose.
