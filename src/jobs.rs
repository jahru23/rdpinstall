//! Runner job: pantau server UpCloud sampai RDP menjawab, lalu simpan kredensial.
//!
//! Hanya polling: status server lewat API (setiap 10 s) dan port 3389 lewat TCP
//! (setiap 15 s). Tidak menahan koneksi apa pun ke server target.

use std::time::Duration;

use crate::db::now;
use crate::models::Job;
use crate::upcloud;
use crate::SharedState;

const POLL_STATE: Duration = Duration::from_secs(10);
const POLL_RDP: Duration = Duration::from_secs(15);
/// Template Windows biasanya siap RDP dalam 2-5 menit; 25 menit sudah longgar.
const DEADLINE: Duration = Duration::from_secs(25 * 60);

pub async fn tcp_open(host: &str, port: u16, timeout: Duration) -> bool {
    let target = format!("{host}:{port}");
    matches!(
        tokio::time::timeout(timeout, tokio::net::TcpStream::connect(&target)).await,
        Ok(Ok(_))
    )
}

pub fn spawn(state: SharedState, job_id: i64) {
    tokio::spawn(async move {
        if let Err(e) = run(state.clone(), job_id).await {
            tracing::warn!("job {job_id} gagal: {e}");
            let _ = finish_failed(&state, job_id, &e.to_string()).await;
        }
    });
}

async fn load_job(state: &SharedState, job_id: i64) -> anyhow::Result<Job> {
    let job = sqlx::query_as::<_, Job>(
        "SELECT id, user_id, mode, provider, status, phase, progress, os_label, zone, plan,
                host, rdp_port, rdp_user, provider_ref, error, created_at, updated_at, done_at, expires_at, wiped
         FROM jobs WHERE id = ?",
    )
    .bind(job_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| anyhow::anyhow!("job {job_id} tidak ada"))?;
    Ok(job)
}

async fn token_for_job(state: &SharedState, job_id: i64) -> anyhow::Result<String> {
    let enc: Option<String> = sqlx::query_scalar(
        "SELECT c.token_enc FROM jobs j JOIN cloud_accounts c ON c.id = j.cloud_account_id
         WHERE j.id = ?",
    )
    .bind(job_id)
    .fetch_optional(&state.pool)
    .await?;
    let enc = enc.ok_or_else(|| anyhow::anyhow!("akun cloud untuk job ini tidak ditemukan"))?;
    crate::crypto::open(&state.cfg.master_key, &enc)
}

async fn phase(
    state: &SharedState,
    job_id: i64,
    phase: &str,
    progress: i64,
    message: &str,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE jobs SET phase = ?, progress = ?, updated_at = ? WHERE id = ?")
        .bind(phase)
        .bind(progress)
        .bind(now())
        .bind(job_id)
        .execute(&state.pool)
        .await?;
    event(state, job_id, phase, message).await?;
    tracing::info!("job {job_id}: {phase} ({progress}%)");
    Ok(())
}

async fn event(state: &SharedState, job_id: i64, phase: &str, message: &str) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO job_events(job_id, ts, phase, message) VALUES(?,?,?,?)")
        .bind(job_id)
        .bind(now())
        .bind(phase)
        .bind(message)
        .execute(&state.pool)
        .await?;
    Ok(())
}

async fn run(state: SharedState, job_id: i64) -> anyhow::Result<()> {
    let job = load_job(&state, job_id).await?;
    if job.provider_ref.is_empty() {
        anyhow::bail!("job tanpa server uuid — dibuat lewat mode template?");
    }
    let token = token_for_job(&state, job_id).await?;
    let client = upcloud::Client::new(&token)?;
    let started = std::time::Instant::now();

    phase(
        &state,
        job_id,
        "server dibuat, menunggu start UpCloud...",
        15,
        "API create diterima",
    )
    .await?;
    let mut server = loop {
        if started.elapsed() > DEADLINE {
            anyhow::bail!("timeout: server tidak mencapai state started");
        }
        let s = client.server(&job.provider_ref).await?;
        if s.is_started() {
            break s;
        }
        if s.is_failed_state() {
            anyhow::bail!("UpCloud melaporkan state server: {}", s.state);
        }
        tokio::time::sleep(POLL_STATE).await;
    };

    let mut host = server.public_ipv4().unwrap_or_default();
    phase(
        &state,
        job_id,
        "Windows booting, menunggu RDP 3389...",
        45,
        &format!(
            "state started, IP {}",
            if host.is_empty() { "-" } else { &host }
        ),
    )
    .await?;

    let mut hits = 0;
    loop {
        if started.elapsed() > DEADLINE {
            anyhow::bail!("timeout: RDP belum menjawab setelah 25 menit");
        }
        if host.is_empty() {
            server = client.server(&job.provider_ref).await?;
            host = server.public_ipv4().unwrap_or_default();
        }
        if !host.is_empty() && tcp_open(&host, job.rdp_port as u16, Duration::from_secs(8)).await {
            hits += 1;
            if hits >= 2 {
                break;
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        } else {
            hits = 0;
            tokio::time::sleep(POLL_RDP).await;
        }
    }

    let ts = now();
    let expires = ts + state.cfg.job_ttl_hours * 3600;
    sqlx::query(
        "UPDATE jobs SET status='done', phase='RDP siap', progress=100, host=?, done_at=?, updated_at=?, expires_at=?
         WHERE id=?",
    )
    .bind(&host)
    .bind(ts)
    .bind(ts)
    .bind(expires)
    .bind(job_id)
    .execute(&state.pool)
    .await?;
    event(
        &state,
        job_id,
        "RDP siap",
        &format!("{host}:{}", job.rdp_port),
    )
    .await?;
    tracing::info!("job {job_id} selesai: {host}:{}", job.rdp_port);
    Ok(())
}

pub async fn finish_failed(state: &SharedState, job_id: i64, reason: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE jobs SET status='failed', phase='gagal', error=?, updated_at=? WHERE id=?")
        .bind(reason.chars().take(300).collect::<String>())
        .bind(now())
        .bind(job_id)
        .execute(&state.pool)
        .await?;
    event(state, job_id, "gagal", reason).await?;
    Ok(())
}

/// Tugas latar: hapus kredensial yang sudah lewat masa simpan.
pub fn spawn_janitor(state: SharedState) {
    tokio::spawn(async move {
        loop {
            let ts = now();
            match sqlx::query(
                "UPDATE jobs SET rdp_pass_enc='', wiped=1, updated_at=?
                 WHERE wiped=0 AND expires_at > 0 AND expires_at < ?",
            )
            .bind(ts)
            .bind(ts)
            .execute(&state.pool)
            .await
            {
                Ok(r) if r.rows_affected() > 0 => {
                    tracing::info!("janitor: {} kredensial dihapus", r.rows_affected())
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("janitor gagal: {e}"),
            }
            tokio::time::sleep(Duration::from_secs(600)).await;
        }
    });
}
