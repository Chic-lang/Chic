use std::collections::{HashMap, HashSet};

use crate::frontend::ast::{Attribute, AttributeArgument};
use crate::frontend::diagnostics::Span;
use crate::mir::Ty;

pub type PrimitiveId = usize;

/// Semantic category for a primitive type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrimitiveKind {
    Int {
        bits: u16,
        signed: bool,
        pointer_sized: bool,
    },
    Float {
        bits: u16,
    },
    Char {
        bits: u16,
    },
    Decimal,
    Bool,
    String,
    Str,
    Void,
}

/// Descriptor recorded in the primitive registry.
#[derive(Clone, Debug)]
pub struct PrimitiveDescriptor {
    pub primitive_name: String,
    pub aliases: Vec<String>,
    pub kind: PrimitiveKind,
    pub c_type: Option<String>,
    pub std_wrapper_type: Option<String>,
    pub span: Option<Span>,
}

/// Registration or lookup failure.
#[derive(Debug, Clone)]
pub struct PrimitiveRegistrationError {
    pub message: String,
    pub span: Option<Span>,
    pub conflicting_span: Option<Span>,
}

impl PrimitiveRegistrationError {
    #[must_use]
    pub fn new(
        message: impl Into<String>,
        span: Option<Span>,
        conflicting_span: Option<Span>,
    ) -> Self {
        Self {
            message: message.into(),
            span,
            conflicting_span,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct PrimitiveRegistry {
    descriptors: Vec<PrimitiveDescriptor>,
    lookup: HashMap<String, PrimitiveId>,
}

impl PrimitiveRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            descriptors: Vec::new(),
            lookup: HashMap::new(),
        }
    }

