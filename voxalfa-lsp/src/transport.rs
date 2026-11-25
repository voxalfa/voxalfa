use std::error::Error;

use async_lsp::{MainLoop, concurrency::ConcurrencyLayer, server::LifecycleLayer};
use futures::{
    AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, StreamExt,
    channel::mpsc::UnboundedReceiver,
    future::{Either, select},
    io::BufReader,
    try_join,
};
use tower::ServiceBuilder;
use voxalfa_core::validation::Validator;

use crate::server::ServerState;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

pub(crate) struct Pipeline<ServerIn, ServerOut, HostOut, HostIn, Post> {
    pub validator: Validator,
    pub server_in: ServerIn,
    pub server_out: ServerOut,
    pub host_out: HostOut,
    pub host_in: HostIn,
    pub messages: UnboundedReceiver<String>,
    pub post_message: Post,
}

impl<ServerIn, ServerOut, HostOut, HostIn, Post>
    Pipeline<ServerIn, ServerOut, HostOut, HostIn, Post>
where
    ServerIn: AsyncRead,
    ServerOut: AsyncWrite,
    HostOut: AsyncWrite + Unpin,
    HostIn: AsyncRead + Unpin,
    Post: Fn(String),
{
    pub(crate) async fn run(self) -> Result<()> {
        let (server, _) = MainLoop::new_server(move |client| {
            ServiceBuilder::new()
                .layer(LifecycleLayer::default())
                .layer(ConcurrencyLayer::default())
                .service(ServerState::new_router(client, self.validator))
        });

        let io = async move {
            try_join!(
                write_loop(self.host_out, self.messages),
                read_loop(self.host_in, self.post_message),
            )
        };

        let server = async move { server.run_buffered(self.server_in, self.server_out).await };

        match select(Box::pin(io), Box::pin(server)).await {
            Either::Left((io_result, server)) => {
                io_result?;
                server.await.map_err(Into::into)
            }
            Either::Right((result, _io)) => result.map_err(Into::into),
        }
    }
}

async fn write_loop<W>(mut writer: W, mut messages: UnboundedReceiver<String>) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    while let Some(message) = messages.next().await {
        let message = format!("Content-Length: {}\r\n\r\n{message}", message.len());

        writer.write_all(message.as_bytes()).await?;
        writer.flush().await?;
    }

    Ok(())
}

async fn read_loop<R, Post>(reader: R, post_message: Post) -> Result<()>
where
    R: AsyncRead + Unpin,
    Post: Fn(String),
{
    let mut reader = BufReader::new(reader);
    let mut header_buffer = String::with_capacity(32);

    loop {
        match read_payload(&mut reader, &mut header_buffer).await? {
            Some(message) => post_message(message),
            None => return Ok(()),
        }
    }
}

async fn read_payload<R>(reader: &mut R, header_buffer: &mut String) -> Result<Option<String>>
where
    R: AsyncBufRead + Unpin,
{
    if reader.read_line(header_buffer).await? == 0 {
        return Ok(None);
    }

    let Some(length) = header_buffer.strip_prefix("Content-Length: ") else {
        header_buffer.clear();

        return Err(
            std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid LSP header").into(),
        );
    };

    let parsed_length = length.trim_end().parse::<usize>()?;

    header_buffer.clear();

    if reader.read_line(header_buffer).await? == 0 {
        return Ok(None);
    }

    header_buffer.clear();

    let mut buffer = vec![0; parsed_length];

    reader.read_exact(&mut buffer).await?;

    Ok(Some(String::from_utf8(buffer)?))
}
