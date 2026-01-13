use crate::frontend::ast::{Block, FunctionDecl, Item, Statement};
use crate::frontend::parser::ParseResult;
use crate::frontend::parser::tests::fixtures::{function_body, parse_ok};
use std::ops::Deref;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Shared helper that owns the [`ParseResult`] for a single-function fixture and
/// exposes ergonomic accessors for its body/statements.
pub(crate) struct FunctionFixture {
    parse: ParseResult,
}

impl FunctionFixture {
    /// Parse the provided source and stage builtin attributes.
    #[must_use]
    pub(crate) fn new(source: &str) -> Self {
        Self {
            parse: parse_ok(source),
        }
    }

    #[must_use]
    pub(crate) fn parse(&self) -> &ParseResult {
        &self.parse
    }

    pub(crate) fn assert_no_diagnostics(&self) -> &Self {
        assert!(
            self.parse.diagnostics.is_empty(),
            "expected no diagnostics, found: {:?}",
            self.parse.diagnostics
        );
        self
    }

    #[must_use]
    pub(crate) fn function(&self) -> &FunctionDecl {
        match &self.parse.module.items[0] {
            Item::Function(func) => func,
            other => panic!("expected first item to be a function, found {other:?}"),
        }
    }

    #[must_use]
    pub(crate) fn body(&self) -> &Block {
        function_body(self.function())
    }

    #[must_use]
    pub(crate) fn statements(&self) -> &[Statement] {
        &self.body().statements
    }
}

impl Deref for FunctionFixture {
    type Target = ParseResult;

    fn deref(&self) -> &Self::Target {
        &self.parse
    }
}

fn telemetry_mutex() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Acquire the global telemetry guard so individual tests can safely toggle the
/// parser recovery instrumentation without interfering with each other.
pub(crate) fn telemetry_guard() -> MutexGuard<'static, ()> {
    telemetry_mutex()
        .lock()
        .expect("failed to acquire telemetry guard")
}
