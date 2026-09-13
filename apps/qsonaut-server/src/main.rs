use std::{env, net::SocketAddr, path::PathBuf};

use anyhow::{Context, Result};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let bind = env::var("QSONAUT_SERVER_BIND").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    let address: SocketAddr = bind
        .parse()
        .with_context(|| format!("parse QSONAUT_SERVER_BIND value {bind:?}"))?;
    let web_root = PathBuf::from(
        env::var("QSONAUT_SERVER_WEB_ROOT").unwrap_or_else(|_| "web/build".to_owned()),
    );
    let database_url =
        env::var("QSONAUT_DATABASE_URL").context("QSONAUT_DATABASE_URL is required")?;
    let secure_cookies = env::var("QSONAUT_SECURE_COOKIES").map_or(true, |value| value != "false");
    let store = qsonaut_store::Store::connect(&database_url)
        .await
        .context("connect to PostgreSQL and apply migrations")?;
    let fallback = web_root.join("200.html");

    let app = qsonaut_api::router_with_store(store, secure_cookies)
        .route_service("/link", ServeFile::new(web_root.join("link.html")))
        .fallback_service(ServeDir::new(&web_root).not_found_service(ServeFile::new(fallback)))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("bind QSONaut Server to {address}"))?;

    info!(%address, web_root = %web_root.display(), "QSONaut Server ready");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serve QSONaut Server")?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("shutdown requested");
}
