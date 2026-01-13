use std::fmt::Write;

use crate::doc::extensions::{DocContext, DocExtensions, ResolvedLink};
use crate::doc::model::{
    BlockContent, CodeBlock, InlineContent, LinkNode, LinkTarget, ListItem, ListKind, ParsedDoc,
    SymbolDocs, SymbolKind,
};

#[derive(Debug, Clone, Default)]
pub struct RenderOptions {
    pub heading_level: usize,
    pub banner: Option<String>,
    pub front_matter: Option<String>,
}

pub fn render_symbol_page(
    symbol: &SymbolDocs,
    exts: &DocExtensions,
    options: &RenderOptions,
) -> String {
    let mut out = String::new();
    if let Some(front_matter) = &options.front_matter {
        out.push_str(front_matter);
        if !front_matter.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
    }
    if let Some(banner) = &options.banner {
        out.push_str(banner);
        if !banner.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
    }
    out.push_str(&render_symbol_body(
        symbol,
        exts,
        options.heading_level.max(1),
    ));
    out
}

pub fn render_symbol_body(
    symbol: &SymbolDocs,
    exts: &DocExtensions,
    heading_level: usize,
) -> String {
    let mut out = String::new();
    render_symbol(symbol, exts, heading_level, &mut out);
    out
}

fn render_symbol(
    symbol: &SymbolDocs,
    exts: &DocExtensions,
    heading_level: usize,
    out: &mut String,
) {
    write_heading(out, heading_level, &symbol_title(symbol));
    if let Some(signature) = &symbol.signature {
        let _ = writeln!(out, "\n`{signature}`\n");
    } else {
        out.push('\n');
    }

    let ctx = DocContext {
        symbol_path: &symbol.full_name,
    };
    render_docmodel(&symbol.doc, &ctx, exts, out);

    if !symbol.members.is_empty() {
        let _ = writeln!(out, "\n{} Members\n", "#".repeat(heading_level + 1));
        for member in &symbol.members {
            render_symbol(member, exts, heading_level + 2, out);
        }
    }
}

fn render_docmodel(doc: &ParsedDoc, ctx: &DocContext, exts: &DocExtensions, out: &mut String) {
    if !doc.model.summary.is_empty() {
        render_blocks(&doc.model.summary, ctx, exts, out);
        out.push('\n');
    }
    if !doc.model.params.is_empty() {
        out.push_str("## Parameters\n\n");
        render_param_table(&doc.model.params, ctx, exts, out);
        out.push('\n');
    }
    if !doc.model.type_params.is_empty() {
        out.push_str("## Type Parameters\n\n");
        render_param_table(&doc.model.type_params, ctx, exts, out);
        out.push('\n');
    }
    if let Some(returns) = &doc.model.returns {
        out.push_str("## Returns\n\n");
        render_blocks(returns, ctx, exts, out);
        out.push('\n');
    }
    if let Some(value) = &doc.model.value {
        out.push_str("## Value\n\n");
        render_blocks(value, ctx, exts, out);
        out.push('\n');
    }
    if !doc.model.examples.is_empty() {
        out.push_str("## Examples\n\n");
        for example in &doc.model.examples {
            if let Some(title) = &example.caption {
                let _ = writeln!(out, "_{}_\n", title.trim());
            }
            render_code_block(&example.code, out);
            out.push('\n');
        }
    }
    if !doc.model.remarks.is_empty() {
        out.push_str("## Remarks\n\n");
        render_blocks(&doc.model.remarks, ctx, exts, out);
        out.push('\n');
    }
    if !doc.model.see_also.is_empty() {
        out.push_str("## See Also\n\n");
        for link in &doc.model.see_also {
            let resolved = exts
                .link_resolver
                .resolve(link, ctx)
                .unwrap_or_else(|| ResolvedLink {
                    text: link
                        .text
                        .clone()
                        .unwrap_or_else(|| link_target_display(link)),
                    target: None,
                });
            if let Some(target) = resolved.target {
                let _ = writeln!(out, "- [{}]({})", resolved.text, target);
            } else {
                let _ = writeln!(out, "- {}", resolved.text);
            }
        }
        out.push('\n');
    }
    for custom in &doc.model.custom_sections {
        let heading = format!("## {}", custom.title);
        let _ = writeln!(out, "{heading}\n");
        render_blocks(&custom.content, ctx, exts, out);
        out.push('\n');
    }
}

fn render_param_table(
    entries: &[crate::doc::model::ParamDoc],
    ctx: &DocContext,
    exts: &DocExtensions,
    out: &mut String,
) {
    out.push_str("| Name | Description |\n| --- | --- |\n");
    for entry in entries {
        let mut description = String::new();
        render_blocks(&entry.content, ctx, exts, &mut description);
        let desc = description.trim();
        let safe = desc.replace('\n', " ");
        let _ = writeln!(out, "| `{}` | {} |", entry.name, safe);
    }
}

