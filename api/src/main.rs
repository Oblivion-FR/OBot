mod auth;
mod cache;
mod config;
mod db;
mod error;
mod hypixel;
mod mojang;
mod routes;

use sqlx::SqlitePool;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

type Error = Box<dyn std::error::Error + Send + Sync>;

/// Shared by every request
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub hypixel: Arc<hypixel::Hypixel>,
    pub mojang: Arc<mojang::Mojang>,
}

const USAGE: &str = "Usage:
  hytools-api                  run the API
  hytools-api key create NAME  create an API key for a service, like `bot` or `dashboard`";

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Only the working directory, so a `.env` in a parent folder is never picked up by accident
    match dotenvy::from_path(".env") {
        Ok(()) => {}
        Err(error) if error.not_found() => {}
        Err(error) => return Err(error.into()),
    }
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] => serve().await,
        ["key", "create", name] => {
            let db = db::connect(&config::Config::database_url()).await?;
            let key = auth::create_key(&db, name).await?;
            println!("API key for `{name}`, shown only once:\n{key}");
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}

async fn serve() -> Result<(), Error> {
    let config = config::Config::from_env()?;
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "starting HyTools API");

    let http = reqwest::Client::builder()
        .user_agent(concat!("HyTools-API/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let state = AppState {
        db: db::connect(&config.database_url).await?,
        hypixel: Arc::new(hypixel::Hypixel::new(
            http.clone(),
            hypixel::BASE_URL.to_owned(),
            config.hypixel_api_key,
        )),
        mojang: Arc::new(mojang::Mojang::new(
            http,
            mojang::PROFILES_URL.to_owned(),
            mojang::SESSION_URL.to_owned(),
        )),
    };

    let listener = tokio::net::TcpListener::bind(&config.bind).await?;
    tracing::info!(
        "listening on http://{}, docs at /docs",
        listener.local_addr()?
    );
    axum::serve(listener, routes::app(state))
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

/// Ctrl+C, or the SIGTERM Docker sends on stop
async fn shutdown() {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("SIGTERM handler installs");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
    tracing::info!("shutting down");
}
