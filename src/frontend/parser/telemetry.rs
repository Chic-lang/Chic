use std::sync::atomic::{AtomicBool, Ordering};

use crate::frontend::diagnostics::Span;
use crate::frontend::lexer::{Token, TokenKind};

static RECOVERY_TELEMETRY_ENABLED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Default)]
pub struct RecoveryTelemetryData {
    pub embedded_statement_invocations: usize,
    pub synchronize_invocations: usize,
    pub last_event: Option<RecoveryTelemetryEvent>,
}

#[derive(Clone, Debug)]
pub struct RecoveryTelemetryEvent {
    pub kind: RecoveryTelemetryKind,
    pub span: Option<Span>,
    pub token_kind: Option<TokenKind>,
}

#[derive(Clone, Debug)]
pub enum RecoveryTelemetryKind {
    EmbeddedStatement,
    Synchronize,
}

impl RecoveryTelemetryData {
    pub fn record(&mut self, kind: RecoveryTelemetryKind, token: Option<&Token>) {
        match kind {
            RecoveryTelemetryKind::EmbeddedStatement => {
                self.embedded_statement_invocations += 1;
            }
            RecoveryTelemetryKind::Synchronize => {
                self.synchronize_invocations += 1;
            }
        }

        self.last_event = Some(RecoveryTelemetryEvent {
            kind,
            span: token.map(|t| t.span),
            token_kind: token.map(|t| t.kind.clone()),
        });
    }
}

pub fn enable_recovery_telemetry() {
    RECOVERY_TELEMETRY_ENABLED.store(true, Ordering::SeqCst);
}

pub fn disable_recovery_telemetry() {
    RECOVERY_TELEMETRY_ENABLED.store(false, Ordering::SeqCst);
}

pub fn recovery_telemetry_enabled() -> bool {
    RECOVERY_TELEMETRY_ENABLED.load(Ordering::SeqCst)
}
