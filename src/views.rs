use askama::Template;

use crate::models::{Job, JobEvent, Payment, User, Voucher};
use crate::reinstall::Edition;

/// Satu titik pada rel fase.
pub struct RailStep {
    pub label: &'static str,
    pub state: &'static str,
}

/// Empat tahap perjalanan instalasi. Diturunkan dari status + fase yang
/// tersimpan, jadi yang ditampilkan selalu keadaan sebenarnya.
pub fn rail_for(job: &Job) -> Vec<RailStep> {
    use crate::models::Stage;
    let idx = match job.stage() {
        Stage::Antre | Stage::Diperiksa => 0,
        Stage::Mengunduh => 1,
        Stage::Reboot => 2,
        Stage::Windows => 3,
        Stage::Selesai => 4,
        Stage::Gagal => 99,
    };
    const LABELS: [&str; 4] = ["diperiksa", "unduh", "reboot", "Windows"];
    LABELS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let state = if idx == 99 {
                if i == 0 {
                    "bad"
                } else {
                    "future"
                }
            } else if i < idx {
                "past"
            } else if i == idx {
                "active"
            } else {
                "future"
            };
            RailStep { label, state }
        })
        .collect()
}

pub fn rupiah(n: i64) -> String {
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(ch);
    }
    if n < 0 {
        format!("-Rp{out}")
    } else {
        format!("Rp{out}")
    }
}

pub fn dt(ts: i64) -> String {
    if ts <= 0 {
        return "-".into();
    }
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|d| {
            let wib = d + chrono::Duration::hours(7);
            wib.format("%d %b %Y %H:%M").to_string()
        })
        .unwrap_or_else(|| "-".into())
}

