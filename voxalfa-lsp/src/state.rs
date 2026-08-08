use std::collections::HashMap;

use async_lsp::{
    ClientSocket, LanguageClient,
    lsp_types::{PublishDiagnosticsParams, Url},
    router::Router,
};
use ropey::Rope;
use voxalfa_validator::{MultiStepValidator, output::FinalOutput};

use crate::diagnostics::convert_diagnostic;

#[derive(Debug)]
pub struct Document {
    pub uri: Url,
    pub source: String,
    pub data: FinalOutput,
    pub rope: Rope,
}

impl Document {
    pub fn new(uri: Url, source: String, data: FinalOutput) -> Self {
        Self {
            rope: Rope::from_str(&source),
            uri,
            source,
            data,
        }
    }
}

pub struct ServerState {
    pub client: ClientSocket,
    pub validator: MultiStepValidator,
    pub documents: HashMap<Url, Document>,
}

impl ServerState {
    pub fn new_router(client: ClientSocket, validator: MultiStepValidator) -> Router<Self> {
        Router::from_language_server(ServerState {
            client,
            validator,
            documents: HashMap::new(),
        })
    }

    pub fn publish_diagnostics(&mut self, uri: Url) {
        if let Some(doc) = self.documents.get(&uri) {
            let diagnostics = doc
                .data
                .diagnostics
                .iter()
                .flat_map(|d| convert_diagnostic(doc, d))
                .collect();

            let params = PublishDiagnosticsParams {
                uri,
                diagnostics,
                version: None,
            };

            if let Err(err) = self.client.publish_diagnostics(params) {
                tracing::error!("{err}")
            }
        }
    }
}
