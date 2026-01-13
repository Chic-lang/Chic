use crate::primitives::PrimitiveRegistry;

/// Simplified C type descriptor used for header generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CType {
    base: String,
    pointer_level: usize,
    is_const: bool,
}

impl CType {
    pub(crate) fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            pointer_level: 0,
            is_const: false,
        }
    }

    pub(crate) fn add_pointer(mut self) -> Self {
        self.pointer_level += 1;
        self
    }

    pub(crate) fn with_const(mut self) -> Self {
        self.is_const = true;
        self
    }

    pub(crate) fn render_return(&self) -> String {
        let const_prefix = if self.is_const { "const " } else { "" };
        if self.pointer_level == 0 {
            format!("{const_prefix}{}", self.base)
        } else {
            let ptrs = "*".repeat(self.pointer_level);
            format!("{const_prefix}{} {}", self.base, ptrs)
        }
    }

    pub(crate) fn render_declarator(&self, name: &str) -> String {
        let const_prefix = if self.is_const { "const " } else { "" };
        if self.pointer_level == 0 {
            format!("{const_prefix}{} {}", self.base, name)
        } else {
            let ptrs = "*".repeat(self.pointer_level);
            format!("{const_prefix}{} {}{}", self.base, ptrs, name)
        }
    }
}

/// Map a Chic type name into a C type descriptor.
pub(crate) fn map_type(registry: &PrimitiveRegistry, name: &str) -> CType {
    let trimmed = name.trim();
    let is_array = trimmed.ends_with("[]");
    let base_name = trimmed.strip_suffix("[]").unwrap_or(trimmed);

    let mut ty = if let Some(desc) = registry.descriptor_for_name(base_name) {
        let base = desc.c_type.as_deref().unwrap_or("void");
        CType::new(base)
    } else {
        CType::new("void").add_pointer()
    };

    if is_array {
        ty = CType::new("void").add_pointer();
    }

    ty
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_primitive_types() {
        let registry = PrimitiveRegistry::with_builtins();
        assert_eq!(map_type(&registry, "int").render_return(), "int32_t");
        assert_eq!(map_type(&registry, "Std.Int64").render_return(), "int64_t");
        assert_eq!(map_type(&registry, "double").render_return(), "double");
        assert_eq!(map_type(&registry, "bool").render_return(), "bool");
        assert_eq!(map_type(&registry, "char").render_return(), "uint16_t");
    }

    #[test]
    fn maps_string_to_runtime_struct() {
        let registry = PrimitiveRegistry::with_builtins();
        let ty = map_type(&registry, "string");
        assert_eq!(ty.render_return(), "struct chic_string");
        assert_eq!(ty.render_declarator("value"), "struct chic_string value");
    }

    #[test]
    fn maps_str_to_runtime_view() {
        let registry = PrimitiveRegistry::with_builtins();
        let ty = map_type(&registry, "str");
        assert_eq!(ty.render_return(), "struct chic_str");
        assert_eq!(ty.render_declarator("view"), "struct chic_str view");
    }

    #[test]
    fn defaults_to_void_pointer_for_unknowns() {
        let registry = PrimitiveRegistry::with_builtins();
        let ty = map_type(&registry, "My.Struct");
        assert_eq!(ty.render_return(), "void *");
    }
}
