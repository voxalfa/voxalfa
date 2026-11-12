use async_lsp::{MainLoop, concurrency::ConcurrencyLayer, server::LifecycleLayer};
use futures::{
    AsyncReadExt, AsyncWriteExt,
    channel::mpsc::{UnboundedSender, unbounded},
};
use tower::ServiceBuilder;
use voxalfa_core::validation::Validator;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::server::ServerState;

const BUFFER_SIZE: usize = 64 * 1024;

#[wasm_bindgen]
pub struct WasmTransport {
    tx: UnboundedSender<String>,
}

#[wasm_bindgen]
impl WasmTransport {
    #[wasm_bindgen(constructor)]
    pub fn new(post: js_sys::Function) -> Result<Self, JsError> {
        let validator = Validator::init()?;

        let (server, _) = MainLoop::new_server(|client| {
            ServiceBuilder::new()
                .layer(LifecycleLayer::default())
                // .layer(CatchUnwindLayer::default())
                .layer(ConcurrencyLayer::default())
                // .layer(ClientProcessMonitorLayer::new(client.clone()))
                .service(ServerState::new_router(client, validator))
        });

        let (stdin, mut stdin_writer) = piper::pipe(BUFFER_SIZE);
        let (mut stdout_reader, stdout) = piper::pipe(BUFFER_SIZE);
        let (tx, mut rx) = unbounded::<String>();

        spawn_local(async move {
            let a = rx.recv().await.unwrap();
            let _ = stdin_writer.write_all(a.as_bytes());
        });

        spawn_local(async move {
            let mut buffer = Vec::new();
            let _ = stdout_reader.read_to_end(&mut buffer);
            let parsed = String::from_utf8(buffer).unwrap();
            let _ = post.call1(&JsValue::NULL, &JsValue::from_str(&parsed));
        });

        spawn_local(async {
            let _ = server.run_buffered(stdin, stdout);
        });

        Ok(Self { tx })
    }

    pub fn process(&mut self, message: &str) {
        let _ = self.tx.unbounded_send(message.to_owned());
    }
}
