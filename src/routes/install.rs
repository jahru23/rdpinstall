//! Jalur berakun: wizard instalasi mode SSH, memakai saldo.
//!
//! Alurnya sama dengan jalur tamu, bedanya di sini ada saldo, riwayat, dan
//! kredensial yang bisa dilihat lagi selama masa simpan belum habis.

use axum::extract::{Form, Path, State};
use axum::http::header;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Json, Redirect, Response};
use serde::Deserialize;
use serde_json::json;
use tokio_stream::wrappers::ReceiverStream;

use crate::auth::{CurrentUser, MaybeUser};
use crate::db::now;
use crate::error::{AppError, AppResult};
use crate::jobs;
use crate::models::Job;
use crate::reinstall::Lang;
use crate::routes::pages::render_spa;
use crate::validate;
use crate::SharedState;

const MAX_ACTIVE_JOBS: i64 = 2;

#[derive(Debug, Deserialize)]
pub struct InstallForm {
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

async fn balance_of(state: &SharedState, user_id: i64) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT amount FROM balances WHERE user_id = ?")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(0)
}

pub async fn wizard(_state: State<SharedState>, user: MaybeUser) -> Response {
    let MaybeUser(Some(_user)) = user else {
        return Redirect::to("/masuk").into_response();
    };
    render_spa().await
}

pub async fn create(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    headers: axum::http::HeaderMap,
    Form(f): Form<InstallForm>,
) -> Response {
    let balance = balance_of(&state, user.id).await;
    let price = state.cfg.price_per_install;

    let active: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM jobs WHERE user_id = ? AND status IN ('queued','running')",
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);
    if active >= MAX_ACTIVE_JOBS {
        return AppError::bad("Masih ada instalasi berjalan. Tunggu sampai selesai.")
            .into_response();
    }
    if balance < price {
        return AppError::bad("Saldo tidak cukup. Topup dulu.").into_response();
    }

    let host = match validate::host(&f.host) {
        Ok(h) => h,
        Err(e) => return e.into_response(),
    };
    let ssh_user = match validate::ssh_user(&f.ssh_user) {
        Ok(u) => u,
        Err(e) => return e.into_response(),
    };
    let rdp_user = match validate::rdp_user(&f.rdp_user) {
        Ok(u) => u,
        Err(e) => return e.into_response(),
    };
    if f.ssh_pass.is_empty() {
        return AppError::bad("Password SSH wajib diisi.").into_response();
    }
    if let Err(e) = validate::windows_password(&f.rdp_pass) {
        return e.into_response();
    }

    let ssh_port = validate::port(&f.ssh_port, 22);
    let rdp_port = validate::port(&f.rdp_port, 3389);
    let edition = validate::edition(&f.edition);
    let lang = Lang::from_code(&f.lang).unwrap_or(Lang::En);

    let ssh_enc = match crate::crypto::seal(&state.cfg.master_key, &f.ssh_pass) {
        Ok(v) => v,
        Err(e) => return AppError::Any(e).into_response(),
    };
    let rdp_enc = match crate::crypto::seal(&state.cfg.master_key, &f.rdp_pass) {
        Ok(v) => v,
        Err(e) => return AppError::Any(e).into_response(),
    };

    let ts = now();
    let mut tx = match state.pool.begin().await {
        Ok(t) => t,
        Err(e) => return AppError::Db(e).into_response(),
    };

    let charged = sqlx::query(
        "UPDATE balances SET amount = amount - ?, updated_at = ? WHERE user_id = ? AND amount >= ?",
    )
    .bind(price)
    .bind(ts)
    .bind(user.id)
    .bind(price)
    .execute(&mut *tx)
    .await;
    match charged {
        Ok(r) if r.rows_affected() == 1 => {}
        Ok(_) => {
            let _ = tx.rollback().await;
            return AppError::bad("Saldo berubah saat proses. Coba lagi.").into_response();
        }
        Err(e) => return AppError::Db(e).into_response(),
    }

    let inserted = sqlx::query(
        "INSERT INTO jobs(user_id, mode, provider, status, phase, progress, os_label, zone, plan,
                          host, rdp_port, rdp_user, rdp_pass_enc, ssh_port, ssh_user, ssh_pass_enc,
                          guest, edition, lang, created_at, updated_at)
         VALUES(?, 'reinstall', 'ssh', 'queued', 'menunggu', 5, ?, '', '', ?, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?, ?)",
    )
    .bind(user.id)
    .bind(edition.label())
    .bind(&host)
    .bind(rdp_port)
    .bind(&rdp_user)
    .bind(&rdp_enc)
    .bind(ssh_port)
    .bind(&ssh_user)
    .bind(&ssh_enc)
    .bind(edition.key())
    .bind(lang.code())
    .bind(ts)
    .bind(ts)
    .execute(&mut *tx)
    .await;

    let job_id = match inserted {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            let _ = tx.rollback().await;
            return AppError::Db(e).into_response();
        }
    };

    let ledger = sqlx::query(
        "INSERT INTO ledger(user_id, delta, kind, ref, note, created_at) VALUES(?,?,'charge',?,?,?)",
    )
    .bind(user.id)
    .bind(-price)
    .bind(job_id.to_string())
    .bind(format!("install {} di {}", edition.label(), host))
    .bind(ts)
    .execute(&mut *tx)
    .await;
    if let Err(e) = ledger {
        let _ = tx.rollback().await;
        return AppError::Db(e).into_response();
    }
    if let Err(e) = tx.commit().await {
        return AppError::Db(e).into_response();
    }

    tracing::info!("job {job_id} dibuat user {} untuk {}", user.id, host);
    jobs::spawn(state.clone(), job_id);

    let wants_json = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("application/json"))
        .unwrap_or(false);

    if wants_json {
        Json(json!({ "job_id": job_id, "redirect": format!("/j/{job_id}") })).into_response()
    } else {
        Redirect::to(&format!("/j/{job_id}")).into_response()
    }
}

