# BoxLang Installation Guide

## System Requirements

### Minimum Requirements
- **OS**: Windows 10/11 (64-bit)
- **RAM**: 4 GB
- **Disk Space**: 2 GB available
- **Network**: Internet connection (for downloading dependencies)

### Recommended Configuration
- **OS**: Windows 11 (64-bit)
- **RAM**: 8 GB
- **Disk Space**: 5 GB available
- **IDE**: Visual Studio 2022 or VS Code

## Prerequisites

Before installing BoxLang, ensure you have the following software installed:

### 1. Git
```bash
# Check if installed
git --version

# Download: https://git-scm.com/download/win
```

### 2. Rust Toolchain
```bash
# Install using rustup (recommended)
# Download: https://rustup.rs/

# Verify installation
rustc --version
cargo --version
```

### 3. LLVM (Optional, for optimized compilation)
```bash
# Download: https://releases.llvm.org/download.html
# Recommended version: 15.0 or higher
```

## Installation Steps

### Method 1: Build from Source

#### 1. Clone the Repository
```bash
git clone https://github.com/yourusername/box-ecosystem.git
cd box-ecosystem
```

#### 2. Build the Compiler
```bash
cd boxlang/compiler
cargo build --release
```

After compilation, the executable is located at:
```
target/release/boxlang.exe
```

#### 3. Add to System PATH
```powershell
# PowerShell (run as administrator)
[Environment]::SetEnvironmentVariable(
    "Path",
    [Environment]::GetEnvironmentVariable("Path", "User") + ";C:\path\to\box-ecosystem\boxlang\compiler\target\release",
    "User"
)
```

### Method 2: Using Install Script (Recommended)

```powershell
# PowerShell
irm https://boxlang.dev/install.ps1 | iex
```

### Method 3: Download Pre-built Binary

1. Visit [GitHub Releases](https://github.com/yourusername/box-ecosystem/releases)
2. Download the latest `boxlang-windows-x64.zip`
3. Extract to your desired directory
4. Add the directory to system PATH

## Verify Installation

```bash
# Check version
boxlang --version

# View help
boxlang --help

# Test compiler
boxlang doctor
```

## Configure Development Environment

### VS Code Setup

#### 1. Install Extensions
- BoxLang Language Support
- BoxLang Debugger

#### 2. Configure settings.json
```json
{
    "boxlang.compilerPath": "C:\\path\\to\\boxlang.exe",
    "boxlang.enableLinter": true,
    "boxlang.formatOnSave": true
}
```

### Visual Studio Setup

#### 1. Install BoxLang VS Extension
```bash
boxlang install-vs-extension
```

#### 2. Configure Project Properties
- Right-click project → Properties → BoxLang
- Set compiler path and build options

## FAQ

### Q: "linker not found" error during compilation
**A**: Install Visual C++ Build Tools
```powershell
# Install via Visual Studio Installer
# Or download standalone version
https://visualstudio.microsoft.com/visual-cpp-build-tools/
```

### Q: cargo build fails with missing dependencies
**A**: Update Rust toolchain
```bash
rustup update
rustup component add rust-src
```

### Q: boxlang command not recognized
**A**: Check PATH configuration
```powershell
# View current PATH
$env:Path -split ";"

# Verify it includes the directory containing boxlang.exe
```

## Update BoxLang

```bash
# Update from source
cd box-ecosystem
git pull
cd boxlang/compiler
cargo build --release

# Or use update command
boxlang self-update
```

## Uninstall BoxLang

```powershell
# Remove installation directory
Remove-Item -Recurse -Force "C:\path\to\box-ecosystem"

# Remove from PATH
# Manually edit system environment variables
```

## Next Steps

- [Quick Start](../03-quickstart/README_EN.md) - Create your first BoxLang project
