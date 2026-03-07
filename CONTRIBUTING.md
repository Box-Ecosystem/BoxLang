# 贡献指南 / Contributing Guide / Beitragsleitfaden

感谢您有兴趣为 BoxLang 做出贡献！本文档将帮助您了解如何参与项目开发。

Thank you for your interest in contributing to BoxLang! This document will help you understand how to participate in the project development.

Vielen Dank für Ihr Interesse an einem Beitrag zu BoxLang! Dieses Dokument hilft Ihnen zu verstehen, wie Sie an der Projektentwicklung teilnehmen können.

---

## 目录 / Table of Contents / Inhaltsverzeichnis

- [行为准则](#行为准则--code-of-conduct--verhaltenskodex)
- [如何贡献](#如何贡献--how-to-contribute--wie-man-beiträgt)
- [开发环境设置](#开发环境设置--development-setup--entwicklungsumgebung)
- [代码规范](#代码规范--code-style--code-stil)
- [提交流程](#提交流程--pull-request-process--pull-request-prozess)
- [问题报告](#问题报告--issue-reporting--problemberichterstattung)
- [社区](#社区--community--community)

---

## 行为准则 / Code of Conduct / Verhaltenskodex

### 中文

我们致力于为每个人提供友好、安全和欢迎的环境。请遵循以下准则：

- 使用友好和包容的语言
- 尊重不同的观点和经验
- 优雅地接受建设性批评
- 关注对社区最有利的事情
- 对其他社区成员表示同理心

### English

We are committed to providing a friendly, safe, and welcoming environment for everyone. Please follow these guidelines:

- Use welcoming and inclusive language
- Be respectful of differing viewpoints and experiences
- Gracefully accept constructive criticism
- Focus on what is best for the community
- Show empathy towards other community members

### Deutsch

Wir verpflichten uns, eine freundliche, sichere und einladende Umgebung für alle zu schaffen. Bitte befolgen Sie diese Richtlinien:

- Verwenden Sie eine einladende und inklusive Sprache
- Respektieren Sie unterschiedliche Ansichten und Erfahrungen
- Nehmen Sie konstruktive Kritik gracefully an
- Konzentrieren Sie sich auf das, was für die Gemeinschaft am besten ist
- Zeigen Sie Empathie gegenüber anderen Gemeinschaftsmitgliedern

---

## 如何贡献 / How to Contribute / Wie man beiträgt

### 中文

#### 贡献类型

我们欢迎以下类型的贡献：

- **代码贡献**：修复 Bug、添加新功能、优化性能
- **文档改进**：修正错误、改进说明、翻译文档
- **测试编写**：添加单元测试、集成测试
- **问题报告**：提交 Bug 报告、功能请求
- **代码审查**：审查 Pull Request

#### 贡献流程

1. Fork 本仓库
2. 创建您的特性分支 (`git checkout -b feature/amazing-feature`)
3. 进行更改
4. 确保代码通过所有测试
5. 提交您的更改 (`git commit -m 'Add some amazing feature'`)
6. 推送到分支 (`git push origin feature/amazing-feature`)
7. 创建 Pull Request

### English

#### Types of Contributions

We welcome the following types of contributions:

- **Code Contributions**: Fix bugs, add new features, optimize performance
- **Documentation Improvements**: Fix errors, improve explanations, translate documentation
- **Test Writing**: Add unit tests, integration tests
- **Issue Reporting**: Submit bug reports, feature requests
- **Code Review**: Review Pull Requests

#### Contribution Process

1. Fork this repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Ensure code passes all tests
5. Commit your changes (`git commit -m 'Add some amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Create a Pull Request

### Deutsch

#### Beitragsarten

Wir begrüßen folgende Arten von Beiträgen:

- **Code-Beiträge**: Fehler beheben, neue Funktionen hinzufügen, Leistung optimieren
- **Dokumentationsverbesserungen**: Fehler korrigieren, Erklärungen verbessern, Dokumentation übersetzen
- **Test-Schreiben**: Unit-Tests, Integrationstests hinzufügen
- **Problemberichterstattung**: Fehlerberichte, Funktionsanfragen einreichen
- **Code-Überprüfung**: Pull Requests überprüfen

#### Beitragsprozess

1. Forken Sie dieses Repository
2. Erstellen Sie Ihren Feature-Branch (`git checkout -b feature/amazing-feature`)
3. Machen Sie Ihre Änderungen
4. Stellen Sie sicher, dass der Code alle Tests besteht
5. Committen Sie Ihre Änderungen (`git commit -m 'Add some amazing feature'`)
6. Pushen Sie zum Branch (`git push origin feature/amazing-feature`)
7. Erstellen Sie einen Pull Request

---

## 开发环境设置 / Development Setup / Entwicklungsumgebung

### 中文

#### 系统要求

- **Rust**: 1.70 或更高版本
- **Cargo**: 随 Rust 一起安装
- **Git**: 用于版本控制
- **LLVM**: 18.0 或更高版本（可选，用于优化构建）

#### 构建步骤

```bash
# 克隆仓库
git clone https://github.com/Box-Ecosystem/BoxLang.git
cd boxlang/compiler

# 构建项目
cargo build

# 构建发布版本
cargo build --release

# 运行测试
cargo test

# 运行编译器
cargo run -- --help
```

#### 项目结构

```
boxlang/
├── compiler/          # 编译器源代码
│   ├── src/
│   │   ├── ast/       # 抽象语法树
│   │   ├── codegen/   # 代码生成
│   │   ├── frontend/  # 词法分析和语法分析
│   │   ├── middle/    # 中间表示和优化
│   │   ├── typeck/    # 类型检查
│   │   └── ...
│   └── Cargo.toml
├── std/               # 标准库
├── examples/          # 示例代码
├── docs/              # 文档
└── boxlang-tutorial/  # 教程
```

### English

#### System Requirements

- **Rust**: 1.70 or higher
- **Cargo**: Installed with Rust
- **Git**: For version control
- **LLVM**: 18.0 or higher (optional, for optimized builds)

#### Build Steps

```bash
# Clone the repository
git clone https://github.com/Box-Ecosystem/BoxLang.git
cd boxlang/compiler

# Build the project
cargo build

# Build release version
cargo build --release

# Run tests
cargo test

# Run the compiler
cargo run -- --help
```

#### Project Structure

```
boxlang/
├── compiler/          # Compiler source code
│   ├── src/
│   │   ├── ast/       # Abstract Syntax Tree
│   │   ├── codegen/   # Code generation
│   │   ├── frontend/  # Lexer and parser
│   │   ├── middle/    # IR and optimization
│   │   ├── typeck/    # Type checking
│   │   └── ...
│   └── Cargo.toml
├── std/               # Standard library
├── examples/          # Example code
├── docs/              # Documentation
└── boxlang-tutorial/  # Tutorials
```

### Deutsch

#### Systemanforderungen

- **Rust**: 1.70 oder höher
- **Cargo**: Mit Rust installiert
- **Git**: Für Versionskontrolle
- **LLVM**: 18.0 oder höher (optional, für optimierte Builds)

#### Build-Schritte

```bash
# Repository klonen
git clone https://github.com/Box-Ecosystem/BoxLang.git
cd boxlang/compiler

# Projekt bauen
cargo build

# Release-Version bauen
cargo build --release

# Tests ausführen
cargo test

# Compiler ausführen
cargo run -- --help
```

#### Projektstruktur

```
boxlang/
├── compiler/          # Compiler-Quellcode
│   ├── src/
│   │   ├── ast/       # Abstrakter Syntaxbaum
│   │   ├── codegen/   # Codegenerierung
│   │   ├── frontend/  # Lexer und Parser
│   │   ├── middle/    # IR und Optimierung
│   │   ├── typeck/    # Typprüfung
│   │   └── ...
│   └── Cargo.toml
├── std/               # Standardbibliothek
├── examples/          # Beispielcode
├── docs/              # Dokumentation
└── boxlang-tutorial/  # Tutorials
```

---

## 代码规范 / Code Style / Code-Stil

### 中文

#### Rust 代码规范

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查代码质量
- 遵循 Rust API 指南
- 为公共 API 编写文档注释

```rust
/// 计算两个数的和。
///
/// # Examples
///
/// ```
/// let result = add(2, 3);
/// assert_eq!(result, 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

#### BoxLang 代码规范

- 使用 4 个空格缩进
- 变量命名使用 snake_case
- 类型命名使用 PascalCase
- 常量使用 SCREAMING_SNAKE_CASE
- 为公共函数添加文档注释

```boxlang
/// 计算两个数的和
pub fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
```

#### 提交信息规范

使用约定式提交格式：

- `feat:` 新功能
- `fix:` Bug 修复
- `docs:` 文档更改
- `style:` 代码格式（不影响功能）
- `refactor:` 代码重构
- `test:` 添加或修改测试
- `chore:` 构建过程或辅助工具的变动

示例：
```
feat: 添加模式匹配支持
fix: 修复类型推断错误
docs: 更新安装指南
```

### English

#### Rust Code Style

- Use `cargo fmt` to format code
- Use `cargo clippy` to check code quality
- Follow Rust API guidelines
- Write documentation comments for public APIs

```rust
/// Calculates the sum of two numbers.
///
/// # Examples
///
/// ```
/// let result = add(2, 3);
/// assert_eq!(result, 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

#### BoxLang Code Style

- Use 4 spaces for indentation
- Use snake_case for variable names
- Use PascalCase for type names
- Use SCREAMING_SNAKE_CASE for constants
- Add documentation comments for public functions

```boxlang
/// Calculates the sum of two numbers
pub fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
```

#### Commit Message Convention

Use Conventional Commits format:

- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation changes
- `style:` Code formatting (does not affect functionality)
- `refactor:` Code refactoring
- `test:` Adding or modifying tests
- `chore:` Build process or auxiliary tool changes

Examples:
```
feat: add pattern matching support
fix: fix type inference error
docs: update installation guide
```

### Deutsch

#### Rust-Code-Stil

- Verwenden Sie `cargo fmt` zum Formatieren des Codes
- Verwenden Sie `cargo clippy` zur Überprüfung der Codequalität
- Befolgen Sie die Rust-API-Richtlinien
- Schreiben Sie Dokumentationskommentare für öffentliche APIs

```rust
/// Berechnet die Summe zweier Zahlen.
///
/// # Beispiele
///
/// ```
/// let result = add(2, 3);
/// assert_eq!(result, 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

#### BoxLang-Code-Stil

- Verwenden Sie 4 Leerzeichen für Einrückungen
- Verwenden Sie snake_case für Variablennamen
- Verwenden Sie PascalCase für Typnamen
- Verwenden Sie SCREAMING_SNAKE_CASE für Konstanten
- Fügen Sie Dokumentationskommentare für öffentliche Funktionen hinzu

```boxlang
/// Berechnet die Summe zweier Zahlen
pub fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
```

#### Commit-Nachrichten-Konvention

Verwenden Sie das Conventional Commits-Format:

- `feat:` Neue Funktion
- `fix:` Fehlerbehebung
- `docs:` Dokumentationsänderungen
- `style:` Code-Formatierung (beeinflusst nicht die Funktionalität)
- `refactor:` Code-Refactoring
- `test:` Hinzufügen oder Ändern von Tests
- `chore:` Build-Prozess oder Hilfstool-Änderungen

Beispiele:
```
feat: Mustererkennung hinzufügen
fix: Typrückschlussfehler beheben
docs: Installationsanleitung aktualisieren
```

---

## 提交流程 / Pull Request Process / Pull-Request-Prozess

### 中文

1. **确保代码质量**
   - 运行 `cargo fmt` 格式化代码
   - 运行 `cargo clippy` 检查代码问题
   - 运行 `cargo test` 确保所有测试通过

2. **创建 Pull Request**
   - 提供清晰的标题和描述
   - 关联相关的 Issue
   - 描述您的更改和动机

3. **代码审查**
   - 响应审查意见
   - 进行必要的修改
   - 保持讨论的专业和建设性

4. **合并要求**
   - 至少需要一位维护者的批准
   - 所有 CI 检查必须通过
   - 没有合并冲突

### English

1. **Ensure Code Quality**
   - Run `cargo fmt` to format code
   - Run `cargo clippy` to check for issues
   - Run `cargo test` to ensure all tests pass

2. **Create Pull Request**
   - Provide a clear title and description
   - Link related issues
   - Describe your changes and motivation

3. **Code Review**
   - Respond to review comments
   - Make necessary modifications
   - Keep discussions professional and constructive

4. **Merge Requirements**
   - At least one maintainer approval required
   - All CI checks must pass
   - No merge conflicts

### Deutsch

1. **Code-Qualität sicherstellen**
   - Führen Sie `cargo fmt` aus, um den Code zu formatieren
   - Führen Sie `cargo clippy` aus, um Probleme zu überprüfen
   - Führen Sie `cargo test` aus, um sicherzustellen, dass alle Tests bestehen

2. **Pull Request erstellen**
   - Geben Sie einen klaren Titel und eine Beschreibung an
   - Verknüpfen Sie verwandte Issues
   - Beschreiben Sie Ihre Änderungen und Motivation

3. **Code-Überprüfung**
   - Reagieren Sie auf Überprüfungskommentare
   - Machen Sie notwendige Änderungen
   - Halten Sie Diskussionen professionell und konstruktiv

4. **Merge-Anforderungen**
   - Mindestens eine Genehmigung eines Maintainers erforderlich
   - Alle CI-Checks müssen bestehen
   - Keine Merge-Konflikte

---

## 问题报告 / Issue Reporting / Problemberichterstattung

### 中文

#### Bug 报告

请包含以下信息：

1. **描述**：清晰简洁地描述 Bug
2. **复现步骤**：如何复现该问题
3. **预期行为**：您期望发生什么
4. **实际行为**：实际发生了什么
5. **环境**：
   - 操作系统
   - Rust 版本
   - BoxLang 版本
6. **代码示例**：最小可复现示例

#### 功能请求

请包含以下信息：

1. **描述**：清晰描述您想要的功能
2. **动机**：为什么需要这个功能
3. **建议方案**：如果有的话，描述您建议的实现方式

### English

#### Bug Reports

Please include the following information:

1. **Description**: A clear and concise description of the bug
2. **Steps to Reproduce**: How to reproduce the issue
3. **Expected Behavior**: What you expected to happen
4. **Actual Behavior**: What actually happened
5. **Environment**:
   - Operating System
   - Rust version
   - BoxLang version
6. **Code Sample**: A minimal reproducible example

#### Feature Requests

Please include the following information:

1. **Description**: A clear description of the feature you want
2. **Motivation**: Why this feature is needed
3. **Proposed Solution**: If available, describe your suggested implementation

### Deutsch

#### Fehlerberichte

Bitte fügen Sie folgende Informationen bei:

1. **Beschreibung**: Eine klare und prägnante Beschreibung des Fehlers
2. **Schritte zur Reproduktion**: Wie das Problem reproduziert werden kann
3. **Erwartetes Verhalten**: Was Sie erwartet haben
4. **Tatsächliches Verhalten**: Was tatsächlich passiert ist
5. **Umgebung**:
   - Betriebssystem
   - Rust-Version
   - BoxLang-Version
6. **Code-Beispiel**: Ein minimales reproduzierbares Beispiel

#### Funktionsanfragen

Bitte fügen Sie folgende Informationen bei:

1. **Beschreibung**: Eine klare Beschreibung der gewünschten Funktion
2. **Motivation**: Warum diese Funktion benötigt wird
3. **Vorgeschlagene Lösung**: Falls verfügbar, beschreiben Sie Ihre vorgeschlagene Implementierung

---

## 社区 / Community / Community

### 中文

- **GitHub Issues**: [https://github.com/box-ecosystem/boxlang/issues](https://github.com/box-ecosystem/boxlang/issues)

### English

- **GitHub Issues**: [https://github.com/box-ecosystem/boxlang/issues](https://github.com/box-ecosystem/boxlang/issues)

### Deutsch

- **GitHub Issues**: [https://github.com/box-ecosystem/boxlang/issues](https://github.com/box-ecosystem/boxlang/issues)

---

## 许可证 / License / Lizenz

通过向本项目贡献，您同意您的贡献将根据 MIT 许可证授权。

By contributing to this project, you agree that your contributions will be licensed under the MIT License.

Durch Ihren Beitrag zu diesem Projekt stimmen Sie zu, dass Ihre Beiträge unter der MIT-Lizenz lizenziert werden.

---

## 贡献者许可协议 / Contributor License Agreement / Beitragslizenzvereinbarung

在提交 Pull Request 之前，请确保您已阅读并同意我们的 [贡献者许可协议 (CLA)](CLA.md)。

Before submitting a Pull Request, please ensure you have read and agreed to our [Contributor License Agreement (CLA)](CLA.md).

Bevor Sie einen Pull Request einreichen, stellen Sie bitte sicher, dass Sie unsere [Beitragslizenzvereinbarung (CLA)](CLA.md) gelesen und akzeptiert haben.

---

**最后更新日期 / Last Updated / Zuletzt aktualisiert**: 2026-02-26
