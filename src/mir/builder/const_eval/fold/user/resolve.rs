use crate::frontend::ast::FunctionDecl;
use crate::frontend::diagnostics::Span;
use crate::mir::ConstEvalContext;
use crate::mir::builder::const_eval::ConstEvalResult;
use crate::mir::builder::const_eval::diagnostics::ConstEvalError;
use crate::mir::builder::symbol_index::{FunctionDeclSymbol, candidate_function_names};

impl<'a> ConstEvalContext<'a> {
    pub(crate) fn resolve_const_function(
        &self,
        namespace: Option<&str>,
        owner: Option<&str>,
        segments: &[String],
        args: &[(Option<String>, ConstEvalResult)],
        span: Option<Span>,
    ) -> Result<FunctionDeclSymbol, ConstEvalError> {
        if segments.is_empty() {
            return Err(ConstEvalError {
                message: "call target is not a valid path".into(),
                span,
            });
        }
        let segment_refs: Vec<&str> = segments.iter().map(String::as_str).collect();
        let mut constexpr_matches = Vec::new();
        let mut inferred_matches = Vec::new();
        for candidate in candidate_function_names(namespace, &segment_refs) {
            if let Some(functions) = self.symbol_index.function_decls(&candidate) {
                Self::collect_matching_const_functions(
                    functions,
                    args,
                    &mut constexpr_matches,
                    &mut inferred_matches,
                );
            }
        }

        if constexpr_matches.is_empty() {
            if let Some(owner) = owner {
                let mut candidate = owner.to_string();
                if !segments.is_empty() {
                    candidate.push_str("::");
                    candidate.push_str(&segments.join("::"));
                }
                if let Some(functions) = self.symbol_index.function_decls(&candidate) {
                    Self::collect_matching_const_functions(
                        functions,
                        args,
                        &mut constexpr_matches,
                        &mut inferred_matches,
                    );
                }
            }
        }

        let matches = if constexpr_matches.is_empty() {
            inferred_matches
        } else {
            constexpr_matches
        };

        if matches.is_empty() {
            let display = segments.join("::");
            return Err(ConstEvalError {
                message: format!(
                    "`{display}` cannot be evaluated at compile time (no eligible function found)"
                ),
                span,
            });
        }

        if matches.len() > 1 {
            let display = segments.join("::");
            return Err(ConstEvalError {
                message: format!(
                    "compile-time call to `{display}` is ambiguous across {} overloads",
                    matches.len()
                ),
                span,
            });
        }

        Ok(matches.into_iter().next().unwrap())
    }

    fn collect_matching_const_functions(
        functions: &[FunctionDeclSymbol],
        args: &[(Option<String>, ConstEvalResult)],
        constexpr_matches: &mut Vec<FunctionDeclSymbol>,
        inferred_matches: &mut Vec<FunctionDeclSymbol>,
    ) {
        for decl in functions {
            if decl.function.signature.parameters.len() != args.len()
                || !Self::named_arguments_match(&decl.function, args)
            {
                continue;
            }

            if decl.function.is_constexpr {
                constexpr_matches.push(decl.clone());
            } else if decl.function.body.is_some() {
                // Allow pure (but not explicitly `constexpr`) functions to run during
                // const-eval when they meet all other restrictions.
                inferred_matches.push(decl.clone());
            }
        }
    }

    fn named_arguments_match(
        function: &FunctionDecl,
        args: &[(Option<String>, ConstEvalResult)],
    ) -> bool {
        for (index, arg) in args.iter().enumerate() {
            if let Some(name) = &arg.0 {
                if let Some(param) = function.signature.parameters.get(index) {
                    if !name.eq_ignore_ascii_case(&param.name) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        true
    }
}
