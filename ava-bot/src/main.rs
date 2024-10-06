use anyhow::Result;
use ava_bot::handlers::{assistant_handler, events_handler, index_page};
use ava_bot::Args;
use clap::Parser;
use mimalloc::MiMalloc;
use rust_embed::RustEmbed;
use salvo::prelude::{RequestId, TcpListener};
use salvo::serve_static::{static_embed, StaticDir};
use salvo::server::ServerHandle;
use salvo::{Listener, Router, Server};
use time::macros::{format_description, offset};
use tokio::signal;
use tracing::info;
use tracing_subscriber::fmt::time::OffsetTime;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(RustEmbed)]
#[folder = "public"]
struct Public;

#[tokio::main]
async fn main() -> Result<()> {
    let timer = OffsetTime::new(
        offset!(+08:00:00),
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3] [offset_hour sign:mandatory]:[offset_minute]"),
    );
    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_max_level(tracing::Level::INFO)
        .with_timer(timer)
        .init();

    let args = Args::parse();
    let router = Router::new()
        .hoop(RequestId::new())
        // .push(Router::with_path("/public/<*path>").get(StaticDir::new(["public"]).auto_list(true)))
        .push(Router::with_path("/public/<*path>").get(static_embed::<Public>()))
        .push(
            Router::with_path("/assets/<*path>")
                .get(StaticDir::new(["tmp/ava-bot"]).auto_list(true)),
            // Router::with_path("/assets/<*path>").get(static_embed::<Asset>()),
        )
        .push(
            Router::new()
                .get(index_page)
                .push(Router::with_path("/events").get(events_handler))
                .push(Router::with_path("/assistant").post(assistant_handler)),
        );

    let addr = format!("0.0.0.0:{}", args.port);
    info!("Listening on {}", addr);
    let server = Server::new(TcpListener::new(addr).bind().await);
    let handle = server.handle();
    tokio::spawn(shutdown_signal(handle));
    server.serve(router).await;

    Ok(())
}

async fn shutdown_signal(handle: ServerHandle) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("ctrl_c signal received"),
        _ = terminate => info!("terminate signal received"),
    }
    handle.stop_graceful(std::time::Duration::from_secs(10));
}
