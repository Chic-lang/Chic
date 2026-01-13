use std::collections::HashSet;

use crate::doc::extensions::{DocContext, DocExtensions};
use crate::doc::model::{
    BlockContent, CodeBlock, CustomSection, DocDiagnostic, InlineContent, LinkNode, LinkTarget,
    ListItem, ListKind, ParamDoc, ParsedDoc,
};
use regex::Regex;

pub fn parse_xml_doc(doc: &str, path: &str, exts: &DocExtensions) -> ParsedDoc {
    let mut parsed = ParsedDoc::default();
    if doc.trim().is_empty() {
        return parsed;
    }

    let prefix_re = Regex::new(r"</?([A-Za-z_][A-Za-z0-9_.-]*):").expect("prefix regex");
    let mut prefixes = HashSet::new();
    for capture in prefix_re.captures_iter(doc) {
        if let Some(prefix) = capture.get(1) {
            prefixes.insert(prefix.as_str().to_string());
        }
    }
    let xmlns = prefixes
        .into_iter()
        .map(|prefix| format!(" xmlns:{prefix}=\"urn:doc:{prefix}\""))
        .collect::<String>();
    let wrapped = format!("<doc{xmlns}>{doc}</doc>");
    let Ok(document) = roxmltree::Document::parse(&wrapped) else {
        parsed.diagnostics.push(DocDiagnostic::error(
            "DOCXML001",
            "XML documentation comment is not well-formed",
            Some(path.to_string()),
        ));
        return parsed;
    };

    let ctx = DocContext { symbol_path: path };
    let mut summary_inline = Vec::new();

    for child in document.root_element().children() {
        if child.is_text() {
            if let Some(text) = child.text() {
                if let Some(text) = normalise_text_segment(text) {
                    summary_inline.push(InlineContent::Text(text));
                }
            }
            continue;
        }
        if !child.is_element() {
            continue;
        }
        let name = full_name(&child);
        match name.as_str() {
            "summary" => {
                parsed.model.summary.extend(parse_block_children(
                    &child,
                    &ctx,
                    exts,
                    &mut parsed.diagnostics,
                ));
            }
            "remarks" => parsed.model.remarks.extend(parse_block_children(
                &child,
                &ctx,
                exts,
                &mut parsed.diagnostics,
            )),
            "returns" => {
                parsed.model.returns = Some(parse_block_children(
                    &child,
                    &ctx,
                    exts,
                    &mut parsed.diagnostics,
                ));
            }
            "value" => {
                parsed.model.value = Some(parse_block_children(
                    &child,
                    &ctx,
                    exts,
                    &mut parsed.diagnostics,
                ));
            }
            "example" => {
                let caption = child.attribute("title").map(str::to_string);
                let mut code_block = None;
                for grand in child.children().filter(|n| n.is_element()) {
                    let grand_name = full_name(&grand);
                    if grand_name == "code" {
                        let lang = grand.attribute("lang").map(str::to_string);
                        let code = grand.text().unwrap_or_default().to_string();
                        code_block = Some(CodeBlock {
                            language: lang.or_else(|| Some("chic".to_string())),
                            title: None,
                            code,
                        });
                    }
                }
                if let Some(code) = code_block {
                    parsed
                        .model
                        .examples
                        .push(crate::doc::model::ExampleBlock { caption, code });
                } else {
                    let code = child.text().unwrap_or_default().to_string();
                    parsed.model.examples.push(crate::doc::model::ExampleBlock {
                        caption,
                        code: CodeBlock {
                            language: Some("chic".to_string()),
                            title: None,
                            code,
                        },
                    });
                }
            }
            "see" | "seealso" => {
                if let Some(link) = link_from_node(&child) {
                    parsed.model.see_also.push(link);
                }
            }
            "param" => {
                if let Some(name) = child.attribute("name") {
                    let content = parse_block_children(&child, &ctx, exts, &mut parsed.diagnostics);
                    parsed.model.params.push(ParamDoc {
                        name: name.to_string(),
                        content,
                    });
                } else {
                    parsed.diagnostics.push(DocDiagnostic::warning(
                        "DOCXML002",
                        "parameter documentation missing 'name' attribute",
                        Some(path.to_string()),
                    ));
                }
            }
            "typeparam" => {
                if let Some(name) = child.attribute("name") {
                    let content = parse_block_children(&child, &ctx, exts, &mut parsed.diagnostics);
                    parsed.model.type_params.push(ParamDoc {
                        name: name.to_string(),
                        content,
                    });
                } else {
                    parsed.diagnostics.push(DocDiagnostic::warning(
                        "DOCXML003",
                        "type parameter documentation missing 'name' attribute",
                        Some(path.to_string()),
                    ));
                }
            }
            other => {
                let mut handled = false;
                for handler in &exts.tag_handlers {
                    if handler.supports(other) {
                        if let Some(output) = handler.handle(&child, &ctx) {
                            if !output.inline.is_empty() {
                                parsed
                                    .model
                                    .remarks
                                    .push(BlockContent::Paragraph(output.inline));
                            }
                            parsed.model.remarks.extend(output.blocks);
                            parsed.diagnostics.extend(output.diagnostics);
                            handled = true;
                            break;
                        }
                    }
                }
                if handled {
                    continue;
                }
                let blocks = parse_block_children(&child, &ctx, exts, &mut parsed.diagnostics);
                if !blocks.is_empty() {
                    parsed.model.custom_sections.push(CustomSection {
                        title: other.to_string(),
                        content: blocks,
                    });
                } else {
                    parsed.diagnostics.push(DocDiagnostic::warning(
                        "DOCXML004",
                        format!("unsupported documentation element '{other}'"),
                        Some(path.to_string()),
                    ));
                }
            }
        }
    }

    if !summary_inline.is_empty() && parsed.model.summary.is_empty() {
        parsed
            .model
            .summary
            .push(BlockContent::Paragraph(summary_inline));
    }

    parsed
}

