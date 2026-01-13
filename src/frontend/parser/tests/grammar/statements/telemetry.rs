use super::super::common::{FunctionFixture, telemetry_guard};
use crate::frontend::parser::{
    RecoveryTelemetryKind, disable_recovery_telemetry, enable_recovery_telemetry,
};

#[test]
fn recovery_telemetry_disabled_by_default() {
    let _guard = telemetry_guard();
    disable_recovery_telemetry();
    let source = r"
public void Noop()
{
    ;
}
";
    let fixture = FunctionFixture::new(source);
    assert!(fixture.parse().diagnostics.is_empty());
    assert!(
        fixture.parse().recovery_telemetry.is_none(),
        "telemetry should be None when disabled"
    );
}

#[test]
fn recovery_telemetry_records_events() {
    let _guard = telemetry_guard();
    disable_recovery_telemetry();
    enable_recovery_telemetry();
    let source = r"
public void TelemetryDemo()
{
    if (true)
        return;
    while (false)
    {
    }
}
";
    let fixture = FunctionFixture::new(source);
    disable_recovery_telemetry();
    fixture.assert_no_diagnostics();
    let telemetry = fixture
        .parse()
        .recovery_telemetry
        .as_ref()
        .expect("expected telemetry data");
    assert!(
        telemetry.embedded_statement_invocations >= 2,
        "expected embedded statements to be recorded: {:?}",
        telemetry
    );
    assert_eq!(telemetry.synchronize_invocations, 0);
    assert!(telemetry.last_event.is_some());
    if let Some(event) = &telemetry.last_event {
        assert!(matches!(
            event.kind,
            RecoveryTelemetryKind::EmbeddedStatement
        ));
    }
}
