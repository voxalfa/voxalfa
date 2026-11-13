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
use web_sys::console;

use crate::server::ServerState;

const BUFFER_SIZE: usize = 64 * 1024;

#[wasm_bindgen]
pub struct WasmTransport {
    tx: UnboundedSender<String>,
}

#[wasm_bindgen]
impl WasmTransport {
    #[wasm_bindgen(constructor)]
    pub fn new(post_message: js_sys::Function) -> Result<Self, JsError> {
        console_error_panic_hook::set_once();

        let validator = Validator::init()?;
        console::debug_1(&"Tree-sitter successfully loaded".into());

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
                let result = stdin_writer.write_all(message.as_bytes()).await;

                if let Some(error) = result.err() {
                    console::error_1(&error.to_string().into());
                }
            }
        });

        spawn_local(async move {
            let mut header_buffer = String::with_capacity(32);

            loop {
                let mut reader = BufReader::new(&mut stdout_reader);
                let result = read_payload(&mut reader, &mut header_buffer).await;

                match result {
                    Ok(Some(json)) => {
                        let _ = post_message.call1(&JsValue::NULL, &json);
                    }
                    Err(error) => console::error_1(&error.into()),
                    _ => {}
                }
            }
        });

        spawn_local(async move {
            console::debug_1(&"Initializing language services".into());
            let _ = server.run_buffered(stdin, stdout).await;
        });

        Ok(Self { tx })
    }

    pub fn process(&mut self, message: &str) -> Result<(), JsError> {
        let length = message.len();

        if message.len() > BUFFER_SIZE {
            return Err(JsError::new(&format!(
                "Message size exceeds buffer size ({length} bytes)"
            )));
        }

        self.tx.unbounded_send(format!(
            // Header: Content-Length + Content-Type (optional)
            "Content-Length: {length}\r\n\r\n{message}",
        ))?;

        Ok(())
    }
}

async fn read_payload(
    mut reader: impl AsyncBufReadExt + Unpin,
    header_buffer: &mut String,
) -> Result<Option<JsValue>, JsError> {
    let line_size = reader.read_line(header_buffer).await?;

    if line_size == 0 {
        return Ok(None);
    }

    let Some(length) = header_buffer.strip_prefix("Content-Length: ") else {
        return Ok(None);
    };

    let parsed_length = length.trim_end().parse::<usize>()?;
    header_buffer.clear();

    reader.read_line(header_buffer).await?;
    assert_eq!(header_buffer, "\r\n");
    header_buffer.clear();

    let mut buffer = vec![0; parsed_length];
    reader.read_exact(&mut buffer).await?;
    let parsed = String::from_utf8(buffer)?;

    Ok(Some(parsed.into()))
}
