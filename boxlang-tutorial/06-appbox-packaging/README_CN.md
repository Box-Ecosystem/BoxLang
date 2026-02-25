# AppBox 打包与发布

本章节介绍如何将 BoxLang 项目打包为 AppBox 格式并发布到 Box Ecosystem。

## 什么是 AppBox？

AppBox 是 Box Ecosystem 的应用程序包格式，类似于其他平台的应用包：

| 平台 | 包格式 |
|------|--------|
| Box Ecosystem | **.appbox** |
| Android | .apk / .aab |
| iOS | .ipa |
| Windows | .msi / .exe |

### AppBox 特点

- **自包含**：包含应用运行所需的所有依赖
- **签名验证**：支持数字签名确保安全性
- **版本管理**：内置版本控制和更新机制
- **跨平台**：支持 Windows 和 zetboxos

## 基本打包

### 快速打包

```bash
# 在项目根目录执行
boxlang package

# 输出：target/package/myproject-1.0.0.appbox
```

### 自定义打包

```bash
# 指定输出目录
boxlang package -o ./dist

# 指定应用名称
boxlang package -n "MyApp"

# 指定版本
boxlang package -v "2.0.0"

# 组合使用
boxlang package -o ./dist -n "MyApp" -v "1.0.0"
```

## 打包配置

### box.toml 配置

```toml
[package]
name = "myapp"
version = "1.0.0"
description = "我的 BoxLang 应用"
authors = ["Your Name"]

[appbox]
# 应用显示名称
name = "My Awesome App"

# 应用图标
icon = "assets/icon.png"

# 启动画面
splash = "assets/splash.png"

# 应用类别
category = "Productivity"

# 支持的架构
architectures = ["x86_64", "arm64"]

# 最低系统要求
min-os-version = "10.0"

# 权限声明
permissions = [
    "network",
    "filesystem",
    "camera",
]

# 资源文件
resources = [
    "assets/**/*",
    "config/*.json",
    "locales/**/*.lang",
]

# 排除文件
exclude = [
    "tests/**/*",
    "docs/**/*",
    "*.log",
]

[appbox.metadata]
# 额外元数据
keywords = ["productivity", "tools"]
homepage = "https://myapp.example.com"
```

## 应用签名

### 生成签名密钥

```bash
# 生成开发者密钥
boxlang keygen --developer

# 生成发布密钥
boxlang keygen --publisher

# 指定密钥文件
boxlang keygen -o ./keys/mykey.pem
```

### 签名应用

```bash
# 使用默认密钥签名
boxlang package --sign

# 指定密钥文件
boxlang package --sign --key ./keys/release.pem

# 指定密钥密码
boxlang package --sign --key ./keys/release.pem --password-file ./keys/pass.txt
```

## 多平台打包

### Windows 桌面应用

```bash
# 打包 Windows 应用
boxlang package --target windows-x64

# 创建安装程序
boxlang package --target windows-x64 --installer msi
```

### zetboxos 嵌入式应用

```bash
# 打包 zetboxos 应用
boxlang package --target zetboxos-arm64

# 优化嵌入式版本
boxlang package --target zetboxos-arm64 --opt-size
```

### 多目标同时打包

```bash
# 打包所有目标
boxlang package --all-targets

# 输出目录结构：
# target/package/
# ├── myapp-1.0.0-windows-x64.appbox
# ├── myapp-1.0.0-zetboxos-arm64.appbox
# └── myapp-1.0.0-zetboxos-armv7.appbox
```

## 应用验证

### 验证包完整性

```bash
# 验证 AppBox 文件
boxlang verify myapp-1.0.0.appbox

# 详细验证
boxlang verify myapp-1.0.0.appbox --verbose
```

### 检查应用信息

```bash
# 查看应用信息
boxlang info myapp-1.0.0.appbox

# 输出示例：
# Name: My Awesome App
# Version: 1.0.0
# Author: Your Name
# Size: 2.5 MB
# Signature: Valid
# Permissions: network, filesystem
```

## 发布应用

### 本地安装

```bash
# 安装 AppBox
boxlang install myapp-1.0.0.appbox

# 指定安装位置
boxlang install myapp-1.0.0.appbox --prefix /opt/apps

# 强制重新安装
boxlang install myapp-1.0.0.appbox --force
```

### 发布到 Box Store

```bash
# 登录 Box Store
boxlang login

# 发布应用
boxlang publish myapp-1.0.0.appbox

# 发布为测试版本
boxlang publish myapp-1.0.0.appbox --beta

# 发布为预览版本
boxlang publish myapp-1.0.0.appbox --alpha
```

### 私有仓库发布

```bash
# 发布到私有仓库
boxlang publish myapp-1.0.0.appbox --registry https://private.registry.com

# 使用 API 密钥
boxlang publish myapp-1.0.0.appbox --registry https://private.registry.com --api-key $API_KEY
```

## 应用更新

### 检查更新

```bash
# 检查应用更新
boxlang update check myapp

# 检查所有应用更新
boxlang update check --all
```

### 自动更新

```toml
# box.toml
[appbox.update]
enabled = true
channel = "stable"  # stable, beta, alpha
auto-check = true
check-interval = "1d"
```

### 手动更新

```bash
# 更新应用到最新版本
boxlang update myapp

# 更新到指定版本
boxlang update myapp --version 2.0.0

# 更新所有应用
boxlang update --all
```

## 高级功能

### 增量更新

```bash
# 生成增量包
boxlang package --delta-from 1.0.0

# 应用增量更新
boxlang update myapp --delta ./myapp-1.0.0-to-1.1.0.delta
```

### 应用插件

```toml
# box.toml
[appbox.plugins]
enabled = true
plugin-dir = "plugins"
```

### 沙盒配置

```toml
# box.toml
[appbox.sandbox]
enabled = true
filesystem = "restricted"
network = "allowed"
permissions = ["camera", "microphone"]
```

## 完整示例

### 示例项目配置

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

### 完整打包流程

```bash
# 1. 运行测试
boxlang test

# 2. 构建发布版本
boxlang build --release

# 3. 打包应用
boxlang package --sign

# 4. 验证包
boxlang verify target/package/weather-app-1.2.0.appbox

# 5. 发布到商店
boxlang publish target/package/weather-app-1.2.0.appbox
```

## 故障排除

### 打包失败

```bash
# 查看详细日志
boxlang package --verbose

# 清理缓存后重试
boxlang clean
boxlang package
```

### 签名问题

```bash
# 检查密钥
boxlang keygen --verify ./keys/release.pem

# 重新生成密钥
boxlang keygen --force
```

### 发布失败

```bash
# 检查网络连接
boxlang doctor --network

# 验证登录状态
boxlang login --status
```

## 下一步

- 查看 [BoxLang 示例项目](https://github.com/box-ecosystem/examples)
- 阅读 [zetboxos 应用开发指南](../../readme/zetboxos/README_CN.md)
- 加入 [BoxLang 社区](https://community.boxlang.dev)
