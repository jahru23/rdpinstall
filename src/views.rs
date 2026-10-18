use askama::Template;

use crate::models::{CloudAccount, Job, User};

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
