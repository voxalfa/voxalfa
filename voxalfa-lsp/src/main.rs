use async_lsp::{
    MainLoop, client_monitor::ClientProcessMonitorLayer, concurrency::ConcurrencyLayer,
    panic::CatchUnwindLayer, server::LifecycleLayer, tracing::TracingLayer,
};
use clap::Parser;
use tower::ServiceBuilder;
use tracing::Level;
use voxalfa_core::validation::Validator;
use voxalfa_lsp::server::ServerState;

#[derive(Parser)]
#[clap(
    about,
    version = voxalfa_build_info::VERSION,
    long_version = voxalfa_build_info::long_version()
)]
pub struct Cli {
    #[arg(short, long, default_value_t = Level::INFO)]
    log_level: Level,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
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
        .with_max_level(cli.log_level)
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
