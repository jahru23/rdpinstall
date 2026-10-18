//! Menu Admin: Pengelolaan seluruh platform rdpinstall.
//!
//! Mencakup ringkasan statistik sistem, monitoring dan aksi instalasi (jobs),
//! manajemen pengguna (saldo & blokir), verifikasi transaksi pembayaran (QRIS),
//! serta pencetakan dan pencabutan voucer.

use axum::extract::{Form, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Json, Redirect, Response};
use serde::Deserialize;
use serde_json::json;

use crate::auth::{CurrentUser, MaybeUser};
use crate::db::now;
use crate::error::AppError;
use crate::models::{AdminPaymentRow, AdminUserRow, Job, User, Voucher};
use crate::routes::pages::render_spa;
use crate::SharedState;

#[derive(Debug, Deserialize)]
pub struct CreateVoucherForm {
    #[serde(default)]
    pub count: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Deserialize)]
pub struct SetBalanceForm {
    #[serde(default)]
    pub amount: i64,
    #[serde(default)]
    pub mode: String, // "set" or "add"
    #[serde(default)]
    pub note: String,
}

pub fn is_admin(state: &SharedState, user: &User) -> bool {
    match (user.tg_id, state.cfg.admin_tg_ids.is_empty()) {
        (Some(id), false) => state.cfg.admin_tg_ids.contains(&id),
        (_, true) => false,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// 1. Overview & Halaman Utama Admin
// ---------------------------------------------------------------------------

pub async fn index(
    State(state): State<SharedState>,
    user: MaybeUser,
    headers: axum::http::HeaderMap,
) -> Response {
    let MaybeUser(Some(user)) = user else {
        return Redirect::to("/masuk").into_response();
    };
    if !is_admin(&state, &user) {
        return (StatusCode::FORBIDDEN, "Akses ditolak: Bukan admin.").into_response();
    }

    let wants_json = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("application/json"))
        .unwrap_or(false);

    if wants_json {
        overview(State(state), CurrentUser(user)).await
    } else {
        render_spa().await
    }
}

pub async fn overview(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
) -> Response {
    if !is_admin(&state, &user) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "forbidden"}))).into_response();
    }

    let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id > 0")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);

    let total_jobs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);

    let active_jobs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE status IN ('queued', 'running')")
            .fetch_one(&state.pool)
            .await
            .unwrap_or(0);

    let done_jobs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE status = 'done'")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);

    let failed_jobs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE status IN ('failed', 'cancelled')")
            .fetch_one(&state.pool)
            .await
            .unwrap_or(0);

    let total_vouchers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vouchers")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);

    let free_vouchers: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM vouchers WHERE used_at = 0 AND revoked = 0")
            .fetch_one(&state.pool)
            .await
            .unwrap_or(0);

    let total_revenue: i64 =
        sqlx::query_scalar("SELECT COALESCE(SUM(amount), 0) FROM payments WHERE status = 'paid'")
            .fetch_one(&state.pool)
            .await
            .unwrap_or(0);

    let total_balance: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(amount), 0) FROM balances")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);

    let vouchers: Vec<Voucher> = sqlx::query_as::<_, Voucher>(
        "SELECT code, created_at, used_at, used_job_id, used_host, revoked, note
         FROM vouchers ORDER BY created_at DESC LIMIT 150",
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(json!({
        "is_admin": true,
        "stats": {
            "total_users": total_users,
            "total_jobs": total_jobs,
            "active_jobs": active_jobs,
            "done_jobs": done_jobs,
            "failed_jobs": failed_jobs,
            "total_vouchers": total_vouchers,
            "free_vouchers": free_vouchers,
            "total_revenue": total_revenue,
            "total_balance": total_balance,
            "price_per_install": state.cfg.price_per_install,
        },
        "vouchers": vouchers,
    }))
    .into_response()
}

// ---------------------------------------------------------------------------
// 2. Kelola Pekerjaan / Instalasi (Jobs)
// ---------------------------------------------------------------------------

pub async fn jobs(State(state): State<SharedState>, CurrentUser(user): CurrentUser) -> Response {
    if !is_admin(&state, &user) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "forbidden"}))).into_response();
    }

    let rows: Vec<Job> = sqlx::query_as::<_, Job>(
        "SELECT id, user_id, mode, provider, status, phase, progress, os_label, zone, plan, host,
                rdp_port, rdp_user, provider_ref, error, created_at, updated_at, done_at, expires_at,
                wiped, ssh_port, ssh_user, guest, voucher_code, edition, lang
         FROM jobs ORDER BY id DESC LIMIT 100",
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(json!({ "jobs": rows })).into_response()
}

