use super::common::RequireExt;
use super::*;
use crate::mmio::AddressSpaceId;

#[test]
fn lowering_emits_mmio_operands_and_stores() {
    let source = r"
namespace Sample;

@mmio(base = 16384, size = 64)
public struct Device
{
    @register(offset = 4, width = 32)
    public int Data;
}

public int Read(Device dev)
{
    unsafe
    {
        return dev.Data;
    }
}

public void Write(Device dev, int value)
{
    unsafe
    {
        dev.Data = value;
    }
}
";
    let parsed = parse_module(source).require("parse mmio module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let read = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name == "Sample::Read")
        .expect("missing Sample::Read function");

    let mut read_found = false;
    for block in &read.body.blocks {
        for statement in &block.statements {
            if let MirStatementKind::Assign { value, .. } = &statement.kind {
                if let Rvalue::Use(Operand::Mmio(spec)) = value {
                    read_found = true;
                    assert_eq!(spec.base_address, 0x4000);
                    assert_eq!(spec.offset, 0x04);
                    assert_eq!(spec.width_bits, 32);
                    assert_eq!(
                        spec.address_space.to_raw(),
                        AddressSpaceId::DEFAULT.to_raw()
                    );
                }
            }
        }
    }
    assert!(read_found, "expected MMIO read assignment in lowered body");

    let write = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name == "Sample::Write")
        .expect("missing Sample::Write function");

    let mut write_found = false;
    for block in &write.body.blocks {
        for statement in &block.statements {
            if let MirStatementKind::MmioStore { target, .. } = &statement.kind {
                write_found = true;
                assert_eq!(target.base_address, 0x4000);
                assert_eq!(target.offset, 0x04);
                assert_eq!(target.width_bits, 32);
                assert_eq!(
                    target.address_space.to_raw(),
                    AddressSpaceId::DEFAULT.to_raw()
                );
            }
        }
    }
    assert!(write_found, "expected MMIO store in lowered body");

    verify_body(&read.body).require("verify read body");
    verify_body(&write.body).require("verify write body");
}

#[test]
fn write_to_read_only_register_reports_diagnostic() {
    let source = r#"
namespace Sample;

@mmio(base = 24576)
public struct Device
{
    @register(offset = 0, width = 16, access = "ro")
    public ushort Status;
}

public void Update(Device dev)
{
    dev.Status = 1;
}
"#;

    let parsed = parse_module(source).require("parse read-only mmio module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("read-only")),
        "expected read-only diagnostic, found {0:?}",
        lowering.diagnostics
    );
}

#[test]
fn read_mmio_requires_unsafe_block() {
    let source = r#"
namespace Sample;

@mmio(base = 4096)
public struct Device
{
    @register(offset = 0, width = 32)
    public int Data;
}

public int Read(Device dev)
{
    return dev.Data;
}
"#;

    let parsed = parse_module(source).require("parse mmio module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("requires an `unsafe` block")),
        "expected unsafe diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn read_from_write_only_register_reports_diagnostic() {
    let source = r#"
namespace Sample;

@mmio(base = 4096)
public struct Device
{
    @register(offset = 0, width = 8, access = "wo")
    public byte Command;
}

public byte Poll(Device dev)
{
    unsafe
    {
        return dev.Command;
    }
}
"#;

    let parsed = parse_module(source).require("parse write-only module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("write-only")),
        "expected write-only diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn lowering_carries_address_space_identifier() {
    let source = r#"
namespace Sample;

@mmio(base = 8192, address_space = "apb")
public struct Device
{
    @register(offset = 4, width = 16)
    public ushort Status;
}

public ushort Read(Device dev)
{
    unsafe
    {
        return dev.Status;
    }
}
"#;
    let parsed = parse_module(source).require("parse mmio module with address space");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    let function = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name == "Sample::Read")
        .expect("missing Sample::Read function");
    let mut found = false;
    for block in &function.body.blocks {
        for statement in &block.statements {
            if let MirStatementKind::Assign { value, .. } = &statement.kind {
                if let Rvalue::Use(Operand::Mmio(spec)) = value {
                    found = true;
                    let expected = AddressSpaceId::from_name("apb");
                    assert_eq!(
                        spec.address_space.to_raw(),
                        expected.to_raw(),
                        "address space id should match hashed value"
                    );
                }
            }
        }
    }
    assert!(found, "expected MMIO read in lowered body");
}
