use chic::runtime::{
    ChicCharSpan, ChicStr, ChicString, chic_rt_string_as_chars, chic_rt_string_drop,
    chic_rt_string_from_slice,
};

unsafe fn span_to_vec(span: ChicCharSpan) -> Vec<u16> {
    if span.ptr.is_null() || span.len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(span.ptr, span.len).to_vec() }
    }
}

fn drop_string(mut value: ChicString) {
    unsafe {
        chic_rt_string_drop(&mut value);
    }
}

#[test]
fn utf8_strings_expose_scalar_chars() {
    let slice = ChicStr {
        ptr: b"abc".as_ptr(),
        len: 3,
    };
    let owned = unsafe { chic_rt_string_from_slice(slice) };
    let decoded = unsafe { span_to_vec(chic_rt_string_as_chars(&owned)) };
    assert_eq!(decoded, vec![b'a' as u16, b'b' as u16, b'c' as u16]);
    drop_string(owned);
}

#[test]
fn multi_byte_scalars_round_trip_in_char_spans() {
    let source = "Hi😊";
    let bytes = source.as_bytes();
    let slice = ChicStr {
        ptr: bytes.as_ptr(),
        len: bytes.len(),
    };
    let owned = unsafe { chic_rt_string_from_slice(slice) };
    let decoded = unsafe { span_to_vec(chic_rt_string_as_chars(&owned)) };
    let expected: Vec<u16> = source.encode_utf16().collect();
    assert_eq!(decoded, expected);
    drop_string(owned);
}
