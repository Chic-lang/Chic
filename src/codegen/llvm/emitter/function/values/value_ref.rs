use crate::codegen::llvm::types::is_float_ty;
use crate::mir::{FloatValue, FloatWidth};

#[derive(Debug)]
pub(crate) struct ValueRef {
    repr: String,
    ty: String,
}

impl ValueRef {
    pub(crate) fn new(name: String, ty: &str) -> Self {
        Self {
            repr: name,
            ty: ty.to_string(),
        }
    }

    pub(crate) fn new_literal(repr: String, ty: &str) -> Self {
        Self {
            repr: normalise_literal_for_ty(repr, ty),
            ty: ty.to_string(),
        }
    }

    pub(crate) fn repr(&self) -> &str {
        &self.repr
    }

    pub(crate) fn ty(&self) -> &str {
        &self.ty
    }
}

fn normalise_literal_for_ty(repr: String, ty: &str) -> String {
    if !is_float_ty(ty) {
        let trimmed_ty = ty.trim_start();
        let trimmed = repr.trim();
        if (trimmed_ty == "ptr" || trimmed_ty.starts_with("ptr ") || trimmed_ty.ends_with('*'))
            && (trimmed == "0" || trimmed == "0.0")
        {
            return "null".to_string();
        }
        if trimmed_ty.starts_with('i') {
            if trimmed.contains('.') || trimmed.contains('e') || trimmed.contains('E') {
                if let Ok(int_val) = trimmed.parse::<i128>() {
                    return int_val.to_string();
                }
                if let Ok(float_val) = trimmed.parse::<f64>() {
                    return (float_val as i128).to_string();
                }
                return "0".to_string();
            }
        }
        return repr;
    }
    let trimmed = repr.trim();
    if trimmed.starts_with("0x") || trimmed.contains('e') || trimmed.contains('E') {
        return repr;
    }
    if trimmed.eq_ignore_ascii_case("nan")
        || trimmed.eq_ignore_ascii_case("inf")
        || trimmed.eq_ignore_ascii_case("-inf")
        || trimmed.eq_ignore_ascii_case("zeroinitializer")
    {
        return repr;
    }
    if ty.trim() == "fp128" {
        if let Ok(value) = trimmed.parse::<f64>() {
            let bits = FloatValue::from_f64_as(value, FloatWidth::F128).bits;
            return format!("0xL{bits:032X}");
        }
        if let Ok(value) = trimmed.parse::<i128>() {
            let bits = FloatValue::from_f64_as(value as f64, FloatWidth::F128).bits;
            return format!("0xL{bits:032X}");
        }
    } else if let Ok(value) = trimmed.parse::<i128>() {
        return format!("{value}.0");
    } else if let Ok(value) = trimmed.parse::<f64>() {
        return value.to_string();
    }
    repr
}