/// Jarak waktu ringkas untuk baris log.
pub fn ago(ts: i64) -> String {
    let now = crate::db::now();
    let d = (now - ts).max(0);
    if d < 60 {
        format!("{d} detik lalu")
    } else if d < 3600 {
        format!("{} menit lalu", d / 60)
    } else if d < 86400 {
        format!("{} jam lalu", d / 3600)
    } else {
        format!("{} hari lalu", d / 86400)
    }
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginPage {
    pub bot_username: String,
    pub auth_url: String,
    pub error: String,
    pub dev_login: bool,
}

impl LoginPage {
    pub fn new(cfg: &crate::config::Config, error: &str) -> Self {
        Self {
            bot_username: cfg.bot_username.clone(),
            auth_url: format!("{}/auth/telegram", cfg.base_url.trim_end_matches('/')),
            error: error.to_string(),
            dev_login: cfg.dev_login,
        }
    }
}

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardPage {
    pub user: User,
    pub balance: i64,
    pub jobs: Vec<Job>,
    pub price: i64,
    pub is_admin: bool,
}

impl DashboardPage {
    pub fn rupiah(&self, n: &i64) -> String {
        rupiah(*n)
    }

    pub fn dt(&self, ts: &i64) -> String {
        dt(*ts)
    }
}

#[derive(Template)]
#[template(path = "instalasi.html")]
pub struct InstallPage {
    pub user: User,
    pub editions: Vec<Edition>,
    pub balance: i64,
    pub price: i64,
    pub error: String,
    pub notice: String,
}

impl InstallPage {
    pub fn rupiah(&self, n: &i64) -> String {
        rupiah(*n)
    }
}

#[derive(Template)]
#[template(path = "job.html")]
pub struct JobPage {
    pub user: User,
    pub card: JobCard,
}

#[derive(Template)]
#[template(path = "job_card.html")]
pub struct JobCard {
    pub job: Job,
    pub password: Option<String>,
    pub events: Vec<JobEvent>,
}

impl JobCard {
    pub fn dt(&self, ts: &i64) -> String {
        dt(*ts)
    }

    pub fn ago(&self, ts: &i64) -> String {
        ago(*ts)
    }

    pub fn rail(&self) -> Vec<RailStep> {
        rail_for(&self.job)
    }

    pub fn running(&self) -> bool {
        self.job.is_active()
    }
}

#[derive(Template)]
#[template(path = "guest.html")]
pub struct GuestPage {
    pub editions: Vec<Edition>,
    pub error: String,
}

#[derive(Template)]
#[template(path = "guest_job.html")]
pub struct GuestJobPage {
    pub card: GuestCard,
}

#[derive(Template)]
#[template(path = "guest_card.html")]
pub struct GuestCard {
    pub job: Job,
    pub password: Option<String>,
    pub events: Vec<JobEvent>,
    pub token: String,
}

impl GuestCard {
    pub fn dt(&self, ts: &i64) -> String {
        dt(*ts)
    }

    pub fn ago(&self, ts: &i64) -> String {
        ago(*ts)
    }

    pub fn rail(&self) -> Vec<RailStep> {
        rail_for(&self.job)
    }

    pub fn running(&self) -> bool {
        self.job.is_active()
    }
}

#[derive(Template)]
#[template(path = "topup.html")]
pub struct TopupPage {
    pub user: User,
    pub balance: i64,
    pub payments: Vec<Payment>,
    pub presets: Vec<i64>,
    pub min: i64,
    pub max: i64,
    pub error: String,
    pub notice: String,
}

impl TopupPage {
    pub fn rupiah(&self, n: &i64) -> String {
        rupiah(*n)
    }

    pub fn dt(&self, ts: &i64) -> String {
        dt(*ts)
    }
}

#[derive(Template)]
#[template(path = "topup_status.html")]
pub struct TopupStatusCard {
    pub p: Payment,
    pub balance: i64,
    #[allow(dead_code)]
    pub min: i64,
    pub fee_note: String,
}

impl TopupStatusCard {
    pub fn rupiah(&self, n: &i64) -> String {
        rupiah(*n)
    }

    pub fn dt(&self, ts: &i64) -> String {
        dt(*ts)
    }
}

#[derive(Template)]
#[template(path = "topup_detail.html")]
pub struct TopupDetailPage {
    pub user: User,
    pub reference: String,
    pub card_html: String,
}

#[derive(Template)]
#[template(path = "admin_vouchers.html")]
pub struct AdminVouchersPage {
    pub user: User,
    pub vouchers: Vec<Voucher>,
    pub free: i64,
    pub notice: String,
}

impl AdminVouchersPage {
    pub fn dt(&self, ts: &i64) -> String {
        dt(*ts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Job, Stage};

    fn job(status: &str, phase: &str) -> Job {
        Job {
            id: 1,
            user_id: 1,
            mode: "reinstall".into(),
            provider: "ssh".into(),
            status: status.into(),
            phase: phase.into(),
            progress: 0,
            os_label: "Server 2022".into(),
            zone: String::new(),
            plan: String::new(),
            host: "203.0.113.9".into(),
            rdp_port: 3389,
            rdp_user: "Administrator".into(),
            provider_ref: String::new(),
            ssh_port: 22,
            ssh_user: "root".into(),
            guest: 0,
            voucher_code: String::new(),
            edition: "srv2022".into(),
            lang: "en-us".into(),
            error: String::new(),
            created_at: 0,
            updated_at: 0,
            done_at: 0,
            expires_at: 0,
            wiped: 0,
        }
    }

    #[test]
    fn rail_follows_the_phase() {
        let steps = rail_for(&job("running", "menghubungi VPS lewat SSH"));
        assert_eq!(steps[0].state, "active");
        assert_eq!(steps[1].state, "future");

        let steps = rail_for(&job("running", "VPS mengunduh image Windows"));
        assert_eq!(steps[0].state, "past");
        assert_eq!(steps[1].state, "active");

        let steps = rail_for(&job("running", "VPS reboot, memasang Windows"));
        assert_eq!(steps[1].state, "past");
        assert_eq!(steps[2].state, "active");

        let steps = rail_for(&job("running", "Windows hidup, memastikan RDP stabil"));
        assert_eq!(steps[3].state, "active");

        let steps = rail_for(&job("done", "RDP siap"));
        assert!(steps.iter().all(|s| s.state == "past"));
    }

    #[test]
    fn failed_job_marks_the_first_step_as_bad() {
        let steps = rail_for(&job("failed", "gagal"));
        assert_eq!(steps[0].state, "bad");
        assert_eq!(steps[3].state, "future");
    }

    #[test]
    fn stage_detection_matches_stored_phase() {
        assert_eq!(job("queued", "menunggu").stage(), Stage::Antre);
        assert_eq!(job("running", "memeriksa mesin").stage(), Stage::Diperiksa);
        assert_eq!(job("done", "apa saja").stage(), Stage::Selesai);
        assert_eq!(job("failed", "apa saja").stage(), Stage::Gagal);
    }

    #[test]
    fn rupiah_uses_indonesian_separators() {
        assert_eq!(rupiah(0), "Rp0");
        assert_eq!(rupiah(5_000), "Rp5.000");
        assert_eq!(rupiah(1_234_567), "Rp1.234.567");
        assert_eq!(rupiah(-5_000), "-Rp5.000");
    }
}
