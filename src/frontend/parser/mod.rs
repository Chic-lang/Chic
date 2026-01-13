use std::cell::{Ref, RefMut};
use std::collections::{HashMap, HashSet};
use std::error::Error as StdError;
use std::fmt;
use std::mem::take;
use std::path::PathBuf;

use crate::frontend::ast::arena::AstArena;
pub use crate::frontend::ast::{
    Attribute, AttributeArgument, AttributeKind, BinaryOperator, BindingModifier, Block,
    CatchClause, ClassDecl, ClassKind, ClassMember, ConstDeclaration, ConstDeclarator,
    ConstItemDecl, ConstMemberDecl, ConstStatement, ConstructorDecl, ConstructorInitTarget,
    ConstructorInitializer, ConstructorKind, ConversionKind, CrateAttributes, CrateMainSetting,
    CrateStdSetting, DelegateDecl, DiInjectAttr, DiLifetime, DiServiceAttr, DocComment, EnumDecl,
    EnumVariant, Expression, ExtensionDecl, ExtensionMember, FieldDecl, FixedStatement,
    ForInitializer, ForStatement, ForeachStatement, FriendDirective, FunctionDecl,
    GenericConstraint, GenericConstraintKind, GenericParam, GenericParams, GotoStatement,
    GotoTarget, IfStatement, ImportDirective, ImportKind, InterfaceDecl, InterfaceMember, Item,
    MemberDispatch, Module, ModuleId, NamespaceDecl, OperatorDecl, OperatorKind, Parameter,
    PropertyAccessor, PropertyAccessorBody, PropertyAccessorKind, PropertyDecl, Signature,
    Statement, StatementKind, StaticDeclaration, StaticDeclarator, StaticItemDecl,
    StaticMutability, StructDecl, SwitchLabel, SwitchSection, SwitchStatement, TestCaseDecl,
    ThrowsClause, TryStatement, TypeExpr, UnaryOperator, UnionDecl, UnionField, UnionMember,
    UnionViewDecl, UsingDirective, UsingKind, UsingResource, UsingStatement, VariableDeclaration,
    VariableDeclarator, VariableModifier, Visibility,
};
use crate::frontend::attributes::stage_builtin_attributes;
use crate::frontend::conditional::{ConditionalDefines, preprocess};
use crate::frontend::diagnostics::{
    Diagnostic, DiagnosticCode, DiagnosticSink, FileCache, FileId, Label, Severity, Span,
    Suggestion,
};
use crate::frontend::lexer::{Keyword, LexOutput, Token, TokenKind, lex};
use crate::syntax::expr::parse_expression;

// Helper macro for parser submodules: wrap new methods in `parser_impl! { ... }`
// instead of spelling out `impl<'a> Parser<'a>` everywhere. This keeps the type
// signature consistent and reduces boilerplate as we keep splitting helpers.
macro_rules! parser_impl {
    ($($items:tt)*) => {
        impl<'a> Parser<'a> {
            $($items)*
        }
    };
}

mod attributes;
mod constructors;
mod core;
pub(super) use core::Modifier;
mod declarations;
mod doc_comments;
mod functions;
mod generics;
mod imports;
mod item_dispatch;
mod items;
mod members;
mod names;
mod namespaces;
mod properties;
mod statements;
mod telemetry;
mod type_expr_parser;
mod types;
pub(crate) use attributes::CollectedAttributes;
use doc_comments::normalise_doc_line;
pub use telemetry::{
    RecoveryTelemetryData, RecoveryTelemetryEvent, RecoveryTelemetryKind,
    disable_recovery_telemetry, enable_recovery_telemetry, recovery_telemetry_enabled,
};
pub(crate) use type_expr_parser::{
    parse_type_expression_text, parse_type_expression_text_with_span,
};

/// Resulting AST and diagnostics from parsing.
#[derive(Debug)]
pub struct ParseResult {
    pub arena: AstArena,
    pub module_id: ModuleId,
    pub file_id: FileId,
    pub diagnostics: Vec<Diagnostic>,
    pub module: Module,
    pub recovery_telemetry: Option<RecoveryTelemetryData>,
}

impl ParseResult {
    #[must_use]
    pub fn module_ref(&self) -> Ref<'_, Module> {
        self.arena.module(self.module_id)
    }

    #[must_use]
    pub fn module_mut(&self) -> RefMut<'_, Module> {
        self.arena.module_mut(self.module_id)
    }

    #[must_use]
    pub fn module_owned(&self) -> Module {
        self.arena.module_owned(self.module_id)
    }
}

