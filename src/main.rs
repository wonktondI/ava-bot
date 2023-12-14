use anyhow::Result;
use ava_bot::handlers::{chats_handler, index_page};
use clap::Parser;
use salvo::prelude::TcpListener;
use salvo::serve_static::StaticDir;
use salvo::{Listener, Router, Server};
use std::time::Duration;
use tracing::info;

#[derive(Debug, Parser)]
#[clap(name = "ava")]
struct Args {
    #[clap(short, long, default_value = "8080")]
    port: u16,
    #[clap(short, long, default_value = ".certs")]
    cert_path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let router = Router::new()
        .push(Router::with_path("/public/<*path>").get(StaticDir::new(["public"]).auto_list(true)))
        .push(
            Router::with_path("/")
                .get(index_page)
                .push(Router::with_path("/chats").get(chats_handler)),
        );

    let addr = format!("0.0.0.0:{}", args.port);
    info!("Listening on {}", addr);
    let acceptor = TcpListener::new(addr).bind().await;
    let server = Server::new(acceptor);
    let handle = server.handle();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        handle.stop_graceful(None);
    });
    server.serve(router).await;

    Ok(())
}
