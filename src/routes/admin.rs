//! Halaman admin: mencetak dan mencabut voucer.
//!
//! Admin ditentukan oleh daftar id Telegram di konfigurasi. Halaman ini tidak
//! disembunyikan dari rute, tapi setiap handler memeriksa ulang id-nya.

use axum::extract::{Form, Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Json, Redirect, Response};
use serde::Deserialize;
use serde_json::json;

use crate::auth::{CurrentUser, MaybeUser};
use crate::db::now;
use crate::error::{AppError, AppResult};
use crate::models::{User, Voucher};
use crate::routes::pages::render_spa;
use crate::SharedState;

#[derive(Debug, Deserialize)]
pub struct CreateForm {
    #[serde(default)]
    pub count: String,
    #[serde(default)]
    pub note: String,
}

fn is_admin(state: &SharedState, user: &User) -> bool {
    match (user.tg_id, state.cfg.admin_tg_ids.is_empty()) {
        (Some(id), false) => state.cfg.admin_tg_ids.contains(&id),
        (_, true) => false,
        _ => false,
    }
}

async fn vouchers(state: &SharedState) -> AppResult<Vec<Voucher>> {
    Ok(sqlx::query_as::<_, Voucher>(
        "SELECT code, created_at, used_at, used_job_id, used_host, revoked, note
         FROM vouchers ORDER BY created_at DESC LIMIT 200",
    )
    .fetch_all(&state.pool)
    .await?)
}

pub async fn index(
    State(state): State<SharedState>,
    user: MaybeUser,
    headers: axum::http::HeaderMap,
) -> Response {
    let MaybeUser(Some(user)) = user else {
        return Redirect::to("/masuk").into_response();
    };
    if !is_admin(&state, &user) {
        return AppError::NotFound.into_response();
    }

    let wants_json = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("application/json"))
        .unwrap_or(false);

    if wants_json {
        let rows = vouchers(&state).await.unwrap_or_default();
        let free = rows.iter().filter(|v| v.is_free()).count() as i64;
        Json(json!({
            "vouchers": rows,
            "free": free,
            "is_admin": true,
        }))
        .into_response()
    } else {
        render_spa().await
    }
}

pub async fn create(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Form(form): Form<CreateForm>,
) -> Response {
    if !is_admin(&state, &user) {
        return AppError::NotFound.into_response();
    }
    let count: i64 = form.count.trim().parse().unwrap_or(1).clamp(1, 100);
    let ts = now();
    let mut made = Vec::new();
    for _ in 0..count {
        let code = format!(
            "RDP-{}-{}",
            crate::crypto::random_code(4),
            crate::crypto::random_code(4)
        );
        let res = sqlx::query(
            "INSERT OR IGNORE INTO vouchers(code, created_by, created_at, note) VALUES(?,?,?,?)",
        )
        .bind(&code)
        .bind(user.tg_id.unwrap_or(0))
        .bind(ts)
        .bind(form.note.trim())
        .execute(&state.pool)
        .await;
        if matches!(res, Ok(r) if r.rows_affected() == 1) {
            made.push(code);
        }
    }
    Redirect::to("/admin/vouchers").into_response()
}

pub async fn revoke(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(code): Path<String>,
) -> Response {
    if !is_admin(&state, &user) {
        return AppError::NotFound.into_response();
    }
    let _ = sqlx::query("UPDATE vouchers SET revoked = 1 WHERE code = ? AND used_at = 0")
        .bind(&code)
        .execute(&state.pool)
        .await;
    Redirect::to("/admin/vouchers").into_response()
}