/// Fatal parse error preventing further compilation.
#[derive(Debug)]
pub struct ParseError {
    message: String,
    diagnostics: Vec<Diagnostic>,
    files: FileCache,
}

impl ParseError {
    pub fn new(message: impl Into<String>, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            message: message.into(),
            diagnostics,
            files: FileCache::default(),
        }
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub fn files(&self) -> &FileCache {
        &self.files
    }

    #[must_use]
    pub fn with_file(mut self, path: impl Into<PathBuf>, source: impl Into<String>) -> Self {
        let mut files = FileCache::default();
        let file_id = files.add_file(path, source);
        for diagnostic in &mut self.diagnostics {
            if let Some(label) = diagnostic.primary_label.as_mut() {
                label.span = label.span.with_file(file_id);
            }
            for label in diagnostic.secondary_labels.iter_mut() {
                label.span = label.span.with_file(file_id);
            }
            for suggestion in diagnostic.suggestions.iter_mut() {
                if let Some(span) = suggestion.span {
                    suggestion.span = Some(span.with_file(file_id));
                }
            }
        }
        self.files = files;
        self
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl StdError for ParseError {}

#[derive(Clone)]
enum LocalDeclStart {
    Let,
    Var,
    Const,
    Typed {
        ty: TypeExpr,
        ty_start: usize,
        name_index: usize,
    },
}

struct ItemDispatch<'a> {
    visibility: Visibility,
    doc: &'a mut Option<DocComment>,
    modifiers: &'a mut Vec<Modifier>,
    is_async: bool,
    is_extern: bool,
}

struct UnionMemberInfo {
    visibility: Visibility,
    is_readonly: bool,
    doc: Option<DocComment>,
}

fn combine_expression_span(outer: Option<Span>, inner: Option<Span>) -> Option<Span> {
    match (outer, inner) {
        (Some(outer), Some(inner)) => {
            let file_id = if outer.file_id != FileId::UNKNOWN {
                outer.file_id
            } else {
                inner.file_id
            };
            Some(Span::in_file(
                file_id,
                outer.start + inner.start,
                outer.start + inner.end,
            ))
        }
        (Some(span), None) => Some(span),
        (None, Some(inner)) => Some(inner),
        _ => None,
    }
}

enum FunctionBodyKind {
    Block(Block),
    Declaration,
}

impl<'a> Parser<'a> {
    fn consume_borrow_qualifier_misuse(&mut self, allow_ref: bool) -> bool {
        let Some(token) = self.peek().cloned() else {
            return false;
        };
        let TokenKind::Keyword(keyword) = token.kind else {
            return false;
        };
        if matches!(keyword, Keyword::In | Keyword::Ref | Keyword::Out) {
            if allow_ref && matches!(keyword, Keyword::Ref) {
                return false;
            }
            let span = Some(token.span);
            let lexeme = token.lexeme.clone();
            self.advance();
            self.push_error(
                format!("`{lexeme}` qualifier is only supported on parameters and receivers"),
                span,
            );
            return true;
        }
        false
    }

    pub(crate) fn consume_all_borrow_qualifier_misuse(&mut self, allow_ref: bool) -> bool {
        let mut consumed = false;
        while self.consume_borrow_qualifier_misuse(allow_ref) {
            consumed = true;
        }
        consumed
    }

    fn type_returns_value(&self, ty: &TypeExpr) -> bool {
        ty.base
            .last()
            .map(|segment| !segment.eq_ignore_ascii_case("void"))
            .unwrap_or(true)
    }

