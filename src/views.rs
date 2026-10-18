use askama::Template;

use crate::models::{CloudAccount, Job, JobEvent, Payment, User};
use crate::upcloud::{Plan, Template as WinTemplate, Zone};

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
    pub cloud_accounts: Vec<CloudAccount>,
    pub price: i64,
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
#[template(path = "cloud.html")]
pub struct CloudPage {
    pub user: User,
    pub accounts: Vec<CloudAccount>,
    pub error: String,
    pub notice: String,
    pub max_accounts: i64,
}

impl CloudPage {
    pub fn dt(&self, ts: &i64) -> String {
        dt(*ts)
    }
}

pub fn rupiah(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
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

#[derive(Template)]
#[template(path = "install.html")]
pub struct InstallPage {
    pub user: User,
    pub accounts: Vec<CloudAccount>,
    pub zones: Vec<Zone>,
    pub plans: Vec<Plan>,
    pub templates: Vec<WinTemplate>,
    pub balance: i64,
    pub price: i64,
    pub error: String,
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
    /// Kartu status yang sudah dirender (dipakai juga oleh polling).
    pub card_html: String,
}
