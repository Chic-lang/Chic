use crate::frontend::diagnostics::{Diagnostic, FileId, Span};

mod diagnostics;
mod numeric;
mod state;
mod trivia;

pub use keyword::Keyword;
pub use numeric::{
    NumericBase, NumericExponent, NumericLiteral, NumericLiteralError, NumericLiteralErrorKind,
    NumericLiteralKind, NumericLiteralSuffix, SuffixRestriction,
};
pub use token::{Token, TokenKind};

mod keyword {
    use super::TokenKind;

    /// Reserved keywords recognised by the lexer.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Keyword {
        Namespace,
        Public,
        Private,
        Internal,
        Protected,
        Of,
        Lends,
        Struct,
        Union,
        Enum,
        Class,
        Error,
        Delegate,
        Interface,
        Extension,
        Trait,
        Impl,
        Let,
        Global,
        Import,
        Var,
        In,
        Ref,
        Mut,
        Out,
        Return,
        Using,
        Static,
        If,
        Else,
        Switch,
        Case,
        Default,
        While,
        Do,
        For,
        Foreach,
        Break,
        Continue,
        Goto,
        Throw,
        Throws,
        Try,
        Catch,
        Finally,
        Region,
        Function,
        Fn,
        Sizeof,
        Alignof,
        Checked,
        Unchecked,
        Lock,
        Atomic,
        Yield,
        Const,
        Fixed,
        Unsafe,
        Await,
        When,
        Testcase,
        Readonly,
        Is,
        As,
        And,
        Or,
        Not,
        Operator,
        Implicit,
        Explicit,
        Get,
        Set,
        Init,
        Convenience,
        Nameof,
        Where,
        Notnull,
        View,
        New,
        Required,
        Dyn,
        Type,
        Typealias,
    }

    impl Keyword {
        #[must_use]
        pub fn from_ident(ident: &str) -> Option<Self> {
            KEYWORDS
                .iter()
                .find_map(|(name, keyword)| (*name == ident).then_some(*keyword))
        }

        pub fn token_kind(self) -> TokenKind {
            TokenKind::Keyword(self)
        }
    }

    const KEYWORDS: &[(&str, Keyword)] = &[
        ("nameof", Keyword::Nameof),
        ("where", Keyword::Where),
        ("notnull", Keyword::Notnull),
        ("new", Keyword::New),
        ("required", Keyword::Required),
        ("namespace", Keyword::Namespace),
        ("public", Keyword::Public),
        ("private", Keyword::Private),
        ("internal", Keyword::Internal),
        ("protected", Keyword::Protected),
        ("of", Keyword::Of),
        ("lends", Keyword::Lends),
        ("struct", Keyword::Struct),
        ("union", Keyword::Union),
        ("enum", Keyword::Enum),
        ("class", Keyword::Class),
        ("delegate", Keyword::Delegate),
        ("interface", Keyword::Interface),
        ("extension", Keyword::Extension),
        ("global", Keyword::Global),
        ("import", Keyword::Import),
        ("let", Keyword::Let),
        ("var", Keyword::Var),
        ("in", Keyword::In),
        ("ref", Keyword::Ref),
        ("mut", Keyword::Mut),
        ("out", Keyword::Out),
        ("return", Keyword::Return),
        ("using", Keyword::Using),
        ("static", Keyword::Static),
        ("if", Keyword::If),
        ("else", Keyword::Else),
        ("switch", Keyword::Switch),
        ("case", Keyword::Case),
        ("default", Keyword::Default),
        ("while", Keyword::While),
        ("do", Keyword::Do),
        ("for", Keyword::For),
        ("foreach", Keyword::Foreach),
        ("break", Keyword::Break),
        ("continue", Keyword::Continue),
        ("goto", Keyword::Goto),
        ("throw", Keyword::Throw),
        ("throws", Keyword::Throws),
        ("try", Keyword::Try),
        ("catch", Keyword::Catch),
        ("finally", Keyword::Finally),
        ("region", Keyword::Region),
        ("function", Keyword::Function),
        ("fn", Keyword::Fn),
        ("sizeof", Keyword::Sizeof),
        ("alignof", Keyword::Alignof),
        ("checked", Keyword::Checked),
        ("unchecked", Keyword::Unchecked),
        ("lock", Keyword::Lock),
        ("atomic", Keyword::Atomic),
        ("yield", Keyword::Yield),
        ("const", Keyword::Const),
        ("fixed", Keyword::Fixed),
        ("unsafe", Keyword::Unsafe),
        ("await", Keyword::Await),
        ("when", Keyword::When),
        ("testcase", Keyword::Testcase),
        ("readonly", Keyword::Readonly),
        ("is", Keyword::Is),
        ("as", Keyword::As),
        ("and", Keyword::And),
        ("or", Keyword::Or),
        ("not", Keyword::Not),
        ("operator", Keyword::Operator),
        ("implicit", Keyword::Implicit),
        ("explicit", Keyword::Explicit),
        ("get", Keyword::Get),
        ("set", Keyword::Set),
        ("init", Keyword::Init),
        ("convenience", Keyword::Convenience),
        ("dyn", Keyword::Dyn),
        ("type", Keyword::Type),
        ("view", Keyword::View),
        ("typealias", Keyword::Typealias),
    ];
}