    fn import_directive_ahead(&self) -> bool {
        self.check_keyword(Keyword::Import) || self.check_keyword(Keyword::Using) || {
            // LL1_ALLOW: Optional `global` prefix on import directives peeks ahead one token to keep the grammar ergonomic without adding another keyword (docs/compiler/parser.md#ll1-allowances).
            self.check_keyword(Keyword::Global)
                && (
                    // LL1_ALLOW: `global import` lookahead
                    self.peek_keyword_n(1, Keyword::Import)
                        // LL1_ALLOW: `global using` lookahead
                        || self.peek_keyword_n(1, Keyword::Using)
                )
        }
    }
}

/// Parse a source string into a module AST.
///
/// # Errors
/// Returns an error when lexical or syntactic issues prevent construction of a valid module.
pub fn parse_module(source: &str) -> Result<ParseResult, ParseError> {
    let lex_output = lex(source);
    parse_module_from_lex(source, lex_output)
}

/// Parse a module after applying conditional preprocessing and `@cfg` filtering.
///
/// # Errors
/// Returns an error when lexical or syntactic issues prevent construction of a valid module.
pub fn parse_module_with_defines(
    source: &str,
    defines: &ConditionalDefines,
) -> Result<ParseResult, ParseError> {
    parse_module_with_defines_in_file(source, defines, FileId::UNKNOWN)
}

/// Parse a module using a specific file id (for source-mapped diagnostics).
pub fn parse_module_in_file(source: &str, file_id: FileId) -> Result<ParseResult, ParseError> {
    let lex_output = crate::frontend::lexer::lex_with_file(source, file_id);
    parse_module_from_lex(source, lex_output)
}

fn parse_module_from_lex(source: &str, lex_output: LexOutput) -> Result<ParseResult, ParseError> {
    let mut parser = Parser::new(source, lex_output);
    let mut module = parser.parse_module();
    let file_id = parser.file_id;
    let (mut diagnostics, telemetry) = parser.finish();
    diagnostics.extend(stage_builtin_attributes(&mut module));
    if diagnostics
        .iter()
        .any(|diag| matches!(diag.severity, Severity::Error))
    {
        Err(ParseError::new(
            "encountered errors while parsing",
            diagnostics,
        ))
    } else {
        let arena = AstArena::new();
        let Module {
            namespace,
            namespace_span,
            namespace_attributes,
            crate_attributes,
            friend_declarations,
            package_imports,
            items,
            ..
        } = module;
        let mut builder = arena.module_builder(namespace);
        builder = builder.with_namespace_span(namespace_span);
        builder = builder.with_crate_attributes(crate_attributes);
        builder = builder.with_namespace_attributes(namespace_attributes);
        let module_id = builder
            .with_friend_declarations(friend_declarations)
            .with_package_imports(package_imports)
            .with_items(items)
            .finish_in();
        let module_snapshot = arena.module_owned(module_id);
        Ok(ParseResult {
            arena,
            module_id,
            file_id,
            diagnostics,
            module: module_snapshot,
            recovery_telemetry: telemetry,
        })
    }
}

/// Parse with preprocessing and explicit file id.
pub fn parse_module_with_defines_in_file(
    source: &str,
    defines: &ConditionalDefines,
    file_id: FileId,
) -> Result<ParseResult, ParseError> {
    let preprocess_result = preprocess(source, defines);
    let rewritten = preprocess_result.rewritten.as_deref().unwrap_or(source);
    let lex_output = crate::frontend::lexer::lex_with_file(rewritten, file_id);
    let mut parsed = parse_module_from_lex(rewritten, lex_output)?;
    parsed.diagnostics.extend(preprocess_result.diagnostics);
    let mut cfg_diags = {
        let mut module = parsed.module_mut();
        crate::frontend::cfg::apply_cfg(&mut module, defines)
    };
    parsed.diagnostics.append(&mut cfg_diags);
    parsed.module = parsed.module_owned();
    Ok(parsed)
}

/// Parse a textual block (including braces) into an AST block.
///
/// # Errors
/// Returns an error when lexical or syntactic issues prevent parsing the block.
pub fn parse_block_text(source: &str) -> Result<Block, ParseError> {
    let lex_output = lex(source);
    let mut parser = Parser::new(source, lex_output);
    let block = match parser.parse_block() {
        Some(block) => block,
        None => {
            let (diagnostics, _) = parser.finish();
            return Err(ParseError::new(
                "encountered errors while parsing block",
                diagnostics,
            ));
        }
    };
    let (diagnostics, _) = parser.finish();
    if diagnostics
        .iter()
        .any(|diag| matches!(diag.severity, Severity::Error))
    {
        Err(ParseError::new(
            "encountered errors while parsing block",
            diagnostics,
        ))
    } else {
        Ok(block)
    }
}

struct Parser<'a> {
    source: &'a str,
    file_id: FileId,
    tokens: Vec<Token>,
    leading_docs: Vec<Option<DocComment>>,
    pending_doc: Option<DocComment>,
    import_aliases: HashMap<String, String>,
    index: usize,
    last_span: Option<Span>,
    namespace_stack: Vec<String>,
    file_namespace: Vec<String>,
    namespace_span: Option<Span>,
    module_import_block_closed: bool,
    diagnostics: DiagnosticSink,
    recovery_telemetry: Option<RecoveryTelemetryData>,
}

#[cfg(test)]
mod tests;
