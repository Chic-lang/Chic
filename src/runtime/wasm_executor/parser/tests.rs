use super::state;
use super::*;
use std::fmt::Display;

fn expect_ok<T, E: Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(err) => panic!("{context}: {err}"),
    }
}

fn expect_err<T, E: Display>(result: Result<T, E>, context: &str) -> E {
    match result {
        Ok(_) => panic!("{context}: expected error"),
        Err(err) => err,
    }
}

#[test]
fn parses_simple_module() {
    let module = simple_module(7);
    let parsed = expect_ok(parse_module(&module), "parse ok");
    assert_eq!(parsed.exports.len(), 1);
    assert!(parsed.exports.contains_key("chic_main"));
    assert_eq!(parsed.functions.len(), 1);
    assert_eq!(parsed.types.len(), 1);
}

#[test]
fn rejects_invalid_magic() {
    let mut module = simple_module(1);
    module[0] = 0xFF;
    let err = expect_err(parse_module(&module), "invalid magic");
    assert!(err.message.contains("invalid wasm header"));
}

#[test]
fn rejects_section_length_overflow() {
    // Header + type section with declared length larger than available bytes.
    let mut module = Vec::new();
    module.extend_from_slice(&WASM_MAGIC);
    module.extend_from_slice(&WASM_VERSION);
    module.push(1); // type section id
    module.push(5); // size
    module.extend_from_slice(&[0, 0, 0]); // less than declared

    let err = expect_err(parse_module(&module), "section length overflow");
    assert!(err.message.contains("section length exceeds module size"));
}

#[test]
fn rejects_unsupported_function_type_form() {
    let mut module = Vec::new();
    module.extend_from_slice(&WASM_MAGIC);
    module.extend_from_slice(&WASM_VERSION);
    module.push(1); // type section id
    module.push(4); // size
    module.extend_from_slice(&[1, 0x61, 0, 0]); // count=1, invalid form, zero params/results

    let err = expect_err(parse_module(&module), "unsupported function type form");
    assert!(err.message.contains("unsupported function type form"));
}

#[test]
fn rejects_unsupported_value_type() {
    let mut module = Vec::new();
    module.extend_from_slice(&WASM_MAGIC);
    module.extend_from_slice(&WASM_VERSION);
    module.push(1); // type section id
    module.push(5); // size
    module.extend_from_slice(&[1, 0x60, 1, 0x7B, 0]); // count, func type, param count, invalid type, results

    let err = expect_err(parse_module(&module), "unsupported value type");
    assert!(err.message.contains("unsupported value type"));
}

#[test]
fn parses_runtime_imports() {
    let module = module_with_runtime_imports(5);
    let parsed = expect_ok(parse_module(&module), "parse module with imports");
    assert_eq!(parsed.imports.len(), 3);
    assert_eq!(parsed.imports[0].module, "chic_rt");
    assert_eq!(parsed.imports[0].name, "panic");
    assert_eq!(parsed.imports[1].name, "abort");
    assert_eq!(parsed.imports[2].name, "throw");
    assert_eq!(parsed.functions.len(), 1);
}

#[test]
fn rejects_string_beyond_section() {
    let mut module = simple_module(1);
    // Locate the export name length byte and inflate it.
    let name_bytes = b"chic_main";
    if let Some(pos) = module
        .windows(name_bytes.len())
        .position(|w| w == name_bytes)
    {
        assert!(pos > 0, "export name unexpectedly at start");
        let len_index = pos - 1;
        module[len_index] = 0xFF;
    } else {
        panic!("chic_main symbol not found in sample module");
    }
    let err = expect_err(parse_module(&module), "string beyond section");
    assert!(err.message.contains("string length exceeds section"));
}

#[test]
fn parse_instructions_covers_all_supported_opcodes() {
    let instructions = expect_ok(
        state::parse_instructions(&supported_opcode_bytes()),
        "parse instructions",
    );
    assert_supported_instructions(&instructions);
}

#[test]
fn parse_instructions_rejects_unknown_opcode() {
    let result = state::parse_instructions(&[0xFF]);

    match result {
        Ok(_) => panic!("expected Err, found Ok result"),
        Err(err) => assert!(err.message.contains("unsupported wasm opcode")),
    }
}

#[test]
fn parse_instructions_rejects_non_empty_block_type() {
    let result = state::parse_instructions(&[0x02, 0x7F]);

    match result {
        Ok(_) => panic!("expected Err, found Ok result"),
        Err(err) => assert!(err.message.contains("only empty block types supported")),
    }
}

