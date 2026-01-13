use crate::diagnostics::Severity;

#[derive(Debug, Clone)]
pub struct DocDiagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub path: Option<String>,
}

impl DocDiagnostic {
    #[must_use]
    pub fn warning(code: &'static str, message: impl Into<String>, path: Option<String>) -> Self {
        Self {
            severity: Severity::Warning,
            code,
            message: message.into(),
            path,
        }
    }

    #[must_use]
    pub fn error(code: &'static str, message: impl Into<String>, path: Option<String>) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message: message.into(),
            path,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DocModel {
    pub summary: Vec<BlockContent>,
    pub remarks: Vec<BlockContent>,
    pub params: Vec<ParamDoc>,
    pub type_params: Vec<ParamDoc>,
    pub returns: Option<Vec<BlockContent>>,
    pub value: Option<Vec<BlockContent>>,
    pub examples: Vec<ExampleBlock>,
    pub see_also: Vec<LinkNode>,
    pub custom_sections: Vec<CustomSection>,
}

#[derive(Debug, Clone)]
pub struct ParamDoc {
    pub name: String,
    pub content: Vec<BlockContent>,
}

#[derive(Debug, Clone)]
pub struct ExampleBlock {
    pub caption: Option<String>,
    pub code: CodeBlock,
}

#[derive(Debug, Clone)]
pub struct CodeBlock {
    pub language: Option<String>,
    pub title: Option<String>,
    pub code: String,
}

#[derive(Debug, Clone)]
pub struct LinkNode {
    pub target: LinkTarget,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub enum LinkTarget {
    Cref(String),
    Url(String),
    Plain(String),
}

#[derive(Debug, Clone)]
pub struct CustomSection {
    pub title: String,
    pub content: Vec<BlockContent>,
}

#[derive(Debug, Clone)]
pub enum BlockContent {
    Paragraph(Vec<InlineContent>),
    CodeBlock(CodeBlock),
    List {
        kind: ListKind,
        items: Vec<ListItem>,
    },
    BlockQuote(Vec<BlockContent>),
}

#[derive(Debug, Clone)]
pub struct ListItem {
    pub term: Option<Vec<InlineContent>>,
    pub body: Vec<BlockContent>,
}

#[derive(Debug, Clone, Copy)]
pub enum ListKind {
    Bullet,
    Numbered,
    Table,
}

#[derive(Debug, Clone)]
pub enum InlineContent {
    Text(String),
    Code(String),
    Link(LinkNode),
    ParamRef(String),
    TypeParamRef(String),
}

#[derive(Debug, Clone, Default)]
pub struct ParsedDoc {
    pub model: DocModel,
    pub diagnostics: Vec<DocDiagnostic>,
}

#[derive(Debug, Clone)]
pub enum SymbolKind {
    Namespace,
    Struct,
    Record,
    Class,
    Enum,
    Interface,
    Union,
    Extension,
    Trait,
    Delegate,
    Impl,
    Function,
    Method,
    Property,
    Field,
    Constructor,
    Const,
    Static,
    TraitMethod,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct SymbolDocs {
    pub name: String,
    pub full_name: String,
    pub kind: SymbolKind,
    pub signature: Option<String>,
    pub doc: ParsedDoc,
    pub parameters: Vec<crate::frontend::metadata::reflection::ParameterDescriptor>,
    pub return_type: Option<String>,
    pub members: Vec<SymbolDocs>,
}
