//! Shared Unicode data and helpers consumed by the frontend and runtime.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub start: u32,
    pub end: u32,
}

impl Range {
    #[inline]
    pub const fn contains(&self, value: u32) -> bool {
        self.start <= value && value <= self.end
    }
}

pub mod escapes;
pub mod grapheme;
pub mod identifier;
pub mod normalization;
