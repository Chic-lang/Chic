use super::*;

mod cursor;
mod expressions;
mod locals;
mod module;
mod recovery;

#[derive(Debug, Clone)]
pub(crate) struct Modifier {
    pub name: String,
    pub span: Span,
}
