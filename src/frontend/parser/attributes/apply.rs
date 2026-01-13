use super::flags::FieldAttributeFlags;
use super::*;

#[derive(Default)]
pub(crate) struct CollectedFieldAttributes {
    pub(crate) builtin: FieldAttributeFlags,
    pub(crate) list: Vec<Attribute>,
}

impl CollectedFieldAttributes {
    pub(crate) fn take_list(&mut self) -> Vec<Attribute> {
        std::mem::take(&mut self.list)
    }
}

parser_impl! {
    pub(in crate::frontend::parser) fn attributes_to_field_attributes(
        &mut self,
        attrs: &mut CollectedAttributes,
    ) -> CollectedFieldAttributes {
        let mut field_attrs = CollectedFieldAttributes::default();

        if attrs.builtin.pin {
            self.push_error(
                "`@pin` attribute is not supported on struct fields",
                attrs.builtin.pin_span,
            );
        }
        if let Some(span) = attrs.builtin.thread_safe_span {
            self.push_error(
                "`@thread_safe` attribute is not supported on struct fields",
                Some(span),
            );
        }
        if let Some(span) = attrs.builtin.shareable_span {
            self.push_error(
                "`@shareable` attribute is not supported on struct fields",
                Some(span),
            );
        }
        if let Some(span) = attrs.builtin.copy_span {
            self.push_error(
                "`@copy` attribute is not supported on struct fields",
                Some(span),
            );
        }
        if let Some(span) = attrs.builtin.flags_span {
            self.push_error(
                "`@flags` attribute is only supported on enum declarations",
                Some(span),
            );
        }
        if attrs.builtin.extern_attr {
            self.push_error(
                "`@extern` attribute is not supported on struct fields",
                attrs.builtin.extern_span,
            );
        }
        if attrs.builtin.link_library.is_some() {
            self.push_error(
                "`@link` attribute is not supported on struct fields",
                attrs.builtin.link_span,
            );
        }
        if !attrs.builtin.c_imports.is_empty() {
            for (_, span) in attrs.builtin.take_c_imports() {
                self.push_error("`@cimport` attribute is not supported on struct fields", span);
            }
        }
        if let Some(span) = attrs.builtin.mmio_span {
            self.push_error(
                "`@mmio` attribute is only supported on struct declarations",
                Some(span),
            );
        }
        if attrs.builtin.intrinsic {
            self.push_error(
                "`@Intrinsic` attribute is only supported on struct declarations",
                attrs.builtin.intrinsic_span,
            );
        }
        if attrs.builtin.struct_layout.is_some() {
            self.push_error(
                "`@StructLayout` attribute is only supported on struct declarations",
                attrs.builtin.struct_layout_span,
            );
        }
        if attrs.builtin.inline_attr.is_some() {
            self.push_error(
                "`@inline` attribute is only supported on type and function declarations",
                attrs.builtin.inline_span,
            );
        }

        let mut remaining = Vec::new();
        for attribute in attrs.take_list() {
            let lowered = attribute.name.to_ascii_lowercase();
            match lowered.as_str() {
                "register" => {
                    if field_attrs.builtin.mmio.is_some() {
                        self.push_error("duplicate `@register` attribute", attribute.span);
                    } else if let Some(attr) = self.register_attr_from_attribute(&attribute) {
                        field_attrs.builtin.mmio = Some(attr);
                    }
                    remaining.push(attribute);
                }
                other => {
                    self.push_error(
                        format!("unknown field attribute `@{other}`"),
                        attribute.span,
                    );
                    remaining.push(attribute);
                }
            }
        }

        field_attrs.list = remaining;
        field_attrs
    }

    pub(in crate::frontend::parser) fn apply_statement_attributes(
        &mut self,
        statement: &mut Statement,
        mut attrs: CollectedAttributes,
    ) {
        let mut list = attrs.take_list();
        if !list.is_empty() {
            statement
                .attributes
                .get_or_insert_with(Vec::new)
                .append(&mut list);
        }
        let flags = &mut attrs.builtin;
        if flags.pin {
            match &mut statement.kind {
                StatementKind::VariableDeclaration(decl) => {
                    decl.is_pinned = true;
                }
                _ => {
                    self.push_error(
                        "`@pin` attribute is only supported on variable declarations",
                        flags.pin_span.or(statement.span),
                    );
                }
            }
        }
        if flags.thread_safe.is_some() || flags.shareable.is_some() {
            let span = flags
                .thread_safe_span
                .or(flags.shareable_span)
                .or(flags.flags_span)
                .or(statement.span);
            self.push_error(
                "`@thread_safe`/`@shareable` attributes are only supported on type declarations",
                span,
                            );
        }
        if flags.copy.is_some() {
            let span = flags.copy_span.or(statement.span);
            self.push_error(
                "`@copy` attribute is only supported on type declarations",
                span,
            );
        }
        if flags.flags {
            let span = flags.flags_span.or(statement.span);
            self.push_error("`@flags` attribute is only supported on enum declarations", span);
        }
        if flags.extern_attr {
            self.push_error(
                "`@extern` attribute is only supported on function declarations",
                flags.extern_span.or(statement.span),
            );
        }
        if flags.link_library.is_some() {
            self.push_error(
                "`@link` attribute is only supported on function declarations",
                flags.link_span.or(statement.span),
            );
        }
        if !flags.c_imports.is_empty() {
            let span = flags
                .c_imports
                .first()
                .and_then(|(_, span)| *span)
                .or(statement.span);
            self.push_error(
                "`@cimport` attribute is only supported at namespace scope",
                span,
                            );
        }
        if flags.mmio_struct.is_some() {
            self.push_error(
                "`@mmio` attribute is only supported on struct declarations",
                flags.mmio_span.or(statement.span),
            );
        }
        if flags.intrinsic {
            self.push_error(
                "`@Intrinsic` attribute is only supported on struct declarations",
                flags.intrinsic_span.or(statement.span),
            );
        }
        if flags.struct_layout.is_some() {
            self.push_error(
                "`@StructLayout` attribute is only supported on struct declarations",
                flags.struct_layout_span.or(statement.span),
            );
        }
        if flags.inline_attr.is_some() {
            self.push_error(
                "`@inline` attribute is only supported on type and function declarations",
                flags.inline_span.or(statement.span),
            );
        }
        if flags.vectorize_hint.is_some() {
            self.push_error(
                "`@vectorize` attribute is only supported on function declarations",
                flags.vectorize_span.or(statement.span),
            );
        }
    }

    pub(in crate::frontend::parser) fn report_attribute_misuse(
        &mut self,
        attrs: CollectedAttributes,
        message: &str,
    ) {
        let (flags, _) = attrs.into_parts();
        if flags.pin {
            self.push_error(message, flags.pin_span);
        }
        if let Some(span) = flags.thread_safe_span {
            self.push_error(message, Some(span));
        }
        if let Some(span) = flags.shareable_span {
            self.push_error(message, Some(span));
        }
        if let Some(span) = flags.flags_span {
            self.push_error(message, Some(span));
        }
        if let Some(span) = flags.extern_span {
            self.push_error(message, Some(span));
        }
        if let Some(span) = flags.link_span {
            self.push_error(message, Some(span));
        }
        if let Some((_, span)) = flags.c_imports.first() {
            self.push_error(message, *span);
        }
        if let Some(span) = flags.mmio_span {
            self.push_error(message, Some(span));
        }
        if flags.intrinsic {
            self.push_error(message, flags.intrinsic_span);
        }
        if let Some(span) = flags.struct_layout_span {
            self.push_error(message, Some(span));
        }
        if let Some(span) = flags.vectorize_span {
            self.push_error(message, Some(span));
        }
    }
}
