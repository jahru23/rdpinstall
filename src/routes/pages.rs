use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};

use crate::auth::MaybeUser;
use crate::error::AppResult;
use crate::models::{CloudAccount, Job, User};
use crate::views::{DashboardPage, LoginPage};
use crate::SharedState;

pub async fn home(State(state): State<SharedState>, MaybeUser(user): MaybeUser) -> Response {
    match user {
        Some(u) => match dashboard(&state, u).await {
            Ok(r) => r,
            Err(e) => e.into_response(),
        },
        None => Redirect::to("/login").into_response(),
    }
}

pub async fn login(State(state): State<SharedState>, MaybeUser(user): MaybeUser) -> Response {
    if user.is_some() {
        return Redirect::to("/").into_response();
    }
    let page = LoginPage::new(&state.cfg, "");
    render(&page)
}

pub async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, Html(NOT_FOUND.to_string())).into_response()
}

const NOT_FOUND: &str = r#"<!doctype html><html lang="id"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><title>404</title>
<link rel="stylesheet" href="/static/style.css"></head><body class="err">
<main class="wrap"><div class="card"><p class="err-code">404</p>
<p class="err-msg">Halaman tidak ada.</p>
<p><a class="btn ghost" href="/">kembali</a></p></div></main></body></html>"#;

pub async fn dashboard(state: &SharedState, user: User) -> AppResult<Response> {
    let balance: i64 = sqlx::query_scalar("SELECT amount FROM balances WHERE user_id = ?")
        .bind(user.id)
        .fetch_optional(&state.pool)
        .await?
        .unwrap_or(0);

    let jobs = sqlx::query_as::<_, Job>(
        "SELECT id, user_id, mode, provider, status, phase, progress, os_label, zone, plan,
                host, rdp_port, rdp_user, error, created_at, updated_at, done_at, expires_at, wiped
         FROM jobs WHERE user_id = ? ORDER BY id DESC LIMIT 10",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;

    let cloud_accounts = sqlx::query_as::<_, CloudAccount>(
        "SELECT id, user_id, provider, label, status, credit, last_checked, last_error, created_at
         FROM cloud_accounts WHERE user_id = ? ORDER BY id DESC",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;

    let page = DashboardPage {
        user,
        balance,
        jobs,
        cloud_accounts,
        price: state.cfg.price_per_install,
    };
    Ok(render(&page))
}

pub fn render<T: Template>(tpl: &T) -> Response {
    match tpl.render() {
        Ok(body) => Html(body).into_response(),
        Err(e) => {
            tracing::error!("render template gagal: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "template error").into_response()
        }
    }
}
