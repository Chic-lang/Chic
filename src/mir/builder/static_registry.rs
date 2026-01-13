use std::collections::HashMap;

use crate::frontend::ast::{Expression, ExternBinding, TypeExpr, Visibility};
use crate::frontend::diagnostics::Span;
use crate::mir::builder::LoweringDiagnostic;
use crate::mir::builder::const_eval::ConstEvalContext;
use crate::mir::data::{ConstValue, MirExternSpec, StaticId, StaticVar, Ty};
use crate::mir::layout::{TypeLayout, TypeLayoutTable, TypeRepr};

#[derive(Default)]
pub(super) struct StaticRegistry {
    pending: Vec<PendingStatic>,
    vars: Vec<StaticVar>,
    index: HashMap<String, StaticId>,
}

#[derive(Clone)]
struct PendingStatic {
    qualified: String,
    owner: Option<String>,
    namespace: Option<String>,
    ty: TypeExpr,
    initializer: Option<Expression>,
    visibility: Visibility,
    is_readonly: bool,
    threadlocal: bool,
    is_weak: bool,
    is_extern: bool,
    extern_abi: Option<String>,
    extern_options: Option<crate::frontend::ast::ExternOptions>,
    link_library: Option<String>,
    is_weak_import: bool,
    span: Option<Span>,
}

impl StaticRegistry {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn register(
        &mut self,
        qualified: String,
        owner: Option<String>,
        namespace: Option<String>,
        ty: TypeExpr,
        initializer: Option<Expression>,
        visibility: Visibility,
        is_readonly: bool,
        threadlocal: bool,
        is_weak: bool,
        is_extern: bool,
        extern_abi: Option<String>,
        extern_options: Option<crate::frontend::ast::ExternOptions>,
        link_library: Option<String>,
        is_weak_import: bool,
        span: Option<Span>,
    ) {
        let qualified = qualified.replace('.', "::");
        let owner = owner.map(|value| value.replace('.', "::"));
        let namespace = namespace.map(|ns| ns.replace('.', "::"));
        if self.index.contains_key(&qualified)
            || self
                .pending
                .iter()
                .any(|entry| entry.qualified == qualified)
        {
            return;
        }
        self.pending.push(PendingStatic {
            qualified,
            owner,
            namespace,
            ty,
            initializer,
            visibility,
            is_readonly,
            threadlocal,
            is_weak,
            is_extern,
            extern_abi,
            extern_options,
            link_library,
            is_weak_import,
            span,
        });
    }

    pub(super) fn finalise(
        &mut self,
        eval_ctx: &mut ConstEvalContext<'_>,
        diagnostics: &mut Vec<LoweringDiagnostic>,
    ) {
        let pending = std::mem::take(&mut self.pending);
        for entry in pending {
            let ty = Ty::from_type_expr(&entry.ty);
            if entry.is_extern
                && entry
                    .extern_options
                    .as_ref()
                    .and_then(|opts| opts.library.as_ref())
                    .is_some()
            {
                diagnostics.push(LoweringDiagnostic {
                    message: "dynamic `@extern(library = ...)` bindings are not supported for globals; link the symbol instead of relying on runtime resolution".to_string(),
                    span: entry
                        .extern_options
                        .as_ref()
                        .and_then(|opts| opts.span)
                        .or(entry.span),
                });
                continue;
            }
            if entry.is_extern {
                if let Some(message) =
                    extern_static_type_error(&ty, eval_ctx.type_layouts, entry.span)
                {
                    diagnostics.push(message);
                    continue;
                }
            }
            let initializer: Option<ConstValue> = if entry.is_extern && entry.initializer.is_none()
            {
                None
            } else {
                entry.initializer.as_ref().and_then(|expr| {
                    match eval_ctx.evaluate_expression(
                        expr,
                        entry.namespace.as_deref(),
                        entry.owner.as_deref(),
                        None,
                        None,
                        &ty,
                        entry.span,
                    ) {
                        Ok(result) => Some(result.value),
                        Err(err) => {
                            let err = err.with_span_if_missing(entry.span);
                            diagnostics.push(LoweringDiagnostic {
                                message: err.message,
                                span: err.span,
                            });
                            None
                        }
                    }
                })
            };

            let id = StaticId(self.vars.len());
            self.index.insert(entry.qualified.clone(), id);
            self.vars.push(StaticVar {
                id,
                qualified: entry.qualified,
                owner: entry.owner.clone(),
                namespace: entry.namespace.clone(),
                ty: ty.clone(),
                visibility: entry.visibility,
                is_readonly: entry.is_readonly,
                threadlocal: entry.threadlocal,
                span: entry.span,
                initializer,
                is_weak: entry.is_weak,
                is_extern: entry.is_extern,
                is_import: entry.is_extern && entry.initializer.is_none(),
                is_weak_import: entry.is_weak_import,
                link_library: entry.link_library.clone(),
                extern_spec: entry.extern_abi.as_ref().map(|abi| MirExternSpec {
                    convention: abi.clone(),
                    library: entry
                        .extern_options
                        .as_ref()
                        .and_then(|opts| opts.library.clone()),
                    alias: entry
                        .extern_options
                        .as_ref()
                        .and_then(|opts| opts.alias.clone()),
                    binding: entry
                        .extern_options
                        .as_ref()
                        .map(|opts| opts.binding)
                        .unwrap_or(ExternBinding::Static),
                    optional: entry
                        .extern_options
                        .as_ref()
                        .map(|opts| opts.optional)
                        .unwrap_or(false),
                    charset: entry
                        .extern_options
                        .as_ref()
                        .and_then(|opts| opts.charset.clone()),
                    weak: entry.is_weak,
                }),
            });
        }
    }