pub async fn cancel_job(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> Response {
    if !is_admin(&state, &user) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "forbidden"}))).into_response();
    }

    let ts = now();
    let res = sqlx::query(
        "UPDATE jobs SET status = 'failed', phase = 'Dibatalkan Admin', error = 'Dibatalkan oleh administrator.', updated_at = ?
         WHERE id = ? AND status IN ('queued', 'running')",
    )
    .bind(ts)
    .bind(id)
    .execute(&state.pool)
    .await;

    match res {
        Ok(r) if r.rows_affected() > 0 => {
            let _ = sqlx::query(
                "INSERT INTO job_events(job_id, ts, phase, message) VALUES(?, ?, 'Dibatalkan', 'Instalasi dihentikan paksa oleh administrator')",
            )
            .bind(id)
            .bind(ts)
            .execute(&state.pool)
            .await;
            Json(json!({ "success": true, "message": "Instalasi berhasil dibatalkan." }))
                .into_response()
        }
        _ => Json(json!({ "success": false, "message": "Job tidak aktif atau tidak ditemukan." }))
            .into_response(),
    }
}

pub async fn delete_job(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> Response {
    if !is_admin(&state, &user) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "forbidden"}))).into_response();
    }

    let _ = sqlx::query("DELETE FROM job_events WHERE job_id = ?")
        .bind(id)
        .execute(&state.pool)
        .await;

    let res = sqlx::query("DELETE FROM jobs WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await;

    match res {
        Ok(r) if r.rows_affected() > 0 => {
            Json(json!({ "success": true, "message": "Job berhasil dihapus." })).into_response()
        }
        _ => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Job tidak ditemukan." })),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// 3. Kelola Pengguna (Users)
// ---------------------------------------------------------------------------

pub async fn users(State(state): State<SharedState>, CurrentUser(user): CurrentUser) -> Response {
    if !is_admin(&state, &user) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "forbidden"}))).into_response();
    }

    let rows: Vec<AdminUserRow> = sqlx::query_as::<_, AdminUserRow>(
        "SELECT u.id, u.tg_id, u.username, u.first_name, u.last_name, u.photo_url, u.banned,
                u.created_at, u.last_seen_at,
                COALESCE(b.amount, 0) as balance,
                (SELECT COUNT(*) FROM jobs WHERE user_id = u.id) as job_count
         FROM users u
         LEFT JOIN balances b ON b.user_id = u.id
         WHERE u.id > 0
         ORDER BY u.id DESC LIMIT 100",
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(json!({ "users": rows })).into_response()
}

pub async fn set_balance(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Form(f): Form<SetBalanceForm>,
) -> Response {
    if !is_admin(&state, &user) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "forbidden"}))).into_response();
    }

    let target_user: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None);

    let Some(target_id) = target_user else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "User tidak ditemukan" })),
        )
            .into_response();
    };

    let ts = now();
    let delta = if f.mode == "add" {
        f.amount
    } else {
        // Mode set saldo ke angka pasti
        let current_balance: i64 =
            sqlx::query_scalar("SELECT amount FROM balances WHERE user_id = ?")
                .bind(target_id)
                .fetch_optional(&state.pool)
                .await
                .ok()
                .flatten()
                .unwrap_or(0);
        f.amount - current_balance
    };

    let mut tx = match state.pool.begin().await {
        Ok(t) => t,
        Err(e) => return AppError::Db(e).into_response(),
    };

    let up = sqlx::query(
        "INSERT INTO balances(user_id, amount, updated_at) VALUES(?, ?, ?)
         ON CONFLICT(user_id) DO UPDATE SET amount = amount + excluded.amount, updated_at = excluded.updated_at",
    )
    .bind(target_id)
    .bind(delta)
    .bind(ts)
    .execute(&mut *tx)
    .await;

    if let Err(e) = up {
        let _ = tx.rollback().await;
        return AppError::Db(e).into_response();
    }

    let note = if f.note.trim().is_empty() {
        format!("Penyesuaian saldo manual oleh admin (user #{})", user.id)
    } else {
        format!("Admin: {}", f.note.trim())
    };

    let _ = sqlx::query(
        "INSERT INTO ledger(user_id, delta, kind, ref, note, created_at) VALUES(?, ?, 'admin_adj', ?, ?, ?)",
    )
    .bind(target_id)
    .bind(delta)
    .bind(user.id.to_string())
    .bind(note)
    .bind(ts)
    .execute(&mut *tx)
    .await;

    if let Err(e) = tx.commit().await {
        return AppError::Db(e).into_response();
    }

    Json(json!({ "success": true, "delta": delta })).into_response()
}

pub async fn toggle_ban(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> Response {
    if !is_admin(&state, &user) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "forbidden"}))).into_response();
    }

    let res = sqlx::query("UPDATE users SET banned = 1 - banned WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await;

    match res {
        Ok(r) if r.rows_affected() > 0 => {
            // Hapus sesi aktif user jika diban
            let _ = sqlx::query("DELETE FROM sessions WHERE user_id = ?")
                .bind(id)
                .execute(&state.pool)
                .await;
            Json(json!({ "success": true })).into_response()
        }
        _ => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "User tidak ditemukan" })),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// 4. Kelola Pembayaran (Payments / Topup)
