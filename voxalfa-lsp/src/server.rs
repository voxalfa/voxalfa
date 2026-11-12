use std::collections::HashMap;

use async_lsp::{
    ClientSocket, LanguageClient,
    lsp_types::{PublishDiagnosticsParams, Url},
    router::Router,
};

use voxalfa_core::validation::Validator;

use crate::{diagnostics::convert_diagnostic, state::Document};

pub struct ServerState {
    pub client: ClientSocket,
    pub validator: Validator,
    pub documents: HashMap<Url, Document>,
}

impl ServerState {
    pub fn new_router(client: ClientSocket, validator: Validator) -> Router<Self> {
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
