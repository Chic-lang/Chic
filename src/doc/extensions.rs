use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, OnceLock, RwLock};

use crate::doc::model::{
    BlockContent, CodeBlock, DocDiagnostic, InlineContent, LinkNode, LinkTarget,
};

#[derive(Clone)]
pub struct DocExtensions {
    pub tag_handlers: Vec<Arc<dyn DocTagHandler>>,
    pub link_resolver: Arc<dyn LinkResolver>,
}

impl Default for DocExtensions {
    fn default() -> Self {
        Self {
            tag_handlers: Vec::new(),
            link_resolver: Arc::new(DefaultLinkResolver),
        }
    }
}

impl fmt::Debug for DocExtensions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DocExtensions")
            .field("tag_handlers", &self.tag_handlers.len())
            .field("link_resolver", &"<custom>")
            .finish()
    }
}

pub trait DocTagHandler: Send + Sync {
    fn handle(&self, element: &roxmltree::Node, ctx: &DocContext) -> Option<HandlerOutput>;
    fn supports(&self, name: &str) -> bool;
}

pub trait LinkResolver: Send + Sync {
    fn resolve(&self, link: &LinkNode, ctx: &DocContext) -> Option<ResolvedLink>;
}

#[derive(Debug, Clone)]
pub struct DocContext<'a> {
    pub symbol_path: &'a str,
}

