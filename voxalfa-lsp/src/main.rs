mod completion;
mod diagnostics;
mod state;
mod utils;

use std::ops::ControlFlow;

use async_lsp::{
    LanguageServer, MainLoop, ResponseError, client_monitor::ClientProcessMonitorLayer,
    concurrency::ConcurrencyLayer, lsp_types::*, panic::CatchUnwindLayer, server::LifecycleLayer,
    tracing::TracingLayer,
};
use futures::future::BoxFuture;
use tower::ServiceBuilder;
use tracing::Level;
use voxalfa_formatter::Formatter;
use voxalfa_validator::MultiStepValidator;

use crate::{
    completion::get_completion_context,
    state::{Document, ServerState},
};

impl LanguageServer for ServerState {
    type Error = ResponseError;
    type NotifyResult = ControlFlow<async_lsp::Result<()>>;

    fn initialize(
        &mut self,
        _params: InitializeParams,
    ) -> BoxFuture<'static, Result<InitializeResult, Self::Error>> {
        let result = InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL, // FIXME: use incremental sync
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec!["{".to_string()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "voxalfa-ls".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string().into(),
            }),
        };

        Box::pin(async { Ok(result) })
    }

    fn did_open(&mut self, params: DidOpenTextDocumentParams) -> Self::NotifyResult {
        let uri = params.text_document.uri;
        let source = params.text_document.text;
        let data = self.validator.analyze(&source);
        let document = Document { source, data };

        self.documents.insert(uri.clone(), document);
        self.publish_diagnostics(uri);

        ControlFlow::Continue(())
    }

    fn did_change(&mut self, params: DidChangeTextDocumentParams) -> Self::NotifyResult {
        let uri = params.text_document.uri;

        if let Some(doc) = self.documents.get_mut(&uri) {
            if let Some(change) = params.content_changes.into_iter().last() {
                doc.data = self.validator.analyze(&change.text);
                doc.source = change.text;
                self.publish_diagnostics(uri);
            }
        }

        ControlFlow::Continue(())
    }

    fn did_save(&mut self, _params: DidSaveTextDocumentParams) -> Self::NotifyResult {
        ControlFlow::Continue(())
    }

    fn completion(
        &mut self,
        params: CompletionParams,
    ) -> BoxFuture<'static, Result<Option<CompletionResponse>, Self::Error>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let line = position.line as usize;
        let column = position.character as usize;

        let result = self
            .documents
            .get(&uri)
            .and_then(|d| get_completion_context(d, line, column))
            .map(|c| CompletionResponse::Array(c.completion_items()));

        Box::pin(async move { Ok(result) })
    }

    fn hover(
        &mut self,
        _params: HoverParams,
    ) -> BoxFuture<'static, Result<Option<Hover>, Self::Error>> {
        Box::pin(async move {
            Ok(Some(Hover {
                contents: HoverContents::Scalar(MarkedString::String("TODO".to_string())),
                range: None,
            }))
        })
    }

    fn formatting(
        &mut self,
        params: DocumentFormattingParams,
    ) -> BoxFuture<'static, Result<Option<Vec<TextEdit>>, Self::Error>> {
        let uri = params.text_document.uri;

        let edits = self.documents.get(&uri).and_then(|doc| {
            if doc.data.has_error() {
                return None;
            }

            let formatter = Formatter::new(&doc.data);
            let formatted_text = formatter.format_to_string().ok()?;
            let line_count = doc.source.lines().count().saturating_sub(1);
            let last_line_len = doc.source.lines().last().map_or(0, |l| l.chars().count());

            let full_range = Range {
                start: Position::new(0, 0),
                end: Position::new(line_count as u32, last_line_len as u32),
            };

            Some(vec![TextEdit {
                range: full_range,
                new_text: formatted_text.trim_end().to_string(), // FIXME
            }])
        });

        Box::pin(async move { Ok(edits) })
    }

    fn did_close(&mut self, params: DidCloseTextDocumentParams) -> Self::NotifyResult {
        self.documents.remove(&params.text_document.uri);

        ControlFlow::Continue(())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let validator = MultiStepValidator::init()?;

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
