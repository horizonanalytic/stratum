//! Stratum Language Server Protocol implementation
//!
//! This crate provides an LSP server for the Stratum programming language,
//! offering real-time diagnostics, hover information, and other IDE features.

mod backend;
mod cache;
mod code_actions;
mod completions;
mod definition;
mod diagnostics;
mod document_symbols;
mod formatting;
mod hover;
mod references;
mod rename;
mod signature_help;
mod workspace_symbols;

pub use backend::StratumLanguageServer;

use tower_lsp::{LspService, Server};

/// Run the LSP server on stdin/stdout
///
/// # Errors
///
/// Returns an error if the server fails to start or encounters an I/O error.
pub async fn run_server() -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(StratumLanguageServer::new);
    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}
