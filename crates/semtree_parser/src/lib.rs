mod event;
mod grammar;
mod parser;
mod sink;

pub use parser::{ParseResult, Parser};

#[cfg(test)]
mod tests;
