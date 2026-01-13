use super::common::RequireExt;
use super::*;

#[test]
fn impl_trait_return_infers_concrete_type_and_emits_bound_constraint() {
    let source = r#"
namespace Demo;

public interface Formatter
{
    public int Format(int value);
}

public class Plain : Formatter
{
    public int Format(int value) { return value; }
}

public Formatter MakeFormatter()
{
    return new Plain();
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected lowering diagnostics: {:?}",
        lowering.diagnostics
    );

    let make_func = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::MakeFormatter"))
        .require("missing MakeFormatter");
    assert!(
        make_func
            .signature
            .ret
            .canonical_name()
            .ends_with("Formatter"),
        "factory should advertise the interface return type, got {}",
        make_func.signature.ret.canonical_name()
    );

    assert!(
        lowering.constraints.iter().any(|constraint| {
            matches!(&constraint.kind, ConstraintKind::ImplementsInterface { type_name, interface }
                if type_name == "Demo::Plain" && interface.ends_with("Formatter"))
        }),
        "expected ImplementsInterface constraint for Plain; got {:?}",
        lowering.constraints
    );
}
