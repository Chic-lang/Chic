use crate::frontend::parser::*;

pub(super) struct CommonTypeAttributes {
    pub thread_safe_override: Option<bool>,
    pub shareable_override: Option<bool>,
    pub copy_override: Option<bool>,
    pub attributes: Vec<Attribute>,
}

pub(super) fn reject_pin_for_type(parser: &mut Parser<'_>, attrs: &mut CollectedAttributes) {
    if attrs.builtin.pin {
        parser.push_error(
            "`@pin` attribute is only supported on variable declarations",
            attrs.builtin.pin_span,
        );
    }
}

pub(super) fn reject_flags_for_non_enum(parser: &mut Parser<'_>, attrs: &mut CollectedAttributes) {
    if attrs.builtin.flags {
        parser.push_error(
            "`@flags` attribute is only supported on enum declarations",
            attrs.builtin.flags_span,
        );
    }
}

pub(super) fn take_common_type_attributes(attrs: &mut CollectedAttributes) -> CommonTypeAttributes {
    let flags = &mut attrs.builtin;
    CommonTypeAttributes {
        thread_safe_override: flags.thread_safe,
        shareable_override: flags.shareable,
        copy_override: flags.copy,
        attributes: attrs.take_list(),
    }
}
