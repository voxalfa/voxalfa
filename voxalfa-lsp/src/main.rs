mod actions;
mod builtin;
mod completion;
mod definition;
mod diagnostics;
mod docs;
mod parameters;
mod rename;
mod state;
mod symbols;
mod utils;

use core::iter::Iterator;
use std::{collections::HashMap, ops::ControlFlow};

use async_lsp::{
    LanguageServer, MainLoop, ResponseError, client_monitor::ClientProcessMonitorLayer,
    concurrency::ConcurrencyLayer, lsp_types::*, panic::CatchUnwindLayer, server::LifecycleLayer,
    tracing::TracingLayer,
};
use futures::future::BoxFuture;
use ropey::Rope;
use tower::ServiceBuilder;
use tracing::Level;
use voxalfa_core::validation::Validator;
use voxalfa_formatter::Formatter;

use crate::{
    actions::resolve_action_commands,
    completion::get_completion_context,
    definition::resolve_symbol_definition,
    docs::create_documentation,
    rename::resolve_rename_edits,
    state::{Document, ServerState},
    symbols::resolve_document_symbols,
    utils::{lsp_pos_to_ts, ts_range_to_lsp},
};

pub const SERVER_NAME: &str = "voxalfa-ls";

type Response<T, E> = BoxFuture<'static, std::result::Result<T, E>>;

impl LanguageServer for ServerState {
    type Error = ResponseError;
    type NotifyResult = ControlFlow<async_lsp::Result<()>>;

