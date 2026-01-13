use super::{
    Attribute, ConstMemberDecl, DiInjectAttr, DiServiceAttr, DocComment, FunctionDecl,
    GenericParams, Item, MemberDispatch, Parameter, Signature, Visibility,
};
use crate::frontend::ast::expressions::{Block, Expression};
use crate::frontend::ast::types::TypeExpr;
use crate::frontend::attributes::LayoutHints;
use crate::frontend::diagnostics::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineAttr {
    Local,
    Cross,
}

#[derive(Debug, Clone)]
pub struct RecordPositionalField {
    pub name: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub visibility: Visibility,
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub properties: Vec<PropertyDecl>,
    pub constructors: Vec<ConstructorDecl>,
    pub consts: Vec<ConstMemberDecl>,
    pub methods: Vec<FunctionDecl>,
    pub nested_types: Vec<Item>,
    pub bases: Vec<TypeExpr>,
    pub thread_safe_override: Option<bool>,
    pub shareable_override: Option<bool>,
    pub copy_override: Option<bool>,
    pub mmio: Option<MmioStructAttr>,
    pub doc: Option<DocComment>,
    pub generics: Option<GenericParams>,
    pub attributes: Vec<Attribute>,
    pub is_readonly: bool,
    pub layout: Option<LayoutHints>,
    pub is_intrinsic: bool,
    pub inline_attr: Option<InlineAttr>,
    pub is_record: bool,
    pub record_positional_fields: Vec<RecordPositionalField>,
}

#[derive(Debug, Clone)]
pub struct UnionDecl {
    pub visibility: Visibility,
    pub name: String,
    pub members: Vec<UnionMember>,
    pub thread_safe_override: Option<bool>,
    pub shareable_override: Option<bool>,
    pub copy_override: Option<bool>,
    pub doc: Option<DocComment>,
    pub generics: Option<GenericParams>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone)]
pub enum UnionMember {
    Field(UnionField),
    View(UnionViewDecl),
}

#[derive(Debug, Clone)]
pub struct UnionField {
    pub visibility: Visibility,
    pub name: String,
    pub ty: TypeExpr,
    pub is_readonly: bool,
    pub doc: Option<DocComment>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone)]
pub struct UnionViewDecl {
    pub visibility: Visibility,
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub is_readonly: bool,
    pub doc: Option<DocComment>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub visibility: Visibility,
    pub name: String,
    pub underlying_type: Option<TypeExpr>,
    pub variants: Vec<EnumVariant>,
    pub thread_safe_override: Option<bool>,
    pub shareable_override: Option<bool>,
    pub copy_override: Option<bool>,
    pub is_flags: bool,
    pub doc: Option<DocComment>,
    pub generics: Option<GenericParams>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone)]