    pub(super) fn lookup(&self, owner: &str, member: &str) -> Option<(StaticId, &StaticVar)> {
        let qualified = format!("{owner}::{member}");
        self.lookup_qualified(&qualified)
    }

    pub(super) fn lookup_qualified(&self, qualified: &str) -> Option<(StaticId, &StaticVar)> {
        self.index
            .get(qualified)
            .and_then(|id| self.vars.get(id.0).map(|var| (*id, var)))
    }

    pub(super) fn lookup_in_namespace(
        &self,
        namespace: Option<&str>,
        name: &str,
    ) -> Option<(StaticId, &StaticVar)> {
        let mut search = Vec::new();
        if let Some(ns) = namespace {
            let mut current = ns.replace('.', "::");
            loop {
                search.push(current.clone());
                if let Some(idx) = current.rfind("::") {
                    current.truncate(idx);
                } else {
                    break;
                }
            }
        }
        search.push(String::new());
        for candidate in search {
            let qualified = if candidate.is_empty() {
                name.to_string()
            } else {
                format!("{candidate}::{name}")
            };
            if let Some(result) = self.lookup_qualified(&qualified) {
                return Some(result);
            }
        }
        None
    }

    pub(super) fn drain_vars(&mut self) -> Vec<StaticVar> {
        self.index.clear();
        std::mem::take(&mut self.vars)
    }
}

fn extern_static_type_error(
    ty: &Ty,
    layouts: &TypeLayoutTable,
    span: Option<Span>,
) -> Option<LoweringDiagnostic> {
    if layouts.primitive_registry.lookup(ty).is_some() {
        return None;
    }
    let error = match ty {
        Ty::Pointer(_) => None,
        Ty::Fn(fn_ty) => match fn_ty.abi {
            crate::mir::Abi::Extern(_) => None,
            crate::mir::Abi::Chic => Some(
                "extern static function pointers must use an `@extern(\"C\")` ABI".to_string(),
            ),
        },
        Ty::Vector(_) => {
            return Some(LoweringDiagnostic {
                message: "extern static uses SIMD vector type; backend lowering for vectors is not available".into(),
                span,
            });
        }
        Ty::Named(named) => {
            let key = named.canonical_path();
            let Some(layout) = layouts.layout_for_name(&key) else {
                return Some(LoweringDiagnostic {
                    message: format!(
                        "extern static uses type `{}` whose layout is not available",
                        key
                    ),
                    span,
                });
            };
            let (repr, size, align) = match layout {
                TypeLayout::Struct(layout) => (layout.repr, layout.size, layout.align),
                TypeLayout::Enum(layout) => (layout.repr, layout.size, layout.align),
                TypeLayout::Union(layout) => (layout.repr, layout.size, layout.align),
                TypeLayout::Class(_) => {
                    return Some(LoweringDiagnostic {
                        message: format!(
                            "extern static uses non-FFI-safe class type `{}`; apply `@repr(c)` to a struct/union instead",
                            key
                        ),
                        span,
                    });
                }
            };
            if !matches!(repr, TypeRepr::C) {
                Some(format!(
                    "extern static type `{key}` must be annotated with `@repr(c)`"
                ))
            } else if size.is_none() || align.is_none() {
                Some(format!(
                    "extern static type `{key}` does not have a fully known size/align"
                ))
            } else {
                None
            }
        }
        Ty::Array(_) => Some(
            "fixed-size array statics are not yet supported in the FFI surface; use a struct wrapper"
                .to_string(),
        ),
        Ty::Nullable(_)
        | Ty::Span(_)
        | Ty::ReadOnlySpan(_)
        | Ty::Rc(_)
        | Ty::Arc(_)
        | Ty::Tuple(_)
        | Ty::Vec(_)
        | Ty::String
        | Ty::Str
        | Ty::Unit
        | Ty::Unknown
        | Ty::Ref(_)
        | Ty::TraitObject(_) => Some(format!(
            "extern static type `{}` is not FFI-safe",
            ty.canonical_name()
        )),
    };
    error.map(|message| LoweringDiagnostic { message, span })
}
