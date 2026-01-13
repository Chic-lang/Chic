use super::*;
use crate::frontend::ast::InlineAttr;
use crate::frontend::ast::{ExternBinding, VectorizeHint};
use crate::frontend::attributes::LayoutHints;
use crate::frontend::diagnostics::Span;

#[derive(Clone, Default)]
pub(crate) struct AttributeFlags {
    pub(in crate::frontend::parser) pin: bool,
    pub(in crate::frontend::parser) pin_span: Option<Span>,
    pub(in crate::frontend::parser) thread_safe: Option<bool>,
    pub(in crate::frontend::parser) thread_safe_span: Option<Span>,
    pub(in crate::frontend::parser) shareable: Option<bool>,
    pub(in crate::frontend::parser) shareable_span: Option<Span>,
    pub(in crate::frontend::parser) copy: Option<bool>,
    pub(in crate::frontend::parser) copy_span: Option<Span>,
    pub(in crate::frontend::parser) flags: bool,
    pub(in crate::frontend::parser) flags_span: Option<Span>,
    pub(in crate::frontend::parser) extern_attr: bool,
    pub(in crate::frontend::parser) extern_span: Option<Span>,
    pub(in crate::frontend::parser) extern_spec: Option<ParsedExternSpec>,
    pub(in crate::frontend::parser) link_library: Option<String>,
    pub(in crate::frontend::parser) link_span: Option<Span>,
    pub(in crate::frontend::parser) c_imports: Vec<(String, Option<Span>)>,
    pub(in crate::frontend::parser) friend_namespaces: Vec<(String, Option<Span>)>,
    pub(in crate::frontend::parser) package_imports: Vec<(String, Option<Span>)>,
    pub(in crate::frontend::parser) mmio_struct: Option<MmioStructAttr>,
    pub(in crate::frontend::parser) mmio_span: Option<Span>,
    pub(in crate::frontend::parser) vectorize_hint: Option<VectorizeHint>,
    pub(in crate::frontend::parser) vectorize_span: Option<Span>,
    pub(in crate::frontend::parser) intrinsic: bool,
    pub(in crate::frontend::parser) intrinsic_span: Option<Span>,
    pub(in crate::frontend::parser) struct_layout: Option<LayoutHints>,
    pub(in crate::frontend::parser) struct_layout_span: Option<Span>,
    pub(in crate::frontend::parser) inline_attr: Option<InlineAttr>,
    pub(in crate::frontend::parser) inline_span: Option<Span>,
    pub(in crate::frontend::parser) fallible: bool,
    pub(in crate::frontend::parser) fallible_span: Option<Span>,
}

#[derive(Default)]
pub(in crate::frontend::parser) struct FieldAttributeFlags {
    pub mmio: Option<MmioFieldAttr>,
    pub mmio_span: Option<Span>,
}

#[derive(Default)]
pub(crate) struct FunctionAttributeSet {
    pub mark_extern: bool,
    pub extern_span: Option<Span>,
    pub extern_spec: Option<ParsedExternSpec>,
    pub link_library: Option<String>,
    pub vectorize_hint: Option<VectorizeHint>,
}

#[derive(Default)]
pub(crate) struct StaticAttributeSet {
    pub mark_extern: bool,
    pub extern_span: Option<Span>,
    pub extern_spec: Option<ParsedExternSpec>,
    pub link_library: Option<String>,
}

impl AttributeFlags {
    #[must_use]
    pub(in crate::frontend::parser) fn is_empty(&self) -> bool {
        !self.pin
            && self.thread_safe.is_none()
            && self.shareable.is_none()
            && self.copy.is_none()
            && !self.flags
            && !self.extern_attr
            && self.link_library.is_none()
            && self.c_imports.is_empty()
            && self.friend_namespaces.is_empty()
            && self.package_imports.is_empty()
            && self.mmio_struct.is_none()
            && self.vectorize_hint.is_none()
            && !self.intrinsic
            && self.struct_layout.is_none()
            && self.inline_attr.is_none()
            && !self.fallible
    }

    pub(in crate::frontend::parser) fn record_pin(&mut self, span: Option<Span>) {
        self.pin = true;
        if self.pin_span.is_none() {
            self.pin_span = span;
        }
    }

    pub(in crate::frontend::parser) fn record_thread_safe(
        &mut self,
        value: bool,
        span: Option<Span>,
    ) {
        self.thread_safe = Some(value);
        if self.thread_safe_span.is_none() {
            self.thread_safe_span = span;
        }
    }

    pub(in crate::frontend::parser) fn record_shareable(
        &mut self,
        value: bool,
        span: Option<Span>,
    ) {
        self.shareable = Some(value);
        if self.shareable_span.is_none() {
            self.shareable_span = span;
        }
    }

