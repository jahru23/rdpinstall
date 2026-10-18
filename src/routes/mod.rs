pub mod admin;
pub mod assets;
pub mod guest;
pub mod install;
pub mod pages;
pub mod telegram;
pub mod topup;

use axum::routing::{get, post};
use axum::Router;

use crate::SharedState;

pub fn router(state: SharedState) -> Router {
    Router::new()
        // publik: instalasi tamu dengan voucer
        .route("/", get(guest::index).post(guest::create))
        .route("/g/{token}", get(guest::progress))
        .route("/g/{token}/events", get(guest::events))
        .route("/masuk", get(pages::login))
        .route("/auth/telegram", get(telegram::callback))
        .route("/auth/dev", post(telegram::dev_login))
        .route("/keluar", post(telegram::logout))
        // berakun
        .route("/dashboard", get(pages::dashboard))
        .route("/instalasi", get(install::wizard).post(install::create))
        .route("/j/{id}", get(install::detail))
        .route("/j/{id}/events", get(install::events))
        .route("/j/{id}/reset", post(install::reset))
        .route("/j/{id}/wipe", post(install::wipe))
        .route("/topup", get(topup::index).post(topup::create))
        .route("/topup/{ref}", get(topup::detail))
        .route("/topup/{ref}/status", get(topup::status))
        .route("/topup/{ref}/cancel", post(topup::cancel))
        // admin
        .route("/admin/vouchers", get(admin::index).post(admin::create))
        .route("/admin/vouchers/{code}/revoke", post(admin::revoke))
        // teknis
        .route("/healthz", get(|| async { "ok" }))
        .route("/static/style.css", get(assets::css))
        .route("/favicon.svg", get(assets::favicon))
        .fallback(pages::not_found)
        .with_state(state)
}
