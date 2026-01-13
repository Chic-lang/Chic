pub const ASYNC_DIAG_STACK_ONLY: &str = "AS0001";
pub const ASYNC_DIAG_FRAME_LIMIT: &str = "AS0002";
pub const ASYNC_DIAG_NO_CAPTURE: &str = "AS0003";
pub const ASYNC_DIAG_ATTRIBUTE: &str = "AS0004";

use crate::frontend::diagnostics::Span;

/// Source-level policy hints applied to an async frame.
#[derive(Debug, Clone, Default)]
pub struct AsyncFramePolicy {
    pub stack_only: Option<AttrSource>,
    pub frame_limit: Option<FrameLimitAttr>,
    pub no_capture: Option<NoCaptureAttr>,
    /// Enables verbose promotion logging for this frame.
    pub log_promotion: bool,
}

/// Marker describing where a policy was declared.
#[derive(Debug, Clone, Default)]
pub struct AttrSource {
    pub span: Option<Span>,
}

/// `@frame_limit(bytes)` payload.
#[derive(Debug, Clone)]
pub struct FrameLimitAttr {
    pub bytes: u64,
    pub span: Option<Span>,
}

/// `@no_capture` payload.
#[derive(Debug, Clone)]
pub struct NoCaptureAttr {
    pub mode: NoCaptureMode,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoCaptureMode {
    Any,
    MoveOnly,
}

impl Default for NoCaptureMode {
    fn default() -> Self {
        Self::Any
    }
}

impl AsyncFramePolicy {
    #[must_use]
    pub fn is_configured(&self) -> bool {
        self.stack_only.is_some()
            || self.frame_limit.is_some()
            || self.no_capture.is_some()
            || self.log_promotion
    }
}
