use crate::frontend::ast::{Attribute, AttributeArgument, AttributeKind};
use crate::frontend::diagnostics::Diagnostic;

use super::model::{
    HygieneId, MacroInvocation, MacroInvocationKind, normalise_name, trim_macro_name,
};

#[derive(Default)]
pub struct HygieneTracker {
    next: u64,
    pass: usize,
}

impl HygieneTracker {
    pub fn start_pass(&mut self, pass: usize) {
        self.pass = pass;
        self.next = 0;
    }

    pub fn fresh(&mut self) -> HygieneId {
        let value = ((self.pass as u64) << 32) | self.next;
        self.next += 1;
        HygieneId::new(value)
    }
}

pub fn collect_invocations(
    attributes: &mut Vec<Attribute>,
    hygiene: &mut HygieneTracker,
) -> (Vec<MacroInvocation>, Vec<Diagnostic>) {
    if attributes.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut diagnostics = Vec::new();
    let mut macros = Vec::new();
    let retained = attributes
        .drain(..)
        .filter_map(|attr| match attr.kind {
            AttributeKind::Macro => {
                if !attr.macro_metadata.expandable {
                    return Some(attr);
                }
                let raw = attr.raw.clone();
                if attr.name.eq_ignore_ascii_case("derive") {
                    if attr.arguments.is_empty() {
                        diagnostics.push(Diagnostic::error(
                            "`@derive` requires at least one macro name",
                            attr.span,
                        ));
                    }
                    let tokens = attr.macro_metadata.tokens.clone();
                    for argument in attr.arguments {
                        push_derive_invocation(
                            argument,
                            attr.span,
                            raw.clone(),
                            tokens.clone(),
                            hygiene.fresh(),
                            &mut macros,
                            &mut diagnostics,
                        );
                    }
                } else {
                    macros.push(MacroInvocation::new(
                        MacroInvocationKind::Attribute,
                        attr.name.clone(),
                        attr.span,
                        raw,
                        attr.macro_metadata.tokens,
                        hygiene.fresh(),
                    ));
                }
                None
            }
            _ => Some(attr),
        })
        .collect();
    *attributes = retained;
    (macros, diagnostics)
}

fn push_derive_invocation(
    argument: AttributeArgument,
    derive_span: Option<crate::frontend::diagnostics::Span>,
    raw: Option<String>,
    tokens: Vec<crate::frontend::lexer::Token>,
    hygiene: HygieneId,
    macros: &mut Vec<MacroInvocation>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let macro_name = trim_macro_name(&argument.value);
    if macro_name.is_empty() {
        diagnostics.push(Diagnostic::error(
            "`@derive` macro name must not be empty",
            argument.span.or(derive_span),
        ));
        return;
    }
    macros.push(MacroInvocation::new(
        MacroInvocationKind::Derive,
        normalise_name(&macro_name),
        argument.span.or(derive_span),
        raw.clone(),
        tokens,
        hygiene,
    ));
}
