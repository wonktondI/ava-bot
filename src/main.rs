use anyhow::Result;
use ava_bot::handlers::{assistant_handler, events_handler, index_page};
use ava_bot::Args;
use clap::Parser;
use rust_embed::RustEmbed;
use salvo::prelude::TcpListener;
use salvo::serve_static::{static_embed, StaticDir};
use salvo::{Listener, Router, Server};
use std::time::Duration;
use tracing::info;

#[derive(RustEmbed)]
#[folder = "./public/"]
struct Public;

#[derive(RustEmbed)]
#[folder = "./tmp/ava-bot/"]
struct Asset;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();
    let router = Router::new()
        // .push(Router::with_path("/public/<*path>").get(StaticDir::new(["public"]).auto_list(true)))
        .push(Router::with_path("/public/<*path>").get(static_embed::<Public>()))
        .push(
            // Router::with_path("/assets/<*path>").get(StaticDir::new(["tmp/ava-bot"]).auto_list(true)),
            Router::with_path("/assets/<*path>").get(static_embed::<Asset>()),
        )
        .push(
            Router::new()
                .get(index_page)
                .push(Router::with_path("/events").get(events_handler))
                .push(Router::with_path("/assistant").post(assistant_handler)),
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