pub async fn detail(
    State(state): State<SharedState>,
    user: MaybeUser,
    Path(id): Path<i64>,
    headers: axum::http::HeaderMap,
) -> Response {
    let MaybeUser(Some(user)) = user else {
        return Redirect::to("/masuk").into_response();
    };

    let wants_json = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("application/json"))
        .unwrap_or(false);

    if wants_json {
        match build_card_data(&state, user.id, id).await {
            Ok(Some(data)) => Json(data).into_response(),
            Ok(None) => AppError::NotFound.into_response(),
            Err(e) => e.into_response(),
        }
    } else {
        render_spa().await
    }
}

pub async fn events(
    State(state): State<SharedState>,
    user: MaybeUser,
    Path(id): Path<i64>,
) -> Response {
    let MaybeUser(Some(user)) = user else {
        return AppError::Unauthorized.into_response();
    };
    if build_card_data(&state, user.id, id)
        .await
        .ok()
        .flatten()
        .is_none()
    {
        return AppError::NotFound.into_response();
    }
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(8);
    let state2 = state.clone();
    let user_id = user.id;
    tokio::spawn(async move {
        let mut last = String::new();
        let started = std::time::Instant::now();
        loop {
            if let Ok(Some(data)) = build_card_data(&state2, user_id, id).await {
                if let Ok(json_str) = serde_json::to_string(&data) {
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

/// Tombol pemulihan: kembalikan entri boot Linux pada mesin yang gagal.
pub async fn reset(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> Response {
    let owner: Option<i64> = sqlx::query_scalar("SELECT user_id FROM jobs WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None);
    if owner != Some(user.id) {
        return AppError::NotFound.into_response();
    }
    match jobs::reset_boot(&state, id).await {
        Ok(out) => {
            let _ = jobs::event(&state, id, "boot linux dikembalikan", &out).await;
            Redirect::to(&format!("/j/{id}")).into_response()
        }
        Err(e) => AppError::bad(format!("Tidak bisa mengembalikan boot: {e}")).into_response(),
    }
}

pub async fn wipe(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> Response {
    let _ = sqlx::query(
        "UPDATE jobs SET rdp_pass_enc='', ssh_pass_enc='', wiped=1, updated_at=?
         WHERE id = ? AND user_id = ?",
    )
    .bind(now())
    .bind(id)
    .bind(user.id)
    .execute(&state.pool)
    .await;
    Redirect::to(&format!("/j/{id}")).into_response()
}

pub async fn build_card_data(
    state: &SharedState,
    user_id: i64,
    job_id: i64,
) -> AppResult<Option<serde_json::Value>> {
    let job = sqlx::query_as::<_, Job>(
        "SELECT id, user_id, mode, provider, status, phase, progress, os_label, zone, plan, host, rdp_port,
                rdp_user, provider_ref, error, created_at, updated_at, done_at, expires_at, wiped,
                ssh_port, ssh_user, guest, voucher_code, edition, lang
         FROM jobs WHERE id = ? AND user_id = ? AND guest = 0",
    )
    .bind(job_id)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?;
    let Some(job) = job else { return Ok(None) };

    let password = if job.wiped == 0 && job.status == "done" {
        sqlx::query_scalar::<_, String>("SELECT rdp_pass_enc FROM jobs WHERE id = ?")
            .bind(job_id)
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
    .bind(job_id)
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
