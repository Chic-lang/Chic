use super::*;

#[derive(Debug)]
pub(crate) struct MemberModifiers {
    remaining: Vec<Modifier>,
    pub async_modifier: Option<Modifier>,
    pub constexpr_modifier: Option<Modifier>,
    pub extern_modifier: Option<Modifier>,
    pub unsafe_modifier: Option<Modifier>,
    pub required_modifiers: Vec<Modifier>,
    virtual_modifier: Option<Modifier>,
    override_modifier: Option<Modifier>,
    sealed_modifier: Option<Modifier>,
    abstract_modifier: Option<Modifier>,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DispatchModifiers {
    pub virtual_span: Option<Span>,
    pub override_span: Option<Span>,
    pub sealed_span: Option<Span>,
    pub abstract_span: Option<Span>,
}

impl DispatchModifiers {
    pub fn any(self) -> bool {
        self.virtual_span.is_some()
            || self.override_span.is_some()
            || self.sealed_span.is_some()
            || self.abstract_span.is_some()
    }

    pub fn to_flags(self) -> MemberDispatch {
        MemberDispatch {
            is_virtual: self.virtual_span.is_some(),
            is_override: self.override_span.is_some(),
            is_sealed: self.sealed_span.is_some(),
            is_abstract: self.abstract_span.is_some(),
        }
    }
}

impl MemberModifiers {
    pub fn new(mut modifiers: Vec<Modifier>) -> Self {
        let async_modifier = Parser::take_modifier(&mut modifiers, "async");
        let constexpr_modifier = Parser::take_modifier(&mut modifiers, "constexpr");
        let extern_modifier = Parser::take_modifier(&mut modifiers, "extern");
        let unsafe_modifier = Parser::take_modifier(&mut modifiers, "unsafe");
        let virtual_modifier = Parser::take_modifier(&mut modifiers, "virtual");
        let override_modifier = Parser::take_modifier(&mut modifiers, "override");
        let sealed_modifier = Parser::take_modifier(&mut modifiers, "sealed");
        let abstract_modifier = Parser::take_modifier(&mut modifiers, "abstract");

        let mut required_modifiers = Vec::new();
        while let Some(modifier) = Parser::take_modifier(&mut modifiers, "required") {
            required_modifiers.push(modifier);
        }

        Self {
            remaining: modifiers,
            async_modifier,
            constexpr_modifier,
            extern_modifier,
            unsafe_modifier,
            required_modifiers,
            virtual_modifier,
            override_modifier,
            sealed_modifier,
            abstract_modifier,
        }
    }

    pub fn has_required(&self) -> bool {
        !self.required_modifiers.is_empty()
    }

    pub fn clone_remaining(&self) -> Vec<Modifier> {
        self.remaining.clone()
    }

    pub fn first_required_span(&self) -> Option<Span> {
        self.required_modifiers
            .first()
            .map(|modifier| modifier.span)
    }

    pub fn duplicate_required_span(&self) -> Option<Span> {
        self.required_modifiers.get(1).map(|modifier| modifier.span)
    }

    pub fn remaining(&self) -> &[Modifier] {
        &self.remaining
    }

    pub fn dispatch_modifiers(&self) -> DispatchModifiers {
        DispatchModifiers {
            virtual_span: self.virtual_modifier.as_ref().map(|m| m.span),
            override_span: self.override_modifier.as_ref().map(|m| m.span),
            sealed_span: self.sealed_modifier.as_ref().map(|m| m.span),
            abstract_span: self.abstract_modifier.as_ref().map(|m| m.span),
        }
    }
}

fn dispatch_marker_entries(markers: DispatchModifiers) -> [(&'static str, Option<Span>); 4] {
    [
        ("virtual", markers.virtual_span),
        ("override", markers.override_span),
        ("sealed", markers.sealed_span),
        ("abstract", markers.abstract_span),
    ]
}

parser_impl! {
    pub(super) fn reject_dispatch_modifiers(&mut self, modifiers: &MemberModifiers, context: &str) {
        let markers = modifiers.dispatch_modifiers();
        self.reject_dispatch_markers(markers, context);
    }

    pub(crate) fn reject_dispatch_markers(&mut self, markers: DispatchModifiers, context: &str) {
        for (name, span) in dispatch_marker_entries(markers) {
            if let Some(span) = span {
                self.push_error(format!("`{name}` modifier is not supported on {context}"), Some(span));
            }
        }
    }

    pub(super) fn build_method_dispatch(
        &mut self,
        modifiers: &MemberModifiers,
        context: &str,
        is_static_member: bool,
        class_is_static: bool,
    ) -> MemberDispatch {
        let markers = modifiers.dispatch_modifiers();
        self.build_dispatch_from_markers(
            markers,
            context,
            is_static_member,
            class_is_static,
            true,
        )
    }

    pub(crate) fn build_dispatch_from_markers(
        &mut self,
        markers: DispatchModifiers,
        context: &str,
        is_static_member: bool,
        class_is_static: bool,
        enforce_sealed_override: bool,
    ) -> MemberDispatch {
        if !markers.any() {
            return MemberDispatch::default();
        }
        if class_is_static {
            self.emit_instance_only_dispatch_errors(markers, "static classes");
            return MemberDispatch::default();
        }
        if is_static_member {
            let subject = format!("static {context}");
            self.emit_instance_only_dispatch_errors(markers, &subject);
            return MemberDispatch::default();
        }
        self.validate_dispatch_markers(markers, context, enforce_sealed_override)
    }

    pub(super) fn validate_dispatch_markers(
        &mut self,
        markers: DispatchModifiers,
        context: &str,
        enforce_sealed_override: bool,
    ) -> MemberDispatch {
        if enforce_sealed_override
            && markers.sealed_span.is_some()
            && markers.override_span.is_none()
        {
            self.push_error(
                format!("`sealed` modifier requires `override` on {context}"),
                markers.sealed_span,
            );
        }
        if markers.virtual_span.is_some() && markers.override_span.is_some() {
            self.push_error(
                format!("`virtual` and `override` modifiers cannot be combined on {context}"),
                markers.override_span.or(markers.virtual_span),
            );
        }
        if markers.abstract_span.is_some() && markers.override_span.is_some() {
            self.push_error(
                format!("`abstract` and `override` modifiers cannot be combined on {context}"),
                markers.abstract_span.or(markers.override_span),
            );
        }
        if markers.sealed_span.is_some() && markers.abstract_span.is_some() {
            self.push_error(
                format!("`sealed` and `abstract` modifiers cannot be combined on {context}"),
                markers.sealed_span.or(markers.abstract_span),
            );
        }
        markers.to_flags()
    }

    fn emit_instance_only_dispatch_errors(&mut self, markers: DispatchModifiers, subject: &str) {
        for (name, span) in dispatch_marker_entries(markers) {
            if let Some(span) = span {
                self.push_error(
                    format!("`{name}` modifier is not supported on {subject}"),
                    Some(span),
                );
            }
        }
    }
}
