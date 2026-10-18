use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Json, Redirect, Response};
use serde_json::json;

use crate::auth::MaybeUser;
use crate::error::AppResult;
use crate::models::{Job, User};
use crate::routes::assets::FrontendAssets;
use crate::SharedState;

pub async fn render_spa() -> Response {
    if let Some(index) = FrontendAssets::get("index.html") {
        return (
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            index.data,
        )
            .into_response();
    }
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "SPA index.html not found",
    )
        .into_response()
}

pub async fn login(_state: State<SharedState>, user: MaybeUser) -> Response {
    if user.0.is_some() {
        return Redirect::to("/dashboard").into_response();
    }
    render_spa().await
}

pub async fn dashboard(
    State(state): State<SharedState>,
    MaybeUser(user): MaybeUser,
    headers: axum::http::HeaderMap,
) -> Response {
    let Some(user) = user else {
        return Redirect::to("/masuk").into_response();
    };

    let wants_json = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("application/json"))
        .unwrap_or(false);

    if wants_json {
        match get_dashboard_data(&state, user).await {
            Ok(data) => Json(data).into_response(),
            Err(e) => e.into_response(),
        }
    } else {
        render_spa().await
    }
}

pub async fn get_dashboard_data(state: &SharedState, user: User) -> AppResult<serde_json::Value> {
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

    Ok(json!({
        "user": user,
        "balance": balance,
        "jobs": jobs,
        "price": state.cfg.price_per_install,
        "is_admin": is_admin,
    }))
}

pub async fn not_found(uri: axum::http::Uri) -> Response {
    let path = uri.path();
    if path.starts_with("/api/") || path.ends_with(".json") {
        return (StatusCode::NOT_FOUND, Json(json!({"error": "not_found"}))).into_response();
    }
    // Any browser direct path fallback to SPA
    render_spa().await
}
