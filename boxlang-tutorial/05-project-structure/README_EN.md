# BoxLang Project Structure

This chapter introduces the standard structure and configuration of BoxLang projects.

## Standard Project Structure

A typical BoxLang project structure looks like this:

```
myproject/
├── box.toml              # Project configuration file (required)
├── README.md             # Project documentation
├── LICENSE               # License file
├── .gitignore            # Git ignore file
├── .boxlang/             # BoxLang internal directory
│   └── cache/            # Compilation cache
├── src/                  # Source code directory
│   ├── main.box          # Main program entry (executable project)
│   ├── lib.box           # Library entry (library project)
│   └── utils/            # Sub-module directory
│       └── helper.box
├── tests/                # Test code
│   └── integration_test.box
├── examples/             # Example code
│   └── basic_usage.box
├── docs/                 # Project documentation
│   └── api.md
└── target/               # Build output directory
    ├── debug/            # Debug build output
    └── release/          # Release build output
```

## box.toml Configuration

### Basic Configuration

```toml
[package]
name = "myproject"           # Project name
version = "1.0.0"            # Version (follows semantic versioning)
authors = ["Your Name <you@example.com>"]
edition = "2024"             # BoxLang version
license = "MIT"
description = "Project description"
repository = "https://github.com/username/myproject"

[dependencies]
# Dependencies
std = { version = "1.0" }
serde = { version = "0.8", features = ["derive"] }

[dependencies.mylib]
path = "../mylib"            # Local path dependency

[dependencies.remote-lib]
git = "https://github.com/user/repo.git"
branch = "main"

[build]
target = "x86_64-pc-windows-msvc"  # Target platform
opt-level = 3                      # Optimization level (0-3)
debug = false                      # Include debug info

[features]
default = ["std"]
std = []
no_std = []
embedded = ["no_std"]
```

### Multi-target Configuration

```toml
# Windows desktop app
[[target]]
name = "windows-app"
target = "x86_64-pc-windows-msvc"
output-type = "exe"

# zetboxos embedded app
[[target]]
name = "zetboxos-app"
target = "thumbv7em-none-eabihf"
output-type = "bin"
```

## Source Code Organization

### Executable Project

```boxlang
// src/main.box
module myproject;

use std::io;
use utils::helper;

pub fn main() {
    println("Program started");
    helper::do_something();
}
```

### Library Project

```boxlang
// src/lib.box
module mylib;

pub mod core;
pub mod utils;

// Public API
pub use core::engine::Engine;
pub use utils::helpers::format_data;
```

### Sub-modules

```boxlang
// src/utils/helper.box
module myproject::utils::helper;

pub fn do_something() {
    println("Doing something...");
}

pub struct Helper {
    name: str,
}

impl Helper {
    pub fn new(name: str) -> Helper {
        Helper { name }
    }
}
```

## Module System

### Module Declaration

```boxlang
// Declare current module
module myproject::core::engine;

// Import standard library
use std::collections::HashMap;
use std::io::{File, Read};

// Import local modules
use crate::utils::helper;
use super::config::Config;

// Public re-export
pub use self::types::EngineType;
```

### Module Visibility

```boxlang
// Default private
fn private_function() {}
struct PrivateStruct {}

// Public
pub fn public_function() {}
pub struct PublicStruct {}

// Crate-visible only
pub(crate) fn crate_function() {}

// Parent module visible only
pub(super) fn parent_visible() {}
```

## Test Organization

### Unit Tests

```boxlang
// src/calculator.box
module myproject::calculator;

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Built-in tests
#[test]
fn test_add() {
    assert_eq!(add(2, 3), 5);
    assert_eq!(add(-1, 1), 0);
}

#[test]
#[should_panic]
fn test_overflow() {
    // Test overflow condition
}
```

### Integration Tests

```boxlang
// tests/integration_test.box
use myproject::calculator;

#[test]
fn test_calculator_integration() {
    let result = calculator::add(10, 20);
    assert_eq!(result, 30);
}
```

## Workspace

### Workspace Configuration

```toml
# box.toml (workspace root)
[workspace]
members = [
    "mylib",
    "myapp",
    "utils",
]

[workspace.dependencies]
serde = "1.0"
```

### Workspace Structure

```
workspace/
├── box.toml              # Workspace configuration
├── mylib/
│   ├── box.toml
│   └── src/
├── myapp/
│   ├── box.toml
│   └── src/
└── utils/
    ├── box.toml
    └── src/
```

## Build Configuration

### Conditional Compilation

```boxlang
// Platform-specific code
#[cfg(target_os = "windows")]
fn platform_specific() {
    // Windows code
}

#[cfg(target_os = "zetboxos")]
fn platform_specific() {
    // zetboxos code
}

// Feature flags
#[cfg(feature = "std")]
use std::collections::HashMap;

#[cfg(not(feature = "std"))]
use alloc::collections::HashMap;
```

### Build Scripts

```toml
# box.toml
[package]
build = "build.box"
```

```boxlang
// build.box
use std::process::Command;

fn main() {
    // Generate code
    println!("cargo:rerun-if-changed=src/schema.json");
    
    // Set environment variables
    println!("cargo:rustc-env=BUILD_TIME=2024-01-01");
}
```

## Next Steps

- [AppBox Packaging](../06-appbox-packaging/README_EN.md) - Learn how to package and publish applications
