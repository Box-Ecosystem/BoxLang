# BoxLang Quick Start

This guide will help you create and run your first BoxLang program in 5 minutes.

## Create a New Project

### Using boxlang new command

```bash
# Create a new project named hello_boxlang
boxlang new hello_boxlang

# Enter the project directory
cd hello_boxlang
```

### Project Structure

After creation, you will see the following directory structure:

```
hello_boxlang/
├── box.toml          # Project configuration file
├── README.md         # Project description
├── .gitignore        # Git ignore file
└── src/
    └── main.box      # Main program entry
```

## Write Your First Program

### Default main.box

```boxlang
module hello_boxlang;

pub fn main() {
    println("Hello, BoxLang!");
}
```

### Custom Program

Let's modify the program to add some interactive features:

```boxlang
module hello_boxlang;

pub fn main() {
    // Print welcome message
    println("Welcome to BoxLang!");
    println("==================");
    
    // Variable declaration
    let name = "BoxLang Developer";
    let version = 1.0;
    
    // String formatting output
    println("Hello, {name}!");
    println("Current version: {version}");
    
    // Call function
    let result = add(10, 20);
    println("10 + 20 = {result}");
}

// Define addition function
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
```

## Compile and Run

### Method 1: Build then Run

```bash
# Compile the project
boxlang build

# Run the compiled program
boxlang run
```

### Method 2: Run Directly (Development Mode)

```bash
# Compile and run immediately
boxlang run --dev
```

### Method 3: Specify Output File

```bash
# Compile to a specific executable name
boxlang compile src/main.box -o myprogram

# Run directly
./myprogram
```

## Development Workflow

### 1. Edit Code
Use your favorite editor to modify the `src/main.box` file.

### 2. Check Syntax
```bash
# Check code syntax without compiling
boxlang check
```

### 3. Build Project
```bash
# Development mode (fast compile, no optimization)
boxlang build --dev

# Release mode (optimized compile)
boxlang build --release
```

### 4. Run Tests
```bash
# Run tests in the project
boxlang test
```

## Common Commands Cheat Sheet

| Command | Description |
|---------|-------------|
| `boxlang new <name>` | Create new project |
| `boxlang build` | Build project |
| `boxlang run` | Run project |
| `boxlang check` | Syntax check |
| `boxlang test` | Run tests |
| `boxlang clean` | Clean build files |
| `boxlang fmt` | Format code |

## Next Steps

- [Basic Syntax](../04-basic-syntax/README_EN.md) - Learn BoxLang core syntax
- [Project Structure](../05-project-structure/README_EN.md) - Understand project configuration and structure
