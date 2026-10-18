//! Halaman admin: mencetak dan mencabut voucer.
//!
//! Admin ditentukan oleh daftar id Telegram di konfigurasi. Halaman ini tidak
//! disembunyikan dari rute, tapi setiap handler memeriksa ulang id-nya.

use axum::extract::{Form, Path, State};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::auth::{CurrentUser, MaybeUser};
use crate::db::now;
use crate::error::{AppError, AppResult};
use crate::models::{User, Voucher};
use crate::routes::pages::render;
use crate::views::AdminVouchersPage;
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

async fn page(state: &SharedState, user: User, notice: &str) -> Response {
    let rows = vouchers(state).await.unwrap_or_default();
    let free = rows.iter().filter(|v| v.is_free()).count() as i64;
    render(&AdminVouchersPage {
        user,
        vouchers: rows,
        free,
        notice: notice.to_string(),
    })
}

pub async fn index(State(state): State<SharedState>, user: MaybeUser) -> Response {
    let MaybeUser(Some(user)) = user else {
        return Redirect::to("/masuk").into_response();
    };
    if !is_admin(&state, &user) {
        return AppError::NotFound.into_response();
    }
    page(&state, user, "").await
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
        // Kode dibaca manusia: tiga kelompok huruf/angka, tanpa karakter mirip.
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
    let notice = if made.is_empty() {
        "Tidak ada voucer baru yang dibuat.".to_string()
    } else {
        format!("{} voucer baru: {}", made.len(), made.join(", "))
    };
    page(&state, user, &notice).await
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
