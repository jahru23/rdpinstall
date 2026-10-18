//! Instalasi tamu: tanpa login, pakai kode voucer sekali pakai.
//!
//! Halaman progres diakses lewat token acak milik job, bukan lewat sesi, karena
//! tamu tidak punya akun. Voucer diklaim dengan UPDATE bersyarat supaya satu
//! kode tidak bisa dipakai dua kali walau dua orang menekan bersamaan.

use axum::extract::{Form, Path, State};
use axum::http::header;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Json, Redirect, Response};
use serde::Deserialize;
use serde_json::json;
use tokio_stream::wrappers::ReceiverStream;

use crate::db::now;
use crate::error::{AppError, AppResult};
use crate::jobs;
use crate::models::{Job, User};
use crate::reinstall::Lang;
use crate::routes::pages::render_spa;
use crate::validate;
use crate::SharedState;

#[derive(Debug, Deserialize)]
pub struct GuestForm {
    #[serde(default)]
    pub voucher: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub ssh_port: String,
    #[serde(default)]
    pub ssh_user: String,
    #[serde(default)]
    pub ssh_pass: String,
    #[serde(default)]
    pub edition: String,
    #[serde(default)]
    pub lang: String,
    #[serde(default)]
    pub rdp_user: String,
    #[serde(default)]
    pub rdp_pass: String,
    #[serde(default)]
    pub rdp_port: String,
}

pub async fn index() -> Response {
    render_spa().await
}

pub async fn create(
    State(state): State<SharedState>,
    headers: axum::http::HeaderMap,
    Form(f): Form<GuestForm>,
) -> Response {
    let code = f.voucher.trim().to_uppercase();
    if code.is_empty() {
        return AppError::bad("Kode voucer wajib diisi.").into_response();
    }

    // 1. Klaim voucer lebih dulu: kalau gagal di sini, tidak ada yang berubah.
    let claim = sqlx::query(
        "UPDATE vouchers SET used_at = ? WHERE code = ? AND used_at = 0 AND revoked = 0",
    )
    .bind(now())
    .bind(&code)
    .execute(&state.pool)
    .await;

    match claim {
        Ok(r) if r.rows_affected() == 1 => {}
        Ok(_) => {
            return AppError::bad("Kode voucer tidak dikenal, sudah dipakai, atau dicabut.")
                .into_response();
        }
        Err(e) => return AppError::Db(e).into_response(),
    }

    // 2. Siapkan job. Kalau gagal, voucer dikembalikan supaya tidak hangus.
    match prepare_job(&state, &f, &code).await {
        Ok(token) => {
            let wants_json = headers
                .get(header::ACCEPT)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.contains("application/json"))
                .unwrap_or(false);

            if wants_json {
                Json(json!({ "token": token, "redirect": format!("/g/{token}") })).into_response()
            } else {
                Redirect::to(&format!("/g/{token}")).into_response()
            }
        }
        Err(e) => {
            let _ =
                sqlx::query("UPDATE vouchers SET used_at = 0 WHERE code = ? AND used_job_id = 0")
                    .bind(&code)
                    .execute(&state.pool)
                    .await;
            e.into_response()
        }
    }
}

async fn prepare_job(state: &SharedState, f: &GuestForm, code: &str) -> AppResult<String> {
    let host = validate::host(&f.host)?;
    let ssh_port = validate::port(&f.ssh_port, 22);
    let ssh_user = validate::ssh_user(&f.ssh_user)?;
    if f.ssh_pass.is_empty() {
        return Err(AppError::bad("Password SSH wajib diisi."));
    }
    let edition = validate::edition(&f.edition);
    let lang = Lang::from_code(&f.lang).unwrap_or(Lang::En);
    let rdp_user = validate::rdp_user(&f.rdp_user)?;
    let rdp_pass = f.rdp_pass.clone();
    validate::windows_password(&rdp_pass)?;
    let rdp_port = validate::port(&f.rdp_port, 3389);

    let token = crate::crypto::random_token(24);
    let ts = now();
    let ssh_enc = crate::crypto::seal(&state.cfg.master_key, &f.ssh_pass).map_err(AppError::Any)?;
    let rdp_enc = crate::crypto::seal(&state.cfg.master_key, &rdp_pass).map_err(AppError::Any)?;

    let ins = sqlx::query(
        "INSERT INTO jobs(user_id, mode, provider, status, phase, progress, os_label, zone, plan,
                          host, rdp_port, rdp_user, rdp_pass_enc, ssh_port, ssh_user, ssh_pass_enc,
                          access_token, guest, voucher_code, edition, lang, created_at, updated_at)
         VALUES(0, 'reinstall', 'ssh', 'queued', 'menunggu', 5, ?, '', '', ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?, ?, ?)",
    )
    .bind(edition.label())
    .bind(&host)
    .bind(rdp_port)
    .bind(&rdp_user)
    .bind(&rdp_enc)
    .bind(ssh_port)
    .bind(&ssh_user)
    .bind(&ssh_enc)
    .bind(&token)
    .bind(code)
    .bind(edition.key())
    .bind(lang.code())
    .bind(ts)
    .bind(ts)
    .execute(&state.pool)
    .await?;
    let job_id = ins.last_insert_rowid();

    sqlx::query("UPDATE vouchers SET used_job_id = ?, used_host = ? WHERE code = ?")
        .bind(job_id)
        .bind(&host)
        .bind(code)
        .execute(&state.pool)
        .await?;

    tracing::info!("job tamu {job_id} dibuat untuk {host} (voucer {code})");
    jobs::spawn(state.clone(), job_id);
    Ok(token)
}

