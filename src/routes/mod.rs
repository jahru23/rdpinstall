pub mod assets;
pub mod pages;
pub mod telegram;

use axum::routing::{get, post};
use axum::Router;

use crate::SharedState;

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/", get(pages::home))
        .route("/login", get(pages::login))
        .route("/auth/telegram", get(telegram::callback))
        .route("/auth/dev", post(telegram::dev_login))
        .route("/logout", post(telegram::logout))
        .route("/healthz", get(|| async { "ok" }))
        .route("/static/style.css", get(assets::css))
        .route("/favicon.svg", get(assets::favicon))
        .fallback(pages::not_found)
        .with_state(state)
}
