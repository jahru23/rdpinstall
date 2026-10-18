//! Wizard install (mode template UpCloud) + halaman job dengan progres live.

use askama::Template;
use axum::extract::{Form, Path, State};
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use tokio_stream::wrappers::ReceiverStream;

use crate::auth::{CurrentUser, MaybeUser};
use crate::db::now;
use crate::error::{AppError, AppResult};
use crate::jobs;
use crate::models::{CloudAccount, User};
use crate::routes::pages::render;
use crate::upcloud;
use crate::views::{InstallPage, JobCard, JobPage};
use crate::SharedState;

const MAX_ACTIVE_JOBS: i64 = 2;

#[derive(Debug, Deserialize)]
pub struct InstallForm {
    #[serde(default)]
    pub account_id: i64,
    #[serde(default)]
    pub template_uuid: String,
    #[serde(default)]
    pub zone: String,
    #[serde(default)]
    pub plan: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub storage_gb: i64,
}

async fn accounts(state: &SharedState, user_id: i64) -> AppResult<Vec<CloudAccount>> {
    Ok(sqlx::query_as::<_, CloudAccount>(
        "SELECT id, user_id, provider, label, status, credit, last_checked, last_error, created_at
         FROM cloud_accounts WHERE user_id = ? ORDER BY id DESC",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?)
}

pub async fn wizard(State(state): State<SharedState>, user: MaybeUser) -> Response {
    let MaybeUser(Some(user)) = user else {
        return Redirect::to("/login").into_response();
    };
    let accounts = match accounts(&state, user.id).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    let usable: Vec<CloudAccount> = accounts.into_iter().filter(|a| a.status == "ok").collect();

    if usable.is_empty() {
        return finish_err(
            &state,
            user,
            "Belum ada akun cloud yang valid. Tambahkan API token UpCloud dulu di halaman Akun Cloud.",
        )
        .await;
    }

    // Data zone/plan/template selalu diambil saat request.
    let token = match token_of(&state, usable[0].id, user.id).await {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    let client = match upcloud::Client::new(&token) {
        Ok(c) => c,
        Err(e) => return err_page(&state, user, &e.to_string()).await,
    };
    let (zones, plans, templates) = match tokio::try_join!(
        client.public_zones(),
        client.plans(),
        client.windows_templates()
    ) {
        Ok(t) => t,
        Err(e) => return err_page(&state, user, &e.to_string()).await,
    };

    let balance = balance_of(&state, user.id).await.unwrap_or(0);
    let page = InstallPage {
        user,
        accounts: usable,
        zones,
        plans,
        templates,
        balance,
        price: state.cfg.price_per_install,
        error: String::new(),
    };
    render(&page)
}

pub async fn create(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Form(form): Form<InstallForm>,
) -> Response {
    let active: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM jobs WHERE user_id = ? AND status IN ('queued','running')",
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);
    if active >= MAX_ACTIVE_JOBS {
        return err_page(
            &state,
            user,
            "Masih ada instalasi berjalan. Tunggu selesai dulu.",
        )
        .await;
    }

    let balance = balance_of(&state, user.id).await.unwrap_or(0);
    if balance < state.cfg.price_per_install {
        return err_page(
            &state,
            user,
            &format!(
                "Saldo tidak cukup. Tarif per install Rp{}, saldo Anda Rp{}. Topup dulu.",
                state.cfg.price_per_install, balance
            ),
        )
        .await;
    }

    let token = match token_of(&state, form.account_id, user.id).await {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    let client = match upcloud::Client::new(&token) {
        Ok(c) => c,
        Err(e) => return err_page(&state, user, &e.to_string()).await,
    };

    // Semua pilihan divalidasi terhadap daftar yang baru diambil dari API,
    // supaya UUID/kode plan palsu tidak bisa disuntikkan lewat form.
    let (zones, plans, templates) = match tokio::try_join!(
        client.public_zones(),
        client.plans(),
        client.windows_templates()
    ) {
        Ok(t) => t,
        Err(e) => return err_page(&state, user, &e.to_string()).await,
    };
    let zone = match zones.iter().find(|z| z.id == form.zone) {
        Some(z) => z.clone(),
        None => return err_page(&state, user, "Zone tidak dikenal / tidak publik.").await,
    };
    let plan = match plans.iter().find(|p| p.name == form.plan) {
        Some(p) => p.clone(),
        None => return err_page(&state, user, "Plan tidak tersedia.").await,
    };
    let template = match templates.iter().find(|t| t.uuid == form.template_uuid) {
        Some(t) => t.clone(),
        None => {
            return err_page(
                &state,
                user,
                "Template Windows tidak ditemukan di akun itu.",
            )
            .await
        }
    };
    if form.storage_gb > 0 && form.storage_gb < template.size {
        return err_page(
            &state,
            user,
            &format!(
                "Ukuran disk minimal {} GB untuk template ini.",
                template.size
            ),
        )
        .await;
    }

    let title = if form.title.trim().is_empty() {
        format!("RDP {}", template.title)
    } else {
        form.title.trim().to_string()
    };
    let spec = upcloud::CreateSpec {
        zone: zone.id.clone(),
        title: title.clone(),
        hostname: upcloud::safe_hostname(&title),
        plan: plan.name.clone(),
        template_uuid: template.uuid.clone(),
        storage_title: template.title.clone(),
        storage_size_gb: form.storage_gb.max(template.size),
        storage_tier: template.tier_hint(),
    };

    let server = match client.create_server(&spec).await {
        Ok(s) => s,
        Err(e) => return err_page(&state, user, &e.to_string()).await,
    };

    // Password Windows hanya ada di respons create ini.
    let rdp_pass_enc = match server.password.as_deref() {
        Some(pw) if !pw.is_empty() => match crate::crypto::seal(&state.cfg.master_key, pw) {
            Ok(s) => s,
            Err(e) => {
                let _ = client.delete_server(&server.uuid).await;
                return AppError::Any(e).into_response();
            }
        },
        _ => String::new(),
    };

    let ts = now();
    let mut tx = match state.pool.begin().await {
        Ok(t) => t,
        Err(e) => return AppError::Db(e).into_response(),
    };
    let charged = sqlx::query(
        "UPDATE balances SET amount = amount - ?, updated_at = ? WHERE user_id = ? AND amount >= ?",
    )
    .bind(state.cfg.price_per_install)
    .bind(ts)
    .bind(user.id)
    .bind(state.cfg.price_per_install)
    .execute(&mut *tx)
    .await;

    match charged {
        Ok(r) if r.rows_affected() == 1 => {}
        Ok(_) => {
            let _ = tx.rollback().await;
            let _ = client.delete_server(&server.uuid).await;
            return err_page(
                &state,
                user,
                "Saldo berubah saat proses; server dibatalkan.",
            )
            .await;
        }
        Err(e) => return AppError::Db(e).into_response(),
    }

    let inserted = sqlx::query(
        "INSERT INTO jobs(user_id, mode, provider, status, phase, progress, os_label, zone, plan,
                          host, rdp_port, rdp_user, rdp_pass_enc, provider_ref, cloud_account_id,
                          storage_size, created_at, updated_at)
         VALUES(?, 'template', 'upcloud', 'queued', 'menunggu', 5, ?, ?, ?, ?, 3389, 'Administrator', ?, ?, ?, ?, ?, ?)",
    )
    .bind(user.id)
    .bind(&template.title)
    .bind(&zone.id)
    .bind(&plan.name)
    .bind(server.public_ipv4().unwrap_or_default())
    .bind(&rdp_pass_enc)
    .bind(&server.uuid)
    .bind(form.account_id)
    .bind(spec.storage_size_gb)
    .bind(ts)
    .bind(ts)
    .execute(&mut *tx)
    .await;

    let job_id = match inserted {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => {
            let _ = tx.rollback().await;
            let _ = client.delete_server(&server.uuid).await;
            return AppError::Db(e).into_response();
        }
    };

    let ledger = sqlx::query(
        "INSERT INTO ledger(user_id, delta, kind, ref, note, created_at) VALUES(?,?, 'charge', ?, ?, ?)",
    )
    .bind(user.id)
    .bind(-state.cfg.price_per_install)
    .bind(job_id.to_string())
    .bind(format!("install {}", template.title))
    .bind(ts)
    .execute(&mut *tx)
    .await;
    if let Err(e) = ledger {
        let _ = tx.rollback().await;
        let _ = client.delete_server(&server.uuid).await;
        return AppError::Db(e).into_response();
    }

    if let Err(e) = tx.commit().await {
        let _ = client.delete_server(&server.uuid).await;
        return AppError::Db(e).into_response();
    }

    tracing::info!(
        "job {job_id} dibuat: {} {} {} server={}",
        template.title,
        zone.id,
        plan.name,
        server.uuid
    );
    jobs::spawn(state.clone(), job_id);
    Redirect::to(&format!("/jobs/{job_id}")).into_response()
}

pub async fn card_fragment(
    state: &SharedState,
    user_id: i64,
    job_id: i64,
) -> AppResult<Option<String>> {
    let row = sqlx::query_as::<_, (i64, i64)>("SELECT user_id, id FROM jobs WHERE id = ?")
        .bind(job_id)
        .fetch_optional(&state.pool)
        .await?;
    let Some((owner, id)) = row else {
        return Ok(None);
    };
    if owner != user_id {
        return Ok(None);
    }
    let view = build_card(state, id).await?;
    Ok(Some(view.render()?))
}

async fn build_card(state: &SharedState, job_id: i64) -> AppResult<JobCard> {
    let job = sqlx::query_as::<_, crate::models::Job>(
        "SELECT id, user_id, mode, provider, status, phase, progress, os_label, zone, plan,
                host, rdp_port, rdp_user, provider_ref, error, created_at, updated_at, done_at, expires_at, wiped
         FROM jobs WHERE id = ?",
    )
    .bind(job_id)
    .fetch_one(&state.pool)
    .await?;

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

    Ok(JobCard {
        job,
        password,
        events,
    })
}

pub async fn detail(
    State(state): State<SharedState>,
    user: MaybeUser,
    Path(id): Path<i64>,
) -> Response {
    let MaybeUser(Some(user)) = user else {
        return Redirect::to("/login").into_response();
    };
    let page = match build_page(&state, user, id).await {
        Ok(Some(p)) => p,
        Ok(None) => return AppError::NotFound.into_response(),
        Err(e) => return e.into_response(),
    };
    render(&page)
}

/// SSE: kirim ulang kartu setiap kali isinya berubah.
pub async fn events(
    State(state): State<SharedState>,
    user: MaybeUser,
    Path(id): Path<i64>,
) -> Response {
    let MaybeUser(Some(user)) = user else {
        return AppError::Unauthorized.into_response();
    };
    // tolak lebih awal kalau job bukan milik user ini
    let owner: Option<i64> = match sqlx::query_scalar("SELECT user_id FROM jobs WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
    {
        Ok(o) => o,
        Err(e) => return AppError::Db(e).into_response(),
    };
    if owner != Some(user.id) {
        return AppError::NotFound.into_response();
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(8);
    let state2 = state.clone();
    tokio::spawn(async move {
        let mut last = String::new();
        let started = std::time::Instant::now();
        loop {
            match card_fragment(&state2, user.id, id).await {
                Ok(Some(html)) => {
                    let flat = html.replace('\n', "");
                    if flat != last {
                        last = flat.clone();
                        if tx
                            .send(Ok(Event::default().event("card").data(flat)))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
                _ => break,
            }
            // berhenti kalau job sudah final dan tidak berubah lagi
            if started.elapsed().as_secs() > 30
                && (last.contains("badge ok") || last.contains("badge bad"))
            {
                break;
            }
            if started.elapsed().as_secs() > 3600 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });
    Sse::new(ReceiverStream::new(rx)).into_response()
}

pub async fn destroy(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> Response {
    let row = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT provider_ref, status, cloud_account_id FROM jobs WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await;
    let (uuid, status, account_id) = match row {
        Ok(Some(r)) => r,
        Ok(None) => return AppError::NotFound.into_response(),
        Err(e) => return AppError::Db(e).into_response(),
    };
    if status == "cancelled" {
        return Redirect::to(&format!("/jobs/{id}")).into_response();
    }

    let token = match token_of(&state, account_id, user.id).await {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    if let Ok(client) = upcloud::Client::new(&token) {
        if let Err(e) = client.delete_server(&uuid).await {
            tracing::warn!("hapus server {uuid} gagal: {e}");
        }
    }
    let ts = now();
    let _ = sqlx::query(
        "UPDATE jobs SET status='cancelled', phase='server dihapus', progress=100,
                rdp_pass_enc='', wiped=1, updated_at=? WHERE id=?",
    )
    .bind(ts)
    .bind(id)
    .execute(&state.pool)
    .await;
    let _ = sqlx::query("INSERT INTO job_events(job_id, ts, phase, message) VALUES(?,?,'dibatalkan','server dihapus lewat panel')")
        .bind(id)
        .bind(ts)
        .execute(&state.pool)
        .await;
    Redirect::to(&format!("/jobs/{id}")).into_response()
}

async fn token_of(state: &SharedState, account_id: i64, user_id: i64) -> AppResult<String> {
    let enc: Option<String> =
        sqlx::query_scalar("SELECT token_enc FROM cloud_accounts WHERE id = ? AND user_id = ?")
            .bind(account_id)
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await?;
    let enc = enc.ok_or_else(|| AppError::bad("Akun cloud tidak ditemukan."))?;
    Ok(crate::crypto::open(&state.cfg.master_key, &enc)?)
}

async fn balance_of(state: &SharedState, user_id: i64) -> AppResult<i64> {
    let b: Option<i64> = sqlx::query_scalar("SELECT amount FROM balances WHERE user_id = ?")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await?;
    Ok(b.unwrap_or(0))
}

async fn build_page(state: &SharedState, user: User, id: i64) -> AppResult<Option<JobPage>> {
    let owner: Option<i64> = sqlx::query_scalar("SELECT user_id FROM jobs WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;
    if owner != Some(user.id) {
        return Ok(None);
    }
    let card = build_card(state, id).await?;
    Ok(Some(JobPage { user, card }))
}

async fn err_page(state: &SharedState, user: User, msg: &str) -> Response {
    finish_err(state, user, msg).await
}

/// Halaman wizard dengan pesan galat di atas form; pilihan tetap ditawarkan
/// supaya user bisa langsung memperbaiki tanpa kehilangan konteks.
async fn finish_err(state: &SharedState, user: User, msg: &str) -> Response {
    let accounts = accounts(state, user.id).await.unwrap_or_default();
    let usable: Vec<CloudAccount> = accounts.into_iter().filter(|a| a.status == "ok").collect();
    let balance = balance_of(state, user.id).await.unwrap_or(0);

    let (zones, plans, templates) = if let Some(first) = usable.first() {
        match token_of(state, first.id, user.id).await {
            Ok(token) => match upcloud::Client::new(&token) {
                Ok(c) => tokio::try_join!(c.public_zones(), c.plans(), c.windows_templates())
                    .unwrap_or_default(),
                Err(_) => Default::default(),
            },
            Err(_) => Default::default(),
        }
    } else {
        Default::default()
    };

    let page = InstallPage {
        user,
        accounts: usable,
        zones,
        plans,
        templates,
        balance,
        price: state.cfg.price_per_install,
        error: msg.to_string(),
    };
    render(&page)
}
