//! Documentation generation for Stratum source code
//!
//! This module provides tools for extracting documentation from Stratum source
//! files and generating documentation in various formats (HTML, Markdown).

mod extractor;
mod html;
mod markdown;
mod types;

pub use extractor::DocExtractor;
pub use html::HtmlGenerator;
pub use markdown::MarkdownGenerator;
pub use types::{DocComment, DocumentedItem, DocumentedModule, ItemKind, ParamDoc};
