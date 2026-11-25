use futures::channel::mpsc::{UnboundedSender, unbounded};
use piper::pipe;
use voxalfa_core::validation::Validator;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::console;

use crate::transport::Pipeline;

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
        let (stdin, stdin_writer) = pipe(BUFFER_SIZE);
        let (stdout_reader, stdout) = pipe(BUFFER_SIZE);
        let (tx, rx) = unbounded::<String>();

        let pipeline = Pipeline {
            validator,
            server_in: stdin,
            server_out: stdout,
            host_out: stdin_writer,
            host_in: stdout_reader,
            messages: rx,
            post_message: move |message: String| {
                let _ = post_message.call1(&JsValue::NULL, &JsValue::from_str(&message));
            },
        };

        spawn_local(async move {
            if let Err(error) = pipeline.run().await {
                console::error_1(&error.to_string().into());
            }
        });

        Ok(Self { tx })
    }

    pub fn process(&mut self, message: &str) -> Result<(), JsError> {
        self.tx.unbounded_send(message.to_string())?;

        Ok(())
    }
}
