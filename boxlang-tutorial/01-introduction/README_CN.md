# BoxLang 简介

## 什么是 BoxLang？

BoxLang（盒子语言）是一个专为 Box Ecosystem 设计的系统级编程语言，融合了 Rust、Go、Zig 等现代语言的优点，旨在为开发者提供高效、安全、易用的编程体验。

## 设计目标

### 🪟 Windows 优先
- 原生支持 Windows 开发环境
- 提供完整的 Windows 工具链
- 无缝集成 Windows 系统 API

### 🔧 嵌入式友好
- 专为 zetboxos (LiteOS-M) 优化
- 低内存占用，高效运行时
- 支持 ESP32 等嵌入式平台

### ⚡ 高性能
- AOT（Ahead-of-Time）编译
- 零成本抽象（Zero-Cost Abstractions）
- 无垃圾回收，可预测的性能

### 📚 易于学习
- 比 Rust 更简洁的语法
- 直观的错误提示
- 丰富的文档和示例

### 🚀 现代特性
- 支持 async/await 异步编程
- 泛型（Generics）支持
- 模式匹配（Pattern Matching）
- 类型推导

### 📦 AppBox 集成
- 原生支持打包为 AppBox 格式
- 一键发布到 Box Ecosystem
- 自动处理依赖关系

## 适用场景

BoxLang 适用于以下场景：

1. **嵌入式系统开发** - IoT 设备、传感器、控制器
2. **系统工具开发** - 命令行工具、系统服务
3. **zetboxos 应用开发** - 原生应用程序
4. **跨平台开发** - Windows + 嵌入式双平台

## 与其他语言的对比

| 特性 | BoxLang | Rust | Go | C |
|------|---------|------|-----|---|
| 内存安全 | ✅ | ✅ | ✅ | ❌ |
| 零成本抽象 | ✅ | ✅ | ❌ | ✅ |
| 学习曲线 | 平缓 | 陡峭 | 平缓 | 中等 |
| 嵌入式支持 | 原生 | 良好 | 一般 | 原生 |
| Windows 支持 | 原生 | 良好 | 良好 | 良好 |
| 编译速度 | 快 | 较慢 | 快 | 快 |

## 下一步

- [安装 BoxLang](../02-installation/README_CN.md)
- [快速开始](../03-quickstart/README_CN.md)
