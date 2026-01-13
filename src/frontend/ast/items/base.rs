use super::aggregates::{
    ClassDecl, DelegateDecl, EnumDecl, ExtensionDecl, ImplDecl, ImportDirective, InterfaceDecl,
    OperatorDecl, StructDecl, TraitDecl, UnionDecl,
};
use crate::frontend::ast::expressions::{Block, Expression};
use crate::frontend::ast::overloads::OverloadCatalog;
use crate::frontend::ast::types::TypeExpr;
use crate::frontend::diagnostics::Span;
use crate::frontend::lexer::Token;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MemberDispatch {
    pub is_virtual: bool,
    pub is_override: bool,
    pub is_sealed: bool,
    pub is_abstract: bool,
}

/// Crate-level attribute settings captured during parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrateStdSetting {
    Unspecified,
    Std { span: Option<Span> },
    NoStd { span: Option<Span> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrateMainSetting {
    Unspecified,
    NoMain { span: Option<Span> },
}

#[derive(Debug, Clone)]
pub struct FriendDirective {
    pub prefix: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct PackageImport {
    pub name: String,
    pub span: Option<Span>,
}

impl CrateStdSetting {
    #[must_use]
    pub fn span(self) -> Option<Span> {
        match self {
            Self::Std { span } | Self::NoStd { span } => span,
            Self::Unspecified => None,
        }
    }

    #[must_use]
    pub fn is_no_std(self) -> bool {
        matches!(self, Self::NoStd { .. })
    }

    #[must_use]
    pub fn is_std(self) -> bool {
        matches!(self, Self::Std { .. })
    }
}

impl Default for CrateStdSetting {
    fn default() -> Self {
        Self::Unspecified
    }
}

impl CrateMainSetting {
    #[must_use]
    pub fn span(self) -> Option<Span> {
        match self {
            Self::NoMain { span } => span,
            Self::Unspecified => None,
        }
    }

    #[must_use]
    pub fn is_no_main(self) -> bool {
        matches!(self, Self::NoMain { .. })
    }
}

impl Default for CrateMainSetting {
    fn default() -> Self {
        Self::Unspecified
    }
}

#[derive(Debug, Clone, Default)]
pub struct CrateAttributes {
    pub std_setting: CrateStdSetting,
    pub main_setting: CrateMainSetting,
}

/// Top-level compilation unit.
#[derive(Debug, Clone)]
pub struct Module {
    pub namespace: Option<String>,
    pub namespace_span: Option<Span>,
    pub crate_attributes: CrateAttributes,
    pub namespace_attributes: Vec<Attribute>,
    pub friend_declarations: Vec<FriendDirective>,
    pub package_imports: Vec<PackageImport>,
    pub items: Vec<Item>,
    pub overloads: OverloadCatalog,
    overloads_dirty: bool,
}

impl Module {
    #[must_use]
    pub fn new(namespace: Option<String>) -> Self {
        Self {
            namespace,
            namespace_span: None,
            crate_attributes: CrateAttributes::default(),
            namespace_attributes: Vec::new(),
            friend_declarations: Vec::new(),
            package_imports: Vec::new(),
            items: Vec::new(),
            overloads: OverloadCatalog::new(),
            overloads_dirty: true,
        }
    }

    pub fn push_item(&mut self, item: Item) {
        self.overloads_dirty = true;
        self.items.push(item);
    }

    #[must_use]
    pub fn with_items(namespace: Option<String>, items: Vec<Item>) -> Self {
        let mut module = Self::new(namespace);
        module.items = items;
        module.rebuild_overloads();
        module
    }

    #[must_use]
    pub fn with_namespace_items(
        namespace: Option<String>,
        namespace_span: Option<Span>,
        namespace_attributes: Vec<Attribute>,
        friend_declarations: Vec<FriendDirective>,
        items: Vec<Item>,
    ) -> Self {
        let mut module = Self::new(namespace);
        module.namespace_span = namespace_span;
        module.namespace_attributes = namespace_attributes;
        module.friend_declarations = friend_declarations;
        module.items = items;
        module.rebuild_overloads();
        module
    }

    pub fn rebuild_overloads(&mut self) {
        self.overloads = OverloadCatalog::from_module(self.namespace.as_deref(), &self.items);
        self.overloads_dirty = false;
    }

    #[must_use]
    pub fn overloads(&self) -> &OverloadCatalog {
        debug_assert!(
            !self.overloads_dirty,
            "module overload catalog is stale; call Module::rebuild_overloads before querying"
        );
        &self.overloads
    }
}

/// Items permitted at namespace scope.
#[derive(Debug, Clone)]
pub enum Item {
    Function(FunctionDecl),
    Struct(StructDecl),
    Union(UnionDecl),
    Enum(EnumDecl),
    Class(ClassDecl),
    Interface(InterfaceDecl),
    Delegate(DelegateDecl),
    Trait(TraitDecl),
    Impl(ImplDecl),
    Extension(ExtensionDecl),
    TypeAlias(TypeAliasDecl),
    TestCase(TestCaseDecl),
    Namespace(NamespaceDecl),
    Import(ImportDirective),
    Const(ConstItemDecl),
    Static(StaticItemDecl),
}

/// XML documentation comment captured from leading `///` trivia.
#[derive(Debug, Clone, Default)]
pub struct DocComment {
    pub lines: Vec<String>,
}

impl DocComment {
    #[must_use]
    pub fn new(lines: Vec<String>) -> Self {
        Self { lines }
    }

    pub fn extend(&mut self, other: DocComment) {
        self.lines.extend(other.lines);
    }

    pub fn push_line(&mut self, line: String) {
        self.lines.push(line);
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    #[must_use]
    pub fn as_text(&self) -> String {
        self.lines.join("\n")
    }
}

/// Classification for surface attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeKind {
    Builtin,
    Macro,
}

/// Macro-specific metadata recorded alongside an attribute.
#[derive(Debug, Clone, Default)]
pub struct AttributeMacroMetadata {
    /// Whether the attribute should be considered during macro expansion.
    pub expandable: bool,
    /// Raw token stream covering the attribute, filtered to exclude whitespace/comments.
    pub tokens: Vec<Token>,
}

impl AttributeMacroMetadata {
    #[must_use]
    pub fn new(expandable: bool, tokens: Vec<Token>) -> Self {
        Self { expandable, tokens }
    }
}

/// Supported dependency-injection lifetimes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiLifetime {
    Transient,
    Scoped,
    Singleton,
    ThreadLocal,
}

/// Metadata captured from `@service` attributes.
#[derive(Debug, Clone)]
pub struct DiServiceAttr {
    pub lifetime: Option<DiLifetime>,
    pub named: Option<String>,
}

impl DiServiceAttr {
    #[must_use]
    pub fn new(lifetime: Option<DiLifetime>, named: Option<String>) -> Self {
        Self { lifetime, named }
    }
}

/// Metadata captured from `@inject` attributes.
#[derive(Debug, Clone)]
pub struct DiInjectAttr {
    pub lifetime: Option<DiLifetime>,
    pub named: Option<String>,
    pub optional: bool,
}

impl DiInjectAttr {
    #[must_use]
    pub fn new(lifetime: Option<DiLifetime>, named: Option<String>, optional: bool) -> Self {
        Self {
            lifetime,
            named,
            optional,
        }
    }
}

/// Attribute applied to a declaration or statement.
#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: String,
    pub arguments: Vec<AttributeArgument>,
    pub span: Option<Span>,
    pub raw: Option<String>,
    pub kind: AttributeKind,
    pub macro_metadata: AttributeMacroMetadata,
}

