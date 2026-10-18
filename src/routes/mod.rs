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
        // publik: instalasi tamu dengan voucer & validasi voucer
        .route("/", get(guest::index).post(guest::create))
        .route(
            "/api/voucher/validate",
            get(guest::validate_voucher).post(guest::validate_voucher_post),
        )
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
        // admin: panel manajemen seluruh web
        .route("/admin", get(admin::index))
        .route("/admin/overview", get(admin::overview))
        .route("/admin/jobs", get(admin::jobs))
        .route("/admin/jobs/{id}/cancel", post(admin::cancel_job))
        .route("/admin/jobs/{id}/delete", post(admin::delete_job))
        .route("/admin/users", get(admin::users))
        .route("/admin/users/{id}/balance", post(admin::set_balance))
        .route("/admin/users/{id}/toggle-ban", post(admin::toggle_ban))
        .route("/admin/payments", get(admin::payments))
        .route(
            "/admin/payments/{ref}/approve",
            post(admin::approve_payment),
        )
        .route("/admin/vouchers", get(admin::index).post(admin::create))
        .route("/admin/vouchers/{code}/revoke", post(admin::revoke))
        .route("/admin/vouchers/{code}/delete", post(admin::delete_voucher))
        // teknis
        .route("/healthz", get(|| async { "ok" }))
        .route("/favicon.svg", get(assets::favicon))
        // static & SPA assets fallback
        .fallback(assets::static_handler)
        .with_state(state)
}
