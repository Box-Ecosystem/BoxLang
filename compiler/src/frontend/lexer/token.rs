use logos::Logos;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum StringLitKind {
    Simple(String),
    Interpolated(String),
}

impl fmt::Display for StringLitKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StringLitKind::Simple(s) => write!(f, "\"{}\"", s),
            StringLitKind::Interpolated(s) => write!(f, "\"{}\"", s),
        }
    }
}

/// Token type for BoxLang
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\f]+")]
#[logos(skip r"//[^\n]*")]
#[logos(skip r"/\*([^*]|\*[^/])*\*/")]
#[logos(skip r"\n|\r\n")]
pub enum Token {
    // Keywords
    #[token("module")]
    Module,
    #[token("import")]
    Import,
    #[token("pub")]
    Pub,
    #[token("fn")]
    Fn,
    #[token("let")]
    Let,
    #[token("mut")]
    Mut,
    #[token("const")]
    Const,
    #[token("static")]
    Static,
    #[token("struct")]
    Struct,
    #[token("enum")]
    Enum,
    #[token("impl")]
    Impl,
    #[token("dyn")]
    Dyn,
    #[token("trait")]
    Trait,
    #[token("for")]
    For,
    #[token("while")]
    While,
    #[token("loop")]
    Loop,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("match")]
    Match,
    #[token("return")]
    Return,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("async")]
    Async,
    #[token("await")]
    Await,
    #[token("spawn")]
    Spawn,
    #[token("defer")]
    Defer,
    #[token("unsafe")]
    Unsafe,
    #[token("extern")]
    Extern,
    #[token("as")]
    As,
    #[token("in")]
    In,
    #[token("ref")]
    Ref,
    #[token("self")]
    SelfLower,
    #[token("Self")]
    SelfUpper,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("void")]
    Void,
    #[token("null")]
    Null,
    #[token("use")]
    Use,
    #[token("mod")]
    Mod,
    #[token("type")]
    Type,
    #[token("where")]
    Where,
    #[token("move")]
    Move,
    #[token("box")]
    Box,

    // New keywords for Phase 2
    #[token("try")]
    Try,
    #[token("catch")]
    Catch,
    #[token("finally")]
    Finally,
    #[token("throw")]
    Throw,
    #[token("ok")]
    Ok,
    #[token("err")]
    Err,
    #[token("some")]
    Some,
    #[token("none")]
    None,
    #[token("channel")]
    Channel,
    #[token("select")]
    Select,
    #[token("timeout")]
    Timeout,
    #[token("arena")]
    Arena,
    #[token("pool")]
    Pool,
    #[token("atomic")]
    Atomic,
    #[token("fence")]
    Fence,
    #[token("asm")]
    Asm,
    #[token("yield")]
    Yield,
    #[token("parallel")]
    Parallel,
    #[token("scope")]
    Scope,
    #[token("typeclass")]
    Typeclass,
    #[token("specialized")]
    Specialized,
    #[token("typestate")]
    Typestate,
    #[token("state")]
    State,
    #[token("derive")]
    Derive,
    #[token("macro")]
    Macro,
    #[token("rules")]
    Rules,
    
    // FFI keywords
    #[token("callback")]
    Callback,
    #[token("safe")]
    Safe,
    #[token("link")]
    Link,
    #[token("section")]
    Section,
    #[token("deprecated")]
    Deprecated,

    // Types
    #[token("i8")]
    I8,
    #[token("i16")]
    I16,
    #[token("i32")]
    I32,
    #[token("i64")]
    I64,
    #[token("u8")]
    U8,
    #[token("u16")]
    U16,
    #[token("u32")]
    U32,
    #[token("u64")]
    U64,
    #[token("f32")]
    F32,
    #[token("f64")]
    F64,
    #[token("bool")]
    Bool,
    #[token("char")]
    Char,
    #[token("str")]
    Str,
    #[token("String")]
    String,
    #[token("Vec")]
    Vec,
    #[token("Option")]
    Option,
    #[token("Result")]
    Result,

    // Identifiers and literals (lower priority than keywords)
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string(), priority = 1)]
    Ident(String),

    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    Integer(i64),

    #[regex(r"0x[0-9a-fA-F]+", |lex| i64::from_str_radix(&lex.slice()[2..], 16).ok())]
    HexInteger(i64),

    #[regex(r"0b[01]+", |lex| i64::from_str_radix(&lex.slice()[2..], 2).ok())]
    BinInteger(i64),

    // Float literals - require at least one digit after the decimal point
    // This prevents `0..5` from being parsed as `0.` + `.5`
    #[regex(r"[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
    Float(f64),

    // String literals with interpolation support
    // Format: "hello {name}" or "value: {x:.2f}"
    #[regex(r#""([^"\\{}]|\\.)*""#, |lex| {
        let s = lex.slice();
        Some(StringLitKind::Simple(s[1..s.len()-1].to_string()))
    })]
    #[regex(r#""([^"\\]|\\.)*\{[^}]*\}([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        Some(StringLitKind::Interpolated(s[1..s.len()-1].to_string()))
    })]
    StringLit(StringLitKind),

    #[regex(r"'([^'\\]|\\.)'", |lex| {
        let s = lex.slice();
        Some(s[1..s.len()-1].chars().next()?)
    })]
    CharLit(char),

    // Operators
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("++")]
    PlusPlus,
    #[token("--")]
    MinusMinus,

    #[token("=")]
    Eq,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,

    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("!")]
    Not,

    #[token("&")]
    And,
    #[token("|")]
    Or,
    #[token("^")]
    Xor,
    #[token("<<")]
    Shl,
    #[token(">>")]
    Shr,
    #[token("~")]
    Tilde,

    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,
    #[token("%=")]
    PercentEq,
    #[token("&=")]
    AndEq,
    #[token("|=")]
    OrEq,
    #[token("^=")]
    XorEq,
    #[token("<<=")]
    ShlEq,
    #[token(">>=")]
    ShrEq,

    // New operators for Phase 2
    #[token("|>")]
    Pipe, // Pipeline operator
    #[token("??")]
    QuestionQuestion, // Null coalescing
    #[token("?:")]
    Elvis, // Elvis operator
    #[token("**")]
    Power, // Power operator
    #[token("**=")]
    PowerEq,

    // Punctuation
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,

    #[token(";")]
    Semi,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("..")]
    DotDot,
    #[token("...")]
    DotDotDot,
    #[token(":")]
    Colon,
    #[token("::")]
    ColonColon,
    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,
    #[token("#")]
    Hash,
    #[token("$")]
    Dollar,
    #[token("?")]
    Question,
    #[token("@")]
    At,
    #[token("_", priority = 2)]
    Underscore,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Ident(s) => write!(f, "{}", s),
            Token::Integer(i) => write!(f, "{}", i),
            Token::HexInteger(i) => write!(f, "0x{:x}", i),
            Token::BinInteger(i) => write!(f, "0b{:b}", i),
            Token::Float(fl) => write!(f, "{}", fl),
            Token::StringLit(s) => write!(f, "\"{}\"", s),
            Token::CharLit(c) => write!(f, "'{}'", c),
            _ => write!(f, "{:?}", self),
        }
    }
}

