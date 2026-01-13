use std::borrow::Borrow;
use std::fmt;

use super::basic_blocks::ParamMode;
use crate::frontend::ast::TypeExpr;

use super::utils::{canonical_fn_name, canonical_tuple_name, canonical_ty_name, ty_from_type_expr};

/// Calling convention/ABI for a function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Abi {
    Chic,
    Extern(String),
}

/// Simplified type information used during MIR construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Named(NamedTy),
    Array(ArrayTy),
    Vec(VecTy),
    Span(SpanTy),
    ReadOnlySpan(ReadOnlySpanTy),
    Rc(RcTy),
    Arc(ArcTy),
    Tuple(TupleTy),
    Fn(FnTy),
    Vector(VectorTy),
    Pointer(Box<PointerTy>),
    Ref(Box<RefTy>),
    String,
    Str,
    Unit,
    Unknown,
    Nullable(Box<Ty>),
    TraitObject(TraitObjectTy),
}

#[derive(Clone, PartialEq, Eq)]
pub struct NamedTy {
    pub name: String,
    pub args: Vec<GenericArg>,
}

/// Generic argument supplied to a named type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericArg {
    Type(Ty),
    Const(ConstGenericArg),
}

/// Normalised representation of a const generic argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstGenericArg {
    value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayTy {
    pub element: Box<Ty>,
    pub rank: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VecTy {
    pub element: Box<Ty>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanTy {
    pub element: Box<Ty>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadOnlySpanTy {
    pub element: Box<Ty>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RcTy {
    pub element: Box<Ty>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArcTy {
    pub element: Box<Ty>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorTy {
    pub element: Box<Ty>,
    pub lanes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleTy {
    pub elements: Vec<Ty>,
    pub element_names: Vec<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnTy {
    pub params: Vec<Ty>,
    pub param_modes: Vec<ParamMode>,
    pub ret: Box<Ty>,
    pub abi: Abi,
    pub variadic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitObjectTy {
    pub traits: Vec<String>,
    pub opaque_impl: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefTy {
    pub element: Ty,
    pub readonly: bool,
}

impl Ty {
    #[must_use]
    pub fn named(name: impl Into<String>) -> Self {
        let text = name.into();
        if let Some(stripped) = text.strip_suffix('?') {
            let inner = Ty::named(stripped.to_string());
            return Ty::Nullable(Box::new(inner));
        }
        match text.as_str() {
            "string" | "System::String" | "Std::String" | "System.String" | "Std.String" => {
                Ty::String
            }
            "str" | "System::Str" | "Std::Str" | "System.Str" | "Std.Str" => Ty::Str,
            _ => Ty::Named(NamedTy::new(text)),
        }
    }

    #[must_use]
    pub fn named_generic(name: impl Into<String>, args: Vec<GenericArg>) -> Self {
        Ty::Named(NamedTy::with_args(name.into(), args))
    }

    #[must_use]
    pub fn from_type_expr(expr: &TypeExpr) -> Self {
        ty_from_type_expr(expr)
    }

    #[must_use]
    pub fn canonical_name(&self) -> String {
        canonical_ty_name(self)
    }

    #[must_use]
    pub fn as_named(&self) -> Option<&NamedTy> {
        if let Ty::Named(named) = self {
            Some(named)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_named_mut(&mut self) -> Option<&mut NamedTy> {
        if let Ty::Named(named) = self {
            Some(named)
        } else {
            None
        }
    }

    #[must_use]
    pub fn base_name(&self) -> Option<String> {
        match self {
            Ty::Named(named) => {
                let canonical = named.canonical_path();
                Some(strip_generics(&canonical).to_string())
            }
            Ty::Nullable(inner) => inner.base_name(),
            Ty::Vector(_) => Some("vector".to_string()),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_var_placeholder(&self) -> bool {
        self.as_named()
            .is_some_and(|named| named.name.eq_ignore_ascii_case("var") && named.args.is_empty())
    }

    #[must_use]
    pub fn is_accelerator_stream(&self) -> bool {
        self.matches_accelerator_name(&["Std::Accelerator::Stream", "Stream"])
    }

    #[must_use]
    pub fn is_accelerator_event(&self) -> bool {
        self.matches_accelerator_name(&["Std::Accelerator::Event", "Event"])
    }

    #[must_use]
    pub fn is_accelerator_memspace(&self) -> bool {
        self.matches_accelerator_name(&[
            "Std::Accelerator::Host",
            "Std::Accelerator::PinnedHost",
            "Std::Accelerator::Gpu",
            "Std::Accelerator::Npu",
            "Std::Accelerator::Unified",
            "Host",
            "PinnedHost",
            "Gpu",
            "Npu",
            "Unified",
        ])
    }

    fn matches_accelerator_name(&self, candidates: &[&str]) -> bool {
        let Some(base) = self.base_name() else {
            return false;
        };
        let short = base.rsplit("::").next().unwrap_or(base.as_str());
        candidates.iter().any(|candidate| {
            let canonical_candidate = candidate.replace('.', "::");
            base == canonical_candidate || short == strip_generics(&canonical_candidate)
        })
    }
}

impl NamedTy {
    #[must_use]
    pub fn new(name: String) -> Self {
        Self {
            name,
            args: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_args(name: String, args: Vec<GenericArg>) -> Self {
        Self { name, args }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn args(&self) -> &[GenericArg] {
        &self.args
    }

    #[must_use]
    pub fn nth_type_arg(&self, index: usize) -> Option<&Ty> {
        self.args()
            .iter()
            .filter_map(|arg| arg.as_type())
            .nth(index)
    }

    #[must_use]
    pub fn canonical_path(&self) -> String {
        self.name.replace('.', "::")
    }

    #[must_use]
    pub fn matches_any(&self, candidates: &[&str]) -> bool {
        let canonical = self.canonical_path();
        candidates
            .iter()
            .any(|target| canonical == target.replace('.', "::"))
    }
}

impl From<String> for NamedTy {
    fn from(value: String) -> Self {
        NamedTy::with_args(value, Vec::new())
    }
}

impl From<&str> for NamedTy {
    fn from(value: &str) -> Self {
        NamedTy::with_args(value.to_string(), Vec::new())
    }
}

impl std::ops::Deref for NamedTy {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for NamedTy {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Debug for NamedTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.args.is_empty() {
            write!(f, "\"{}\"", self.name)
        } else {
            let args = self
                .args
                .iter()
                .map(|arg| match arg {
                    GenericArg::Type(ty) => ty.canonical_name(),
                    GenericArg::Const(value) => value.value().to_string(),
                })
                .collect::<Vec<_>>()
                .join(", ");
            write!(f, "\"{}<{}>\"", self.name, args)
        }
    }
}

impl Borrow<str> for NamedTy {
    fn borrow(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for NamedTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.args.is_empty() {
            write!(f, "{}", self.name)
        } else {
            let args = self
                .args
                .iter()
                .map(|arg| match arg {
                    GenericArg::Type(ty) => ty.canonical_name(),
                    GenericArg::Const(value) => value.value().to_string(),
                })
                .collect::<Vec<_>>()
                .join(", ");
            write!(f, "{}<{}>", self.name, args)
        }
    }
}

impl GenericArg {
    #[must_use]
    pub fn as_type(&self) -> Option<&Ty> {
        if let GenericArg::Type(ty) = self {
            Some(ty)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_const(&self) -> Option<&ConstGenericArg> {
        if let GenericArg::Const(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

impl ConstGenericArg {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl ArrayTy {
    #[must_use]
    pub fn new(element: Box<Ty>, rank: usize) -> Self {
        Self { element, rank }
    }
}

impl VecTy {
    #[must_use]
    pub fn new(element: Box<Ty>) -> Self {
        Self { element }
    }
}

impl SpanTy {
    #[must_use]
    pub fn new(element: Box<Ty>) -> Self {
        Self { element }
    }
}

impl ReadOnlySpanTy {
    #[must_use]
    pub fn new(element: Box<Ty>) -> Self {
        Self { element }
    }
}

impl RcTy {
    #[must_use]
    pub fn new(element: Box<Ty>) -> Self {
        Self { element }
    }
}

impl ArcTy {
    #[must_use]
    pub fn new(element: Box<Ty>) -> Self {
        Self { element }
    }
}

impl VectorTy {
    #[must_use]
    pub fn new(element: Box<Ty>, lanes: u32) -> Self {
        Self { element, lanes }
    }
}

impl TupleTy {
    #[must_use]
    pub fn new(elements: Vec<Ty>) -> Self {
        let names = vec![None; elements.len()];
        Self {
            elements,
            element_names: names,
        }
    }

    #[must_use]
    pub fn with_names(elements: Vec<Ty>, names: Vec<Option<String>>) -> Self {
        debug_assert_eq!(elements.len(), names.len());
        Self {
            elements,
            element_names: names,
        }
    }

    #[must_use]
    pub fn canonical_name(&self) -> String {
        canonical_tuple_name(self)
    }
}

impl FnTy {
    #[must_use]
    pub fn new(params: Vec<Ty>, ret: Ty, abi: Abi) -> Self {
        let param_modes = vec![ParamMode::Value; params.len()];
        Self::with_modes(params, param_modes, ret, abi, false)
    }

    #[must_use]
    pub fn with_modes(
        params: Vec<Ty>,
        param_modes: Vec<ParamMode>,
        ret: Ty,
        abi: Abi,
        variadic: bool,
    ) -> Self {
        Self {
            params,
            param_modes,
            ret: Box::new(ret),
            abi,
            variadic,
        }
    }

    #[must_use]
    pub fn canonical_name(&self) -> String {
        canonical_fn_name(self)
    }
}

impl TraitObjectTy {
    #[must_use]
    pub fn new(traits: Vec<String>) -> Self {
        let canonical = traits
            .into_iter()
            .map(|name| name.replace('.', "::"))
            .collect();
        Self {
            traits: canonical,
            opaque_impl: false,
        }
    }

    #[must_use]
    pub fn new_impl(traits: Vec<String>) -> Self {
        let mut ty = Self::new(traits);
        ty.opaque_impl = true;
        ty
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointerTy {
    pub element: Ty,
    pub mutable: bool,
    pub qualifiers: PointerQualifiers,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PointerQualifiers {
    pub restrict: bool,
    pub noalias: bool,
    pub readonly: bool,
    pub expose_address: bool,
    pub alignment: Option<u32>,
}

impl PointerTy {
    #[must_use]
    pub fn new(element: Ty, mutable: bool) -> Self {
        Self {
            element,
            mutable,
            qualifiers: PointerQualifiers::default(),
        }
    }

    #[must_use]
    pub fn with_qualifiers(element: Ty, mutable: bool, qualifiers: PointerQualifiers) -> Self {
        Self {
            element,
            mutable,
            qualifiers,
        }
    }
}

impl RefTy {
    #[must_use]
    pub fn new(element: Ty, readonly: bool) -> Self {
        Self { element, readonly }
    }
}

impl PointerQualifiers {
    #[must_use]
    pub fn render_tokens(&self) -> Option<String> {
        let mut parts = Vec::new();
        if self.restrict {
            parts.push("@restrict".to_string());
        }
        if self.noalias && !self.restrict {
            parts.push("@noalias".to_string());
        }
        if self.readonly {
            parts.push("@readonly".to_string());
        }
        if self.expose_address {
            parts.push("@expose_address".to_string());
        }
        if let Some(alignment) = self.alignment {
            parts.push(format!("@aligned({alignment})"));
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" "))
        }
    }
}

fn strip_generics(name: &str) -> &str {
    name.split('<').next().unwrap_or(name)
}
