use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};

use crate::auth::MaybeUser;
use crate::error::AppResult;
use crate::models::{Job, User};
use crate::views::{DashboardPage, LoginPage};
use crate::SharedState;

pub fn render<T: Template>(tpl: &T) -> Response {
    match tpl.render() {
        Ok(body) => Html(body).into_response(),
        Err(e) => {
            tracing::error!("render template gagal: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(error_page(500, "Halaman gagal disusun. Coba muat ulang.")),
            )
                .into_response()
        }
    }
}

pub fn error_page(code: u16, message: &str) -> String {
    format!(
        r#"<!doctype html><html lang="id"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{code} · rdpinstall</title><link rel="stylesheet" href="/static/style.css"></head>
<body><main class="wrap narrow"><div class="card center">
<p class="code">{code}</p><p class="muted">{message}</p>
<p><a class="btn" href="/">kembali ke halaman depan</a></p></div></main></body></html>"#
    )
}

pub async fn login(State(state): State<SharedState>, user: MaybeUser) -> Response {
    if user.0.is_some() {
        return Redirect::to("/dashboard").into_response();
    }
    render(&LoginPage::new(&state.cfg, ""))
}

pub async fn dashboard(State(state): State<SharedState>, MaybeUser(user): MaybeUser) -> Response {
    let Some(user) = user else {
        return Redirect::to("/masuk").into_response();
    };
    match dashboard_page(&state, user).await {
        Ok(r) => r,
        Err(e) => e.into_response(),
    }
}

pub async fn dashboard_page(state: &SharedState, user: User) -> AppResult<Response> {
    let balance: i64 = sqlx::query_scalar("SELECT amount FROM balances WHERE user_id = ?")
        .bind(user.id)
        .fetch_optional(&state.pool)
        .await?
        .unwrap_or(0);

    let jobs = sqlx::query_as::<_, Job>(
        "SELECT id, user_id, mode, provider, status, phase, progress, os_label, zone, plan, host, rdp_port,
                rdp_user, provider_ref, error, created_at, updated_at, done_at, expires_at, wiped,
                ssh_port, ssh_user, guest, voucher_code, edition, lang
         FROM jobs WHERE user_id = ? AND guest = 0 ORDER BY id DESC LIMIT 10",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;

    let is_admin = user
        .tg_id
        .map(|id| state.cfg.admin_tg_ids.contains(&id))
        .unwrap_or(false);

    Ok(render(&DashboardPage {
        user,
        balance,
        jobs,
        price: state.cfg.price_per_install,
        is_admin,
    }))
}

pub async fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Html(error_page(404, "Halaman ini tidak ada.")),
    )
        .into_response()
}
