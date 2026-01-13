use super::super::*;
use std::collections::HashSet;

struct ParsedInitializer {
    type_name: String,
    members: Vec<String>,
}

fn skip_whitespace(bytes: &[u8], idx: &mut usize) {
    while *idx < bytes.len() {
        if !bytes[*idx].is_ascii_whitespace() {
            break;
        }
        *idx += 1;
    }
}

fn parse_object_initializer(text: &str) -> Option<ParsedInitializer> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut idx = 0usize;

    skip_whitespace(bytes, &mut idx);
    if idx >= len {
        return None;
    }

    if bytes[idx..].starts_with(b"new") {
        let next = idx + 3;
        if next >= len
            || !bytes[next].is_ascii_alphanumeric() && bytes[next] != b'_' && bytes[next] != b':'
        {
            idx = next;
            skip_whitespace(bytes, &mut idx);
        }
    }

    let type_start = idx;
    let mut depth_angle = 0i32;
    while idx < len {
        let ch = bytes[idx];
        match ch {
            b'<' => {
                depth_angle += 1;
                idx += 1;
            }
            b'>' => {
                if depth_angle == 0 {
                    break;
                }
                depth_angle -= 1;
                idx += 1;
            }
            b'{' | b'(' if depth_angle == 0 => break,
            ch if ch.is_ascii_whitespace() && depth_angle == 0 => break,
            _ => idx += 1,
        }
    }
    if idx == type_start {
        return None;
    }
    let type_name = text[type_start..idx].trim().to_string();
    if type_name.is_empty() {
        return None;
    }

    skip_whitespace(bytes, &mut idx);

    if idx < len && bytes[idx] == b'(' {
        idx += 1;
        let mut depth = 1i32;
        while idx < len && depth > 0 {
            match bytes[idx] {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {}
            }
            idx += 1;
        }
        if depth != 0 {
            return None;
        }
        skip_whitespace(bytes, &mut idx);
    }

    if idx >= len || bytes[idx] != b'{' {
        return None;
    }
    idx += 1;

    let mut members = Vec::new();

    skip_whitespace(bytes, &mut idx);
    if idx >= len {
        return None;
    }

    while idx < len {
        if bytes[idx] == b'}' {
            idx += 1;
            break;
        }

        let name_start = idx;
        while idx < len {
            let ch = bytes[idx];
            if ch == b'=' {
                break;
            }
            if ch.is_ascii_whitespace() {
                let mut look = idx;
                while look < len && bytes[look].is_ascii_whitespace() {
                    look += 1;
                }
                if look < len && bytes[look] == b'=' {
                    idx = look;
                    break;
                }
            }
            idx += 1;
        }
        if idx >= len || bytes[idx] != b'=' {
            return None;
        }
        let member_name = text[name_start..idx].trim();
        if member_name.is_empty() {
            return None;
        }
        members.push(member_name.to_string());
        idx += 1;

        let mut brace = 0i32;
        let mut paren = 0i32;
        let mut bracket = 0i32;
        while idx < len {
            let ch = bytes[idx];
            match ch {
                b'{' => brace += 1,
                b'}' => {
                    if brace == 0 && paren == 0 && bracket == 0 {
                        break;
                    }
                    if brace > 0 {
                        brace -= 1;
                    } else {
                        return None;
                    }
                }
                b'(' => paren += 1,
                b')' => {
                    if paren == 0 {
                        return None;
                    }
                    paren -= 1;
                }
                b'[' => bracket += 1,
                b']' => {
                    if bracket == 0 {
                        return None;
                    }
                    bracket -= 1;
                }
                b',' if brace == 0 && paren == 0 && bracket == 0 => break,
                _ => {}
            }
            idx += 1;
        }

        if idx >= len {
            return None;
        }

        let delimiter = bytes[idx];
        if delimiter == b',' {
            idx += 1;
            continue;
        } else if delimiter == b'}' {
            idx += 1;
            break;
        } else {
            return None;
        }
    }

    skip_whitespace(bytes, &mut idx);
    if idx != len {
        return None;
    }

    Some(ParsedInitializer { type_name, members })
}

fn format_missing_required_message(ty: &str, missing: &[String]) -> String {
    if missing.len() == 1 {
        format!(
            "initializer for `{}` must assign required member `{}`",
            ty, missing[0]
        )
    } else {
        let (last, head) = missing.split_last().expect("non-empty slicing");
        let prefix = head
            .iter()
            .map(|name| format!("`{}`", name))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "initializer for `{}` must assign required members {} and `{}`",
            ty, prefix, last
        )
    }
}

