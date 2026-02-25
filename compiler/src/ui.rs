//! UI formatting module for BoxLang compiler
//!
//! Provides beautiful, zetboxos-style output formatting with colors,
//! progress indicators, and structured information display.

use std::io::{self, Write};
use std::sync::OnceLock;
use std::time::Instant;

/// Color codes for terminal output
#[allow(dead_code)]
pub mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const ITALIC: &str = "\x1b[3m";
    pub const UNDERLINE: &str = "\x1b[4m";

    // Foreground colors
    pub const BLACK: &str = "\x1b[30m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";

    // Bright foreground colors
    pub const BRIGHT_BLACK: &str = "\x1b[90m";
    pub const BRIGHT_RED: &str = "\x1b[91m";
    pub const BRIGHT_GREEN: &str = "\x1b[92m";
    pub const BRIGHT_YELLOW: &str = "\x1b[93m";
    pub const BRIGHT_BLUE: &str = "\x1b[94m";
    pub const BRIGHT_MAGENTA: &str = "\x1b[95m";
    pub const BRIGHT_CYAN: &str = "\x1b[96m";
    pub const BRIGHT_WHITE: &str = "\x1b[97m";

    // Background colors
    pub const BG_BLACK: &str = "\x1b[40m";
    pub const BG_RED: &str = "\x1b[41m";
    pub const BG_GREEN: &str = "\x1b[42m";
    pub const BG_YELLOW: &str = "\x1b[43m";
    pub const BG_BLUE: &str = "\x1b[44m";
    pub const BG_MAGENTA: &str = "\x1b[45m";
    pub const BG_CYAN: &str = "\x1b[46m";
    pub const BG_WHITE: &str = "\x1b[47m";
}

/// zetboxos-style UI formatter
pub struct UI {
    use_colors: bool,
    verbose: bool,
    start_time: Instant,
}

impl UI {
    /// Create a new UI formatter
    pub fn new() -> Self {
        Self {
            use_colors: Self::supports_colors(),
            verbose: false,
            start_time: Instant::now(),
        }
    }

    /// Create a new UI formatter with verbose mode
    pub fn verbose() -> Self {
        Self {
            use_colors: Self::supports_colors(),
            verbose: true,
            start_time: Instant::now(),
        }
    }

    /// Check if terminal supports colors
    fn supports_colors() -> bool {
        // Check if running in a terminal that supports colors
        if let Ok(term) = std::env::var("TERM") {
            if term == "dumb" {
                return false;
            }
        }

        // Check if NO_COLOR is set
        if std::env::var("NO_COLOR").is_ok() {
            return false;
        }

        // Windows 10+ supports ANSI colors
        #[cfg(windows)]
        {
            return true;
        }

        #[cfg(not(windows))]
        {
            return true;
        }
    }

    /// Print a colorized string
    pub fn color(&self, text: &str, color: &str) -> String {
        if self.use_colors {
            format!("{}{}{}", color, text, colors::RESET)
        } else {
            text.to_string()
        }
    }

    /// Print the header section
    pub fn header(&self, title: &str) {
        let width = 60;
        let line = "─".repeat(width);

        println!();
        println!("{}", self.color(&line, colors::BLUE));
        println!("{}", self.color(&format!("🔷 {}", title), colors::BOLD));
        println!("{}", self.color(&line, colors::BLUE));
        println!();
    }

    /// Print a section header
    pub fn section(&self, title: &str) {
        println!("{}", self.color(&format!("▶ {}", title), colors::CYAN));
    }

    /// Print an info message
    pub fn info(&self, label: &str, value: &str) {
        println!(
            "    {} {}",
            self.color(&format!("• {}", label), colors::BRIGHT_BLACK),
            self.color(value, colors::WHITE)
        );
    }

    /// Print a success message
    pub fn success(&self, message: &str) {
        println!(
            "  {} {}",
            self.color("✓", colors::GREEN),
            self.color(message, colors::GREEN)
        );
    }

    /// Print a warning message
    pub fn warning(&self, message: &str) {
        println!(
            "  {} {}",
            self.color("⚠", colors::YELLOW),
            self.color(message, colors::YELLOW)
        );
    }

    /// Print an error message
    pub fn error(&self, message: &str) {
        println!(
            "  {} {}",
            self.color("✗", colors::RED),
            self.color(message, colors::RED)
        );
    }

    /// Print a step in progress
    pub fn step(&self, current: usize, total: usize, message: &str) {
        println!(
            "    {} {}",
            self.color(&format!("Step {}/{}", current, total), colors::BRIGHT_BLACK),
            message
        );
    }