#[derive(Debug, Clone)]
pub struct ResolvedLink {
    pub text: String,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct HandlerOutput {
    pub blocks: Vec<BlockContent>,
    pub inline: Vec<InlineContent>,
    pub diagnostics: Vec<DocDiagnostic>,
}

impl HandlerOutput {
    #[must_use]
    pub fn block(block: BlockContent) -> Self {
        Self {
            blocks: vec![block],
            inline: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

pub struct DocExtensionRegistry {
    tag_handlers: RwLock<HashMap<String, Arc<dyn DocTagHandler>>>,
    link_resolvers: RwLock<HashMap<String, Arc<dyn LinkResolver>>>,
}

impl DocExtensionRegistry {
    fn new() -> Self {
        let registry = Self {
            tag_handlers: RwLock::new(HashMap::new()),
            link_resolvers: RwLock::new(HashMap::new()),
        };
        registry.register_tag_handler("chic:sample", Arc::new(ChicSampleHandler));
        registry.register_tag_handler("chic:note", Arc::new(ChicNoteHandler));
        registry.register_link_resolver("default", Arc::new(DefaultLinkResolver));
        registry
    }

    pub fn register_tag_handler(&self, name: impl Into<String>, handler: Arc<dyn DocTagHandler>) {
        let mut map = self.tag_handlers.write().expect("lock tag_handlers");
        map.insert(name.into(), handler);
    }

    pub fn register_link_resolver(&self, name: impl Into<String>, resolver: Arc<dyn LinkResolver>) {
        let mut map = self.link_resolvers.write().expect("lock link_resolvers");
        map.insert(name.into(), resolver);
    }

    #[must_use]
    pub fn tag_handler(&self, name: &str) -> Option<Arc<dyn DocTagHandler>> {
        let map = self.tag_handlers.read().expect("read tag_handlers");
        map.get(name).cloned()
    }

    #[must_use]
    pub fn link_resolver(&self, name: &str) -> Option<Arc<dyn LinkResolver>> {
        let map = self.link_resolvers.read().expect("read link_resolvers");
        map.get(name).cloned()
    }
}

fn registry() -> &'static DocExtensionRegistry {
    static REGISTRY: OnceLock<DocExtensionRegistry> = OnceLock::new();
    REGISTRY.get_or_init(DocExtensionRegistry::new)
}

#[must_use]
pub fn resolve_extensions(
    handler_names: &[String],
    link_resolver_name: Option<&str>,
) -> DocExtensions {
    let reg = registry();
    let mut handlers: Vec<Arc<dyn DocTagHandler>> = Vec::new();

    for name in handler_names {
        if let Some(handler) = reg.tag_handler(name) {
            handlers.push(handler);
        }
    }

    // Always include built-in Chic handlers.
    for builtin in ["chic:sample", "chic:note"] {
        if let Some(handler) = reg.tag_handler(builtin) {
            if !handlers.iter().any(|existing| existing.supports(builtin)) {
                handlers.push(handler);
            }
        }
    }

    let resolver = link_resolver_name
        .and_then(|name| reg.link_resolver(name))
        .unwrap_or_else(|| reg.link_resolver("default").unwrap());

    DocExtensions {
        tag_handlers: handlers,
        link_resolver: resolver,
    }
}

pub fn register_tag_handler(name: impl Into<String>, handler: Arc<dyn DocTagHandler>) {
    registry().register_tag_handler(name, handler);
}

pub fn register_link_resolver(name: impl Into<String>, resolver: Arc<dyn LinkResolver>) {
    registry().register_link_resolver(name, resolver);
}

#[derive(Debug)]
struct DefaultLinkResolver;

impl LinkResolver for DefaultLinkResolver {
    fn resolve(&self, link: &LinkNode, ctx: &DocContext) -> Option<ResolvedLink> {
        match &link.target {
            LinkTarget::Cref(cref) => {
                let anchor = anchor_for_cref(cref);
                let text = link.text.clone().unwrap_or_else(|| cref.clone());
                Some(ResolvedLink {
                    text,
                    target: Some(format!("#{anchor}")),
                })
            }
            LinkTarget::Url(url) => Some(ResolvedLink {
                text: link.text.clone().unwrap_or_else(|| url.clone()),
                target: Some(url.clone()),
            }),
            LinkTarget::Plain(text) => Some(ResolvedLink {
                text: link.text.clone().unwrap_or_else(|| text.clone()),
                target: None,
            }),
        }
        .or_else(|| {
            let text = link
                .text
                .clone()
                .unwrap_or_else(|| ctx.symbol_path.to_string());
            Some(ResolvedLink { text, target: None })
        })
    }
}

fn anchor_for_cref(cref: &str) -> String {
    let mut anchor = String::with_capacity(cref.len());
    for ch in cref.chars() {
        if ch.is_alphanumeric() {
            anchor.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '.' | ':' | '_' | '{' | '}' | '<' | '>' | '`') {
            anchor.push('-');
        } else if ch.is_whitespace() {
            anchor.push('-');
        }
    }
    while anchor.ends_with('-') {
        anchor.pop();
    }
    anchor
}

#[derive(Debug)]
struct ChicSampleHandler;

impl DocTagHandler for ChicSampleHandler {
    fn handle(&self, element: &roxmltree::Node, _ctx: &DocContext) -> Option<HandlerOutput> {
        if !self.supports(element.tag_name().name()) {
            return None;
        }
        let lang = element
            .attribute("lang")
            .map(str::to_string)
            .or_else(|| Some("chic".to_string()));
        let title = element.attribute("title").map(str::to_string);
        let code = element.text().unwrap_or_default().trim().to_string();
        let block = BlockContent::CodeBlock(CodeBlock {
            language: lang,
            title,
            code,
        });
        Some(HandlerOutput::block(block))
    }

    fn supports(&self, name: &str) -> bool {
        name.eq_ignore_ascii_case("chic:sample") || name.eq_ignore_ascii_case("sample")
    }
}

#[derive(Debug)]
struct ChicNoteHandler;

impl DocTagHandler for ChicNoteHandler {
    fn handle(&self, element: &roxmltree::Node, _: &DocContext) -> Option<HandlerOutput> {
        if !self.supports(element.tag_name().name()) {
            return None;
        }
        let text = element
            .text()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("Note");
        let block =
            BlockContent::BlockQuote(vec![BlockContent::Paragraph(vec![InlineContent::Text(
                format!("Note: {text}"),
            )])]);
        Some(HandlerOutput::block(block))
    }

    fn supports(&self, name: &str) -> bool {
        name.eq_ignore_ascii_case("chic:note") || name.eq_ignore_ascii_case("note")
    }
}