fn parse_block_children(
    node: &roxmltree::Node,
    ctx: &DocContext,
    exts: &DocExtensions,
    diags: &mut Vec<DocDiagnostic>,
) -> Vec<BlockContent> {
    let mut blocks = Vec::new();
    let mut current_inline: Vec<InlineContent> = Vec::new();

    for child in node.children() {
        if child.is_text() {
            if let Some(text) = child.text() {
                if let Some(text) = normalise_text_segment(text) {
                    current_inline.push(InlineContent::Text(text));
                }
            }
            continue;
        }
        if !child.is_element() {
            continue;
        }
        let name = full_name(&child);
        match name.as_str() {
            "para" => {
                if !current_inline.is_empty() {
                    blocks.push(BlockContent::Paragraph(current_inline));
                    current_inline = Vec::new();
                }
            }
            "code" => {
                if !current_inline.is_empty() {
                    blocks.push(BlockContent::Paragraph(current_inline));
                    current_inline = Vec::new();
                }
                let lang = child.attribute("lang").map(str::to_string);
                let title = child.attribute("title").map(str::to_string);
                let code = child.text().unwrap_or_default().to_string();
                blocks.push(BlockContent::CodeBlock(CodeBlock {
                    language: lang.or_else(|| Some("chic".to_string())),
                    title,
                    code,
                }));
            }
            "list" => {
                if !current_inline.is_empty() {
                    blocks.push(BlockContent::Paragraph(current_inline));
                    current_inline = Vec::new();
                }
                blocks.push(parse_list(&child, ctx, exts, diags));
            }
            "see" | "seealso" => {
                if let Some(link) = link_from_node(&child) {
                    current_inline.push(InlineContent::Link(link));
                }
            }
            "c" => {
                let text = child.text().unwrap_or_default();
                current_inline.push(InlineContent::Code(text.to_string()));
            }
            "paramref" => {
                if let Some(name) = child.attribute("name") {
                    current_inline.push(InlineContent::ParamRef(name.to_string()));
                }
            }
            "typeparamref" => {
                if let Some(name) = child.attribute("name") {
                    current_inline.push(InlineContent::TypeParamRef(name.to_string()));
                }
            }
            other => {
                let mut handled = false;
                for handler in &exts.tag_handlers {
                    if handler.supports(other) {
                        if let Some(output) = handler.handle(&child, ctx) {
                            if !current_inline.is_empty() && !output.blocks.is_empty() {
                                blocks.push(BlockContent::Paragraph(current_inline));
                                current_inline = Vec::new();
                            }
                            blocks.extend(output.blocks);
                            current_inline.extend(output.inline);
                            diags.extend(output.diagnostics);
                            handled = true;
                            break;
                        }
                    }
                }
                if !handled {
                    diags.push(DocDiagnostic::warning(
                        "DOCXML005",
                        format!("unhandled documentation element '{other}'"),
                        Some(ctx.symbol_path.to_string()),
                    ));
                }
            }
        }
    }

    if !current_inline.is_empty() {
        blocks.push(BlockContent::Paragraph(current_inline));
    }
    blocks
}