// ---------------------------------------------------------------------------

pub async fn payments(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
) -> Response {
    if !is_admin(&state, &user) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "forbidden"}))).into_response();
    }

    let rows: Vec<AdminPaymentRow> = sqlx::query_as::<_, AdminPaymentRow>(
        "SELECT p.id, p.user_id, p.gateway, p.ref, p.gateway_id, p.amount, p.total_amount,
                p.status, p.qr_string, p.pay_url, p.txn_id, p.created_at, p.paid_at, p.expires_at,
                u.username as user_username, u.first_name as user_first_name
         FROM payments p
         LEFT JOIN users u ON u.id = p.user_id
         ORDER BY p.id DESC LIMIT 100",
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(json!({ "payments": rows })).into_response()
}

pub async fn approve_payment(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(reference): Path<String>,
) -> Response {
    if !is_admin(&state, &user) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "forbidden"}))).into_response();
    }

    let p: Option<crate::models::Payment> = sqlx::query_as::<_, crate::models::Payment>(
        "SELECT id, user_id, gateway, ref, gateway_id, amount, total_amount, status, qr_string,
                pay_url, txn_id, created_at, paid_at, expires_at
         FROM payments WHERE ref = ?",
    )
    .bind(&reference)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let Some(payment) = p else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Tagihan tidak ditemukan." })),
        )
            .into_response();
    };

    if payment.status == "paid" {
        return Json(
            json!({ "success": true, "message": "Tagihan ini sudah berstatus lunas sebelumnya." }),
        )
        .into_response();
    }

    let ts = now();
    let mut tx = match state.pool.begin().await {
        Ok(t) => t,
        Err(e) => return AppError::Db(e).into_response(),
    };

    let up = sqlx::query(
        "UPDATE payments SET status = 'paid', paid_at = ?, txn_id = 'MANUAL-ADMIN' WHERE id = ? AND status <> 'paid'",
    )
    .bind(ts)
    .bind(payment.id)
    .execute(&mut *tx)
    .await;

    match up {
        Ok(r) if r.rows_affected() == 1 => {}
        _ => {
            let _ = tx.rollback().await;
            return Json(
                json!({ "success": false, "message": "Gagal mengupdate status pembayaran." }),
            )
            .into_response();
        }
    }

    let cred = sqlx::query(
        "INSERT INTO balances(user_id, amount, updated_at) VALUES(?, ?, ?)
         ON CONFLICT(user_id) DO UPDATE SET amount = amount + excluded.amount, updated_at = excluded.updated_at",
    )
    .bind(payment.user_id)
    .bind(payment.amount)
    .bind(ts)
    .execute(&mut *tx)
    .await;

    if let Err(e) = cred {
        let _ = tx.rollback().await;
        return AppError::Db(e).into_response();
    }

    let _ = sqlx::query(
        "INSERT INTO ledger(user_id, delta, kind, ref, note, created_at) VALUES(?, ?, 'topup_manual', ?, ?, ?)",
    )
    .bind(payment.user_id)
    .bind(payment.amount)
    .bind(&payment.reference)
    .bind(format!("Acc manual oleh admin (user #{})", user.id))
    .bind(ts)
    .execute(&mut *tx)
    .await;

    if let Err(e) = tx.commit().await {
        return AppError::Db(e).into_response();
    }

    Json(json!({ "success": true, "message": "Tagihan berhasil disetujui dan saldo user telah bertambah." })).into_response()
}

// ---------------------------------------------------------------------------
// 5. Kelola Voucer
// ---------------------------------------------------------------------------

pub async fn create(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Form(form): Form<CreateVoucherForm>,
) -> Response {
    if !is_admin(&state, &user) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "forbidden"}))).into_response();
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
    Json(json!({ "success": true, "count": made.len(), "created": made })).into_response()
}

pub async fn revoke(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(code): Path<String>,
) -> Response {
    if !is_admin(&state, &user) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "forbidden"}))).into_response();
    }
    let res = sqlx::query("UPDATE vouchers SET revoked = 1 WHERE code = ? AND used_at = 0")
        .bind(&code)
        .execute(&state.pool)
        .await;

    match res {
        Ok(r) if r.rows_affected() > 0 => Json(json!({ "success": true })).into_response(),
        _ => Json(
            json!({ "success": false, "message": "Voucer tidak ditemukan atau sudah dipakai." }),
        )
        .into_response(),
    }
}

pub async fn delete_voucher(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(code): Path<String>,
) -> Response {
    if !is_admin(&state, &user) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "forbidden"}))).into_response();
    }
    let res = sqlx::query("DELETE FROM vouchers WHERE code = ? AND used_at = 0")
        .bind(&code)
        .execute(&state.pool)
        .await;

    match res {
        Ok(r) if r.rows_affected() > 0 => Json(json!({ "success": true })).into_response(),
        _ => Json(json!({ "success": false, "message": "Voucer yang sudah dipakai tidak dapat dihapus." })).into_response(),
    }
}
