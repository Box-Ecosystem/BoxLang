# AppBox Packaging and Publishing

This chapter covers how to package BoxLang projects as AppBox format and publish to Box Ecosystem.

## What is AppBox?

AppBox is the application package format for Box Ecosystem, similar to other platform packages:

| Platform | Package Format |
|----------|----------------|
| Box Ecosystem | **.appbox** |
| Android | .apk / .aab |
| iOS | .ipa |
| Windows | .msi / .exe |

### AppBox Features

- **Self-contained**: Includes all dependencies needed to run the app
- **Signature verification**: Supports digital signatures for security
- **Version management**: Built-in version control and update mechanism
- **Cross-platform**: Supports Windows and zetboxos

## Basic Packaging

### Quick Package

```bash
# Execute in project root directory
boxlang package

# Output: target/package/myproject-1.0.0.appbox
```

### Custom Packaging

```bash
# Specify output directory
boxlang package -o ./dist

# Specify application name
boxlang package -n "MyApp"

# Specify version
boxlang package -v "2.0.0"

# Combined usage
boxlang package -o ./dist -n "MyApp" -v "1.0.0"
```

## Packaging Configuration

### box.toml Configuration

```toml
[package]
name = "myapp"
version = "1.0.0"
description = "My BoxLang Application"
authors = ["Your Name"]

[appbox]
# Application display name
name = "My Awesome App"

# Application icon
icon = "assets/icon.png"

# Splash screen
splash = "assets/splash.png"

# Application category
category = "Productivity"

# Supported architectures
architectures = ["x86_64", "arm64"]

# Minimum system requirements
min-os-version = "10.0"

# Permission declarations
permissions = [
    "network",
    "filesystem",
    "camera",
]

# Resource files
resources = [
    "assets/**/*",
    "config/*.json",
    "locales/**/*.lang",
]

# Excluded files
exclude = [
    "tests/**/*",
    "docs/**/*",
    "*.log",
]

[appbox.metadata]
# Additional metadata
keywords = ["productivity", "tools"]
homepage = "https://myapp.example.com"
```

## Application Signing

### Generate Signing Key

```bash
# Generate developer key
boxlang keygen --developer

# Generate publisher key
boxlang keygen --publisher

# Specify key file
boxlang keygen -o ./keys/mykey.pem
```

### Sign Application

```bash
# Sign with default key
boxlang package --sign

# Specify key file
boxlang package --sign --key ./keys/release.pem

# Specify key password
boxlang package --sign --key ./keys/release.pem --password-file ./keys/pass.txt
```

## Multi-platform Packaging

### Windows Desktop App

```bash
# Package Windows app
boxlang package --target windows-x64

# Create installer
boxlang package --target windows-x64 --installer msi
```

### zetboxos Embedded App

```bash
# Package zetboxos app
boxlang package --target zetboxos-arm64

# Optimize for embedded
boxlang package --target zetboxos-arm64 --opt-size
```

### Multi-target Packaging

```bash
# Package all targets
boxlang package --all-targets

# Output directory structure:
# target/package/
# ├── myapp-1.0.0-windows-x64.appbox
# ├── myapp-1.0.0-zetboxos-arm64.appbox
# └── myapp-1.0.0-zetboxos-armv7.appbox
```

## Application Verification

### Verify Package Integrity

```bash
# Verify AppBox file
boxlang verify myapp-1.0.0.appbox

# Detailed verification
boxlang verify myapp-1.0.0.appbox --verbose
```

### Check Application Info

```bash
# View application info
boxlang info myapp-1.0.0.appbox

# Example output:
# Name: My Awesome App
# Version: 1.0.0
# Author: Your Name
# Size: 2.5 MB
# Signature: Valid
# Permissions: network, filesystem
```

## Publishing Applications

### Local Installation

```bash
# Install AppBox
boxlang install myapp-1.0.0.appbox

# Specify installation location
boxlang install myapp-1.0.0.appbox --prefix /opt/apps

# Force reinstall
boxlang install myapp-1.0.0.appbox --force
```

### Publish to Box Store

```bash
# Login to Box Store
boxlang login

# Publish application
boxlang publish myapp-1.0.0.appbox

# Publish as beta
boxlang publish myapp-1.0.0.appbox --beta

# Publish as alpha
boxlang publish myapp-1.0.0.appbox --alpha
```

### Private Registry Publishing

```bash
# Publish to private registry
boxlang publish myapp-1.0.0.appbox --registry https://private.registry.com

# Use API key
boxlang publish myapp-1.0.0.appbox --registry https://private.registry.com --api-key $API_KEY
```

## Application Updates

### Check for Updates

```bash
# Check for app updates
boxlang update check myapp

# Check all app updates
boxlang update check --all
```

### Auto Update

```toml
# box.toml
[appbox.update]
enabled = true
channel = "stable"  # stable, beta, alpha
auto-check = true
check-interval = "1d"
```

### Manual Update

```bash
# Update app to latest version
boxlang update myapp

# Update to specific version
boxlang update myapp --version 2.0.0

# Update all apps
boxlang update --all
```

## Advanced Features

### Delta Updates

```bash
# Generate delta package
boxlang package --delta-from 1.0.0

# Apply delta update
boxlang update myapp --delta ./myapp-1.0.0-to-1.1.0.delta
```

### Application Plugins

```toml
# box.toml
[appbox.plugins]
enabled = true
plugin-dir = "plugins"
```

### Sandbox Configuration

```toml
# box.toml
[appbox.sandbox]
enabled = true
filesystem = "restricted"
network = "allowed"
permissions = ["camera", "microphone"]
```

## Complete Example

### Example Project Configuration

```toml
# box.toml
[package]
name = "weather-app"
version = "1.2.0"
edition = "2024"

[appbox]
name = "Weather"
description = "A simple weather application"
icon = "assets/weather-icon.png"
category = "Utilities"

[appbox.permissions]
network = true
location = true
notifications = true

[appbox.resources]
assets = ["assets/**/*"]
themes = ["themes/**/*"]
locales = ["locales/**/*.json"]
```

### Complete Packaging Workflow

```bash
# 1. Run tests
boxlang test

# 2. Build release version
boxlang build --release

# 3. Package application
boxlang package --sign

# 4. Verify package
boxlang verify target/package/weather-app-1.2.0.appbox

# 5. Publish to store
boxlang publish target/package/weather-app-1.2.0.appbox
```

## Troubleshooting

### Packaging Failed

```bash
# View detailed logs
boxlang package --verbose

# Clean cache and retry
boxlang clean
boxlang package
```

### Signing Issues

```bash
# Check key
boxlang keygen --verify ./keys/release.pem

# Regenerate key
boxlang keygen --force
```

### Publishing Failed

```bash
# Check network connection
boxlang doctor --network

# Verify login status
boxlang login --status
```

## Next Steps

- View [BoxLang Example Projects](https://github.com/box-ecosystem/examples)
- Read [zetboxos Application Development Guide](../../readme/zetboxos/README_EN.md)
- Join [BoxLang Community](https://community.boxlang.dev)