    fn initialize(&mut self, _params: InitializeParams) -> Response<InitializeResult, Self::Error> {
        let result = InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL, // FIXME: use incremental sync
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some("{,".chars().map(|c| c.to_string()).collect()),
                    ..Default::default()
                }),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![CodeActionKind::REFACTOR_REWRITE]),
                        ..Default::default()
                    },
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: SERVER_NAME.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string().into(),
            }),
        };

        Box::pin(async { Ok(result) })
    }

    fn initialized(&mut self, _params: InitializedParams) -> Self::NotifyResult {
        tracing::info!("Server initialized successfully");

        ControlFlow::Continue(())
    }

    fn did_open(&mut self, params: DidOpenTextDocumentParams) -> Self::NotifyResult {
        let uri = params.text_document.uri;
        let source = params.text_document.text;
        let data = self.validator.analyze(&source);
        let document = Document::new(uri.clone(), source, data);

        self.documents.insert(uri.clone(), document);
        self.publish_diagnostics(uri);

        ControlFlow::Continue(())
    }

    fn did_close(&mut self, params: DidCloseTextDocumentParams) -> Self::NotifyResult {
        self.documents.remove(&params.text_document.uri);

        ControlFlow::Continue(())
    }

    fn did_change(&mut self, params: DidChangeTextDocumentParams) -> Self::NotifyResult {
        let uri = params.text_document.uri;

        if let Some(doc) = self.documents.get_mut(&uri)
            && let Some(change) = params.content_changes.into_iter().last()
        {
            doc.data = self.validator.analyze(&change.text);
            doc.rope = Rope::from_str(&change.text); // FIXME: apply incremental updates
            doc.source = change.text;

            self.publish_diagnostics(uri);
        }

        ControlFlow::Continue(())
    }

    fn did_save(&mut self, _params: DidSaveTextDocumentParams) -> Self::NotifyResult {
        ControlFlow::Continue(())
    }

    fn completion(
        &mut self,
        params: CompletionParams,
    ) -> Response<Option<CompletionResponse>, Self::Error> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let line = position.line as usize;
        let column = position.character as usize;

        let result = self
            .documents
            .get(&uri)
            .and_then(|d| get_completion_context(d, line, column))
            .map(|c| CompletionResponse::Array(c.completion_items()));

        Box::pin(async { Ok(result) })
    }

    fn hover(&mut self, params: HoverParams) -> Response<Option<Hover>, Self::Error> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let data = self
            .documents
            .get(&uri)
            .and_then(|doc| create_documentation(doc, position))
            .map(|value| Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: None,
            });

        Box::pin(async { Ok(data) })
    }

    fn document_symbol(
        &mut self,
        params: DocumentSymbolParams,
    ) -> Response<Option<DocumentSymbolResponse>, Self::Error> {
        let response = self
            .documents
            .get(&params.text_document.uri)
            .and_then(resolve_document_symbols);

        Box::pin(async { Ok(response) })
    }

    fn definition(
        &mut self,
        params: GotoDefinitionParams,
    ) -> Response<Option<GotoDefinitionResponse>, Self::Error> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let result = self
            .documents
            .get(&uri)
            .and_then(|doc| resolve_symbol_definition(doc, position));

        Box::pin(async { Ok(result) })
    }

    fn references(
        &mut self,
        params: ReferenceParams,
    ) -> Response<Option<Vec<Location>>, Self::Error> {
        let uri = params.text_document_position.text_document.uri;
        let raw_position = params.text_document_position.position;

        let references = self.documents.get(&uri).and_then(|doc| {
            let position = lsp_pos_to_ts(&doc.rope, raw_position);
            let ranges = doc.data.symbols.get_symbol_refs(&position)?;

            let result = ranges
                .iter()
                .map(|r| ts_range_to_lsp(&doc.rope, r))
                .map(|r| Location::new(uri.clone(), r))
                .collect();

            Some(result)
        });

        Box::pin(async { Ok(references) })
    }

    fn formatting(
        &mut self,
        params: DocumentFormattingParams,
    ) -> Response<Option<Vec<TextEdit>>, Self::Error> {
        let uri = params.text_document.uri;

        let edits = self.documents.get(&uri).and_then(|doc| {
            if doc.data.has_syntax_error() {
                return None;
            }

            let formatter = Formatter::new(&doc.data);
            let formatted_text = formatter.format_to_string().ok()?;
            let last_line_idx = doc.rope.len_lines().saturating_sub(1);
            let last_line_utf16 = doc.rope.line(last_line_idx).len_utf16_cu();
            let start_pos = Position::new(0, 0);
            let end_pos = Position::new(last_line_idx as u32, last_line_utf16 as u32);

            Some(vec![TextEdit {
                range: Range::new(start_pos, end_pos),
                new_text: formatted_text.trim_end().to_string(),
            }])
        });

        Box::pin(async { Ok(edits) })
    }

    fn rename(&mut self, params: RenameParams) -> Response<Option<WorkspaceEdit>, Self::Error> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let result = self
            .documents
            .get(&uri)
            .and_then(|doc| resolve_rename_edits(params.new_name, position, doc))
            .map(|edits| WorkspaceEdit {
                changes: Some(HashMap::from([(uri, edits)])),
                ..Default::default()
            });

        Box::pin(async { Ok(result) })
    }

    fn code_action(
        &mut self,
        params: CodeActionParams,
    ) -> Response<Option<Vec<CodeActionOrCommand>>, Self::Error> {
        let uri = params.text_document.uri;
        let position = params.range.start;

        let result = self
            .documents
            .get(&uri)
            .and_then(|doc| resolve_action_commands(doc, position));

        Box::pin(async { Ok(result) })
    }

    fn shutdown(&mut self, _: ()) -> Response<(), Self::Error> {
        tracing::info!("Shutting down server");

        self.documents.clear();

        Box::pin(async { Ok(()) })
    }

    fn exit(&mut self, _: ()) -> Self::NotifyResult {
        tracing::info!("Exiting server loop");

        ControlFlow::Break(Ok(()))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let validator = Validator::init()?;

    let (server, _) = MainLoop::new_server(|client| {
        ServiceBuilder::new()
            .layer(TracingLayer::default())
            .layer(LifecycleLayer::default())
            .layer(CatchUnwindLayer::default())
            .layer(ConcurrencyLayer::default())
            .layer(ClientProcessMonitorLayer::new(client.clone()))
            .service(ServerState::new_router(client, validator))
    });

    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .init();

    #[cfg(unix)]
    let (stdin, stdout) = (
        async_lsp::stdio::PipeStdin::lock_tokio()?,
        async_lsp::stdio::PipeStdout::lock_tokio()?,
    );

    #[cfg(not(unix))]
    let (stdin, stdout) = (
        tokio_util::compat::TokioAsyncReadCompatExt::compat(tokio::io::stdin()),
        tokio_util::compat::TokioAsyncWriteCompatExt::compat_write(tokio::io::stdout()),
    );

    server.run_buffered(stdin, stdout).await?;

    Ok(())
}
