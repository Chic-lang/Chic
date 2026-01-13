use crate::error::Error;

use super::ensure_u32;

pub(crate) fn push_i32_const_expr(buf: &mut Vec<u8>, value: i32) {
    push_i32_const(buf, value);
    push_byte(buf, 0x0B);
}

pub(crate) fn push_block_like(buf: &mut Vec<u8>, opcode: u8) {
    buf.push(opcode);
    buf.push(0x40);
}

pub(crate) fn push_byte(buf: &mut Vec<u8>, opcode: u8) {
    buf.push(opcode);
}

pub(crate) fn push_br(buf: &mut Vec<u8>, depth: u32) {
    buf.push(0x0C);
    write_u32(buf, depth);
}

pub(crate) fn push_call(buf: &mut Vec<u8>, index: u32) {
    buf.push(0x10);
    write_u32(buf, index);
}

pub(crate) fn push_call_indirect(buf: &mut Vec<u8>, type_index: u32, table_index: u32) {
    buf.push(0x11);
    write_u32(buf, type_index);
    write_u32(buf, table_index);
}

pub(crate) fn push_i32_const(buf: &mut Vec<u8>, value: i32) {
    buf.push(0x41);
    write_i32(buf, value);
}

pub(crate) fn push_i64_const(buf: &mut Vec<u8>, value: i64) {
    buf.push(0x42);
    write_i64(buf, value);
}