mod token {
    use super::keyword::Keyword;
    use super::numeric::NumericLiteral;
    use crate::frontend::literals::{CharLiteral, StringLiteral};

    /// Token emitted by the lexer.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Token {
        pub kind: TokenKind,
        pub lexeme: String,
        pub span: super::Span,
    }

    /// Token categories understood by the parser scaffold.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum TokenKind {
        Identifier,
        NumberLiteral(NumericLiteral),
        StringLiteral(StringLiteral),
        CharLiteral(CharLiteral),
        Keyword(Keyword),
        Punctuation(char),
        Operator(&'static str),
        Comment,
        DocComment,
        Whitespace,
        Unknown(char),
    }
}

/// Result of lexing a source string.
#[derive(Debug, Default)]
pub struct LexOutput {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
    pub file_id: FileId,
}

/// Lex an entire source string.
#[must_use]
pub fn lex(source: &str) -> LexOutput {
    state::run(source)
}

/// Lex an entire source string with a known file id.
#[must_use]
pub fn lex_with_file(source: &str, file_id: FileId) -> LexOutput {
    state::run_with_file(source, file_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_identifier_sequence() {
        let output = lex("alpha beta");
        let idents: Vec<_> = output
            .tokens
            .iter()
            .filter(|token| matches!(token.kind, TokenKind::Identifier))
            .collect();
        assert_eq!(idents.len(), 2);
        assert!(output.diagnostics.is_empty());
    }

    #[test]
    fn primitive_names_are_not_keywords() {
        let output = lex("int bool string decimal");
        let idents: Vec<_> = output
            .tokens
            .iter()
            .filter(|token| matches!(token.kind, TokenKind::Identifier))
            .map(|token| token.lexeme.as_str())
            .collect();
        assert_eq!(idents, ["int", "bool", "string", "decimal"]);
        assert!(
            output
                .tokens
                .iter()
                .all(|token| !matches!(token.kind, TokenKind::Keyword(_)))
        );
        assert!(output.diagnostics.is_empty());
    }

    #[test]
    fn lexes_unicode_identifiers_and_emoji() {
        let output = lex(r#"let π = 3.14159; let 你好 = "你好世界"; let 🐶🐮 = "dogcow";"#);
        let idents: Vec<_> = output
            .tokens
            .iter()
            .filter(|token| matches!(token.kind, TokenKind::Identifier))
            .map(|token| token.lexeme.as_str())
            .collect();
        assert_eq!(idents, ["π", "你好", "🐶🐮"]);
        assert!(output.diagnostics.is_empty());
    }

    #[test]
    fn reports_and_normalises_non_nfc_identifier() {
        let output = lex("let cafe\u{0301} = 1;");
        let idents: Vec<_> = output
            .tokens
            .iter()
            .filter(|token| matches!(token.kind, TokenKind::Identifier))
            .collect();
        assert_eq!(idents.len(), 1);
        assert_eq!(idents[0].lexeme, "café");
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diag| diag.message.contains("NFC")),
            "expected NFC diagnostic, got {:?}",
            output.diagnostics
        );
        assert!(
            output.diagnostics.iter().any(|diag| {
                diag.suggestions
                    .iter()
                    .any(|s| s.replacement.as_deref() == Some("café"))
            }),
            "expected NFC suggestion, got {:?}",
            output.diagnostics
        );
    }

    #[test]
    fn rejects_forbidden_invisible_identifier_codepoints() {
        let output = lex("let bad\u{200F}name = 1;");
        let idents: Vec<_> = output
            .tokens
            .iter()
            .filter(|token| matches!(token.kind, TokenKind::Identifier))
            .map(|token| token.lexeme.as_str())
            .collect();
        assert_eq!(idents, ["badname"]);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diag| diag.message.contains("200F")),
            "expected bidi control diagnostic, got {:?}",
            output.diagnostics
        );
        assert!(
            output.diagnostics.iter().any(|diag| {
                diag.suggestions
                    .iter()
                    .any(|s| s.replacement.as_deref() == Some("badname"))
            }),
            "expected sanitisation suggestion, got {:?}",
            output.diagnostics
        );
    }
}