fn parse_list(
    node: &roxmltree::Node,
    ctx: &DocContext,
    exts: &DocExtensions,
    diags: &mut Vec<DocDiagnostic>,
) -> BlockContent {
    let kind = match node.attribute("type").map(str::trim) {
        Some("number") | Some("ordered") => ListKind::Numbered,
        Some("table") => ListKind::Table,
        _ => ListKind::Bullet,
    };
    let mut items = Vec::new();
    for item in node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "item")
    {
        let mut term = Vec::new();
        let mut body = Vec::new();
        for child in item.children().filter(|n| n.is_element()) {
            match full_name(&child).as_str() {
                "term" => {
                    term.extend(parse_inline_children(&child, ctx, exts, diags));
                }
                "description" => {
                    body.extend(parse_block_children(&child, ctx, exts, diags));
                }
                _ => {}
            }
        }
        if body.is_empty() {
            body.extend(parse_block_children(&item, ctx, exts, diags));
        }
        items.push(ListItem {
            term: if term.is_empty() { None } else { Some(term) },
            body,
        });
    }
    BlockContent::List { kind, items }
}

fn parse_inline_children(
    node: &roxmltree::Node,
    ctx: &DocContext,
    exts: &DocExtensions,
    diags: &mut Vec<DocDiagnostic>,
) -> Vec<InlineContent> {
    let mut result = Vec::new();
    for child in node.children() {
        if child.is_text() {
            if let Some(text) = child.text() {
                if let Some(text) = normalise_text_segment(text) {
                    result.push(InlineContent::Text(text));
                }
            }
            continue;
        }
        if !child.is_element() {
            continue;
        }
        let name = full_name(&child);
        match name.as_str() {
            "c" => result.push(InlineContent::Code(
                child.text().unwrap_or_default().to_string(),
            )),
            "see" | "seealso" => {
                if let Some(link) = link_from_node(&child) {
                    result.push(InlineContent::Link(link));
                }
            }
            "paramref" => {
                if let Some(name) = child.attribute("name") {
                    result.push(InlineContent::ParamRef(name.to_string()));
                }
            }
            "typeparamref" => {
                if let Some(name) = child.attribute("name") {
                    result.push(InlineContent::TypeParamRef(name.to_string()));
                }
            }
            other => {
                for handler in &exts.tag_handlers {
                    if handler.supports(other) {
                        if let Some(output) = handler.handle(&child, ctx) {
                            result.extend(output.inline);
                            result.extend(
                                output
                                    .blocks
                                    .into_iter()
                                    .map(|block| InlineContent::Text(render_block_inline(block))),
                            );
                            diags.extend(output.diagnostics);
                            break;
                        }
                    }
                }
            }
        }
    }
    result
}

