//! LSP Backend implementation for Stratum
//!
//! This module contains the main `LanguageServer` trait implementation.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::cache::DocumentCache;
use crate::code_actions;
use crate::completions;
use crate::definition;
use crate::diagnostics;
use crate::document_symbols;
use crate::formatting;
use crate::hover;
use crate::references;
use crate::rename;
use crate::signature_help;
use crate::workspace_symbols;

/// The Stratum Language Server implementation
pub struct StratumLanguageServer {
    /// LSP client for sending notifications
    client: Client,
    /// Open documents indexed by URI with cached analysis data
    documents: Arc<RwLock<HashMap<Url, DocumentCache>>>,
}

impl StratumLanguageServer {
    /// Create a new language server instance
    #[must_use]
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Publish diagnostics for a document using cached data
    async fn publish_diagnostics_cached(&self, uri: Url, version: Option<i32>) {
        let diags = {
            let mut docs = self.documents.write().await;
            if let Some(cache) = docs.get_mut(&uri) {
                let data = cache.get_all_cached();
                diagnostics::compute_diagnostics_cached(&data)
            } else {
                vec![]
            }
        };

        self.client.publish_diagnostics(uri, diags, version).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for StratumLanguageServer {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string()]),
                    resolve_provider: Some(false),
                    ..Default::default()
                }),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    retrigger_characters: None,
                    work_done_progress_options: Default::default(),
                }),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_range_formatting_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![
                            CodeActionKind::QUICKFIX,
                            CodeActionKind::REFACTOR_EXTRACT,
                        ]),
                        work_done_progress_options: Default::default(),
                        resolve_provider: Some(false),
                    },
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "stratum-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Stratum language server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text;
        let version = params.text_document.version;

        // Store the document with cache
        {
            let mut docs = self.documents.write().await;
            docs.insert(uri.clone(), DocumentCache::new(content, version));
        }

        // Publish diagnostics using cached data
        self.publish_diagnostics_cached(uri, Some(version)).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        // Apply all changes to the document (supports both incremental and full sync)
        {
            let mut docs = self.documents.write().await;
            if let Some(cache) = docs.get_mut(&uri) {
                for change in params.content_changes {
                    cache.apply_change(change.range, change.text, version);
                }
            }
        }

        // Publish diagnostics using cached data
        self.publish_diagnostics_cached(uri, Some(version)).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        // Remove document from tracking
        {
            let mut docs = self.documents.write().await;
            docs.remove(&uri);
        }

        // Clear diagnostics for closed document
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        // Re-check on save to pick up any external changes
        // If content is provided, we could update the cache, but typically
        // the editor has already sent didChange events
        self.publish_diagnostics_cached(params.text_document.uri, None)
            .await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // Get the document and use cached data
        let mut docs = self.documents.write().await;
        if let Some(cache) = docs.get_mut(&uri) {
            let data = cache.get_all_cached();
            if let Some(info) = hover::compute_hover_cached(&data, position) {
                return Ok(Some(hover::hover_info_to_lsp(info)));
            }
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // Get the document and use cached data
        let mut docs = self.documents.write().await;
        if let Some(cache) = docs.get_mut(&uri) {
            let data = cache.get_all_cached();
            if let Some(result) = definition::compute_definition_cached(&uri, &data, position) {
                return Ok(Some(GotoDefinitionResponse::Scalar(result.location)));
            }
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;

        // Get the document and use cached data
        let mut docs = self.documents.write().await;
        if let Some(cache) = docs.get_mut(&uri) {
            let data = cache.get_all_cached();
            let refs =
                references::compute_references_cached(&uri, &data, position, include_declaration);
            if !refs.is_empty() {
                return Ok(Some(refs));
            }
        }

        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        // Get the document and use cached data
        let mut docs = self.documents.write().await;
        if let Some(cache) = docs.get_mut(&uri) {
            let data = cache.get_all_cached();
            let items = completions::compute_completions_cached(&data, position);
            if !items.is_empty() {
                return Ok(Some(CompletionResponse::Array(items)));
            }
        }

        Ok(None)
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // Get the document and use cached data
        let mut docs = self.documents.write().await;
        if let Some(cache) = docs.get_mut(&uri) {
            let data = cache.get_all_cached();
            if let Some(help) = signature_help::compute_signature_help_cached(&data, position) {
                return Ok(Some(help));
            }
        }

        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;

        // Get the document and use cached data
        let mut docs = self.documents.write().await;
        if let Some(cache) = docs.get_mut(&uri) {
            let data = cache.get_all_cached();
            let symbols = document_symbols::compute_document_symbols_cached(&data);
            if !symbols.is_empty() {
                return Ok(Some(DocumentSymbolResponse::Nested(symbols)));
            }
        }

        Ok(None)
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let query = &params.query;

        // Collect all open documents with their content
        let docs = self.documents.read().await;
        let documents: Vec<(Url, String)> = docs
            .iter()
            .map(|(uri, cache)| (uri.clone(), cache.content().to_string()))
            .collect();

        let symbols = workspace_symbols::compute_workspace_symbols(query, &documents);
        if symbols.is_empty() {
            Ok(None)
        } else {
            Ok(Some(symbols))
        }
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let position = params.position;

        // Get the document and use cached data
        let mut docs = self.documents.write().await;
        if let Some(cache) = docs.get_mut(&uri) {
            let data = cache.get_all_cached();
            if let Some(response) = rename::prepare_rename_cached(&data, position) {
                return Ok(Some(response));
            }
        }

        Ok(None)
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = &params.new_name;

        // Get the document and use cached data
        let mut docs = self.documents.write().await;
        if let Some(cache) = docs.get_mut(&uri) {
            let data = cache.get_all_cached();
            if let Some(edit) = rename::compute_rename_cached(&uri, &data, position, new_name) {
                return Ok(Some(edit));
            }
        }

        Ok(None)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;

        // Get the document content
        let docs = self.documents.read().await;
        if let Some(cache) = docs.get(&uri) {
            if let Some(edits) = formatting::compute_formatting(cache.content()) {
                return Ok(Some(edits));
            }
        }

        Ok(None)
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let range = params.range;

        // Get the document content
        let docs = self.documents.read().await;
        if let Some(cache) = docs.get(&uri) {
            if let Some(edits) = formatting::compute_range_formatting(cache.content(), range) {
                return Ok(Some(edits));
            }
        }

        Ok(None)
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let range = params.range;
        let diagnostics = &params.context.diagnostics;

        // Get the document and use cached data
        let mut docs = self.documents.write().await;
        if let Some(cache) = docs.get_mut(&uri) {
            let data = cache.get_all_cached();
            let actions =
                code_actions::compute_code_actions_cached(&uri, &data, range, diagnostics);
            if !actions.is_empty() {
                return Ok(Some(actions));
            }
        }

        Ok(None)
    }
}