pub(crate) fn push_f32_const(buf: &mut Vec<u8>, value: f32) {
    buf.push(0x43);
    buf.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn push_f64_const(buf: &mut Vec<u8>, value: f64) {
    buf.push(0x44);
    buf.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn push_local(buf: &mut Vec<u8>, opcode: u8, index: u32) {
    buf.push(opcode);
    write_u32(buf, index);
}

pub(crate) fn push_global(buf: &mut Vec<u8>, opcode: u8, index: u32) {
    buf.push(opcode);
    write_u32(buf, index);
}

pub(crate) fn push_memory(buf: &mut Vec<u8>, opcode: u8, offset: u32) {
    buf.push(opcode);
    write_u32(buf, 0); // alignment immediate (use natural alignment)
    write_u32(buf, offset);
}

pub(crate) fn push_atomic_memory(buf: &mut Vec<u8>, opcode: u8, offset: u32) {
    buf.push(0xFE);
    buf.push(opcode);
    write_u32(buf, 0);
    write_u32(buf, offset);
}

pub(crate) fn push_atomic_fence(buf: &mut Vec<u8>) {
    buf.push(0xFE);
    buf.push(0x03);
    buf.push(0x00);
}

pub(crate) fn write_u32(buf: &mut Vec<u8>, value: u32) {
    let mut val = value;
    loop {
        let chunk = val & 0x7F;
        let Ok(mut byte) = u8::try_from(chunk) else {
            panic!("WASM leb128 encoding overflowed u8 range (chunk={chunk})");
        };
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if val == 0 {
            break;
        }
    }
}

pub(crate) fn push_string(buf: &mut Vec<u8>, text: &str) -> Result<(), Error> {
    let len = ensure_u32(text.len(), "string literal exceeds WebAssembly limits")?;
    write_u32(buf, len);
    buf.extend_from_slice(text.as_bytes());
    Ok(())
}

fn write_i32(buf: &mut Vec<u8>, value: i32) {
    let mut val = i64::from(value);
    loop {
        let chunk = val & 0x7F;
        let Ok(mut byte) = u8::try_from(chunk) else {
            panic!("WASM signed leb128 encoding overflowed u8 range (chunk={chunk})");
        };
        let sign_bit = byte & 0x40;
        val >>= 7;
        let done = (val == 0 && sign_bit == 0) || (val == -1 && sign_bit != 0);
        if !done {
            byte |= 0x80;
        }
        buf.push(byte);
        if done {
            break;
        }
    }
}

fn write_i64(buf: &mut Vec<u8>, value: i64) {
    let mut val = value;
    loop {
        let chunk = val & 0x7F;
        let Ok(mut byte) = u8::try_from(chunk) else {
            panic!("WASM signed leb128 encoding overflowed u8 range (chunk={chunk})");
        };
        let sign_bit = byte & 0x40;
        val >>= 7;
        let done = (val == 0 && sign_bit == 0) || (val == -1 && sign_bit != 0);
        if !done {
            byte |= 0x80;
        }
        buf.push(byte);
        if done {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        push_block_like, push_br, push_byte, push_call, push_call_indirect, push_f32_const,
        push_f64_const, push_global, push_i32_const, push_i32_const_expr, push_i64_const,
        push_local, push_memory, push_string, write_u32,
    };

    #[test]
    fn push_block_like_emits_block_header() {
        let mut buf = Vec::new();
        push_block_like(&mut buf, 0x02);
        assert_eq!(buf, vec![0x02, 0x40]);
    }

    #[test]
    fn push_i32_const_expr_appends_end_opcode() {
        let mut buf = Vec::new();
        push_i32_const_expr(&mut buf, 42);
        // 0x41 = i32.const, 0x2A = 42, 0x0B = end
        assert_eq!(buf[0], 0x41);
        assert_eq!(buf.last(), Some(&0x0B));
    }

    #[test]
    fn push_br_uses_leb128_encoding() {
        let mut buf = Vec::new();
        push_br(&mut buf, 512);
        // 0x0C = br instruction, remaining bytes encode 512 -> 0x80 0x04
        assert_eq!(buf, vec![0x0C, 0x80, 0x04]);
    }

    #[test]
    fn push_memory_writes_alignment_and_offset() {
        let mut buf = Vec::new();
        push_memory(&mut buf, 0x28, 24);
        assert_eq!(buf, vec![0x28, 0x00, 0x18]);
    }

    #[test]
    fn push_string_prefixes_length() {
        let mut buf = Vec::new();
        push_string(&mut buf, "wasm").expect("string encoding succeeds");
        // 4-byte length prefix followed by ascii bytes.
        assert_eq!(buf[0], 4);
        assert_eq!(&buf[1..], b"wasm");
    }

    #[test]
    fn write_u32_encodes_multi_byte_values() {
        let mut buf = Vec::new();
        write_u32(&mut buf, 0x3FFF); // requires two bytes
        assert_eq!(buf, vec![0xFF, 0x7F]);
    }

    #[test]
    fn push_call_and_locals_share_encoding_helpers() {
        let mut buf = Vec::new();
        push_call(&mut buf, 3);
        push_local(&mut buf, 0x20, 7);
        push_global(&mut buf, 0x23, 1);
        assert_eq!(buf, vec![0x10, 0x03, 0x20, 0x07, 0x23, 0x01]);
    }

    #[test]
    fn push_call_indirect_encodes_type_and_table() {
        let mut buf = Vec::new();
        push_call_indirect(&mut buf, 3, 0);
        assert_eq!(buf, vec![0x11, 0x03, 0x00]);
    }

    #[test]
    fn push_i64_const_handles_negative_values() {
        let mut buf = Vec::new();
        push_i64_const(&mut buf, -123456789);
        assert_eq!(buf[0], 0x42);
        assert!(buf.len() > 1);
    }

    #[test]
    fn push_i32_const_handles_negative_values() {
        let mut buf = Vec::new();
        push_i32_const(&mut buf, -1);
        // 0x41 opcode followed by signed LEB128 representation of -1 (0x7F).
        assert_eq!(buf, vec![0x41, 0x7F]);
    }

    #[test]
    fn push_f32_const_encodes_value() {
        let mut buf = Vec::new();
        push_f32_const(&mut buf, 1.5);
        assert_eq!(buf[0], 0x43);
        assert_eq!(&buf[1..], &1.5f32.to_le_bytes());
    }

    #[test]
    fn push_f64_const_encodes_value() {
        let mut buf = Vec::new();
        push_f64_const(&mut buf, -2.25);
        assert_eq!(buf[0], 0x44);
        assert_eq!(&buf[1..], &(-2.25f64).to_le_bytes());
    }

    #[test]
    fn push_byte_appends_single_opcode() {
        let mut buf = vec![0xAA];
        push_byte(&mut buf, 0x55);
        assert_eq!(buf, vec![0xAA, 0x55]);
    }
}