fn render_block_inline(block: BlockContent) -> String {
    match block {
        BlockContent::Paragraph(inlines) => inlines
            .into_iter()
            .map(|inline| match inline {
                InlineContent::Text(text) => text,
                InlineContent::Code(text) => format!("`{text}`"),
                InlineContent::Link(link) => {
                    if let Some(text) = link.text.clone() {
                        text
                    } else {
                        link_target_display(&link)
                    }
                }
                InlineContent::ParamRef(name) | InlineContent::TypeParamRef(name) => {
                    format!("`{name}`")
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
        BlockContent::CodeBlock(code) => code.code,
        BlockContent::List { items, .. } => items
            .into_iter()
            .map(|item| {
                let term = item
                    .term
                    .map(|t| {
                        t.into_iter()
                            .map(|inline| match inline {
                                InlineContent::Text(text) => text,
                                InlineContent::Code(text) => format!("`{text}`"),
                                InlineContent::Link(link) => {
                                    if let Some(text) = link.text.clone() {
                                        text
                                    } else {
                                        link_target_display(&link)
                                    }
                                }
                                InlineContent::ParamRef(name)
                                | InlineContent::TypeParamRef(name) => format!("`{name}`"),
                            })
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();
                let body = item
                    .body
                    .into_iter()
                    .map(render_block_inline)
                    .collect::<Vec<_>>()
                    .join(" ");
                if term.is_empty() {
                    body
                } else {
                    format!("{term}: {body}")
                }
            })
            .collect::<Vec<_>>()
            .join(", "),
        BlockContent::BlockQuote(content) => content
            .into_iter()
            .map(render_block_inline)
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn link_from_node(node: &roxmltree::Node) -> Option<LinkNode> {
    let cref = node.attribute("cref");
    let href = node.attribute("href");
    let text = node
        .text()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if let Some(cref) = cref {
        let target = if cref.starts_with("http://") || cref.starts_with("https://") {
            LinkTarget::Url(cref.to_string())
        } else {
            LinkTarget::Cref(cref.to_string())
        };
        return Some(LinkNode {
            target,
            text: text.or_else(|| href.map(str::to_string)),
        });
    }
    if let Some(href) = href {
        return Some(LinkNode {
            target: LinkTarget::Url(href.to_string()),
            text,
        });
    }
    text.map(|value| LinkNode {
        target: LinkTarget::Plain(value.clone()),
        text: Some(value),
    })
}

fn full_name(node: &roxmltree::Node) -> String {
    let local = node.tag_name().name();
    if let Some(ns) = node.tag_name().namespace() {
        if ns.starts_with("urn:doc:") {
            let prefix = ns.trim_start_matches("urn:doc:");
            format!("{prefix}:{local}")
        } else if ns.is_empty() {
            local.to_string()
        } else {
            format!("{ns}:{local}")
        }
    } else if local.contains(':') {
        local.to_string()
    } else {
        local.to_string()
    }
}

fn link_target_display(link: &LinkNode) -> String {
    match &link.target {
        LinkTarget::Cref(cref) | LinkTarget::Plain(cref) => cref.clone(),
        LinkTarget::Url(url) => url.clone(),
    }
}

fn normalise_text_segment(text: &str) -> Option<String> {
    let collapsed = text
        .split_whitespace()
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if collapsed.is_empty() {
        return None;
    }
    let mut value = collapsed;
    if text.chars().next().is_some_and(|ch| ch.is_whitespace()) {
        value.insert(0, ' ');
    }
    if text
        .chars()
        .rev()
        .next()
        .is_some_and(|ch| ch.is_whitespace())
    {
        if !value.ends_with(' ') {
            value.push(' ');
        }
    }
    Some(value)
}
