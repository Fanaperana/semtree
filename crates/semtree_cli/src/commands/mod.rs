mod init;
mod parse;
mod check;
mod format;
mod query;
mod benchmark;
mod import;
mod doctor;

pub use init::init;
pub use parse::parse;
pub use check::check;
pub use format::format;
pub use query::query;
pub use benchmark::benchmark;
pub use import::import;
pub use doctor::doctor;

pub type Result = std::result::Result<(), Box<dyn std::error::Error>>;
