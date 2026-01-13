//! AST type syntax nodes.

use super::expressions::Expression;
use crate::frontend::diagnostics::Span;
use std::cell::RefCell;

#[derive(Debug, Clone)]
pub struct TypeExpr {
    pub name: String,
    pub base: Vec<String>,
    pub suffixes: Vec<TypeSuffix>,
    pub span: Option<Span>,
    pub generic_span: Option<Span>,
    pub tuple_elements: Option<Vec<TypeExpr>>,
    pub tuple_element_names: Option<Vec<Option<String>>>,
    pub fn_signature: Option<FnTypeExpr>,
    pub trait_object: Option<TraitObjectTypeExpr>,
    pub ref_kind: Option<RefKind>,
    /// Whether this is a view (non-owning) projection of the base type.
    pub is_view: bool,
}

impl TypeExpr {
    #[must_use]
    pub fn simple(name: impl Into<String>) -> Self {
        let text = name.into();
        let base = text.split('.').map(str::to_string).collect();
        Self {
            name: text,
            base,
            suffixes: Vec::new(),
            span: None,
            generic_span: None,
            tuple_elements: None,
            tuple_element_names: None,
            fn_signature: None,
            trait_object: None,
            ref_kind: None,
            is_view: false,
        }
    }

    #[must_use]
    pub fn self_type() -> Self {
        Self {
            name: "Self".to_string(),
            base: vec!["Self".to_string()],
            suffixes: Vec::new(),
            span: None,
            generic_span: None,
            tuple_elements: None,
            tuple_element_names: None,
            fn_signature: None,
            trait_object: None,
            ref_kind: None,
            is_view: false,
        }
    }

    #[must_use]
    pub fn tuple(elements: Vec<TypeExpr>) -> Self {
        let mut text = String::from("(");
        let count = elements.len();
        for (index, element) in elements.iter().enumerate() {
            if index > 0 {
                text.push_str(", ");
            }
            text.push_str(element.name.as_str());
        }
        text.push(')');
        Self {
            name: text,
            base: Vec::new(),
            suffixes: Vec::new(),
            span: None,
            generic_span: None,
            tuple_elements: Some(elements),
            tuple_element_names: Some(vec![None; count]),
            fn_signature: None,
            trait_object: None,
            ref_kind: None,
            is_view: false,
        }
    }

    #[must_use]
    pub fn generic_arguments(&self) -> Option<&[GenericArgument]> {
        self.suffixes.iter().rev().find_map(|suffix| match suffix {
            TypeSuffix::GenericArgs(args) => Some(args.as_slice()),
            _ => None,
        })
    }

    #[must_use]
    pub fn array_ranks(&self) -> impl Iterator<Item = &ArrayRankSpecifier> {
        self.suffixes.iter().filter_map(|suffix| match suffix {
            TypeSuffix::Array(spec) => Some(spec),
            _ => None,
        })
    }

    #[must_use]
    pub fn is_nullable(&self) -> bool {
        self.suffixes
            .iter()
            .any(|suffix| matches!(suffix, TypeSuffix::Nullable))
    }

    #[must_use]
    pub fn pointer_depth(&self) -> usize {
        self.suffixes
            .iter()
            .filter(|suffix| matches!(suffix, TypeSuffix::Pointer { .. }))
            .count()
    }

    #[must_use]
    pub fn is_tuple(&self) -> bool {
        self.tuple_elements.is_some()
    }

    #[must_use]
    pub fn tuple_elements(&self) -> Option<&[TypeExpr]> {
        self.tuple_elements.as_deref()
    }

    #[must_use]
    pub fn tuple_element_names(&self) -> Option<&[Option<String>]> {
        self.tuple_element_names.as_deref()
    }

    #[must_use]
    pub fn is_fn(&self) -> bool {
        self.fn_signature.is_some()
    }

    #[must_use]
    pub fn fn_signature(&self) -> Option<&FnTypeExpr> {
        self.fn_signature.as_ref()
    }

    #[must_use]
    pub fn is_trait_object(&self) -> bool {
        self.trait_object.is_some()
    }

    #[must_use]
    pub fn trait_object(&self) -> Option<&TraitObjectTypeExpr> {
        self.trait_object.as_ref()
    }

    #[must_use]
    pub fn is_impl_trait(&self) -> bool {
        self.trait_object
            .as_ref()
            .is_some_and(|object| object.opaque_impl)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    Ref,
    ReadOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PointerModifier {
    Restrict,
    NoAlias,
    ReadOnly,
    Aligned(u32),
    ExposeAddress,
}

#[derive(Debug, Clone)]
pub enum TypeSuffix {
    GenericArgs(Vec<GenericArgument>),
    Array(ArrayRankSpecifier),
    Nullable,
    Pointer {
        mutable: bool,
        modifiers: Vec<PointerModifier>,
    },
    Qualifier(String),
}

#[derive(Debug, Clone)]
pub struct GenericArgument {
    pub ty: Option<TypeExpr>,
    pub expr: Expression,
    evaluated: RefCell<Option<String>>,
}

impl GenericArgument {
    #[must_use]
    pub fn new(ty: Option<TypeExpr>, expr: Expression) -> Self {
        Self {
            ty,
            expr,
            evaluated: RefCell::new(None),
        }
    }

    #[must_use]
    pub fn from_type_expr(ty: TypeExpr) -> Self {
        let expr = Expression::new(ty.name.clone(), None);
        Self {
            ty: Some(ty),
            expr,
            evaluated: RefCell::new(None),
        }
    }

    #[must_use]
    pub fn ty(&self) -> Option<&TypeExpr> {
        self.ty.as_ref()
    }

    #[must_use]
    pub fn expression(&self) -> &Expression {
        &self.expr
    }

    pub fn set_evaluated_value(&self, value: impl Into<String>) {
        *self.evaluated.borrow_mut() = Some(value.into());
    }

    #[must_use]
    pub fn evaluated_value(&self) -> Option<String> {
        self.evaluated.borrow().clone()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ArrayRankSpecifier {
    pub dimensions: usize,
}

impl ArrayRankSpecifier {
    #[must_use]
    pub fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }
}

#[derive(Debug, Clone)]
pub struct FnTypeExpr {
    pub abi: FnTypeAbi,
    pub params: Vec<TypeExpr>,
    pub return_type: Box<TypeExpr>,
    pub variadic: bool,
}

impl FnTypeExpr {
    #[must_use]
    pub fn new(abi: FnTypeAbi, params: Vec<TypeExpr>, return_type: TypeExpr) -> Self {
        Self {
            abi,
            params,
            return_type: Box::new(return_type),
            variadic: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TraitObjectTypeExpr {
    pub bounds: Vec<TypeExpr>,
    pub opaque_impl: bool,
}

#[derive(Debug, Clone)]
pub enum FnTypeAbi {
    Chic,
    Extern(String),
}
