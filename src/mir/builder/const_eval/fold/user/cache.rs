use crate::mir::ConstEvalContext;
use crate::mir::builder::const_eval::ConstEvalResult;
use crate::mir::builder::const_eval::environment::ConstFnCacheKey;
use crate::mir::builder::symbol_index::FunctionDeclSymbol;
use crate::mir::data::ConstValue;
use std::char;

impl<'a> ConstEvalContext<'a> {
    pub(crate) fn const_fn_cache_key(
        &self,
        symbol: &FunctionDeclSymbol,
        args: &[(Option<String>, ConstEvalResult)],
    ) -> ConstFnCacheKey {
        let mut labels = Vec::with_capacity(args.len());
        for (_, value) in args {
            labels.push(self.const_value_label(&value.value));
        }
        ConstFnCacheKey::new(symbol.qualified.clone(), labels)
    }

    pub(crate) fn const_value_label(&self, value: &ConstValue) -> String {
        match value {
            ConstValue::Int(v) | ConstValue::Int32(v) => format!("i:{v}"),
            ConstValue::UInt(v) => format!("u:{v}"),
            ConstValue::Float(v) => format!("f:{:?}:{}", v.width, v.hex_bits()),
            ConstValue::Decimal(v) => format!("dec:{v:?}"),
            ConstValue::Bool(v) => format!("b:{v}"),
            ConstValue::Char(ch) => {
                let escaped = if let Some(scalar) = char::from_u32(u32::from(*ch)) {
                    scalar.escape_default().collect::<String>()
                } else {
                    format!("\\u{ch:04X}")
                };
                format!("char:'{escaped}'")
            }
            ConstValue::Str { value, .. } => format!("str:{value}"),
            ConstValue::RawStr(value) => format!("raw:{value}"),
            ConstValue::Symbol(sym) => format!("sym:{sym}"),
            ConstValue::Enum {
                type_name,
                variant,
                discriminant,
            } => format!("enum:{type_name}::{variant}#{discriminant}"),
            ConstValue::Struct { type_name, fields } => {
                let mut entries = fields
                    .iter()
                    .map(|(name, value)| (name.clone(), self.const_value_label(value)))
                    .collect::<Vec<_>>();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                let joined = entries
                    .into_iter()
                    .map(|(name, value)| format!("{name}={value}"))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("struct:{type_name}{{{joined}}}")
            }
            ConstValue::Null => "null".into(),
            ConstValue::Unit => "unit".into(),
            ConstValue::Unknown => "unknown".into(),
        }
    }
}