fn supported_opcode_bytes() -> Vec<u8> {
    vec![
        0x02, 0x40, // block
        0x03, 0x40, // loop
        0x04, 0x40, // if
        0x0B, // end if
        0x0C, 0x00, // br depth 0
        0x0B, // end loop
        0x0B, // end block
        0x00, // unreachable
        0x1A, // drop
        0x10, 0x00, // call
        0x41, 0x01, // const
        0x45, // eqz
        0x41, 0x02, 0x41, 0x03, 0x46, // eq
        0x41, 0x02, 0x41, 0x03, 0x47, // ne
        0x41, 0x02, 0x41, 0x03, 0x48, // lt_s
        0x41, 0x02, 0x41, 0x03, 0x49, // lt_u
        0x41, 0x02, 0x41, 0x03, 0x4C, // le_s
        0x41, 0x02, 0x41, 0x03, 0x4D, // le_u
        0x41, 0x05, 0x41, 0x03, 0x4A, // gt_s
        0x41, 0x05, 0x41, 0x03, 0x4B, // gt_u
        0x41, 0x05, 0x41, 0x03, 0x4E, // ge_s
        0x41, 0x05, 0x41, 0x03, 0x4F, // ge_u
        0x41, 0x05, 0x41, 0x03, 0x6A, // add
        0x41, 0x05, 0x41, 0x03, 0x6B, // sub
        0x41, 0x05, 0x41, 0x03, 0x6C, // mul
        0x41, 0x06, 0x41, 0x03, 0x6D, // div_s
        0x41, 0x06, 0x41, 0x03, 0x6E, // div_u
        0x41, 0x07, 0x41, 0x03, 0x6F, // rem_s
        0x41, 0x07, 0x41, 0x03, 0x70, // rem_u
        0x41, 0x05, 0x41, 0x03, 0x71, // and
        0x41, 0x05, 0x41, 0x03, 0x72, // or
        0x41, 0x05, 0x41, 0x03, 0x73, // xor
        0x41, 0x01, 0x41, 0x02, 0x74, // shl
        0x41, 0x08, 0x41, 0x01, 0x75, // shr_s
        0x41, 0x08, 0x41, 0x01, 0x76, // shr_u
        0x42, 0x02, // i64.const
        0x41, 0x01, 0xAD, // i64.extend_i32_u
        0x20, 0x00, // local.get
        0x21, 0x00, // local.set
        0x0F, // return
        0x42, 0x00, 0x50, // i64.eqz
        0x42, 0x05, 0x42, 0x03, 0x51, // i64.eq
        0x42, 0x05, 0x42, 0x03, 0x52, // i64.ne
        0x42, 0x05, 0x42, 0x03, 0x53, // i64.lt_s
        0x42, 0x05, 0x42, 0x03, 0x54, // i64.lt_u
        0x42, 0x05, 0x42, 0x03, 0x55, // i64.gt_s
        0x42, 0x05, 0x42, 0x03, 0x56, // i64.gt_u
        0x42, 0x05, 0x42, 0x03, 0x57, // i64.le_s
        0x42, 0x05, 0x42, 0x03, 0x58, // i64.le_u
        0x42, 0x05, 0x42, 0x03, 0x59, // i64.ge_s
        0x42, 0x05, 0x42, 0x03, 0x5A, // i64.ge_u
        0x42, 0x05, 0x42, 0x03, 0x7C, // i64.add
        0x42, 0x09, 0x42, 0x04, 0x7D, // i64.sub
        0x42, 0x02, 0x42, 0x03, 0x7E, // i64.mul
        0x42, 0x08, 0x42, 0x02, 0x7F, // i64.div_s
        0x42, 0x08, 0x42, 0x02, 0x80, // i64.div_u
        0x42, 0x08, 0x42, 0x02, 0x81, // i64.rem_s
        0x42, 0x08, 0x42, 0x02, 0x82, // i64.rem_u
        0x42, 0x0C, 0x42, 0x0A, 0x83, // i64.and
        0x42, 0x0C, 0x42, 0x0A, 0x84, // i64.or
        0x42, 0x0C, 0x42, 0x0A, 0x85, // i64.xor
        0x42, 0x01, 0x42, 0x03, 0x86, // i64.shl
        0x42, 0x08, 0x42, 0x01, 0x87, // i64.shr_s
        0x42, 0x10, 0x42, 0x01, 0x88, // i64.shr_u
        0x29, 0x00, 0x00, // i64.load offset 0
        0x37, 0x00, 0x00, // i64.store offset 0
        0x0B, // end function
    ]
}

fn assert_supported_instructions(instructions: &[super::super::instructions::Instruction]) {
    use super::super::instructions::Instruction;
    let mut seen = std::collections::HashSet::new();
    for inst in instructions {
        seen.insert(std::mem::discriminant(inst));
    }
    assert!(seen.contains(&std::mem::discriminant(&Instruction::Block { end: 0 })));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::Loop { end: 0 })));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::If { end: 0 })));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::Br { depth: 0 })));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::Call { func: 0 })));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::Unreachable)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::Drop)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I32Add)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I32RemS)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I32RemU)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I32DivU)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I32LtU)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I32LeU)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I32GtU)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I32GeU)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I32ShrU)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64Const(0))));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64ExtendI32U)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64Eqz)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64Eq)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64Ne)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64LtS)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64LtU)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64LeS)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64LeU)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64GtS)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64GtU)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64GeS)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64GeU)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64Add)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64DivU)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64RemU)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64And)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64Or)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64Xor)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64Shl)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64ShrS)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64ShrU)));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::I64Load { offset: 0 })));
    assert!(
        seen.contains(&std::mem::discriminant(&Instruction::I64Store {
            offset: 0
        }))
    );
    assert!(seen.contains(&std::mem::discriminant(&Instruction::LocalGet(0))));
    assert!(seen.contains(&std::mem::discriminant(&Instruction::LocalSet(0))));
    assert!(
        instructions
            .iter()
            .any(|inst| matches!(inst, Instruction::End))
    );
}

