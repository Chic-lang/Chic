pub(crate) use super::super::DescriptorQuery;
pub(crate) use crate::frontend::ast::Module;
pub(crate) use crate::frontend::ast::expressions::Expression;
pub(crate) use crate::frontend::ast::items::{
    Attribute, AttributeKind, AutoTraitConstraint, BindingModifier, ClassDecl, ClassKind,
    ClassMember, ConstDeclaration, ConstDeclarator, ConstItemDecl, ConstMemberDecl,
    ConstructorDecl, ConstructorKind, DocComment, EnumDecl, EnumVariant, ExtensionCondition,
    ExtensionDecl, ExtensionMember, ExtensionMethodDecl, FieldDecl, FunctionDecl,
    GenericConstraint, GenericConstraintKind, GenericParam, GenericParams, ImplDecl, ImplMember,
    InterfaceDecl, InterfaceMember, Item, MemberDispatch, MmioAccess, MmioFieldAttr, NamespaceDecl,
    Parameter, PropertyAccessor, PropertyAccessorBody, PropertyAccessorKind, PropertyDecl,
    RecordPositionalField, Signature, StaticDeclaration, StaticDeclarator, StaticItemDecl,
    StaticMutability, StructDecl, ThrowsClause, TraitAssociatedType, TraitDecl, TraitMember,
    UnionDecl, UnionField, UnionMember, UnionViewDecl, Variance, VectorizeHint, Visibility,
};
pub(crate) use crate::frontend::ast::types::TypeExpr;
pub(crate) use crate::frontend::attributes::{AlignHint, LayoutHints, PackingHint};
pub(crate) use crate::frontend::metadata::reflection::{
    MemberKind, TypeKind, VisibilityDescriptor,
};
pub(crate) use crate::frontend::metadata::reflection::{
    collect_reflection_tables, serialize_reflection_tables,
};

pub(crate) fn attr(name: &str) -> Attribute {
    Attribute::new(name, Vec::new(), None, None, AttributeKind::Builtin)
}

pub(crate) fn doc(text: &str) -> Option<DocComment> {
    Some(DocComment::new(vec![text.to_string()]))
}
