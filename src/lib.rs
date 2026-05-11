//! CeWL — Custom Word List generator (Rust port).
//!
//! Copyright: original Ruby CeWL by Robin Wood; this port preserves behaviour where practical.
//! Licence: GPL-3.0+ (see upstream CeWL).

pub mod capture;
pub mod cli;
pub mod crawler;
pub mod fetcher;
pub mod html;
pub mod metadata;
pub mod tree;
pub mod words;

#[cfg(test)]
mod tests;

pub use cli::{CewlArgs, FabArgs};
pub use crawler::CrawlResult;