    #[must_use]
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        for builtin in builtin_descriptors() {
            let _ = registry.register(builtin);
        }
        registry
    }

    #[must_use]
    pub fn descriptors(&self) -> &[PrimitiveDescriptor] {
        &self.descriptors
    }

    pub fn register(
        &mut self,
        mut desc: PrimitiveDescriptor,
    ) -> Result<PrimitiveId, PrimitiveRegistrationError> {
        desc.primitive_name =
            normalize_name(&desc.primitive_name).ok_or_else(|| PrimitiveRegistrationError {
                message: "primitive name must not be empty".to_string(),
                span: desc.span,
                conflicting_span: None,
            })?;

        validate_descriptor(&desc)?;

        let mut all_aliases = HashSet::new();
        all_aliases.insert(desc.primitive_name.clone());
        for alias in &desc.aliases {
            if let Some(normalized) = normalize_name(alias) {
                all_aliases.insert(normalized);
            }
        }
        if let Some(wrapper) = desc.std_wrapper_type.as_ref() {
            if let Some(normalized) = normalize_name(wrapper) {
                all_aliases.insert(normalized);
            }
        }

        if let Some(existing_id) = self.lookup.get(&desc.primitive_name).copied() {
            let existing = self.descriptors.get_mut(existing_id).ok_or_else(|| {
                PrimitiveRegistrationError::new(
                    "internal error: primitive registry is inconsistent",
                    desc.span,
                    None,
                )
            })?;
            if existing.kind != desc.kind {
                return Err(PrimitiveRegistrationError::new(
                    format!("conflicting primitive kind for `{}`", desc.primitive_name),
                    desc.span,
                    existing.span,
                ));
            }
            if let Some(wrapper) = desc.std_wrapper_type.take() {
                if existing.std_wrapper_type.is_none() {
                    existing.std_wrapper_type = Some(wrapper);
                }
            }
            if let Some(c_type) = desc.c_type.take() {
                match &existing.c_type {
                    None => existing.c_type = Some(c_type),
                    Some(existing_type) if existing_type == &c_type => {}
                    Some(existing_type) => {
                        return Err(PrimitiveRegistrationError::new(
                            format!(
                                "conflicting C type mapping for primitive `{}`: `{}` vs `{c_type}`",
                                desc.primitive_name, existing_type
                            ),
                            desc.span,
                            existing.span,
                        ));
                    }
                }
            }
            for alias in &all_aliases {
                if let Some(other_id) = self.lookup.get(alias) {
                    if *other_id != existing_id {
                        let conflict_span = self
                            .descriptors
                            .get(*other_id)
                            .and_then(|existing_desc| existing_desc.span);
                        return Err(PrimitiveRegistrationError::new(
                            format!("duplicate primitive alias `{alias}`"),
                            desc.span,
                            conflict_span,
                        ));
                    }
                    continue;
                }
                self.lookup.insert(alias.clone(), existing_id);
                if !existing.aliases.contains(alias) {
                    existing.aliases.push(alias.clone());
                }
            }
            return Ok(existing_id);
        }

        for alias in &all_aliases {
            if let Some(existing) = self.lookup.get(alias) {
                let conflict_span = self
                    .descriptors
                    .get(*existing)
                    .and_then(|existing_desc| existing_desc.span);
                return Err(PrimitiveRegistrationError::new(
                    format!("duplicate primitive alias `{alias}`"),
                    desc.span,
                    conflict_span,
                ));
            }
        }

        let id = self.descriptors.len();
        desc.aliases = all_aliases.iter().cloned().collect();
        self.descriptors.push(desc);
        for alias in all_aliases {
            self.lookup.insert(alias, id);
        }
        Ok(id)
    }

    #[must_use]
    pub fn lookup(&self, ty: &Ty) -> Option<PrimitiveId> {
        match ty {
            Ty::Nullable(inner) => self.lookup(inner),
            Ty::Ref(inner) => self.lookup(&inner.element),
            Ty::String => self.lookup_by_name("string"),
            Ty::Str => self.lookup_by_name("str"),
            Ty::Named(named) => self.lookup_by_name(named.as_str()),
            _ => None,
        }
    }

    #[must_use]
    pub fn lookup_by_name(&self, name: &str) -> Option<PrimitiveId> {
        let normalized = normalize_name(name)?;
        self.lookup.get(&normalized).copied()
    }

    #[must_use]
    pub fn descriptor_for_name(&self, name: &str) -> Option<&PrimitiveDescriptor> {
        let id = self.lookup_by_name(name)?;
        self.descriptor(id)
    }

    #[must_use]
    pub fn is_primitive(&self, ty: &Ty) -> bool {
        self.lookup(ty).is_some()
    }

    #[must_use]
    pub fn descriptor(&self, id: PrimitiveId) -> Option<&PrimitiveDescriptor> {
        self.descriptors.get(id)
    }

    #[must_use]
    pub fn kind_for(&self, ty: &Ty) -> Option<&PrimitiveKind> {
        let id = self.lookup(ty)?;
        self.descriptors.get(id).map(|desc| &desc.kind)
    }

    #[must_use]
    pub fn size_align(&self, ty: &Ty, pointer_size: u32, pointer_align: u32) -> Option<(u32, u32)> {
        let id = self.lookup(ty)?;
        let desc = self.descriptors.get(id)?;
        size_align_for_kind(&desc.kind, pointer_size, pointer_align)
    }

    #[must_use]
    pub fn size_align_for_name(
        &self,
        name: &str,
        pointer_size: u32,
        pointer_align: u32,
    ) -> Option<(u32, u32)> {
        let id = self.lookup_by_name(name)?;
        let desc = self.descriptors.get(id)?;
        size_align_for_kind(&desc.kind, pointer_size, pointer_align)
    }

    #[must_use]
    pub fn kind_for_name(&self, name: &str) -> Option<&PrimitiveKind> {
        let id = self.lookup_by_name(name)?;
        self.descriptors.get(id).map(|desc| &desc.kind)
    }
}

/// Error produced when parsing a `@primitive` attribute.
#[derive(Debug, Clone)]
pub struct PrimitiveAttributeError {
    pub message: String,
    pub span: Option<Span>,
}

