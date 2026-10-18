//! Runner job mode reinstall: SSH masuk, precheck, kirim skrip, lalu pantau.
//!
//! Yang dipantau hanya port TCP. Instalasi memutus SSH saat mesin reboot, jadi
//! tidak ada sesi SSH yang ditahan: SSH terbuka berarti masih Linux atau masih
//! mengunduh, SSH tertutup berarti reboot (normal, bukan gagal), dan port 3389
//! terbuka berarti Windows sudah hidup.

use std::time::Duration;

use crate::db::now;
use crate::models::Job;
use crate::reinstall::{self, Edition, InstallSpec, Lang};
use crate::ssh;
use crate::SharedState;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const PRECHECK_TIMEOUT: Duration = Duration::from_secs(90);
const PUSH_TIMEOUT: Duration = Duration::from_secs(240);
const POLL: Duration = Duration::from_secs(45);
/// Instalasi Windows lewat netboot biasanya 30 sampai 60 menit.
const DEADLINE: Duration = Duration::from_secs(90 * 60);

pub fn spawn(state: SharedState, job_id: i64) {
    tokio::spawn(async move {
        if let Err(e) = run(state.clone(), job_id).await {
            let msg = e.to_string();
            tracing::warn!("job {job_id} gagal: {msg}");
            let _ = finish_failed(&state, job_id, &msg).await;
        }
    });
}

pub async fn load_job(state: &SharedState, job_id: i64) -> anyhow::Result<Job> {
    sqlx::query_as::<_, Job>(
        "SELECT id, user_id, mode, provider, status, phase, progress, os_label, zone, plan, host, rdp_port,
                rdp_user, provider_ref, error, created_at, updated_at, done_at, expires_at, wiped,
                ssh_port, ssh_user, guest, voucher_code, edition, lang
         FROM jobs WHERE id = ?",
    )
        .bind(job_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("job {job_id} tidak ada"))
}

async fn secrets(state: &SharedState, job_id: i64) -> anyhow::Result<(String, String)> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT ssh_pass_enc, rdp_pass_enc FROM jobs WHERE id = ?")
            .bind(job_id)
            .fetch_optional(&state.pool)
            .await?;
    let (ssh_enc, rdp_enc) = row.ok_or_else(|| anyhow::anyhow!("job {job_id} tidak ada"))?;
    Ok((
        if ssh_enc.is_empty() {
            String::new()
        } else {
            crate::crypto::open(&state.cfg.master_key, &ssh_enc)?
        },
        if rdp_enc.is_empty() {
            String::new()
        } else {
            crate::crypto::open(&state.cfg.master_key, &rdp_enc)?
        },
    ))
}

async fn phase(
    state: &SharedState,
    job_id: i64,
    label: &str,
    progress: i64,
    message: &str,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE jobs SET phase = ?, progress = ?, updated_at = ? WHERE id = ?")
        .bind(label)
        .bind(progress)
        .bind(now())
        .bind(job_id)
        .execute(&state.pool)
        .await?;
    event(state, job_id, label, message).await?;
    tracing::info!("job {job_id}: {label} ({progress}%)");
    Ok(())
}

pub async fn event(
    state: &SharedState,
    job_id: i64,
    phase: &str,
    message: &str,
) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO job_events(job_id, ts, phase, message) VALUES(?,?,?,?)")
        .bind(job_id)
        .bind(now())
        .bind(phase)
        // Pesan bisa berasal dari mesin pelanggan: potong dan redaksi.
        .bind(
            reinstall::redact(message)
                .chars()
                .take(300)
                .collect::<String>(),
        )
        .execute(&state.pool)
        .await?;
    Ok(())
}

