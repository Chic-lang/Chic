//! Frontend components: lexer, parser, and diagnostic utilities.

pub mod ast;
pub mod attributes;
pub mod cfg;
pub mod conditional;
pub mod diagnostics;
pub mod import_resolver;
pub mod lexer;
pub mod literals;
pub mod local_functions;
pub mod macro_expander;
pub mod metadata;
pub mod parser;
pub mod type_alias;
pub mod type_utils;
