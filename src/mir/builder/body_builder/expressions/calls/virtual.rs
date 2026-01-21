use super::call_support::CallBindingInfo;
use super::*;
use crate::frontend::ast::GenericConstraintKind;
use crate::mir::TraitObjectTy;

fn strip_generics(name: &str) -> &str {
    name.split('<').next().unwrap_or(name)
}

body_builder_impl! {
    pub(super) fn try_build_virtual_dispatch(
        &self,
        info: &CallBindingInfo,
        receiver_index: usize,
    ) -> Option<VirtualDispatch> {
        let symbol = info.resolved_symbol.as_ref()?;
        if symbol.is_static {
            return None;
        }
        let owner = info.receiver_owner.as_deref()?;
        let member = info.member_name.as_deref()?;
        let slot_index = self
            .class_virtual_slots
            .get(owner)
            .and_then(|slots| slots.get(member))?
            .to_owned();
        let base_owner = info.force_base_receiver.then(|| owner.to_string());
        Some(VirtualDispatch {
            slot_index,
            receiver_index,
            base_owner,
        })
    }

    pub(super) fn try_build_trait_object_dispatch(
        &mut self,
        receiver_operand: &mut Operand,
        member_name: &str,
        span: Option<Span>,
    ) -> Option<TraitObjectDispatch> {
        let operand_ty = self.operand_ty(receiver_operand)?;
        let mut operand_ty = Self::strip_nullable(&operand_ty).clone();
        while let Ty::Ref(reference) = operand_ty {
            operand_ty = reference.element.clone();
        }
        let debug_dispatch = std::env::var("CHIC_DEBUG_INTERFACE_DISPATCH").is_ok()
            && (self.function_name.contains("ThreadFunctionRunner::Run")
                || self.function_name.contains("RuntimeCallbacks"));
        let debug_traits = debug_dispatch || std::env::var("CHIC_DEBUG_TRAIT_OBJECT").is_ok();
        if debug_dispatch {
            eprintln!(
                "[dispatch-trait] func={} receiver={receiver_operand:?} ty={operand_ty:?} member={member_name}",
                self.function_name
            );
        }
        let (object_ty, impl_type) = match operand_ty {
            Ty::TraitObject(object_ty) => (object_ty, None),
            Ty::Named(named) => {
                let canonical = self
                    .resolve_ty_name(&Ty::Named(named.clone()))
                    .unwrap_or_else(|| named.canonical_path());
                let candidates = self.trait_name_candidates(&canonical);
                if debug_dispatch {
                    eprintln!(
                        "[dispatch-trait] canonical={canonical} registry_has={} candidates={:?}",
                        self.trait_registry.contains_key(&canonical),
                        candidates
                    );
                }
                if !candidates.is_empty() {
                    (TraitObjectTy::new(candidates), Some(canonical))
                } else if self.generic_param_index.contains_key(&named.name) {
                    let bounds = self.generic_param_trait_bounds(&named.name);
                    if bounds.is_empty() {
                        return None;
                    }
                    (TraitObjectTy::new(bounds), Some(canonical))
                } else {
                    return None;
                }
            }
            _ => {
                return None;
            }
        };
        if debug_traits {
            eprintln!(
                "[dispatch-trait] object_traits={:?} member={member_name} impl_type={impl_type:?}",
                object_ty.traits
            );
        }
        if object_ty.traits.is_empty() {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "trait object call to `{member_name}` is not supported because the `dyn` type has no trait bounds"
                ),
                span,
            });
            return None;
        }

        let mut matches = Vec::new();
        let mut missing_traits = Vec::new();
        for trait_name in &object_ty.traits {
            let candidates = self.trait_name_candidates(trait_name);
            if candidates.is_empty() {
                missing_traits.push(trait_name.clone());
                continue;
            }
            if candidates.len() > 1 {
                let options = candidates.join(", ");
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "trait `{trait_name}` resolves to multiple candidates ({options}); use a fully-qualified trait name in the `dyn` type"
                    ),
                    span,
                });
                continue;
            }
            let resolved = &candidates[0];
            if let Some(info) = self.trait_registry.get(resolved) {
                if let Some(pos) = info.methods.iter().position(|method| method.name == member_name)
                {
                    matches.push((resolved.clone(), pos, info.methods.len()));
                }
            }
        }

        if matches.is_empty() {
            if !missing_traits.is_empty() {
                for trait_name in missing_traits {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "trait `{trait_name}` metadata is not available for dyn dispatch; ensure the trait is declared in this compilation unit"
                        ),
                        span,
                    });
                }
            } else {
                let trait_list = object_ty.traits.join(" + ");
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "method `{member_name}` is not defined on `dyn {trait_list}`"
                    ),
                    span,
                });
            }
            return None;
        }
        if debug_dispatch {
            eprintln!(
                "[dispatch-trait] matches={} trait_list={:?}",
                matches.len(),
                object_ty.traits
            );
        }
        if matches.len() > 1 {
            let trait_names = matches
                .iter()
                .map(|(name, _, _)| name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "method `{member_name}` is provided by multiple traits in the object (`{trait_names}`); cast to a specific trait before calling"
                ),
                span,
            });
            return None;
        }

        let (trait_name, slot_index_usize, slot_count_usize) = matches.remove(0);
        let slot_index = match u32::try_from(slot_index_usize) {
            Ok(value) => value,
            Err(_) => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "trait `{trait_name}` declares too many methods for dyn dispatch"
                    ),
                    span,
                });
                return None;
            }
        };
        let slot_count = match u32::try_from(slot_count_usize) {
            Ok(value) => value,
            Err(_) => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "trait `{trait_name}` declares too many methods for dyn dispatch"
                    ),
                    span,
                });
                return None;
            }
        };

        let local = self.ensure_operand_local(receiver_operand.clone(), span);
        *receiver_operand = Operand::Copy(Place::new(local));

        Some(TraitObjectDispatch {
            trait_name,
            method: member_name.to_string(),
            slot_index,
            slot_count,
            receiver_index: 0,
            impl_type,
        })
    }

    fn trait_name_candidates(&self, raw_name: &str) -> Vec<String> {
        let canonical = raw_name.replace('.', "::");
        let canonical_base = strip_generics(&canonical).to_string();
        if self.trait_registry.contains_key(&canonical_base) {
            return vec![canonical_base];
        }
        if self.trait_registry.contains_key(&canonical) {
            return vec![canonical];
        }
        let short = strip_generics(
            canonical_base
                .rsplit("::")
                .next()
                .unwrap_or(canonical_base.as_str()),
        )
        .to_string();
        let candidates: Vec<String> = self
            .trait_registry
            .keys()
            .filter_map(|candidate| {
                let candidate_base = strip_generics(candidate);
                let candidate_short = strip_generics(
                    candidate_base
                        .rsplit("::")
                        .next()
                        .unwrap_or(candidate_base),
                );
                if candidate_short == short {
                    Some(candidate.clone())
                } else {
                    None
                }
            })
            .collect();
        if candidates.is_empty() && std::env::var("CHIC_DEBUG_TRAIT_OBJECT").is_ok() {
            let keys = self.trait_registry.keys().cloned().collect::<Vec<_>>();
            eprintln!(
                "[dispatch-trait] trait lookup miss raw={raw_name} canonical_base={canonical_base} short={short} registry_keys={keys:?}"
            );
        }
        candidates
    }

    fn generic_param_trait_bounds(&self, name: &str) -> Vec<String> {
        let Some(decls) = self.symbol_index.function_decls(&self.function_name) else {
            return Vec::new();
        };
        let Some(decl) = decls.first() else {
            return Vec::new();
        };
        let Some(generics) = decl.function.generics.as_ref() else {
            return Vec::new();
        };
        for param in &generics.params {
            if param.name != name {
                continue;
            }
            let Some(data) = param.as_type() else {
                continue;
            };
            let mut bounds = Vec::new();
            for constraint in &data.constraints {
                if let GenericConstraintKind::Type(ty) = &constraint.kind {
                    let mut bound = Ty::from_type_expr(ty).canonical_name();
                    bound = bound.replace('.', "::");
                    bounds.push(strip_generics(&bound).to_string());
                }
            }
            bounds.sort();
            bounds.dedup();
            return bounds;
        }
        Vec::new()
    }
}