/// A token with its position in the source
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: logos::Span,
    pub line: usize,
    pub column: usize,
}

impl SpannedToken {
    pub fn new(token: Token, span: logos::Span, line: usize, column: usize) -> Self {
        Self {
            token,
            span,
            line,
            column,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keywords() {
        let mut lex = Token::lexer("fn let mut pub struct impl");
        assert_eq!(lex.next(), Some(Ok(Token::Fn)));
        assert_eq!(lex.next(), Some(Ok(Token::Let)));
        assert_eq!(lex.next(), Some(Ok(Token::Mut)));
        assert_eq!(lex.next(), Some(Ok(Token::Pub)));
        assert_eq!(lex.next(), Some(Ok(Token::Struct)));
        assert_eq!(lex.next(), Some(Ok(Token::Impl)));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn test_identifiers() {
        let mut lex = Token::lexer("foo _bar baz123");
        assert_eq!(lex.next(), Some(Ok(Token::Ident("foo".to_string()))));
        assert_eq!(lex.next(), Some(Ok(Token::Ident("_bar".to_string()))));
        assert_eq!(lex.next(), Some(Ok(Token::Ident("baz123".to_string()))));
    }

    #[test]
    fn test_numbers() {
        let mut lex = Token::lexer("42 3.14 0xFF 0b1010");
        assert_eq!(lex.next(), Some(Ok(Token::Integer(42))));
        assert_eq!(lex.next(), Some(Ok(Token::Float(3.14))));
        assert_eq!(lex.next(), Some(Ok(Token::HexInteger(255))));
        assert_eq!(lex.next(), Some(Ok(Token::BinInteger(10))));
    }

    #[test]
    fn test_strings() {
        let mut lex = Token::lexer(r#""hello" 'a'"#);
        assert_eq!(
            lex.next(),
            Some(Ok(Token::StringLit(StringLitKind::Simple(
                "hello".to_string()
            ))))
        );
        assert_eq!(lex.next(), Some(Ok(Token::CharLit('a'))));
    }

    #[test]
    fn test_operators() {
        let mut lex = Token::lexer("+ - * / == != <= >= && ||");
        assert_eq!(lex.next(), Some(Ok(Token::Plus)));
        assert_eq!(lex.next(), Some(Ok(Token::Minus)));
        assert_eq!(lex.next(), Some(Ok(Token::Star)));
        assert_eq!(lex.next(), Some(Ok(Token::Slash)));
        assert_eq!(lex.next(), Some(Ok(Token::EqEq)));
        assert_eq!(lex.next(), Some(Ok(Token::NotEq)));
        assert_eq!(lex.next(), Some(Ok(Token::Le)));
        assert_eq!(lex.next(), Some(Ok(Token::Ge)));
        assert_eq!(lex.next(), Some(Ok(Token::AndAnd)));
        assert_eq!(lex.next(), Some(Ok(Token::OrOr)));
    }

    #[test]
    fn test_comments() {
        // Line comments are skipped, and newlines are also skipped now
        let mut lex = Token::lexer("fn // this is a comment\nlet");
        assert_eq!(lex.next(), Some(Ok(Token::Fn)));
        assert_eq!(lex.next(), Some(Ok(Token::Let)));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn test_block_comments() {
        let mut lex = Token::lexer("fn /* block comment */ let");
        assert_eq!(lex.next(), Some(Ok(Token::Fn)));
        assert_eq!(lex.next(), Some(Ok(Token::Let)));
    }
}
