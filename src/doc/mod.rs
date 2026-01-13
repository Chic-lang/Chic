pub mod extensions;
pub mod generator;
pub mod markdown;
pub mod model;
pub mod template;
pub mod xml;

pub use extensions::{
    DocExtensions, DocTagHandler, LinkResolver, register_link_resolver, register_tag_handler,
    resolve_extensions,
};
pub use generator::{
    DocGenerationOptions, DocGenerationResult, DocOutputLayout, GeneratedDocFile, generate_markdown,
};
pub use model::{ParsedDoc, SymbolDocs};
pub use template::DocTemplate;
pub use xml::parse_xml_doc;