async fn run(state: SharedState, job_id: i64) -> anyhow::Result<()> {
    let job = load_job(&state, job_id).await?;
    if job.host.is_empty() {
        anyhow::bail!("job tanpa alamat VPS");
    }
    let (ssh_pass, rdp_pass) = secrets(&state, job_id).await?;
    if ssh_pass.is_empty() {
        anyhow::bail!("kredensial SSH tidak tersedia");
    }
    let edition = Edition::from_key(&job.edition).unwrap_or(Edition::Server2022);
    let lang = Lang::from_code(&job.lang).unwrap_or(Lang::En);

    phase(
        &state,
        job_id,
        "menghubungi VPS lewat SSH",
        8,
        &format!("{}:{} sebagai {}", job.host, job.ssh_port, job.ssh_user),
    )
    .await?;

    let handle = match ssh::connect(
        &job.host,
        job.ssh_port as u16,
        &job.ssh_user,
        &ssh_pass,
        CONNECT_TIMEOUT,
    )
    .await
    {
        Ok(h) => h,
        Err(ssh::Error::Auth) => anyhow::bail!(
            "SSH menolak login. Periksa user dan password, dan pastikan login root diizinkan."
        ),
        Err(e) => anyhow::bail!("{e}"),
    };

    // Precheck dulu: kalau mesin tidak memenuhi syarat, lebih baik berhenti
    // sekarang daripada meninggalkannya setengah terpasang.
    phase(
        &state,
        job_id,
        "memeriksa mesin",
        15,
        "membaca virtualisasi, RAM, dan disk",
    )
    .await?;
    let pre = ssh::exec(&handle, reinstall::PRECHECK_SCRIPT, PRECHECK_TIMEOUT).await?;
    if !pre.ok {
        ssh::disconnect(handle).await;
        anyhow::bail!(
            "pemeriksaan mesin gagal: {}",
            reinstall::redact(pre.stderr.trim())
                .chars()
                .take(200)
                .collect::<String>()
        );
    }
    let info = reinstall::parse_precheck(&pre.stdout);
    if let Err(e) = reinstall::validate(&info, edition) {
        ssh::disconnect(handle).await;
        anyhow::bail!("{e}");
    }
    event(
        &state,
        job_id,
        "mesin lolos pemeriksaan",
        &format!(
            "{} {} | RAM {:.1} GB | disk {:.0} GB | {}",
            info.os, info.arch, info.ram_gb, info.disk_gb, info.bootmode
        ),
    )
    .await?;

    phase(
        &state,
        job_id,
        "mengirim skrip installer",
        25,
        "reinstall.sh dijalankan detached di mesin Anda",
    )
    .await?;
    let spec = InstallSpec {
        image_name: edition.image_name().to_string(),
        lang: lang.code().to_string(),
        username: job.rdp_user.clone(),
        password: rdp_pass,
        rdp_port: job.rdp_port as u16,
        ssh_port: job.ssh_port as u16,
        allow_ping: false,
    };
    let boot = reinstall::build_bootstrap(&spec);
    let pushed = ssh::exec(&handle, &boot, PUSH_TIMEOUT).await?;
    ssh::disconnect(handle).await;
    if !pushed.ok || !pushed.stdout.contains("LAUNCHED") {
        anyhow::bail!(
            "skrip tidak mau jalan: {}",
            reinstall::redact(&pushed.stderr)
                .chars()
                .take(200)
                .collect::<String>()
        );
    }

    // Fase pemantauan: hanya port TCP.
    let started = std::time::Instant::now();
    let mut saw_ssh_down = false;
    let mut rdp_hits = 0;
    loop {
        if started.elapsed() > DEADLINE {
            anyhow::bail!(
                "lewat 90 menit dan RDP belum menjawab di port {}",
                job.rdp_port
            );
        }
        let ssh_up = ssh::tcp_open(&job.host, job.ssh_port as u16, Duration::from_secs(8)).await;
        let rdp_up = ssh::tcp_open(&job.host, job.rdp_port as u16, Duration::from_secs(8)).await;

        if rdp_up {
            rdp_hits += 1;
            if rdp_hits >= 2 {
                break;
            }
            phase(
                &state,
                job_id,
                "Windows hidup, memastikan RDP stabil",
                88,
                "port 3389 menjawab sekali, menunggu konfirmasi kedua",
            )
            .await?;
            tokio::time::sleep(Duration::from_secs(15)).await;
            continue;
        }
        rdp_hits = 0;

        if ssh_up {
            if saw_ssh_down {
                phase(
                    &state,
                    job_id,
                    "Windows menyala, RDP belum siap",
                    70,
                    "SSH kembali terbuka setelah reboot",
                )
                .await?;
            } else {
                phase(
                    &state,
                    job_id,
                    "VPS mengunduh image Windows",
                    35,
                    "SSH masih terbuka, unduhan sedang berjalan",
                )
                .await?;
            }
        } else if !saw_ssh_down {
            saw_ssh_down = true;
            phase(
                &state,
                job_id,
                "VPS reboot, memasang Windows",
                60,
                "SSH tertutup itu normal pada tahap ini",
            )
            .await?;
        }
        tokio::time::sleep(POLL).await;
    }

    let ts = now();
    let expires = ts + state.cfg.job_ttl_hours * 3600;
    // Setelah Windows hidup, SSH sudah tidak ada lagi: password SSH tidak
    // berguna dan langsung dibuang, bukan menunggu janitor.
    sqlx::query(
        "UPDATE jobs SET status='done', phase='RDP siap', progress=100, done_at=?, updated_at=?,
                expires_at=?, ssh_pass_enc='' WHERE id=?",
    )
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
        &format!("{}:{}", job.host, job.rdp_port),
    )
    .await?;
    tracing::info!("job {job_id} selesai: {}:{}", job.host, job.rdp_port);
    Ok(())
}