fn render_blocks(
    blocks: &[BlockContent],
    ctx: &DocContext,
    exts: &DocExtensions,
    out: &mut String,
) {
    for block in blocks {
        match block {
            BlockContent::Paragraph(inlines) => {
                render_inlines(inlines, ctx, exts, out);
                out.push_str("\n\n");
            }
            BlockContent::CodeBlock(code) => {
                render_code_block(code, out);
                out.push_str("\n\n");
            }
            BlockContent::List { kind, items } => {
                match kind {
                    ListKind::Bullet => render_bullet_list(items, ctx, exts, out),
                    ListKind::Numbered => render_numbered_list(items, ctx, exts, out),
                    ListKind::Table => render_table(items, ctx, exts, out),
                }
                out.push('\n');
            }
            BlockContent::BlockQuote(content) => {
                let mut inner = String::new();
                render_blocks(content, ctx, exts, &mut inner);
                for line in inner.lines() {
                    let _ = writeln!(out, "> {}", line);
                }
                out.push('\n');
            }
        }
    }
}

fn render_code_block(code: &CodeBlock, out: &mut String) {
    let lang = code.language.as_deref().unwrap_or("chic").trim();
    let _ = writeln!(out, "```{lang}");
    out.push_str(&code.code);
    if !code.code.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("```\n");
}

fn render_bullet_list(
    items: &[ListItem],
    ctx: &DocContext,
    exts: &DocExtensions,
    out: &mut String,
) {
    for item in items {
        out.push_str("- ");
        if let Some(term) = &item.term {
            render_inlines(term, ctx, exts, out);
            if !item.body.is_empty() {
                out.push_str(": ");
            }
        }
        if item.body.is_empty() {
            out.push('\n');
        } else {
            let mut body = String::new();
            render_blocks(&item.body, ctx, exts, &mut body);
            out.push_str(body.trim());
            out.push('\n');
        }
    }
}

fn render_numbered_list(
    items: &[ListItem],
    ctx: &DocContext,
    exts: &DocExtensions,
    out: &mut String,
) {
    for (idx, item) in items.iter().enumerate() {
        let _ = write!(out, "{}. ", idx + 1);
        if let Some(term) = &item.term {
            render_inlines(term, ctx, exts, out);
            if !item.body.is_empty() {
                out.push_str(": ");
            }
        }
        if item.body.is_empty() {
            out.push('\n');
        } else {
            let mut body = String::new();
            render_blocks(&item.body, ctx, exts, &mut body);
            out.push_str(body.trim());
            out.push('\n');
        }
    }
}

fn render_table(items: &[ListItem], ctx: &DocContext, exts: &DocExtensions, out: &mut String) {
    out.push_str("| Term | Description |\n| --- | --- |\n");
    for item in items {
        let mut term = String::new();
        if let Some(t) = &item.term {
            render_inlines(t, ctx, exts, &mut term);
        }
        let mut desc = String::new();
        render_blocks(&item.body, ctx, exts, &mut desc);
        let _ = writeln!(
            out,
            "| {} | {} |",
            term.trim(),
            desc.trim().replace('\n', " ")
        );
    }
}

fn render_inlines(
    inlines: &[InlineContent],
    ctx: &DocContext,
    exts: &DocExtensions,
    out: &mut String,
) {
    for inline in inlines {
        match inline {
            InlineContent::Text(text) => out.push_str(text),
            InlineContent::Code(code) => {
                out.push('`');
                out.push_str(code);
                out.push('`');
            }
            InlineContent::Link(link) => {
                if let Some(resolved) = exts.link_resolver.resolve(link, ctx) {
                    if let Some(target) = resolved.target {
                        let _ = write!(out, "[{}]({})", resolved.text, target);
                    } else {
                        out.push_str(&resolved.text);
                    }
                } else {
                    out.push_str(
                        &link
                            .text
                            .clone()
                            .unwrap_or_else(|| link_target_display(link)),
                    );
                }
            }
            InlineContent::ParamRef(name) | InlineContent::TypeParamRef(name) => {
                out.push('`');
                out.push_str(name);
                out.push('`');
            }
        }
    }
}

fn link_target_display(link: &LinkNode) -> String {
    match &link.target {
        LinkTarget::Cref(cref) | LinkTarget::Plain(cref) => cref.clone(),
        LinkTarget::Url(url) => url.clone(),
    }
}

fn write_heading(out: &mut String, level: usize, title: &str) {
    let hashes = "#".repeat(level.max(1));
    let _ = writeln!(out, "{hashes} {title}");
}

pub fn symbol_kind_label(kind: &SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Namespace => "Namespace",
        SymbolKind::Struct => "Struct",
        SymbolKind::Record => "Record",
        SymbolKind::Class => "Class",
        SymbolKind::Enum => "Enum",
        SymbolKind::Interface => "Interface",
        SymbolKind::Union => "Union",
        SymbolKind::Extension => "Extension",
        SymbolKind::Trait => "Trait",
        SymbolKind::Delegate => "Delegate",
        SymbolKind::Impl => "Impl",
        SymbolKind::Function => "Function",
        SymbolKind::Method => "Method",
        SymbolKind::Property => "Property",
        SymbolKind::Field => "Field",
        SymbolKind::Constructor => "Constructor",
        SymbolKind::Const => "Const",
        SymbolKind::Static => "Static",
        SymbolKind::TraitMethod => "Trait Method",
        SymbolKind::Unknown => "Symbol",
    }
}

pub fn symbol_title(symbol: &SymbolDocs) -> String {
    let kind = symbol_kind_label(&symbol.kind);
    format!("{kind} {}", symbol.name)
}
