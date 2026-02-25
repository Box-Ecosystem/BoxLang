pub mod token;

use logos::Logos;
use token::{SpannedToken, Token};

/// Lexer for BoxLang source code
pub struct Lexer<'a> {
    inner: logos::Lexer<'a, Token>,
    source: &'a str,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            inner: Token::lexer(source),
            source,
            line: 1,
            column: 1,
        }
    }

    /// Get the next token with span information
    pub fn next_token(&mut self) -> Option<SpannedToken> {
        let token_result = self.inner.next()?;
        let span = self.inner.span();

        // Calculate line and column based on the start of the token
        let start = span.start;
        let source_before = &self.source[..start];

        // Count newlines before this token to get the line number
        let line = source_before.matches('\n').count() + 1;

        // Find the start of the current line
        let line_start = source_before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let column = start - line_start + 1;

        match token_result {
            Ok(token) => Some(SpannedToken::new(token, span, line, column)),
            Err(_) => {
                // Handle error token - skip it and continue
                self.next_token()
            }
        }
    }

    /// Tokenize the entire source into a vector of tokens
    pub fn tokenize(mut self) -> Vec<SpannedToken> {
        let mut tokens = Vec::new();
        while let Some(token) = self.next_token() {
            tokens.push(token);
        }
        tokens
    }

    /// Get the source string
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// Get the current line number
    pub fn line(&self) -> usize {
        self.line
    }

    /// Get the current column number
    pub fn column(&self) -> usize {
        self.column
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = SpannedToken;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

/// Convenience function to tokenize source code
pub fn tokenize(source: &str) -> Vec<SpannedToken> {
    Lexer::new(source).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_basic() {
        let source = "fn main() { }";
        let tokens = tokenize(source);

        // "fn main() { }" produces 6 tokens: Fn, Ident("main"), LParen, RParen, LBrace, RBrace
        assert_eq!(
            tokens.len(),
            6,
            "Expected 6 tokens for 'fn main() {{ }}', got {:?}",
            tokens
                .iter()
                .map(|t| format!("{:?}", t.token))
                .collect::<Vec<_>>()
        );
        assert!(matches!(tokens[0].token, Token::Fn));
        assert!(matches!(tokens[1].token, Token::Ident(ref s) if s == "main"));
        assert!(matches!(tokens[2].token, Token::LParen));
        assert!(matches!(tokens[3].token, Token::RParen));
        assert!(matches!(tokens[4].token, Token::LBrace));
        assert!(matches!(tokens[5].token, Token::RBrace));
    }

    #[test]
    fn test_lexer_function() {
        let source = r#"
pub fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
"#;
        let tokens = tokenize(source);

        // Check that we got the right tokens
        let token_types: Vec<_> = tokens.iter().map(|t| &t.token).collect();

        assert!(matches!(token_types[0], Token::Pub));
        assert!(matches!(token_types[1], Token::Fn));
        assert!(matches!(token_types[2], Token::Ident(ref s) if s == "add"));
        assert!(matches!(token_types[3], Token::LParen));
    }

    #[test]
    fn test_lexer_line_column() {
        let source = "fn\nmain()";
        let tokens = tokenize(source);

        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[0].column, 1);

        assert_eq!(tokens[1].line, 2); // Newline increments line
        assert_eq!(tokens[1].column, 1);
    }
}
