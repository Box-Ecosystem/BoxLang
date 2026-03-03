//! Diagnostic system for unified error handling
//!
//! Provides structured error reporting and collection across all compiler stages.

use std::fmt;

/// Severity level of a diagnostic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Note => write!(f, "note"),
            Severity::Help => write!(f, "help"),
        }
    }
}

/// A single diagnostic message
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub code: Option<String>,
    pub span: Option<Span>,
    pub labels: Vec<Label>,
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn new(severity: Severity, message: impl Into<String>) -> Self {
        Self {
            severity,
            message: message.into(),
            code: None,
            span: None,
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(Severity::Error, message)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, message)
    }

    pub fn note(message: impl Into<String>) -> Self {
        Self::new(Severity::Note, message)
    }

    pub fn help(message: impl Into<String>) -> Self {
        Self::new(Severity::Help, message)
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

/// A label pointing to a specific span in the source code
#[derive(Debug, Clone)]
pub struct Label {
    pub span: Span,
    pub message: String,
    pub style: LabelStyle,
}

impl Label {
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            style: LabelStyle::Primary,
        }
    }

    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            style: LabelStyle::Secondary,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelStyle {
    Primary,
    Secondary,
}

/// Source code span information
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn merge(&self, other: &Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// Collects and manages diagnostics during compilation
#[derive(Debug, Default)]
pub struct DiagnosticCollector {
    diagnostics: Vec<Diagnostic>,
    source: String,
    source_name: String,
}

impl DiagnosticCollector {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            source: String::new(),
            source_name: String::new(),
        }
    }

    pub fn with_source(mut self, name: impl Into<String>, source: impl Into<String>) -> Self {
        self.source_name = name.into();
        self.source = source.into();
        self
    }

    pub fn set_source(&mut self, name: impl Into<String>, source: impl Into<String>) {
        self.source_name = name.into();
        self.source = source.into();
    }

    pub fn emit(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn error(&mut self, message: impl Into<String>) {
        self.emit(Diagnostic::error(message));
    }

    pub fn warning(&mut self, message: impl Into<String>) {
        self.emit(Diagnostic::warning(message));
    }

    pub fn note(&mut self, message: impl Into<String>) {
        self.emit(Diagnostic::note(message));
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    pub fn has_warnings(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Warning)
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn errors(&self) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect()
    }

    pub fn warnings(&self) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect()
    }

    pub fn clear(&mut self) {
        self.diagnostics.clear();
    }

    /// Format and print all diagnostics
    pub fn print_all(&self) {
        for diagnostic in &self.diagnostics {
            self.print_diagnostic(diagnostic);
        }
    }

    fn print_diagnostic(&self, diagnostic: &Diagnostic) {
        use crate::ui::{colors, ui};

        let ui = ui();
        let color = match diagnostic.severity {
            Severity::Error => colors::RED,
            Severity::Warning => colors::YELLOW,
            Severity::Note => colors::BLUE,
            Severity::Help => colors::GREEN,
        };

        // Print header
        let severity_str = format!("{}", diagnostic.severity);
        if let Some(ref code) = diagnostic.code {
            println!(
                "{}[{} {}]: {}",
                ui.color(&severity_str, color),
                ui.color("E", color),
                code,
                diagnostic.message
            );
        } else {
            println!(
                "{}: {}",
                ui.color(&severity_str, color),
                diagnostic.message
            );
        }

        // Print location if available
        if let Some(ref span) = diagnostic.span {
            let (line, column) = self.span_to_line_column(span);
            println!(
                "  {} {}:{}:{}",
                ui.color("-->", colors::BRIGHT_BLACK),
                self.source_name,
                line,
                column
            );

            // Print source line
            if let Some(source_line) = self.get_source_line(line) {
                println!(
                    "   {} {}",
                    ui.color(&format!("{:4}", line), colors::BRIGHT_BLACK),
                    source_line
                );

                // Print underline
                let underline = self.generate_underline(span, column);
                println!(
                    "      {}{}",
                    " ".repeat(column.saturating_sub(1)),
                    ui.color(&underline, color)
                );
            }
        }

        // Print labels
        for label in &diagnostic.labels {
            let (line, column) = self.span_to_line_column(&label.span);
            let style_color = match label.style {
                LabelStyle::Primary => color,
                LabelStyle::Secondary => colors::CYAN,
            };
            println!(
                "  {} {}:{}: {}",
                ui.color("=>", style_color),
                line,
                column,
                label.message
            );
        }

        // Print notes
        for note in &diagnostic.notes {
            println!("  {} note: {}", ui.color("=", colors::BRIGHT_BLACK), note);
        }

        println!();
    }

    fn span_to_line_column(&self, span: &Span) -> (usize, usize) {
        if self.source.is_empty() || span.start >= self.source.len() {
            return (0, 0);
        }

        let source_before = &self.source[..span.start];
        let line = source_before.matches('\n').count() + 1;
        let line_start = source_before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let column = span.start - line_start + 1;

        (line, column)
    }

    fn get_source_line(&self, line_num: usize) -> Option<&str> {
        self.source.lines().nth(line_num.saturating_sub(1))
    }

    fn generate_underline(&self, span: &Span, _column: usize) -> String {
        let length = span.end.saturating_sub(span.start).max(1);
        "^".repeat(length)
    }

    /// Convert to a Result, returning Err if there are errors
    pub fn into_result<T>(self, value: T) -> Result<T, Vec<Diagnostic>> {
        if self.has_errors() {
            Err(self.diagnostics)
        } else {
            Ok(value)
        }
    }
}

/// A convenient type alias for results with diagnostics
pub type DiagnosticResult<T> = Result<T, Vec<Diagnostic>>;

/// Extension trait for converting errors to diagnostics
pub trait IntoDiagnostic {
    fn into_diagnostic(self) -> Diagnostic;
}

/// Macro for easy diagnostic creation
#[macro_export]
macro_rules! diag {
    (error, $msg:expr) => {
        $crate::diagnostics::Diagnostic::error($msg)
    };
    (warning, $msg:expr) => {
        $crate::diagnostics::Diagnostic::warning($msg)
    };
    (note, $msg:expr) => {
        $crate::diagnostics::Diagnostic::note($msg)
    };
    (help, $msg:expr) => {
        $crate::diagnostics::Diagnostic::help($msg)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_creation() {
        let diag = Diagnostic::error("test error").with_code("E0001");
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.message, "test error");
        assert_eq!(diag.code, Some("E0001".to_string()));
    }

    #[test]
    fn test_collector() {
        let mut collector = DiagnosticCollector::new();
        collector.error("error 1");
        collector.warning("warning 1");
        collector.error("error 2");

        assert!(collector.has_errors());
        assert!(collector.has_warnings());
        assert_eq!(collector.len(), 3);
        assert_eq!(collector.errors().len(), 2);
        assert_eq!(collector.warnings().len(), 1);
    }

    #[test]
    fn test_span_merge() {
        let span1 = Span::new(0, 10);
        let span2 = Span::new(5, 15);
        let merged = span1.merge(&span2);
        assert_eq!(merged.start, 0);
        assert_eq!(merged.end, 15);
    }
}
