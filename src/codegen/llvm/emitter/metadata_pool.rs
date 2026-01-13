#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MetadataId(pub(crate) usize);

#[derive(Default)]
pub(crate) struct MetadataRegistry {
    nodes: Vec<String>,
}

impl MetadataRegistry {
    pub(crate) fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub(crate) fn allocate_node(&mut self, body: String) -> MetadataId {
        let id = self.nodes.len();
        self.nodes.push(body);
        MetadataId(id)
    }

    pub(crate) fn alias_scope_domain(&mut self, label: &str) -> MetadataId {
        let sanitized = sanitize_label(label);
        let id = self.nodes.len();
        let body = format!("distinct !{{!{id}, !\"{sanitized}\"}}");
        self.nodes.push(body);
        MetadataId(id)
    }

    pub(crate) fn alias_scope(&mut self, domain: MetadataId, label: &str) -> MetadataId {
        let sanitized = sanitize_label(label);
        let id = self.nodes.len();
        let body = format!("distinct !{{!{id}, !{}, !\"{sanitized}\"}}", domain.0);
        self.nodes.push(body);
        MetadataId(id)
    }

    pub(crate) fn alias_scope_set(&mut self, scopes: &[MetadataId]) -> Option<MetadataId> {
        if scopes.is_empty() {
            return None;
        }
        let mut body = String::from("!{");
        for (index, scope) in scopes.iter().enumerate() {
            if index > 0 {
                body.push_str(", ");
            }
            body.push_str(&format!("!{}", scope.0));
        }
        body.push('}');
        Some(self.allocate_node(body))
    }

    pub(crate) fn reference(&self, id: MetadataId) -> String {
        format!("!{}", id.0)
    }

    pub(crate) fn emit(&self, out: &mut String) {
        if self.nodes.is_empty() {
            return;
        }
        for (index, body) in self.nodes.iter().enumerate() {
            writeln!(out, "!{index} = {body}").ok();
        }
    }
}

fn sanitize_label(label: &str) -> String {
    label
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | ':' | '.' => ch,
            _ => '_',
        })
        .collect()
}

use std::fmt::Write;