body_builder_impl! {
    pub(crate) fn validate_required_initializer(&mut self, expr: &crate::frontend::ast::Expression) {
        let parsed = match parse_object_initializer(expr.text.trim()) {
            Some(parsed) => parsed,
            None => return,
        };

        let Some(resolved_type) = self.resolve_initializer_type(&parsed.type_name) else {
            return;
        };

        let required = self.required_members_for_type(&resolved_type);
        if required.is_empty() {
            return;
        }

        let mut missing: Vec<String> = required
            .into_iter()
            .filter(|member| !parsed.members.iter().any(|entry| entry == member))
            .collect();

        if missing.is_empty() {
            return;
        }
        missing.sort();
        let message = format_missing_required_message(&resolved_type, &missing);
        self.diagnostics.push(LoweringDiagnostic {
            message,
            span: expr.span,
        });
    }

    pub(crate) fn resolve_initializer_type(&self, raw: &str) -> Option<String> {
        let base = raw.split('<').next().unwrap_or(raw).trim();
        if base.is_empty() {
            return None;
        }
        if base == "Self" {
            if let Some(self_type) = self.current_self_type_name() {
                if self.type_layouts.types.contains_key(&self_type) {
                    return Some(self_type);
                }
            }
        }
        let canonical = base.replace('.', "::");
        let mut candidates = Vec::new();
        candidates.push(canonical.clone());
        if canonical != base {
            candidates.push(base.to_string());
        }
        if let Some(namespace) = &self.namespace {
            let mut current = namespace.as_str();
            loop {
                candidates.push(format!("{current}::{canonical}"));
                if let Some(idx) = current.rfind("::") {
                    current = &current[..idx];
                } else {
                    break;
                }
            }
        }
        for candidate in candidates {
            if self.type_layouts.types.contains_key(&candidate) {
                return Some(candidate);
            }
        }
        None
    }

    pub(crate) fn required_members_for_type(&self, type_name: &str) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut members = HashSet::new();
        self.collect_required_members_recursive(type_name, &mut visited, &mut members);
        members
    }

    pub(crate) fn collect_required_members_recursive(
        &self,
        type_name: &str,
        visited: &mut HashSet<String>,
        members: &mut HashSet<String>,
    ) {
        if !visited.insert(type_name.to_string()) {
            return;
        }

        let layout = match self.type_layouts.types.get(type_name) {
            Some(TypeLayout::Struct(layout)) | Some(TypeLayout::Class(layout)) => layout,
            _ => return,
        };

        for field in &layout.fields {
            if field.is_required {
                let name = field
                    .display_name
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| field.name.clone());
                members.insert(name);
            }
        }

        if let Some(info) = &layout.class {
            for base in &info.bases {
                let canonical = base.replace('.', "::");
                let candidate = if self.type_layouts.types.contains_key(base) {
                    base.clone()
                } else if self.type_layouts.types.contains_key(&canonical) {
                    canonical
                } else {
                    continue;
                };
                self.collect_required_members_recursive(&candidate, visited, members);
            }
        }

        if let Some(props) = self.symbol_index.property_symbols(type_name) {
            for (name, symbol) in props {
                if symbol.is_required {
                    members.insert(name.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_object_initializer_covers_success_and_failure() {
        assert!(
            parse_object_initializer("").is_none(),
            "empty text should fail"
        );
        assert!(
            parse_object_initializer("new Type(").is_none(),
            "unbalanced constructor call should fail"
        );
        let parsed = parse_object_initializer(
            "new Demo.Point<int> { X = 1, Y = other(2), Z = { nested = true }, W = array[0] }",
        )
        .expect("initializer should parse");
        assert_eq!(parsed.type_name, "Demo.Point<int>");
        assert_eq!(parsed.members, vec!["X", "Y", "Z", "W"]);
    }

    #[test]
    fn parse_object_initializer_rejects_malformed_inputs() {
        let cases = [
            "",
            "new",
            "new { x = 1 }",
            "Type(",
            "Type { = 1 }",
            "Type { value = (1 }",
            "Type { value = [1 }",
            "Type { value 1 }",
            "Type { value = 1 } trailing",
        ];
        for text in cases {
            assert!(
                parse_object_initializer(text).is_none(),
                "initializer `{text}` should be rejected"
            );
        }
    }

    #[test]
    fn format_missing_required_message_handles_plural() {
        let single = format_missing_required_message("Demo", &["Field".to_string()]);
        assert!(
            single.contains("required member `Field`"),
            "single required member should be mentioned"
        );
        let multiple =
            format_missing_required_message("Demo", &["A".into(), "B".into(), "C".into()]);
        assert!(
            multiple.contains("members `A`, `B` and `C`"),
            "plural required members should be enumerated"
        );
    }
}
