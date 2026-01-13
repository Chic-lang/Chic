mod base;
mod checker;
#[cfg(test)]
mod tests;

pub(super) use checker::BorrowChecker;