pub struct DelegateDecl {
    pub visibility: Visibility,
    pub name: String,
    pub signature: Signature,
    pub generics: Option<GenericParams>,
    pub attributes: Vec<Attribute>,
    pub doc: Option<DocComment>,
    pub is_unsafe: bool,
    pub modifiers: Vec<String>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClassKind {
    #[default]
    Class,
    Error,
}

#[derive(Debug, Clone)]
pub struct ClassDecl {
    pub visibility: Visibility,
    pub kind: ClassKind,
    pub name: String,
    pub bases: Vec<TypeExpr>,
    pub members: Vec<ClassMember>,
    pub nested_types: Vec<Item>,
    pub thread_safe_override: Option<bool>,
    pub shareable_override: Option<bool>,
    pub copy_override: Option<bool>,
    pub doc: Option<DocComment>,
    pub generics: Option<GenericParams>,
    pub attributes: Vec<Attribute>,
    pub di_service: Option<DiServiceAttr>,
    pub di_module: bool,
    pub is_static: bool,
    pub is_abstract: bool,
    pub is_sealed: bool,
}

#[derive(Debug, Clone)]
pub struct ConstructorDecl {
    pub visibility: Visibility,
    pub kind: ConstructorKind,
    pub parameters: Vec<Parameter>,
    pub body: Option<Block>,
    pub initializer: Option<ConstructorInitializer>,
    pub doc: Option<DocComment>,
    pub span: Option<Span>,
    pub attributes: Vec<Attribute>,
    pub di_inject: Option<DiInjectAttr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructorKind {
    Designated,
    Convenience,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructorInitTarget {
    SelfType,
    Super,
}

#[derive(Debug, Clone)]
pub struct ConstructorInitializer {
    pub target: ConstructorInitTarget,
    pub arguments: Vec<Expression>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct InterfaceDecl {
    pub visibility: Visibility,
    pub name: String,
    pub bases: Vec<TypeExpr>,
    pub members: Vec<InterfaceMember>,
    pub thread_safe_override: Option<bool>,
    pub shareable_override: Option<bool>,
    pub copy_override: Option<bool>,
    pub doc: Option<DocComment>,
    pub generics: Option<GenericParams>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone)]
pub struct TraitDecl {
    pub visibility: Visibility,
    pub name: String,
    pub super_traits: Vec<TypeExpr>,
    pub members: Vec<TraitMember>,
    pub thread_safe_override: Option<bool>,
    pub shareable_override: Option<bool>,
    pub copy_override: Option<bool>,
    pub doc: Option<DocComment>,
    pub generics: Option<GenericParams>,
    pub attributes: Vec<Attribute>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub enum TraitMember {
    Method(FunctionDecl),
    AssociatedType(TraitAssociatedType),
    Const(ConstMemberDecl),
}

#[derive(Debug, Clone)]
pub struct TraitAssociatedType {
    pub name: String,
    pub generics: Option<GenericParams>,
    pub default: Option<TypeExpr>,
    pub doc: Option<DocComment>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct ImplDecl {
    pub visibility: Visibility,
    pub trait_ref: Option<TypeExpr>,
    pub target: TypeExpr,
    pub generics: Option<GenericParams>,
    pub members: Vec<ImplMember>,
    pub doc: Option<DocComment>,
    pub attributes: Vec<Attribute>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub enum ImplMember {
    Method(FunctionDecl),
    AssociatedType(TraitAssociatedType),
    Const(ConstMemberDecl),
}

#[derive(Debug, Clone)]
pub struct ExtensionDecl {
    pub visibility: Visibility,
    pub target: TypeExpr,
    pub generics: Option<GenericParams>,
    pub members: Vec<ExtensionMember>,
    pub doc: Option<DocComment>,
    pub attributes: Vec<Attribute>,
    pub conditions: Vec<ExtensionCondition>,
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub visibility: Visibility,
    pub name: String,
    pub ty: TypeExpr,
    pub initializer: Option<Expression>,
    pub mmio: Option<MmioFieldAttr>,
    pub doc: Option<DocComment>,
    pub is_required: bool,
    pub display_name: Option<String>,
    pub attributes: Vec<Attribute>,
    pub is_readonly: bool,
    pub is_static: bool,
    pub view_of: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PropertyDecl {
    pub visibility: Visibility,
    pub modifiers: Vec<String>,
    pub name: String,
    pub ty: TypeExpr,
    pub parameters: Vec<Parameter>,
    pub accessors: Vec<PropertyAccessor>,
    pub doc: Option<DocComment>,
    pub is_required: bool,
    pub is_static: bool,
    pub initializer: Option<Expression>,
    pub span: Option<Span>,
    pub attributes: Vec<Attribute>,
    pub di_inject: Option<DiInjectAttr>,
    pub dispatch: MemberDispatch,
    pub explicit_interface: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PropertyAccessor {
    pub kind: PropertyAccessorKind,
    pub visibility: Option<Visibility>,
    pub body: PropertyAccessorBody,
    pub doc: Option<DocComment>,
    pub span: Option<Span>,
    pub attributes: Option<Vec<Attribute>>,
    pub dispatch: MemberDispatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyAccessorKind {
    Get,
    Set,
    Init,
}

#[derive(Debug, Clone)]
pub enum PropertyAccessorBody {
    Auto,
    Block(Block),
    Expression(Expression),
}

impl PropertyDecl {
    #[must_use]
    pub fn accessor(&self, kind: PropertyAccessorKind) -> Option<&PropertyAccessor> {
        self.accessors.iter().find(|accessor| accessor.kind == kind)
    }

    #[must_use]
    pub fn accessor_mut(&mut self, kind: PropertyAccessorKind) -> Option<&mut PropertyAccessor> {
        self.accessors
            .iter_mut()
            .find(|accessor| accessor.kind == kind)
    }

    #[must_use]
    pub fn accessor_method_name(&self, kind: PropertyAccessorKind) -> String {
        format!("{}{}", kind.method_prefix(), self.name)
    }

    #[must_use]
    pub fn is_auto(&self) -> bool {
        self.parameters.is_empty()
            && !self.accessors.is_empty()
            && self
                .accessors
                .iter()
                .all(|accessor| accessor.body.is_auto())
    }

    #[must_use]
    pub fn backing_field_name(&self) -> String {
        format!("__property_{}", self.name)
    }
}

impl PropertyAccessorKind {
    #[must_use]
    pub const fn method_prefix(self) -> &'static str {
        match self {
            PropertyAccessorKind::Get => "get_",
            PropertyAccessorKind::Set => "set_",
            PropertyAccessorKind::Init => "init_",
        }
    }
}

impl PropertyAccessorBody {
    #[must_use]
    pub const fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }
}

#[derive(Debug, Clone)]
pub struct MmioStructAttr {
    pub base_address: u64,
    pub size: Option<u64>,
    pub address_space: Option<String>,
    pub endianness: MmioEndianness,
    pub requires_unsafe: bool,
}

#[derive(Debug, Clone)]
pub struct MmioFieldAttr {
    pub offset: u32,
    pub width_bits: u16,
    pub access: MmioAccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmioAccess {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmioEndianness {
    Little,
    Big,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub discriminant: Option<Expression>,
    pub doc: Option<DocComment>,
}

#[derive(Debug, Clone)]
pub struct ImportDirective {
    pub doc: Option<DocComment>,
    pub is_global: bool,
    pub span: Option<Span>,
    pub kind: ImportKind,
}

#[derive(Debug, Clone)]
pub enum ImportKind {
    Namespace { path: String },
    Alias { alias: String, target: String },
    Static { target: String },
    CImport { header: String },
}

pub type UsingDirective = ImportDirective;
pub type UsingKind = ImportKind;

#[derive(Debug, Clone)]
pub struct OperatorDecl {
    pub kind: OperatorKind,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub enum OperatorKind {
    Unary(UnaryOperator),
    Binary(BinaryOperator),
    Conversion(ConversionKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOperator {
    Negate,
    UnaryPlus,
    LogicalNot,
    OnesComplement,
    Increment,
    Decrement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionKind {
    Implicit,
    Explicit,
}

#[derive(Debug, Clone)]
pub enum ClassMember {
    Field(FieldDecl),
    Method(FunctionDecl),
    Property(PropertyDecl),
    Constructor(ConstructorDecl),
    Const(ConstMemberDecl),
}

#[derive(Debug, Clone)]
pub enum InterfaceMember {
    Method(FunctionDecl),
    Property(PropertyDecl),
    AssociatedType(TraitAssociatedType),
    Const(ConstMemberDecl),
}

#[derive(Debug, Clone)]
pub enum ExtensionMember {
    Method(ExtensionMethodDecl),
}

#[derive(Debug, Clone)]
pub struct ExtensionMethodDecl {
    pub function: FunctionDecl,
    pub is_default: bool,
}

#[derive(Debug, Clone)]
pub struct ExtensionCondition {
    pub target: TypeExpr,
    pub constraint: TypeExpr,
    pub span: Option<Span>,
}
