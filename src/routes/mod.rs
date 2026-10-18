pub mod assets;
pub mod cloud;
pub mod install;
pub mod pages;
pub mod telegram;
pub mod topup;

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
        .route("/cloud", get(cloud::index).post(cloud::add))
        .route("/cloud/{id}/check", post(cloud::check))
        .route("/cloud/{id}/delete", post(cloud::delete))
        .route("/install", get(install::wizard).post(install::create))
        .route("/jobs/{id}", get(install::detail))
        .route("/jobs/{id}/events", get(install::events))
        .route("/jobs/{id}/destroy", post(install::destroy))
        .route("/topup", get(topup::index).post(topup::create))
        .route("/topup/{ref}", get(topup::detail))
        .route("/topup/{ref}/status", get(topup::status))
        .route("/topup/{ref}/cancel", post(topup::cancel))
        .route("/healthz", get(|| async { "ok" }))
        .route("/static/style.css", get(assets::css))
        .route("/favicon.svg", get(assets::favicon))
        .fallback(pages::not_found)
        .with_state(state)
}