/// Kembalikan entri boot Linux di mesin yang gagal tapi masih bisa dihubungi.
pub async fn reset_boot(state: &SharedState, job_id: i64) -> anyhow::Result<String> {
    let job = load_job(state, job_id).await?;
    let (ssh_pass, _) = secrets(state, job_id).await?;
    let handle = ssh::connect(
        &job.host,
        job.ssh_port as u16,
        &job.ssh_user,
        &ssh_pass,
        CONNECT_TIMEOUT,
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    let out = ssh::exec(
        &handle,
        &format!(
            "cd /root 2>/dev/null || cd /tmp; {}",
            reinstall::reset_command()
        ),
        Duration::from_secs(60),
    )
    .await?;
    ssh::disconnect(handle).await;
    if !out.ok {
        anyhow::bail!(
            "reset gagal: {}",
            reinstall::redact(&out.stderr)
                .chars()
                .take(200)
                .collect::<String>()
        );
    }
    Ok(out.stdout.chars().take(400).collect())
}

pub async fn finish_failed(state: &SharedState, job_id: i64, reason: &str) -> anyhow::Result<()> {
    // Job gagal tetap diberi masa kedaluwarsa: tanpa ini, password SSH yang
    // tersimpan tidak akan pernah dihapus janitor.
    let ts = now();
    let expires = ts + state.cfg.job_ttl_hours * 3600;
    sqlx::query(
        "UPDATE jobs SET status='failed', phase='gagal', error=?, updated_at=?,
                expires_at=CASE WHEN expires_at = 0 THEN ? ELSE expires_at END
         WHERE id=?",
    )
    .bind(reason.chars().take(300).collect::<String>())
    .bind(ts)
    .bind(expires)
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
                "UPDATE jobs SET rdp_pass_enc='', ssh_pass_enc='', wiped=1, updated_at=?
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Tanpa expires_at, password SSH pada job gagal tidak akan pernah
    /// dihapus janitor. Ini pernah benar-benar terjadi sebelum diperbaiki.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn failed_job_gets_an_expiry_so_the_janitor_can_clean_it() {
        let db_path = std::env::temp_dir().join(format!(
            "rdpi-jobs-test-{}.db",
            crate::crypto::random_token(6)
        ));
        let db = db_path.to_string_lossy().to_string();
        let pool = crate::db::connect(&db).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let cfg = crate::config::Config {
            env: "test".into(),
            bind: "127.0.0.1:0".into(),
            base_url: "http://127.0.0.1".into(),
            db_path: db.clone(),
            bot_token: String::new(),
            bot_username: String::new(),
            master_key: [3u8; 32],
            session_ttl_hours: 1,
            job_ttl_hours: 24,
            price_per_install: 5000,
            dev_login: true,
            paygate_url: String::new(),
            paygate_public_url: String::new(),
            paygate_key: String::new(),
            paygate_provider: "gobiz".into(),
            admin_tg_ids: vec![],
            admin_user: "admin".into(),
            admin_pass: "admin".into(),
            cryptogate_url: String::new(),
            cryptogate_key: String::new(),
        };
        let state: SharedState = std::sync::Arc::new(crate::AppState { cfg, pool });
        sqlx::query(
            "INSERT INTO jobs(id, user_id, mode, provider, status, phase, host, ssh_pass_enc,
                              rdp_pass_enc, created_at, updated_at)
             VALUES(1, 0, 'reinstall', 'ssh', 'running', 'menghubungi VPS lewat SSH',
                    '203.0.113.9', 'v1:aa:bb', 'v1:cc:dd', 1, 1)",
        )
        .execute(&state.pool)
        .await
        .unwrap();

        finish_failed(&state, 1, "koneksi ditolak").await.unwrap();

        let (status, expires): (String, i64) =
            sqlx::query_as("SELECT status, expires_at FROM jobs WHERE id = 1")
                .fetch_one(&state.pool)
                .await
                .unwrap();
        assert_eq!(status, "failed");
        assert!(expires > crate::db::now(), "job gagal harus kedaluwarsa");

        // setelah lewat masa simpan, janitor membersihkan kedua kredensial
        sqlx::query("UPDATE jobs SET expires_at = ? WHERE id = 1")
            .bind(crate::db::now() - 10)
            .execute(&state.pool)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE jobs SET rdp_pass_enc='', ssh_pass_enc='', wiped=1, updated_at=?
             WHERE wiped=0 AND expires_at > 0 AND expires_at < ?",
        )
        .bind(crate::db::now())
        .bind(crate::db::now())
        .execute(&state.pool)
        .await
        .unwrap();
        let (ssh, rdp, wiped): (String, String, i64) =
            sqlx::query_as("SELECT ssh_pass_enc, rdp_pass_enc, wiped FROM jobs WHERE id = 1")
                .fetch_one(&state.pool)
                .await
                .unwrap();
        assert!(ssh.is_empty() && rdp.is_empty(), "kredensial harus kosong");
        assert_eq!(wiped, 1);

        let _ = std::fs::remove_file(db);
    }
}
