// Fase 1 (kerangka) belum memakai sebagian besar model/kripto — dipakai mulai F2/F3.
#![allow(dead_code)]

mod auth;
mod config;
mod crypto;
mod db;
mod error;
mod jobs;
mod models;
mod paygate;
mod reinstall;
mod routes;
mod ssh;
mod validate;
mod views;

use std::sync::Arc;

use anyhow::Result;
use tower_http::{limit::RequestBodyLimitLayer, trace::TraceLayer};

pub struct AppState {
    pub cfg: config::Config,
    pub pool: sqlx::SqlitePool,
}

pub type SharedState = Arc<AppState>;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rdpinstall=info,tower_http=warn".into()),
        )
        .init();

    let cfg = config::Config::from_env()?;
    db::prepare(&cfg.db_path)?;
    let pool = db::connect(&cfg.db_path).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("database ready at {}", cfg.db_path);

    if cfg.bot_token.is_empty() {
        tracing::warn!("TELEGRAM_BOT_TOKEN kosong — login Telegram tidak akan bisa diverifikasi");
    }
    if !cfg.dev_login {
        tracing::info!("dev login nonaktif");
    }

    let state: SharedState = Arc::new(AppState {
        cfg: cfg.clone(),
        pool,
    });

    let app = routes::router(state.clone())
        .layer(RequestBodyLimitLayer::new(64 * 1024))
        .layer(TraceLayer::new_for_http());

    jobs::spawn_janitor(state.clone());

    let listener = tokio::net::TcpListener::bind(&cfg.bind).await?;
    tracing::info!(
        "rdpinstall v{} listening on {} (env={})",
        env!("CARGO_PKG_VERSION"),
        cfg.bind,
        cfg.env
    );
    axum::serve(listener, app).await?;
    Ok(())
}