pub async fn progress(
    State(state): State<SharedState>,
    Path(token): Path<String>,
    headers: axum::http::HeaderMap,
) -> Response {
    let wants_json = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("application/json"))
        .unwrap_or(false);

    if wants_json {
        match build_card_data(&state, &token).await {
            Ok(Some(data)) => Json(data).into_response(),
            Ok(None) => AppError::NotFound.into_response(),
            Err(e) => e.into_response(),
        }
    } else {
        render_spa().await
    }
}

pub async fn events(State(state): State<SharedState>, Path(token): Path<String>) -> Response {
    if build_card_data(&state, &token)
        .await
        .ok()
        .flatten()
        .is_none()
    {
        return AppError::NotFound.into_response();
    }
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(8);
    let state2 = state.clone();
    tokio::spawn(async move {
        let mut last = String::new();
        let started = std::time::Instant::now();
        loop {
            if let Ok(Some(card)) = build_card_data(&state2, &token).await {
                if let Ok(json_str) = serde_json::to_string(&card) {
                    if json_str != last {
                        last = json_str.clone();
                        if tx
                            .send(Ok(Event::default().event("card").data(json_str)))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
            if started.elapsed().as_secs() > 30
                && (last.contains("\"done\"") || last.contains("\"failed\""))
            {
                break;
            }
            if started.elapsed().as_secs() > 7200 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    });
    Sse::new(ReceiverStream::new(rx)).into_response()
}

pub async fn build_card_data(
    state: &SharedState,
    token: &str,
) -> AppResult<Option<serde_json::Value>> {
    let job = sqlx::query_as::<_, Job>(
        "SELECT id, user_id, mode, provider, status, phase, progress, os_label, zone, plan,
                host, rdp_port, rdp_user, provider_ref, error, created_at, updated_at, done_at,
                expires_at, wiped, ssh_port, ssh_user, guest, voucher_code, edition, lang
         FROM jobs WHERE access_token = ? AND guest = 1",
    )
    .bind(token)
    .fetch_optional(&state.pool)
    .await?;

    let Some(job) = job else { return Ok(None) };

    let password = if job.wiped == 0 && job.status == "done" {
        sqlx::query_scalar::<_, String>("SELECT rdp_pass_enc FROM jobs WHERE id = ?")
            .bind(job.id)
            .fetch_one(&state.pool)
            .await
            .ok()
            .filter(|e| !e.is_empty())
            .and_then(|e| crate::crypto::open(&state.cfg.master_key, &e).ok())
    } else {
        None
    };

    let events = sqlx::query_as::<_, crate::models::JobEvent>(
        "SELECT ts, phase, message FROM job_events WHERE job_id = ? ORDER BY id DESC LIMIT 8",
    )
    .bind(job.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let rail = crate::views::rail_for(&job);

    Ok(Some(json!({
        "id": job.id,
        "status": job.status,
        "phase": job.phase,
        "progress": job.progress,
        "os_label": job.os_label,
        "host": job.host,
        "rdp_port": job.rdp_port,
        "rdp_user": job.rdp_user,
        "ssh_port": job.ssh_port,
        "ssh_user": job.ssh_user,
        "guest": job.guest,
        "voucher_code": job.voucher_code,
        "edition": job.edition,
        "lang": job.lang,
        "error": job.error,
        "created_at": job.created_at,
        "done_at": job.done_at,
        "expires_at": job.expires_at,
        "wiped": job.wiped,
        "password": password,
        "events": events,
        "rail": rail,
    })))
}

pub fn guest_user() -> User {
    User {
        id: 0,
        tg_id: None,
        username: String::new(),
        first_name: "Tamu".into(),
        last_name: String::new(),
        photo_url: String::new(),
        banned: 0,
        created_at: 0,
        last_seen_at: 0,
    }
}

#[derive(Debug, Deserialize)]
pub struct VoucherQuery {
    #[serde(default)]
    pub code: String,
}

pub async fn validate_voucher(
    State(state): State<SharedState>,
    axum::extract::Query(q): axum::extract::Query<VoucherQuery>,
) -> Response {
    let code = q.code.trim().to_uppercase();
    if code.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({ "valid": false, "message": "Kode voucer wajib diisi." })),
        )
            .into_response();
    }

    let row = sqlx::query_as::<_, crate::models::Voucher>(
        "SELECT code, created_at, used_at, used_job_id, used_host, revoked, note
         FROM vouchers WHERE code = ?",
    )
    .bind(&code)
    .fetch_optional(&state.pool)
    .await;

    match row {
        Ok(Some(v)) => {
            if v.revoked != 0 {
                Json(json!({
                    "valid": false,
                    "code": code,
                    "message": "Kode voucer ini telah dicabut."
                }))
                .into_response()
            } else if v.used_at != 0 {
                Json(json!({
                    "valid": false,
                    "code": code,
                    "message": "Kode voucer ini sudah pernah digunakan."
                }))
                .into_response()
            } else {
                Json(json!({
                    "valid": true,
                    "code": code,
                    "message": "Kode voucer valid dan siap digunakan.",
                    "note": v.note
                }))
                .into_response()
            }
        }
        Ok(None) => Json(json!({
            "valid": false,
            "code": code,
            "message": "Kode voucer tidak ditemukan di sistem."
        }))
        .into_response(),
        Err(e) => AppError::Db(e).into_response(),
    }
}

pub async fn validate_voucher_post(
    State(state): State<SharedState>,
    Form(f): Form<VoucherQuery>,
) -> Response {
    validate_voucher(State(state), axum::extract::Query(f)).await
}