/// Parse a `@primitive(...)` attribute attached to a type.
///
/// The returned descriptor does not include the declaring type as an alias; callers
/// should add the qualified type name as an alias before registering to the
/// registry so wrapper types resolve correctly.
#[must_use]
pub fn parse_primitive_attribute(
    attr: &Attribute,
    qualified_name: &str,
) -> (Option<PrimitiveDescriptor>, Vec<PrimitiveAttributeError>) {
    let mut diagnostics = Vec::new();
    let primitive_name = match find_arg(attr, "primitive")
        .and_then(|arg| parse_string(arg, "primitive", &mut diagnostics))
    {
        Some(name) => name,
        None => {
            diagnostics.push(PrimitiveAttributeError {
                message: format!(
                    "`@primitive` on `{qualified_name}` requires `primitive = \"<name>\"`"
                ),
                span: attr.span,
            });
            return (None, diagnostics);
        }
    };
    let kind_name = match find_arg(attr, "kind")
        .and_then(|arg| parse_string(arg, "kind", &mut diagnostics))
    {
        Some(kind) => kind,
        None => {
            diagnostics.push(PrimitiveAttributeError {
                message: format!(
                    "`@primitive` on `{qualified_name}` requires `kind = \"<int|float|decimal|char|bool|string|str|void>\"`"
                ),
                span: attr.span,
            });
            return (None, diagnostics);
        }
    };

    let bits = find_arg(attr, "bits").and_then(|arg| parse_bits(arg, &mut diagnostics));
    let signed =
        find_arg(attr, "signed").and_then(|arg| parse_bool(arg, "signed", &mut diagnostics));
    let pointer_sized = find_arg(attr, "pointer_sized")
        .or_else(|| find_arg(attr, "pointerSized"))
        .and_then(|arg| parse_bool(arg, "pointer_sized", &mut diagnostics))
        .unwrap_or(false);
    let aliases = find_arg(attr, "aliases")
        .map(parse_aliases)
        .unwrap_or_default();
    let c_type = find_arg(attr, "c_type")
        .or_else(|| find_arg(attr, "ctype"))
        .or_else(|| find_arg(attr, "cType"))
        .and_then(|arg| parse_string(arg, "c_type", &mut diagnostics));

    let kind = match kind_name.to_ascii_lowercase().as_str() {
        "int" => PrimitiveKind::Int {
            bits: bits.unwrap_or(0),
            signed: signed.unwrap_or(true),
            pointer_sized,
        },
        "float" => {
            if pointer_sized {
                diagnostics.push(PrimitiveAttributeError {
                    message: "`pointer_sized` is not valid for float primitives".to_string(),
                    span: attr.span,
                });
            }
            let Some(width) = bits.or(Some(32)) else {
                diagnostics.push(PrimitiveAttributeError {
                    message: "`@primitive(kind=\"float\")` requires `bits`".to_string(),
                    span: attr.span,
                });
                return (None, diagnostics);
            };
            PrimitiveKind::Float { bits: width }
        }
        "char" => {
            if pointer_sized {
                diagnostics.push(PrimitiveAttributeError {
                    message: "`pointer_sized` is not valid for char primitives".to_string(),
                    span: attr.span,
                });
            }
            PrimitiveKind::Char {
                bits: bits.unwrap_or(16),
            }
        }
        "decimal" => {
            if bits.is_some() || signed.is_some() || pointer_sized {
                diagnostics.push(PrimitiveAttributeError {
                    message: "`decimal` primitives ignore `bits`, `signed`, and `pointer_sized`"
                        .to_string(),
                    span: attr.span,
                });
            }
            PrimitiveKind::Decimal
        }
        "bool" => {
            if bits.is_some() || signed.is_some() || pointer_sized {
                diagnostics.push(PrimitiveAttributeError {
                    message: "`bool` primitives ignore `bits`, `signed`, and `pointer_sized`"
                        .to_string(),
                    span: attr.span,
                });
            }
            PrimitiveKind::Bool
        }
        "string" => {
            if bits.is_some() || signed.is_some() || pointer_sized {
                diagnostics.push(PrimitiveAttributeError {
                    message: "`string` primitives ignore `bits`, `signed`, and `pointer_sized`"
                        .to_string(),
                    span: attr.span,
                });
            }
            PrimitiveKind::String
        }
        "str" => {
            if bits.is_some() || signed.is_some() || pointer_sized {
                diagnostics.push(PrimitiveAttributeError {
                    message: "`str` primitives ignore `bits`, `signed`, and `pointer_sized`"
                        .to_string(),
                    span: attr.span,
                });
            }
            PrimitiveKind::Str
        }
        "void" => {
            if bits.is_some() || signed.is_some() || pointer_sized {
                diagnostics.push(PrimitiveAttributeError {
                    message: "`void` primitives ignore `bits`, `signed`, and `pointer_sized`"
                        .to_string(),
                    span: attr.span,
                });
            }
            PrimitiveKind::Void
        }
        other => {
            diagnostics.push(PrimitiveAttributeError {
                message: format!("unsupported primitive kind `{other}`"),
                span: attr.span,
            });
            return (None, diagnostics);
        }
    };

    let descriptor = PrimitiveDescriptor {
        primitive_name,
        aliases,
        kind,
        c_type,
        std_wrapper_type: Some(qualified_name.to_string()),
        span: attr.span,
    };

    (Some(descriptor), diagnostics)
}

