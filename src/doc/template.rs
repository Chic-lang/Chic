use std::path::Path;

use crate::doc::extensions::DocExtensions;
use crate::doc::markdown::{render_symbol_body, symbol_kind_label, symbol_title};
use crate::doc::model::SymbolDocs;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Default)]
pub struct DocTemplate {
    body: Option<String>,
}

impl DocTemplate {
    #[must_use]
    pub fn none() -> Self {
        Self { body: None }
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path).map_err(|err| Error::Io(err))?;
        Ok(Self {
            body: Some(contents),
        })
    }

    #[must_use]
    pub fn render(
        &self,
        symbol: &SymbolDocs,
        exts: &DocExtensions,
        heading_level: usize,
    ) -> String {
        if let Some(body) = &self.body {
            let mut rendered = body.clone();
            rendered = rendered.replace("{{title}}", &symbol_title(symbol));
            rendered = rendered.replace("{{name}}", &symbol.name);
            rendered = rendered.replace("{{full_name}}", &symbol.full_name);
            rendered = rendered.replace("{{kind}}", symbol_kind_label(&symbol.kind));
            rendered = rendered.replace(
                "{{signature}}",
                symbol.signature.as_deref().unwrap_or_default(),
            );
            let content = render_symbol_body(symbol, exts, heading_level);
            rendered = rendered.replace("{{content}}", content.trim());
            return rendered;
        }

        render_symbol_body(symbol, exts, heading_level)
    }
}
