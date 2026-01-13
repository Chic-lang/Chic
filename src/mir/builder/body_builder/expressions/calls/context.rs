use super::call_support::CallBindingInfo;
use super::*;

/// Immutable view over the data that every call-lowering helper cares about.
///
/// Passing a `CallContext` avoids threading long parameter lists (span,
/// destination, generics, receiver state, dispatch mode, etc.) through the
/// intrinsic/direct/virtual helpers.  The context borrows the live
/// `CallBindingInfo` so helpers can inspect the current resolution state while
/// `lower_call_with_destination` continues to mutate it when needed.
#[derive(Clone, Copy)]
pub(crate) struct CallContext<'a> {
    span: Option<Span>,
    call_info: &'a CallBindingInfo,
    has_receiver: bool,
}

impl<'a> CallContext<'a> {
    pub(crate) fn new(
        span: Option<Span>,
        call_info: &'a CallBindingInfo,
        has_receiver: bool,
    ) -> Self {
        Self {
            span,
            call_info,
            has_receiver,
        }
    }

    pub(crate) fn span(&self) -> Option<Span> {
        self.span
    }

    pub(crate) fn info(&self) -> &'a CallBindingInfo {
        self.call_info
    }

    pub(crate) fn has_receiver(&self) -> bool {
        self.has_receiver
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::diagnostics::Span;

    #[test]
    fn context_exposes_span_info_and_receiver_flag() {
        let info = CallBindingInfo {
            member_name: Some("Touch".into()),
            ..CallBindingInfo::default()
        };
        let ctx = CallContext::new(Some(Span::new(1, 5)), &info, true);
        assert_eq!(ctx.span(), Some(Span::new(1, 5)));
        assert!(ctx.has_receiver());
        assert_eq!(ctx.info().member_name.as_deref(), Some("Touch"));
    }
}