fn find_arg<'a>(attr: &'a Attribute, name: &str) -> Option<&'a AttributeArgument> {
    attr.arguments.iter().find(|arg| {
        arg.name
            .as_deref()
            .is_some_and(|n| n.eq_ignore_ascii_case(name))
    })
}

fn parse_string(
    arg: &AttributeArgument,
    key: &str,
    diagnostics: &mut Vec<PrimitiveAttributeError>,
) -> Option<String> {
    let trimmed = arg.value.trim().trim_matches(|ch| ch == '"' || ch == '\'');
    if trimmed.is_empty() {
        diagnostics.push(PrimitiveAttributeError {
            message: format!("`{key}` cannot be empty in `@primitive`"),
            span: arg.span,
        });
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_bool(
    arg: &AttributeArgument,
    key: &str,
    diagnostics: &mut Vec<PrimitiveAttributeError>,
) -> Option<bool> {
    match arg.value.trim().to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        other => {
            diagnostics.push(PrimitiveAttributeError {
                message: format!("`{key}` must be `true` or `false`, found `{other}`"),
                span: arg.span,
            });
            None
        }
    }
}

fn parse_bits(
    arg: &AttributeArgument,
    diagnostics: &mut Vec<PrimitiveAttributeError>,
) -> Option<u16> {
    let text = arg.value.trim();
    match text.parse::<u16>() {
        Ok(bits) => Some(bits),
        Err(_) => {
            diagnostics.push(PrimitiveAttributeError {
                message: format!("`bits` must be an integer, found `{text}`"),
                span: arg.span,
            });
            None
        }
    }
}

fn parse_aliases(arg: &AttributeArgument) -> Vec<String> {
    let mut text = arg.value.trim();
    if let Some(stripped) = text.strip_prefix('[') {
        if let Some(end) = stripped.rfind(']') {
            text = &stripped[..end];
        } else {
            text = stripped;
        }
    }
    text.split(',')
        .filter_map(|alias| {
            let trimmed = alias.trim().trim_matches(|ch| ch == '"' || ch == '\'');
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

fn validate_descriptor(desc: &PrimitiveDescriptor) -> Result<(), PrimitiveRegistrationError> {
    match desc.kind {
        PrimitiveKind::Int {
            bits,
            pointer_sized,
            ..
        } => {
            if bits == 0 && !pointer_sized {
                return Err(PrimitiveRegistrationError::new(
                    "integer primitive must specify a width",
                    desc.span,
                    None,
                ));
            }
            if bits % 8 != 0 && !pointer_sized {
                return Err(PrimitiveRegistrationError::new(
                    "integer primitive width must be a multiple of 8 bits",
                    desc.span,
                    None,
                ));
            }
        }
        PrimitiveKind::Float { bits } => {
            if bits == 0 {
                return Err(PrimitiveRegistrationError::new(
                    "floating-point primitive must specify a width",
                    desc.span,
                    None,
                ));
            }
            if bits % 8 != 0 {
                return Err(PrimitiveRegistrationError::new(
                    "floating-point primitive width must be a multiple of 8 bits",
                    desc.span,
                    None,
                ));
            }
        }
        PrimitiveKind::Char { bits } => {
            if bits != 16 {
                return Err(PrimitiveRegistrationError::new(
                    "char primitive width must be 16 bits",
                    desc.span,
                    None,
                ));
            }
        }
        PrimitiveKind::Decimal
        | PrimitiveKind::Bool
        | PrimitiveKind::String
        | PrimitiveKind::Str
        | PrimitiveKind::Void => {}
    }

    if let Some(c_type) = &desc.c_type {
        if !is_valid_c_type(c_type) {
            return Err(PrimitiveRegistrationError::new(
                format!("invalid C type name `{c_type}` for primitive"),
                desc.span,
                None,
            ));
        }
    }

    Ok(())
}

fn is_valid_c_type(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == ' ' || ch == ':')
}

fn size_align_for_kind(
    kind: &PrimitiveKind,
    pointer_size: u32,
    pointer_align: u32,
) -> Option<(u32, u32)> {
    match kind {
        PrimitiveKind::Bool => Some((1, 1)),
        PrimitiveKind::Int {
            bits,
            signed: _,
            pointer_sized,
        } => {
            let size_bits = if *pointer_sized {
                pointer_size.saturating_mul(8)
            } else {
                u32::from(*bits)
            };
            let size = (size_bits / 8).max(1);
            let align = if *pointer_sized { pointer_align } else { size };
            Some((size, align.max(1)))
        }
        PrimitiveKind::Float { bits } => {
            let size = (u32::from(*bits) / 8).max(2);
            Some((size, size.max(2)))
        }
        PrimitiveKind::Char { bits } => {
            let size = (u32::from(*bits) / 8).max(1);
            Some((size, size.max(1)))
        }
        PrimitiveKind::Decimal => Some((16, 16)),
        // `string` is ABI-compatible with the native `ChicString` handle:
        // `{ ptr, usize, usize, [32 x byte] }`.
        PrimitiveKind::String => Some((
            pointer_size.saturating_mul(3).saturating_add(32),
            pointer_align,
        )),
        PrimitiveKind::Str => Some((pointer_size.saturating_mul(2), pointer_align)),
        PrimitiveKind::Void => Some((0, 1)),
    }
}

/// Normalise a primitive name or alias.
#[must_use]
pub fn normalize_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains('*') {
        return None;
    }
    let mut base = trimmed;
    if let Some(idx) = base.find('<') {
        base = &base[..idx];
    }
    if let Some(idx) = base.find('[') {
        base = &base[..idx];
    }
    let base = base.trim_end_matches('?');
    if base.is_empty() {
        return None;
    }
    let lowered = base.replace("::", ".").to_ascii_lowercase();
    let mut segments: Vec<&str> = lowered
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect();

    while let Some(first) = segments.first().copied() {
        if matches!(first, "system" | "std") {
            segments.remove(0);
            continue;
        }
        if segments.len() > 1 && matches!(first, "numeric" | "runtime") {
            segments.remove(0);
            continue;
        }
        break;
    }

    segments.last().map(|segment| segment.to_string())
}

fn builtin_descriptors() -> Vec<PrimitiveDescriptor> {
    vec![
        PrimitiveDescriptor {
            primitive_name: "bool".into(),
            aliases: vec!["boolean".into()],
            kind: PrimitiveKind::Bool,
            c_type: Some("bool".into()),
            std_wrapper_type: Some("Std::Boolean".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "sbyte".into(),
            aliases: vec!["i8".into(), "int8".into()],
            kind: PrimitiveKind::Int {
                bits: 8,
                signed: true,
                pointer_sized: false,
            },
            c_type: Some("int8_t".into()),
            std_wrapper_type: Some("Std::SByte".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "byte".into(),
            aliases: vec!["u8".into(), "uint8".into()],
            kind: PrimitiveKind::Int {
                bits: 8,
                signed: false,
                pointer_sized: false,
            },
            c_type: Some("uint8_t".into()),
            std_wrapper_type: Some("Std::Byte".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "short".into(),
            aliases: vec!["i16".into(), "int16".into()],
            kind: PrimitiveKind::Int {
                bits: 16,
                signed: true,
                pointer_sized: false,
            },
            c_type: Some("int16_t".into()),
            std_wrapper_type: Some("Std::Int16".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "ushort".into(),
            aliases: vec!["u16".into(), "uint16".into()],
            kind: PrimitiveKind::Int {
                bits: 16,
                signed: false,
                pointer_sized: false,
            },
            c_type: Some("uint16_t".into()),
            std_wrapper_type: Some("Std::UInt16".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "char".into(),
            aliases: vec![
                "Char".into(),
                "Std.Char".into(),
                "Std.Numeric.Char".into(),
                "System.Char".into(),
            ],
            kind: PrimitiveKind::Char { bits: 16 },
            c_type: Some("uint16_t".into()),
            std_wrapper_type: Some("Std::Char".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "int".into(),
            aliases: vec!["i32".into(), "int32".into()],
            kind: PrimitiveKind::Int {
                bits: 32,
                signed: true,
                pointer_sized: false,
            },
            c_type: Some("int32_t".into()),
            std_wrapper_type: Some("Std::Int32".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "uint".into(),
            aliases: vec!["u32".into(), "uint32".into()],
            kind: PrimitiveKind::Int {
                bits: 32,
                signed: false,
                pointer_sized: false,
            },
            c_type: Some("uint32_t".into()),
            std_wrapper_type: Some("Std::UInt32".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "long".into(),
            aliases: vec!["i64".into(), "int64".into()],
            kind: PrimitiveKind::Int {
                bits: 64,
                signed: true,
                pointer_sized: false,
            },
            c_type: Some("int64_t".into()),
            std_wrapper_type: Some("Std::Int64".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "ulong".into(),
            aliases: vec!["u64".into(), "uint64".into()],
            kind: PrimitiveKind::Int {
                bits: 64,
                signed: false,
                pointer_sized: false,
            },
            c_type: Some("uint64_t".into()),
            std_wrapper_type: Some("Std::UInt64".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "int128".into(),
            aliases: vec!["i128".into()],
            kind: PrimitiveKind::Int {
                bits: 128,
                signed: true,
                pointer_sized: false,
            },
            c_type: Some("int128_t".into()),
            std_wrapper_type: Some("Std::Int128".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "uint128".into(),
            aliases: vec!["u128".into()],
            kind: PrimitiveKind::Int {
                bits: 128,
                signed: false,
                pointer_sized: false,
            },
            c_type: Some("uint128_t".into()),
            std_wrapper_type: Some("Std::UInt128".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "nint".into(),
            aliases: vec!["isize".into(), "intptr".into()],
            kind: PrimitiveKind::Int {
                bits: 64,
                signed: true,
                pointer_sized: true,
            },
            c_type: Some("intptr_t".into()),
            std_wrapper_type: Some("Std::IntPtr".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "nuint".into(),
            aliases: vec!["usize".into(), "uintptr".into()],
            kind: PrimitiveKind::Int {
                bits: 64,
                signed: false,
                pointer_sized: true,
            },
            c_type: Some("uintptr_t".into()),
            std_wrapper_type: Some("Std::UIntPtr".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "float16".into(),
            aliases: vec!["half".into(), "f16".into()],
            kind: PrimitiveKind::Float { bits: 16 },
            c_type: Some("_Float16".into()),
            std_wrapper_type: None,
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "float".into(),
            aliases: vec!["float32".into(), "f32".into(), "single".into()],
            kind: PrimitiveKind::Float { bits: 32 },
            c_type: Some("float".into()),
            std_wrapper_type: Some("Std::Float32".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "double".into(),
            aliases: vec!["float64".into(), "f64".into()],
            kind: PrimitiveKind::Float { bits: 64 },
            c_type: Some("double".into()),
            std_wrapper_type: Some("Std::Float64".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "float128".into(),
            aliases: vec!["quad".into(), "f128".into()],
            kind: PrimitiveKind::Float { bits: 128 },
            c_type: Some("__float128".into()),
            std_wrapper_type: Some("Std::Float128".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "decimal".into(),
            aliases: vec![],
            kind: PrimitiveKind::Decimal,
            c_type: Some("decimal128_t".into()),
            std_wrapper_type: Some("Std::Decimal".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "string".into(),
            aliases: vec!["std.string".into(), "system.string".into()],
            kind: PrimitiveKind::String,
            c_type: Some("struct chic_string".into()),
            std_wrapper_type: Some("Std::String".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "str".into(),
            aliases: vec!["std.str".into(), "system.str".into()],
            kind: PrimitiveKind::Str,
            c_type: Some("struct chic_str".into()),
            std_wrapper_type: Some("Std::Str".into()),
            span: None,
        },
        PrimitiveDescriptor {
            primitive_name: "void".into(),
            aliases: vec![],
            kind: PrimitiveKind::Void,
            c_type: Some("void".into()),
            std_wrapper_type: None,
            span: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_aliases() {
        let registry = PrimitiveRegistry::with_builtins();
        let names = [
            "int",
            "Int",
            "INT",
            "Std.Int32",
            "Std.Numeric.Int32",
            "System.Int32",
            "int32",
            "i32",
        ];
        let ids: Vec<_> = names
            .iter()
            .map(|name| registry.lookup_by_name(name).expect("alias registered"))
            .collect();
        assert!(ids.windows(2).all(|w| w[0] == w[1]));

        for name in ["string", "System.String", "Std.String"] {
            assert!(
                registry.lookup_by_name(name).is_some(),
                "expected string alias `{name}` to resolve"
            );
        }
    }

    #[test]
    fn rejects_duplicate_alias() {
        let mut registry = PrimitiveRegistry::with_builtins();
        let result = registry.register(PrimitiveDescriptor {
            primitive_name: "int".into(),
            aliases: vec!["uint".into()],
            kind: PrimitiveKind::Int {
                bits: 32,
                signed: true,
                pointer_sized: false,
            },
            c_type: None,
            std_wrapper_type: None,
            span: None,
        });
        assert!(result.is_err(), "conflicting alias should produce an error");
    }

    #[test]
    fn size_align_uses_pointer_width_for_string() {
        let registry = PrimitiveRegistry::with_builtins();
        let (size, align) = registry
            .size_align(&Ty::String, 8, 8)
            .expect("string registered");
        assert_eq!(size, 8 * 3 + 32);
        assert_eq!(align, 8);
        let (slice_size, slice_align) =
            registry.size_align(&Ty::Str, 8, 8).expect("str registered");
        assert_eq!(slice_size, 16);
        assert_eq!(slice_align, 8);
    }

    #[test]
    fn char_layout_and_c_type_are_registered() {
        let registry = PrimitiveRegistry::with_builtins();
        let (size, align) = registry
            .size_align_for_name("char", 8, 8)
            .expect("char registered");
        assert_eq!(size, 2);
        assert_eq!(align, 2);
        let desc = registry
            .descriptor_for_name("char")
            .expect("char descriptor available");
        assert_eq!(desc.c_type.as_deref(), Some("uint16_t"));
    }
}
