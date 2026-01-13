use super::common::RequireExt;
use super::*;

#[test]
fn lowers_inline_asm_inside_unsafe_block() {
    let source = r#"
namespace Sample;

public int UseAsm(int input)
{
    unsafe
    {
        asm!("mov {0}, {1}", out(reg) input, in(reg) input, options(volatile), clobber("xmm0"));
    }

    return input;
}
"#;
    let parsed = parse_module(source).require("parse inline asm module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name == "Sample::UseAsm")
        .expect("missing UseAsm function");

    let asm = func
        .body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .find_map(|statement| match &statement.kind {
            StatementKind::InlineAsm(asm) => Some(asm),
            _ => None,
        })
        .expect("expected inline asm statement in lowered body");

    assert_eq!(asm.operands.len(), 2, "expected in/out operands");
    assert!(matches!(
        asm.operands[0].kind,
        InlineAsmOperandKind::Out { .. }
    ));
    assert!(matches!(
        asm.operands[1].kind,
        InlineAsmOperandKind::In { .. }
    ));
    assert!(asm.options.volatile, "volatile option should be set");
    assert_eq!(asm.clobbers.len(), 1);
    assert!(matches!(
        asm.clobbers[0],
        InlineAsmRegister::Explicit(ref reg) if reg == "xmm0"
    ));
    assert!(
        asm.template.iter().any(|piece| matches!(
            piece,
            InlineAsmTemplatePiece::Placeholder { operand_idx: 0, .. }
        )),
        "template should reference output operand"
    );
    verify_body(&func.body).require("inline asm body verification");
}
