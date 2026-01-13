//! C header generation for Chic public APIs.

mod generator;
mod types;

pub use generator::{HeaderError, HeaderOptions, generate_header};
