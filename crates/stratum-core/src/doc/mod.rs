//! Documentation generation for Stratum source code
//!
//! This module provides tools for extracting documentation from Stratum source
//! files and generating documentation in various formats (HTML, Markdown).
//!
//! ## Features
//!
//! - **Documentation extraction**: Extract doc comments from AST
//! - **Cross-linking**: Automatic linking between types and functions
//! - **Search**: Client-side fuzzy search across all symbols
//! - **Multiple formats**: HTML and Markdown output

mod crosslink;
mod extractor;
mod html;
mod markdown;
mod project;
mod search;
mod types;

pub use crosslink::{extract_type_names, CrossLinkConfig, CrossLinker};
pub use extractor::DocExtractor;
pub use html::{HtmlGenerator, HtmlOptions};
pub use markdown::MarkdownGenerator;
pub use project::{build_project_doc, ProjectDoc, SymbolInfo};
pub use search::{generate_search_css, generate_search_index, generate_search_js, SearchEntry};
pub use types::{DocComment, DocumentedItem, DocumentedModule, ItemKind, ParamDoc};
