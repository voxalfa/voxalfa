use futures::channel::mpsc::{UnboundedSender, unbounded};
use tokio::io::duplex;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing::error;
use voxalfa_core::validation::Validator;

use crate::transport::Pipeline;

const BUFFER_SIZE: usize = 64 * 1024;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct NativeTransport {
    tx: UnboundedSender<String>,
}

impl NativeTransport {
    pub fn spawn(post_message: impl Fn(String) + Send + 'static) -> Result<Self> {
        let validator = Validator::init()?;
        let (stdin, stdin_writer) = duplex(BUFFER_SIZE);
        let (stdout_reader, stdout) = duplex(BUFFER_SIZE);
        let (tx, rx) = unbounded::<String>();

        let pipeline = Pipeline {
            validator,
            server_in: stdin.compat(),
            server_out: stdout.compat_write(),
            host_out: stdin_writer.compat_write(),
            host_in: stdout_reader.compat(),
            messages: rx,
            post_message,
        };

        tokio::spawn(async move {
            if let Err(error) = pipeline.run().await {
                error!("language server stopped: {error}");
            }
        });

        Ok(Self { tx })
    }

    pub fn process(&self, message: &str) -> Result<()> {
        self.tx.unbounded_send(message.to_string())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;
    use tokio::{sync::mpsc::unbounded_channel, time::timeout};

    use super::NativeTransport;

    const TIMEOUT: Duration = Duration::from_secs(10);

    #[tokio::test]
    async fn serves_a_document_over_the_native_transport() {
        let (tx, mut rx) = unbounded_channel::<String>();

        let transport = NativeTransport::spawn(move |message| {
            let _ = tx.send(message);
        })
        .unwrap();

        timeout(TIMEOUT, async {
            transport
                .process(
                    &json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "initialize",
                        "params": { "capabilities": {} }
                    })
                    .to_string(),
                )
                .unwrap();

            let response = rx.recv().await.expect("server closed the channel");

            assert!(response.contains("\"id\":1"));

            transport
                .process(
                    &json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} })
                    .to_string(),
                )
                .unwrap();

            transport
                .process(
                    &json!({
                        "jsonrpc": "2.0",
                        "method": "textDocument/didOpen",
                        "params": {
                            "textDocument": {
                                "uri": "file:///score.vfa",
                                "languageId": "voxalfa",
                                "version": 1,
                                "text": "[$] key={C} | time={4,4} | voices={S}\n\n--\n\n[S] |d :r !m :f ||"
                            }
                        }
                    })
                    .to_string(),
                )
                .unwrap();

            let notification = rx.recv().await.expect("server closed the channel");
            assert!(notification.contains("textDocument/publishDiagnostics"));
        })
        .await
        .expect("LSP test timed out");
    }
}