    pub(in crate::frontend::parser) fn record_copy(&mut self, value: bool, span: Option<Span>) {
        self.copy = Some(value);
        if self.copy_span.is_none() {
            self.copy_span = span;
        }
    }

    pub(in crate::frontend::parser) fn record_flags(&mut self, span: Option<Span>) {
        self.flags = true;
        if self.flags_span.is_none() {
            self.flags_span = span;
        }
    }

    pub(in crate::frontend::parser) fn record_extern(
        &mut self,
        spec: ParsedExternSpec,
        span: Option<Span>,
    ) {
        self.extern_attr = true;
        if self.extern_span.is_none() {
            self.extern_span = span;
        }
        self.extern_spec = Some(spec);
    }

    pub(in crate::frontend::parser) fn record_link(&mut self, library: String, span: Option<Span>) {
        if self.link_library.is_none() {
            self.link_library = Some(library);
            self.link_span = span;
        }
    }

    pub(in crate::frontend::parser) fn push_c_import(
        &mut self,
        header: String,
        span: Option<Span>,
    ) {
        self.c_imports.push((header, span));
    }

    pub(in crate::frontend::parser) fn take_c_imports(&mut self) -> Vec<(String, Option<Span>)> {
        std::mem::take(&mut self.c_imports)
    }

    pub(in crate::frontend::parser) fn push_friend_namespace(
        &mut self,
        prefix: String,
        span: Option<Span>,
    ) {
        self.friend_namespaces.push((prefix, span));
    }

    pub(in crate::frontend::parser) fn take_friend_namespaces(
        &mut self,
    ) -> Vec<(String, Option<Span>)> {
        std::mem::take(&mut self.friend_namespaces)
    }

    pub(in crate::frontend::parser) fn push_package_import(
        &mut self,
        name: String,
        span: Option<Span>,
    ) {
        self.package_imports.push((name, span));
    }

    pub(in crate::frontend::parser) fn take_package_imports(
        &mut self,
    ) -> Vec<(String, Option<Span>)> {
        std::mem::take(&mut self.package_imports)
    }

    pub(in crate::frontend::parser) fn record_mmio_struct(
        &mut self,
        attr: MmioStructAttr,
        span: Option<Span>,
    ) {
        self.mmio_struct = Some(attr);
        if self.mmio_span.is_none() {
            self.mmio_span = span;
        }
    }

    pub(in crate::frontend::parser) fn record_vectorize(
        &mut self,
        hint: VectorizeHint,
        span: Option<Span>,
    ) {
        self.vectorize_hint = Some(hint);
        if self.vectorize_span.is_none() {
            self.vectorize_span = span;
        }
    }

    pub(in crate::frontend::parser) fn record_intrinsic(&mut self, span: Option<Span>) {
        self.intrinsic = true;
        if self.intrinsic_span.is_none() {
            self.intrinsic_span = span;
        }
    }

    pub(in crate::frontend::parser) fn record_struct_layout(
        &mut self,
        hints: LayoutHints,
        span: Option<Span>,
    ) {
        self.struct_layout = Some(hints);
        if self.struct_layout_span.is_none() {
            self.struct_layout_span = span;
        }
    }

    pub(in crate::frontend::parser) fn record_inline(
        &mut self,
        attr: InlineAttr,
        span: Option<Span>,
    ) {
        self.inline_attr = Some(attr);
        if self.inline_span.is_none() {
            self.inline_span = span;
        }
    }

    pub(in crate::frontend::parser) fn record_fallible(&mut self, span: Option<Span>) {
        self.fallible = true;
        if self.fallible_span.is_none() {
            self.fallible_span = span;
        }
    }

    pub(in crate::frontend::parser) fn take_function_attributes(&mut self) -> FunctionAttributeSet {
        let attrs = FunctionAttributeSet {
            mark_extern: self.extern_attr,
            extern_span: self.extern_span,
            extern_spec: self.extern_spec.take(),
            link_library: self.link_library.take(),
            vectorize_hint: self.vectorize_hint.take(),
        };
        self.extern_attr = false;
        self.extern_span = None;
        self.link_span = None;
        self.vectorize_span = None;
        attrs
    }

    pub(in crate::frontend::parser) fn take_static_attributes(&mut self) -> StaticAttributeSet {
        let attrs = StaticAttributeSet {
            mark_extern: self.extern_attr,
            extern_span: self.extern_span,
            extern_spec: self.extern_spec.take(),
            link_library: self.link_library.take(),
        };
        self.extern_attr = false;
        self.extern_span = None;
        self.link_span = None;
        attrs
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ParsedExternSpec {
    pub convention: Option<String>,
    pub library: Option<String>,
    pub alias: Option<String>,
    pub binding: Option<ExternBinding>,
    pub optional: Option<bool>,
    pub charset: Option<String>,
    pub span: Option<Span>,
}