fn simple_module(constant: i32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&WASM_MAGIC);
    bytes.extend_from_slice(&WASM_VERSION);

    // Type section
    bytes.push(1);
    let payload = vec![1, 0x60, 0, 1, 0x7F];
    write_uleb(&mut bytes, len_u32(&payload));
    bytes.extend_from_slice(&payload);

    // Function section
    bytes.push(3);
    let payload = vec![1, 0];
    write_uleb(&mut bytes, len_u32(&payload));
    bytes.extend_from_slice(&payload);

    // Export section
    bytes.push(7);
    let mut payload = vec![1];
    push_string(&mut payload, "chic_main");
    payload.extend_from_slice(&[0, 0]);
    write_uleb(&mut bytes, len_u32(&payload));
    bytes.extend_from_slice(&payload);

    // Code section
    bytes.push(10);
    let mut payload = vec![1];
    let mut body = vec![0, 0x41];
    write_i32(&mut body, constant);
    body.extend_from_slice(&[0x0F, 0x0B]);
    write_uleb(&mut payload, len_u32(&body));
    payload.extend_from_slice(&body);
    write_uleb(&mut bytes, len_u32(&payload));
    bytes.extend_from_slice(&payload);
    bytes
}

fn module_with_runtime_imports(constant: i32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&WASM_MAGIC);
    bytes.extend_from_slice(&WASM_VERSION);

    // Type section (4 entries: entry, panic, abort, throw)
    bytes.push(1);
    let payload = vec![
        4, // count
        0x60, 0, 0, // entry: no params/results
        0x60, 1, 0x7F, 0, // panic: i32 param
        0x60, 1, 0x7F, 0, // abort: i32 param
        0x60, 2, 0x7F, 0x7E, 0, // throw: (i32, i64) params
    ];
    write_uleb(&mut bytes, len_u32(&payload));
    bytes.extend_from_slice(&payload);

    // Import section (panic, abort, throw)
    bytes.push(2);
    let mut import_payload = vec![3];
    push_string(&mut import_payload, "chic_rt");
    push_string(&mut import_payload, "panic");
    import_payload.push(0x00);
    write_uleb(&mut import_payload, 1);
    push_string(&mut import_payload, "chic_rt");
    push_string(&mut import_payload, "abort");
    import_payload.push(0x00);
    write_uleb(&mut import_payload, 2);
    push_string(&mut import_payload, "chic_rt");
    push_string(&mut import_payload, "throw");
    import_payload.push(0x00);
    write_uleb(&mut import_payload, 3);
    write_uleb(&mut bytes, len_u32(&import_payload));
    bytes.extend_from_slice(&import_payload);

    // Function section (one entry referencing type 0)
    bytes.push(3);
    let payload = vec![1, 0];
    write_uleb(&mut bytes, len_u32(&payload));
    bytes.extend_from_slice(&payload);

    // Export section (function index 3: after three imports)
    bytes.push(7);
    let mut payload = vec![1];
    push_string(&mut payload, "chic_main");
    payload.push(0);
    write_uleb(&mut payload, 3);
    write_uleb(&mut bytes, len_u32(&payload));
    bytes.extend_from_slice(&payload);

    // Code section
    bytes.push(10);
    let mut payload = vec![1];
    let mut body = vec![0, 0x41];
    write_i32(&mut body, constant);
    body.extend_from_slice(&[0x0F, 0x0B]);
    write_uleb(&mut payload, len_u32(&body));
    payload.extend_from_slice(&body);
    write_uleb(&mut bytes, len_u32(&payload));
    bytes.extend_from_slice(&payload);

    bytes
}

fn write_uleb(out: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = u8::try_from(value & 0x7F)
            .unwrap_or_else(|_| unreachable!("masked value outside byte range"));
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn write_i32(out: &mut Vec<u8>, mut value: i32) {
    loop {
        let masked = value & 0x7F;
        let byte = u8::try_from(masked)
            .unwrap_or_else(|_| unreachable!("masked value outside byte range"));
        value >>= 7;
        let done = (value == 0 && (byte & 0x40) == 0) || (value == -1 && (byte & 0x40) != 0);
        if done {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
}

fn push_string(buf: &mut Vec<u8>, text: &str) {
    write_uleb(buf, len_u32(text.as_bytes()));
    buf.extend_from_slice(text.as_bytes());
}

fn len_u32(slice: &[u8]) -> u32 {
    match u32::try_from(slice.len()) {
        Ok(len) => len,
        Err(err) => panic!("slice too large to encode: {err}"),
    }
}
