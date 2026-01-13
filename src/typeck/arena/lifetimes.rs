//! Borrow lifetime diagnostics helpers extracted from the arena.

use crate::mir::ParamMode;

/// Categorizes how a borrow escapes from its originating function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BorrowEscapeCategory {
    Return,
    Store { target: String },
    Capture { closure: String },
}

/// Captures the human-readable diagnostics for a borrow escape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BorrowEscapeMessages {
    pub(super) error: String,
    pub(super) lint: &'static str,
    pub(super) note: &'static str,
}

impl BorrowEscapeMessages {
    #[must_use]
    pub fn describe(
        function: &str,
        parameter: &str,
        mode: ParamMode,
        escape: &BorrowEscapeCategory,
    ) -> Self {
        let mode_keyword = mode_keyword(mode);
        let (error, lint, note) = match escape {
            BorrowEscapeCategory::Return => (
                format!(
                    "borrowed `{mode_keyword}` parameter `{parameter}` escapes from `{function}` by returning it"
                ),
                "borrow escapes scope: borrowed parameters must not be returned; see docs/guides/second_class_borrows.md",
                "consider returning an owned value instead, for example by copying or cloning the data",
            ),
            BorrowEscapeCategory::Store { target } => (
                format!(
                    "borrowed `{mode_keyword}` parameter `{parameter}` escapes from `{function}` by storing it in `{target}`"
                ),
                "borrow escapes scope: borrowed parameters must not be stored; see docs/guides/second_class_borrows.md",
                "store an owned copy of the data instead of the borrow, or restructure the code to finish using the borrow inside the call",
            ),
            BorrowEscapeCategory::Capture { closure } => (
                format!(
                    "borrowed `{mode_keyword}` parameter `{parameter}` escapes from `{function}` by capturing it in closure `{closure}`"
                ),
                "borrow escapes scope: borrowed parameters must not be captured; see docs/guides/second_class_borrows.md",
                "capture an owned clone inside the closure or rearrange the code so the borrow is consumed before the closure escapes",
            ),
        };
        Self { error, lint, note }
    }
}

fn mode_keyword(mode: ParamMode) -> &'static str {
    match mode {
        ParamMode::In => "in",
        ParamMode::Ref => "ref",
        ParamMode::Out => "out",
        ParamMode::Value => "value",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_borrows_use_return_message() {
        let messages = BorrowEscapeMessages::describe(
            "Demo::Run",
            "reader",
            ParamMode::Ref,
            &BorrowEscapeCategory::Return,
        );
        assert!(
            messages
                .error
                .contains("escapes from `Demo::Run` by returning it")
        );
        assert!(messages.lint.starts_with("borrow escapes scope"));
        assert!(
            messages
                .note
                .starts_with("consider returning an owned value")
        );
    }

    #[test]
    fn store_borrows_point_at_target() {
        let messages = BorrowEscapeMessages::describe(
            "Demo::Persist",
            "writer",
            ParamMode::Ref,
            &BorrowEscapeCategory::Store {
                target: "field".to_string(),
            },
        );
        assert!(
            messages.error.contains("storing it in `field`"),
            "unexpected error: {}",
            messages.error
        );
        assert!(messages.note.contains("store an owned copy"));
    }

    #[test]
    fn capture_borrows_name_closure() {
        let messages = BorrowEscapeMessages::describe(
            "Demo::Async",
            "token",
            ParamMode::In,
            &BorrowEscapeCategory::Capture {
                closure: "lambda#1".to_string(),
            },
        );
        assert!(
            messages.error.contains("closure `lambda#1`"),
            "closure missing from error: {}",
            messages.error
        );
        assert!(messages.note.contains("capture an owned clone"));
    }
}
