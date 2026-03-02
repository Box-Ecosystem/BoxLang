# BoxLang Programming Language

BoxLang is a versatile system-level programming language designed for various environments, drawing inspiration from modern languages like Rust, Go, and Zig while providing its own unique features and syntax.

## Version

Current version: 0.1.0-alpha

## Key Features

- **Windows First**: Native support for Windows development
- **Embedded Friendly**: Designed for seamless operation in embedded environments
- **High Performance**: AOT compilation, zero-cost abstraction
- **Easy to Learn**: Simpler syntax than Rust
- **Modern Features**: Support for async/await, generics, pattern matching
- **Packaging Support**: Native support for application packaging

## Trademarks

"BoxLang" is a project-specific name used to identify this programming language. This name is not a registered trademark unless otherwise indicated.

## Documentation / Dokumentation / Dokumentation

This module supports multiple languages. Please choose your preferred language:

- [中文文档 (Chinese)](boxlang/README_CN.md)
- [English Documentation](boxlang/README_EN.md)
- [Deutsche Dokumentation](boxlang/README_DE.md)

## Tutorials

Comprehensive tutorials are available in multiple languages:

- [中文教程 (Chinese Tutorials)](boxlang-tutorial/01-introduction/README_CN.md)
- [English Tutorials](boxlang-tutorial/01-introduction/README_EN.md)
- [Deutsche Tutorials](boxlang-tutorial/01-introduction/README_DE.md)

## Quick Start

```bash
# Clone the repository
git clone https://github.com/Box-Ecosystem/BoxLang.git
cd boxlang/compiler

# Build the compiler
cargo build --release

# Create a new project
boxlang new myproject
cd myproject

# Write your first program
# Create src/main.box with your code

# Compile and run
boxlang compile src/main.box -o hello
boxlang build
boxlang run
```

## Project Structure

A typical BoxLang project has the following structure:

```
myproject/
├── box.toml          # Project configuration
├── README.md         # Project documentation
├── .gitignore        # Git ignore file
└── src/
    ├── main.box      # Main program
    └── lib.box       # Library code
```

## Basic Syntax

Here are some basic BoxLang syntax examples:

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

// Pattern matching
fn process_option<T>(opt: Option<T>) {
    match opt {
        Option::Some(value) => println("Got value: {}", value),
        Option::None => println("Got nothing"),
    }
}
```

*Code examples in this document are licensed under the MIT License.*

## Command Line Tools

BoxLang provides a comprehensive command line tool for project management and compilation:

```bash
# Create a new project
boxlang new <project_name>

# Build the project
boxlang build

# Run the project
boxlang run

# Compile a single file
boxlang compile <input_file> -o <output_file>

# Package application
boxlang package
# Package with custom options
boxlang package -o ./dist -n "myapp" -v "1.0.0"

# Show help
boxlang help
# Show help for a specific command
boxlang help <command>
```

## License

BoxLang is licensed under the MIT License. For the full license text, see the [LICENSE](LICENSE) file.

## Contributing

We welcome contributions to BoxLang! To contribute, please:

1. Fork the repository
2. Create a new branch for your feature or bug fix
3. Make your changes
4. Submit a pull request

### Contributor License Agreement (CLA)

By contributing to this project, you agree to the terms of the [Contributor License Agreement (CLA)](CLA.md). This agreement ensures that your contributions are properly licensed and that the project maintainers have the necessary rights to include your work in the project.

Please ensure your code follows the project's coding guidelines and passes all tests.

## FAQ

### Q: What is the difference between BoxLang and Rust?
A: BoxLang is designed to be easier to learn than Rust while retaining many of its safety features. It also offers a more flexible approach to various programming environments.

### Q: Can I use BoxLang for embedded development?
A: Yes, BoxLang is designed to be embedded friendly and can run seamlessly in embedded environments.

### Q: How does BoxLang achieve high performance?
A: BoxLang uses AOT (Ahead-of-Time) compilation and zero-cost abstractions, similar to Rust.

## Community

Join our community to connect with other BoxLang users and contributors:

- [BoxLang GitHub Issues](https://github.com/Box-Ecosystem/BoxLang/issues)

## Disclaimer

BoxLang is an open-source project under active development. While we strive to provide a reliable and secure programming language, please use it at your own risk. The project maintainers make no warranties, express or implied, regarding the functionality or suitability of BoxLang for any particular purpose.

---

Made by BoxLang Community

