use super::utils;
use super::*;

parser_impl! {
    pub(in crate::frontend::parser) fn register_attr_from_attribute(
        &mut self,
        attribute: &Attribute,
    ) -> Option<MmioFieldAttr> {
        let mut offset: Option<u64> = None;
        let mut width_bits: Option<u16> = None;
        let mut access = MmioAccess::ReadWrite;

        for argument in &attribute.arguments {
            let name = argument
                .name
                .as_deref()
                .unwrap_or(&attribute.name)
                .to_ascii_lowercase();
            let raw_value = argument.value.trim();
            match name.as_str() {
                "offset" => match utils::parse_u64_literal(raw_value) {
                    Some(value) => offset = Some(value),
                    None => self.push_error(
                        "`offset` argument for `@register` must be an integer literal",
                        argument.span,
                    ),
                },
                "width" => match utils::parse_u64_literal(raw_value) {
                    Some(value @ (8 | 16 | 32 | 64)) => width_bits = Some(value as u16),
                    Some(_) => self.push_error(
                        "`width` argument must be one of 8, 16, 32, or 64",
                        argument.span,
                    ),
                    None => self.push_error(
                        "`width` argument for `@register` must be an integer literal",
                        argument.span,
                    ),
                },
                "access" => {
                    let text = utils::unquote(raw_value).to_ascii_lowercase();
                    match text.as_str() {
                        "rw" | "read_write" | "read-write" => access = MmioAccess::ReadWrite,
                        "ro" | "read_only" | "read-only" => access = MmioAccess::ReadOnly,
                        "wo" | "write_only" | "write-only" => access = MmioAccess::WriteOnly,
                        "true" => access = MmioAccess::ReadWrite,
                        "false" => access = MmioAccess::ReadOnly,
                        other => self.push_error(
                            format!(
                                "unsupported access mode `{other}`; expected rw, ro, or wo"
                            ),
                            argument.span,
                        ),
                    }
                }
                other => self.push_error(
                    format!("unknown argument `{other}` in `@register` attribute"),
                    argument.span,
                ),
            }
        }

        let Some(offset_value) = offset else {
            self.push_error(
                "`@register` attribute requires an `offset` argument",
                attribute.span,
            );
            return None;
        };

        let Ok(offset_u32) = u32::try_from(offset_value) else {
            self.push_error(
                "`offset` for `@register` must fit within 32 bits",
                attribute.span,
            );
            return None;
        };

        let width_bits = width_bits.unwrap_or(32);

        Some(MmioFieldAttr {
            offset: offset_u32,
            width_bits,
            access,
        })
    }

    pub(in crate::frontend::parser) fn parse_mmio_struct_attribute(
        &mut self,
        attr_span: Option<Span>,
    ) -> Option<MmioStructAttr> {
        let args = match self.parse_attribute_kv_arguments("mmio") {
            Some(args) => args,
            None => return None,
        };
        let mut base: Option<u64> = None;
        let mut size: Option<u64> = None;
        let mut address_space: Option<String> = None;
        let mut endianness = MmioEndianness::Little;
        let mut requires_unsafe = true;

        for arg in args {
            match arg.name.as_str() {
                "base" => match arg.value {
                    ParsedAttributeValue::Int(value) => base = Some(value),
                    _ => self.push_error(
                        "`base` argument for `@mmio` must be an integer literal",
                        Some(arg.span),
                    ),
                },
                "size" => match arg.value {
                    ParsedAttributeValue::Int(value) => size = Some(value),
                    _ => self.push_error(
                        "`size` argument for `@mmio` must be an integer literal",
                        Some(arg.span),
                    ),
                },
                "address_space" | "space" => match arg.value {
                    ParsedAttributeValue::Str(text) => address_space = Some(text),
                    ParsedAttributeValue::Bool(_) => self.push_error(
                        "`address_space` argument for `@mmio` expects a string value",
                        Some(arg.span),
                    ),
                    ParsedAttributeValue::Int(_) => self.push_error(
                        "`address_space` argument for `@mmio` expects a string value",
                        Some(arg.span),
                    ),
                },
                "endian" | "endianness" => match arg.value {
                    ParsedAttributeValue::Str(text) => {
                        if text.eq_ignore_ascii_case("little")
                            || text.eq_ignore_ascii_case("le")
                            || text.eq_ignore_ascii_case("little_endian")
                        {
                            endianness = MmioEndianness::Little;
                        } else if text.eq_ignore_ascii_case("big")
                            || text.eq_ignore_ascii_case("be")
                            || text.eq_ignore_ascii_case("big_endian")
                        {
                            endianness = MmioEndianness::Big;
                        } else {
                            self.push_error(
                                "endianness must be `little` or `big`",
                                Some(arg.span),
                            );
                        }
                    }
                    ParsedAttributeValue::Int(_) | ParsedAttributeValue::Bool(_) => {
                        self.push_error(
                            "`endianness` argument for `@mmio` expects a string value",
                            Some(arg.span),
                        );
                    }
                },
                "unsafe" | "requires_unsafe" => match arg.value {
                    ParsedAttributeValue::Bool(value) => requires_unsafe = value,
                    ParsedAttributeValue::Str(text) => {
                        if text.eq_ignore_ascii_case("true") {
                            requires_unsafe = true;
                        } else if text.eq_ignore_ascii_case("false") {
                            requires_unsafe = false;
                        } else {
                            self.push_error(
                                "`unsafe` argument for `@mmio` expects a boolean value",
                                Some(arg.span),
                            );
                        }
                    }
                    ParsedAttributeValue::Int(_) => self.push_error(
                        "`unsafe` argument for `@mmio` expects a boolean value",
                        Some(arg.span),
                    ),
                },
                other => {
                    self.push_error(
                        format!("unknown argument `{other}` in `@mmio` attribute"),
                        Some(arg.span),
                    );
                }
            }
        }

        let base_address = match base {
            Some(value) => value,
            None => {
                self.push_error("`@mmio` attribute requires a `base` argument", attr_span);
                return None;
            }
        };

        Some(MmioStructAttr {
            base_address,
            size,
            address_space,
            endianness,
            requires_unsafe,
        })
    }
}