impl Attribute {
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        arguments: Vec<AttributeArgument>,
        span: Option<Span>,
        raw: Option<String>,
        kind: AttributeKind,
    ) -> Self {
        Self {
            name: name.into(),
            arguments,
            span,
            raw,
            kind,
            macro_metadata: AttributeMacroMetadata::default(),
        }
    }

    #[must_use]
    pub fn with_macro_metadata(mut self, metadata: AttributeMacroMetadata) -> Self {
        self.macro_metadata = metadata;
        self
    }
}

/// Individual argument supplied to an attribute.
#[derive(Debug, Clone)]
pub struct AttributeArgument {
    pub name: Option<String>,
    pub value: String,
    pub span: Option<Span>,
}

impl AttributeArgument {
    #[must_use]
    pub fn new(name: Option<String>, value: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            name,
            value: value.into(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NamespaceDecl {
    pub name: String,
    pub items: Vec<Item>,
    pub doc: Option<DocComment>,
    pub attributes: Vec<Attribute>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct TypeAliasDecl {
    pub visibility: Visibility,
    pub name: String,
    pub target: TypeExpr,
    pub generics: Option<GenericParams>,
    pub attributes: Vec<Attribute>,
    pub doc: Option<DocComment>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticMutability {
    Const,
    Mutable,
}

#[derive(Debug, Clone)]
pub struct StaticItemDecl {
    pub visibility: Visibility,
    pub declaration: StaticDeclaration,
}

#[derive(Debug, Clone)]
pub struct StaticDeclaration {
    pub mutability: StaticMutability,
    pub ty: TypeExpr,
    pub declarators: Vec<StaticDeclarator>,
    pub attributes: Vec<Attribute>,
    pub is_extern: bool,
    pub extern_abi: Option<String>,
    pub extern_options: Option<ExternOptions>,
    pub link_library: Option<String>,
    pub is_weak_import: bool,
    pub doc: Option<DocComment>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct StaticDeclarator {
    pub name: String,
    pub initializer: Option<Expression>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct ConstItemDecl {
    pub visibility: Visibility,
    pub declaration: ConstDeclaration,
}

#[derive(Debug, Clone)]
pub struct ConstMemberDecl {
    pub visibility: Visibility,
    pub modifiers: Vec<String>,
    pub declaration: ConstDeclaration,
}

#[derive(Debug, Clone)]
pub struct ConstStatement {
    pub declaration: ConstDeclaration,
}

#[derive(Debug, Clone)]
pub struct ConstDeclaration {
    pub ty: TypeExpr,
    pub declarators: Vec<ConstDeclarator>,
    pub doc: Option<DocComment>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct ConstDeclarator {
    pub name: String,
    pub initializer: Expression,
    pub span: Option<Span>,
}

/// Type parameter list declared on a generic item.
#[derive(Debug, Clone, Default)]
pub struct GenericParams {
    pub span: Option<Span>,
    pub params: Vec<GenericParam>,
}

impl GenericParams {
    #[must_use]
    pub fn new(span: Option<Span>, params: Vec<GenericParam>) -> Self {
        Self { span, params }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }
}

/// Individual parameter declared on a generic item.
#[derive(Debug, Clone)]
pub struct GenericParam {
    pub name: String,
    pub span: Option<Span>,
    pub kind: GenericParamKind,
}

impl GenericParam {
    #[must_use]
    pub fn type_param(name: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            name: name.into(),
            span,
            kind: GenericParamKind::Type(TypeParamData::default()),
        }
    }

    #[must_use]
    pub fn const_param(name: impl Into<String>, span: Option<Span>, ty: TypeExpr) -> Self {
        Self {
            name: name.into(),
            span,
            kind: GenericParamKind::Const(ConstParamData::new(ty)),
        }
    }

    #[must_use]
    pub fn as_type(&self) -> Option<&TypeParamData> {
        if let GenericParamKind::Type(data) = &self.kind {
            Some(data)
        } else {
            None
        }
    }

    pub fn as_type_mut(&mut self) -> Option<&mut TypeParamData> {
        if let GenericParamKind::Type(data) = &mut self.kind {
            Some(data)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_const(&self) -> Option<&ConstParamData> {
        if let GenericParamKind::Const(data) = &self.kind {
            Some(data)
        } else {
            None
        }
    }

    pub fn as_const_mut(&mut self) -> Option<&mut ConstParamData> {
        if let GenericParamKind::Const(data) = &mut self.kind {
            Some(data)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub enum GenericParamKind {
    Type(TypeParamData),
    Const(ConstParamData),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Variance {
    #[default]
    Invariant,
    Covariant,
    Contravariant,
}

impl Variance {
    #[must_use]
    pub fn keyword(self) -> Option<&'static str> {
        match self {
            Variance::Invariant => None,
            Variance::Covariant => Some("out"),
            Variance::Contravariant => Some("in"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeParamData {
    pub constraints: Vec<GenericConstraint>,
    pub variance: Variance,
}

impl Default for TypeParamData {
    fn default() -> Self {
        Self {
            constraints: Vec::new(),
            variance: Variance::Invariant,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConstParamData {
    pub ty: TypeExpr,
    pub constraints: Vec<ConstWherePredicate>,
}

impl ConstParamData {
    #[must_use]
    pub fn new(ty: TypeExpr) -> Self {
        Self {
            ty,
            constraints: Vec::new(),
        }
    }
}

/// Constraint attached to a const generic parameter.
#[derive(Debug, Clone)]
pub struct ConstWherePredicate {
    pub expr: Expression,
    pub span: Option<Span>,
}

impl ConstWherePredicate {
    #[must_use]
    pub fn new(expr: Expression, span: Option<Span>) -> Self {
        Self { expr, span }
    }
}

/// Constraint applied to a generic parameter.
#[derive(Debug, Clone)]
pub struct GenericConstraint {
    pub kind: GenericConstraintKind,
    pub span: Option<Span>,
}

impl GenericConstraint {
    #[must_use]
    pub fn new(kind: GenericConstraintKind, span: Option<Span>) -> Self {
        Self { kind, span }
    }
}

/// Supported generic constraint kinds.
#[derive(Debug, Clone)]
pub enum GenericConstraintKind {
    Type(TypeExpr),
    Struct,
    Class,
    NotNull,
    DefaultConstructor,
    AutoTrait(AutoTraitConstraint),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoTraitConstraint {
    ThreadSafe,
    Shareable,
}

impl AutoTraitConstraint {
    #[must_use]
    pub fn attribute_name(self) -> &'static str {
        match self {
            AutoTraitConstraint::ThreadSafe => "@thread_safe",
            AutoTraitConstraint::Shareable => "@shareable",
        }
    }
}

/// Function declaration (free or associated).
#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub visibility: Visibility,
    pub name: String,
    pub name_span: Option<Span>,
    pub signature: Signature,
    pub body: Option<Block>,
    pub is_async: bool,
    pub is_constexpr: bool,
    pub doc: Option<DocComment>,
    pub modifiers: Vec<String>,
    pub is_unsafe: bool,
    pub attributes: Vec<Attribute>,
    pub is_extern: bool,
    pub extern_abi: Option<String>,
    pub extern_options: Option<ExternOptions>,
    pub link_name: Option<String>,
    pub link_library: Option<String>,
    pub operator: Option<OperatorDecl>,
    pub generics: Option<GenericParams>,
    pub vectorize_hint: Option<VectorizeHint>,
    pub dispatch: MemberDispatch,
}

/// Additional metadata describing an extern function binding.
#[derive(Debug, Clone)]
pub struct ExternOptions {
    pub convention: String,
    pub library: Option<String>,
    pub alias: Option<String>,
    pub binding: ExternBinding,
    pub optional: bool,
    pub charset: Option<String>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorizeHint {
    Decimal,
}

impl VectorizeHint {
    #[must_use]
    pub fn is_decimal(self) -> bool {
        matches!(self, VectorizeHint::Decimal)
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            VectorizeHint::Decimal => "decimal",
        }
    }
}

impl ExternOptions {
    #[must_use]
    pub fn new(
        convention: impl Into<String>,
        library: Option<String>,
        alias: Option<String>,
        binding: ExternBinding,
        optional: bool,
        charset: Option<String>,
        span: Option<Span>,
    ) -> Self {
        Self {
            convention: convention.into(),
            library,
            alias,
            binding,
            optional,
            charset,
            span,
        }
    }
}

/// Binding strategy for extern functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternBinding {
    /// Symbol resolved by the static linker (current behaviour).
    Static,
    /// Symbol resolved lazily via the dynamic loader the first time it is invoked.
    Lazy,
    /// Symbol resolved eagerly during startup.
    Eager,
}

impl ExternBinding {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ExternBinding::Static => "static",
            ExternBinding::Lazy => "lazy",
            ExternBinding::Eager => "eager",
        }
    }
}

impl fmt::Display for ExternBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct TestCaseDecl {
    pub name: String,
    pub signature: Option<Signature>,
    pub body: Block,
    pub is_async: bool,
    pub doc: Option<DocComment>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone)]
pub struct Signature {
    pub parameters: Vec<Parameter>,
    pub return_type: TypeExpr,
    pub lends_to_return: Option<LendsClause>,
    pub throws: Option<ThrowsClause>,
    pub variadic: bool,
}

#[derive(Debug, Clone)]
pub struct ThrowsClause {
    pub types: Vec<TypeExpr>,
    pub span: Option<Span>,
}

impl ThrowsClause {
    #[must_use]
    pub fn new(types: Vec<TypeExpr>, span: Option<Span>) -> Self {
        Self { types, span }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub binding: BindingModifier,
    pub binding_nullable: bool,
    pub name: String,
    pub name_span: Option<Span>,
    pub ty: TypeExpr,
    pub attributes: Vec<Attribute>,
    pub di_inject: Option<DiInjectAttr>,
    pub default: Option<Expression>,
    pub default_span: Option<Span>,
    pub lends: Option<LendsClause>,
    pub is_extension_this: bool,
}

#[derive(Debug, Clone)]
pub struct LendsClause {
    pub targets: Vec<String>,
    pub span: Option<Span>,
}

impl LendsClause {
    #[must_use]
    pub fn new(targets: Vec<String>, span: Option<Span>) -> Self {
        Self { targets, span }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Internal,
    Protected,
    Private,
    ProtectedInternal,
    PrivateProtected,
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Internal
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum BindingModifier {
    In,
    Ref,
    Out,
    #[default]
    Value,
}
