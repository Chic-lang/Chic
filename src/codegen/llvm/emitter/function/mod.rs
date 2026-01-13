mod blocks;
mod builder;
mod runtime;
mod statements;
mod terminators;
mod trace;
mod values;
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use builder::emit_function;
pub(crate) use builder::emit_function_with_async;

#[cfg(test)]
mod tests;
