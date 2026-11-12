use async_lsp::{MainLoop, concurrency::ConcurrencyLayer, server::LifecycleLayer};
use futures::{
    AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, StreamExt,
    channel::mpsc::{UnboundedSender, unbounded},
    io::BufReader,
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
        console_error_panic_hook::set_once();

        let validator = Validator::init()?;

        let (server, _) = MainLoop::new_server(|client| {
            ServiceBuilder::new()
                .layer(LifecycleLayer::default())
                .layer(ConcurrencyLayer::default())
                .service(ServerState::new_router(client, validator))
        });

        let (stdin, mut stdin_writer) = piper::pipe(BUFFER_SIZE);
        let (mut stdout_reader, stdout) = piper::pipe(BUFFER_SIZE);
        let (tx, mut rx) = unbounded::<String>();

        spawn_local(async move {
            while let Some(message) = rx.next().await {
                let _ = stdin_writer.write_all(message.as_bytes()).await;
            }
        });

        spawn_local(async move {
            let mut header = String::new();

            loop {
                let mut reader = BufReader::new(&mut stdout_reader);

                match reader.read_line(&mut header).await {
                    Err(err) => panic!("Read error: {err}"),
                    Ok(0) => continue,
                    _ => {}
                };

                let Some(length) = header.strip_prefix("Content-Length: ") else {
                    continue;
                };

                let parsed_length = length.trim_end().parse::<usize>().unwrap();
                header.clear();

                let _ = reader.read_line(&mut header).await;
                assert_eq!(&header, "\r\n");
                header.clear();

                let mut buffer = vec![0; parsed_length];
                let _ = reader.read_exact(&mut buffer).await;
                let parsed = String::from_utf8(buffer).unwrap();

                let _ = post.call1(&JsValue::NULL, &JsValue::from_str(&parsed));
            }
        });

        spawn_local(async move {
            let _ = server.run_buffered(stdin, stdout).await;
        });

        Ok(Self { tx })
    }

    pub fn process(&mut self, message: &str) {
        let framed = format!("Content-Length: {}\r\n\r\n{message}", message.len());
        self.tx.unbounded_send(framed).unwrap();
    }
}