    /// Print elapsed time
    pub fn elapsed(&self) -> String {
        let elapsed = self.start_time.elapsed();
        format!("{:.1}s", elapsed.as_secs_f64())
    }

    /// Print a final success message
    pub fn final_success(&self, message: &str) {
        let line = "─".repeat(60);
        println!();
        println!("{}", self.color(&line, colors::GREEN));
        println!(
            "{} {} {}",
            self.color("✓", colors::GREEN),
            self.color(message, colors::BOLD),
            self.color(&format!("({})", self.elapsed()), colors::BRIGHT_BLACK)
        );
        println!("{}", self.color(&line, colors::GREEN));
        println!();
    }

    /// Print a final error message
    pub fn final_error(&self, message: &str) {
        let line = "─".repeat(60);
        println!();
        println!("{}", self.color(&line, colors::RED));
        println!(
            "{} {} {}",
            self.color("✗", colors::RED),
            self.color(message, colors::BOLD),
            self.color(&format!("({})", self.elapsed()), colors::BRIGHT_BLACK)
        );
        println!("{}", self.color(&line, colors::RED));
        println!();
    }

    /// Print a divider line
    pub fn divider(&self) {
        println!("{}", self.color(&"─".repeat(60), colors::BLUE));
    }

    /// Print a code block
    pub fn code_block(&self, title: &str, content: &str) {
        let width = 60;
        let top = format!("┌{:─^width$}┐", format!(" {} ", title), width = width - 2);
        let bottom = format!("└{:─>width$}┘", "", width = width - 2);

        println!("{}", self.color(&top, colors::BLUE));
        for line in content.lines() {
            println!(
                "{} {:<width$} {}",
                self.color("│", colors::BLUE),
                line,
                self.color("│", colors::BLUE),
                width = width - 4
            );
        }
        println!("{}", self.color(&bottom, colors::BLUE));
    }

    /// Print a progress bar
    pub fn progress(&self, current: usize, total: usize, message: &str) {
        let width = 40;
        let filled = (current * width) / total;
        let empty = width - filled;

        let bar = format!(
            "[{}{}]",
            self.color(&"█".repeat(filled), colors::GREEN),
            self.color(&"░".repeat(empty), colors::BRIGHT_BLACK)
        );

        let percentage = (current * 100) / total;

        print!(
            "\r  {} {} {}% - {}",
            bar,
            self.color(&format!("{:>3}", percentage), colors::BRIGHT_CYAN),
            percentage,
            message
        );

        if current == total {
            println!();
        } else {
            // Ignore flush errors in production - they're not critical
            let _ = io::stdout().flush();
        }
    }

    /// Print verbose information
    pub fn verbose_info(&self, message: &str) {
        if self.verbose {
            println!(
                "    {} {}",
                self.color("ℹ", colors::BRIGHT_BLUE),
                self.color(message, colors::BRIGHT_BLACK)
            );
        }
    }
}

impl Default for UI {
    fn default() -> Self {
        Self::new()
    }
}

/// Global UI instance for convenience
static GLOBAL_UI: OnceLock<UI> = OnceLock::new();

/// Initialize the global UI
pub fn init_ui(verbose: bool) {
    let ui = if verbose { UI::verbose() } else { UI::new() };
    let _ = GLOBAL_UI.set(ui);
}

/// Get the global UI instance
pub fn ui() -> &'static UI {
    // Use a default UI if not initialized (for safety in production)
    GLOBAL_UI.get_or_init(UI::new)
}

/// Print header using global UI
pub fn header(title: &str) {
    ui().header(title);
}

/// Print section using global UI
pub fn section(title: &str) {
    ui().section(title);
}

/// Print info using global UI
pub fn info(label: &str, value: &str) {
    ui().info(label, value);
}

/// Print success using global UI
pub fn success(message: &str) {
    ui().success(message);
}

/// Print warning using global UI
pub fn warning(message: &str) {
    ui().warning(message);
}

/// Print error using global UI
pub fn error(message: &str) {
    ui().error(message);
}

/// Print final success using global UI
pub fn final_success(message: &str) {
    ui().final_success(message);
}

/// Print final error using global UI
pub fn final_error(message: &str) {
    ui().final_error(message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_creation() {
        let ui = UI::new();
        assert!(!ui.verbose);
    }

    #[test]
    fn test_ui_verbose() {
        let ui = UI::verbose();
        assert!(ui.verbose);
    }

    #[test]
    fn test_color_output() {
        let ui = UI::new();
        let colored = ui.color("test", colors::RED);
        assert!(colored.contains("test"));
    }
}
