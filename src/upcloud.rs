//! Klien tipis untuk UpCloud API 1.3.
//!
//! Hanya operasi yang dibutuhkan alur installer: verifikasi akun (sekaligus cek
//! kredit), daftar zone/plan/template, dan pembuatan server dari template.
//! Zone, plan, dan UUID template selalu diambil saat request — slug plan dan
//! UUID template berubah, jadi menanamkannya di kode akan gagal diam-diam.

use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;

pub const BASE: &str = "https://api.upcloud.com/1.3";
const TOKEN_PREFIX: &str = "ucat_";

#[derive(Debug)]
pub enum Error {
    /// Token tidak diterima (401/403) atau formatnya jelas salah.
    Auth(String),
    /// UpCloud menjawab, tapi dengan error lain.
    Api { status: u16, message: String },
    /// Tidak sampai ke UpCloud.
    Transport(String),
    /// Jawaban tidak sesuai bentuk yang diharapkan.
    Decode(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Auth(m) => write!(f, "Token ditolak UpCloud: {m}"),
            Error::Api { status, message } => write!(f, "UpCloud menjawab {status}: {message}"),
            Error::Transport(m) => write!(f, "Tidak bisa menghubungi UpCloud: {m}"),
            Error::Decode(m) => write!(f, "Jawaban UpCloud tidak terbaca: {m}"),
        }
    }
}

impl std::error::Error for Error {}

pub fn looks_like_token(token: &str) -> bool {
    let t = token.trim();
    t.len() >= 20
        && t.starts_with(TOKEN_PREFIX)
        && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[derive(Debug, Clone)]
pub struct Account {
    pub username: String,
    pub credits: f64,
    pub raw: Value,
}

impl Account {
    pub fn credits_label(&self) -> String {
        format!("{:.2} credit", self.credits)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Zone {
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "yes")]
    pub public: String,
}

impl Zone {
    pub fn is_public(&self) -> bool {
        self.public.eq_ignore_ascii_case("yes")
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Plan {
    pub name: String,
    #[serde(default)]
    pub core_number: i64,
    #[serde(default)]
    pub memory_amount: i64,
    #[serde(default)]
    pub storage_size: i64,
    #[serde(default)]
    pub storage_tier: Option<String>,
    #[serde(default)]
    pub public_traffic_out: i64,
    #[serde(default)]
    pub gpu_amount: Option<i64>,
    #[serde(default)]
    pub gpu_model: Option<String>,
}

impl Plan {
    pub fn ram_gb(&self) -> f64 {
        self.memory_amount as f64 / 1024.0
    }

    pub fn label(&self) -> String {
        if self.gpu_model.is_some() {
            format!(
                "{} · {} core · {} GB RAM (GPU)",
                self.name,
                self.core_number,
                self.ram_gb()
            )
        } else {
            format!(
                "{} · {} core · {} GB RAM",
                self.name,
                self.core_number,
                self.ram_gb()
            )
        }
    }

    /// Windows 11 / Server 2025 butuh 2 GB; plan kecil tetap ditawarkan untuk
    /// edisi ringan, ini hanya penanda untuk UI.
    pub fn fits_windows_11(&self) -> bool {
        self.memory_amount >= 2048
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Template {
    pub uuid: String,
    pub title: String,
    #[serde(default)]
    pub size: i64,
    #[serde(default)]
    pub license: f64,
    #[serde(default)]
    pub zone: String,
    #[serde(default)]
    pub access: String,
    #[serde(default)]
    pub template_type: String,
}

impl Template {
    pub fn is_windows(&self) -> bool {
        let t = self.title.to_lowercase();
        t.contains("windows") || t.contains("server 20")
    }
}

fn yes() -> String {
    "yes".to_string()
}

pub struct Client {
    http: reqwest::Client,
    token: String,
    base: String,
}

impl Client {
    pub fn new(token: &str) -> Result<Self, Error> {
        let token = token.trim().to_string();
        if token.is_empty() {
            return Err(Error::Auth("token kosong".into()));
        }
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("rdpinstall/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| Error::Transport(e.to_string()))?;
        Ok(Self {
            http,
            token,
            base: BASE.to_string(),
        })
    }

    /// Untuk pengujian: arahkan ke server tiruan.
    pub fn with_base(token: &str, base: &str) -> Result<Self, Error> {
        let mut c = Self::new(token)?;
        c.base = base.trim_end_matches('/').to_string();
        Ok(c)
    }

    async fn get(&self, path: &str) -> Result<Value, Error> {
        let url = format!("{}{}", self.base, path);
        let res = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| Error::Transport(e.to_string()))?;

        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(Error::Auth(format!(
                "HTTP {} — periksa kembali tokennya (harus diawali {})",
                status.as_u16(),
                TOKEN_PREFIX
            )));
        }
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: first_error_message(&body),
            });
        }
        serde_json::from_str::<Value>(&body).map_err(|e| Error::Decode(e.to_string()))
    }

    /// Token valid + sisa kredit. Ini juga panggilan uji saat akun ditambahkan.
    pub async fn account(&self) -> Result<Account, Error> {
        let v = self.get("/account").await?;
        let a = v
            .get("account")
            .ok_or_else(|| Error::Decode("tanpa objek account".into()))?;
        let credits = a.get("credits").and_then(Value::as_f64).unwrap_or(0.0);
        let username = a
            .get("username")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        Ok(Account {
            username,
            credits,
            raw: a.clone(),
        })
    }

    pub async fn public_zones(&self) -> Result<Vec<Zone>, Error> {
        let v = self.get("/zone").await?;
        let list = v
            .pointer("/zones/zone")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Decode("tanpa daftar zones.zone".into()))?;
        let mut zones: Vec<Zone> = serde_json::from_value(Value::Array(list.clone()))
            .map_err(|e| Error::Decode(e.to_string()))?;
        zones.retain(Zone::is_public);
        zones.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(zones)
    }

    pub async fn plans(&self) -> Result<Vec<Plan>, Error> {
        let v = self.get("/plan").await?;
        let list = v
            .pointer("/plans/plan")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Decode("tanpa daftar plans.plan".into()))?;
        let mut plans: Vec<Plan> = serde_json::from_value(Value::Array(list.clone()))
            .map_err(|e| Error::Decode(e.to_string()))?;
        // GPU di luar cakupan installer RDP biasa.
        plans.retain(|p| p.gpu_model.is_none());
        plans.sort_by_key(|p| (p.memory_amount, p.core_number, p.name.clone()));
        Ok(plans)
    }

    /// Template publik. Windows difilter di sisi klien karena API hanya
    /// menyediakan pemisahan per type, bukan per keluarga OS.
    pub async fn public_templates(&self) -> Result<Vec<Template>, Error> {
        let v = self.get("/storage/template").await?;
        let list = v
            .pointer("/storages/storage")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Decode("tanpa daftar storages.storage".into()))?;
        let all: Vec<Template> = serde_json::from_value(Value::Array(list.clone()))
            .map_err(|e| Error::Decode(e.to_string()))?;
        let mut t: Vec<Template> = all
            .into_iter()
            .filter(|s| s.access.eq_ignore_ascii_case("public"))
            .collect();
        t.sort_by(|a, b| a.title.cmp(&b.title));
        Ok(t)
    }

    pub async fn windows_templates(&self) -> Result<Vec<Template>, Error> {
        Ok(self
            .public_templates()
            .await?
            .into_iter()
            .filter(Template::is_windows)
            .collect())
    }
}

/// UpCloud mengembalikan {"error":{"error_code":"...","error_message":"..."}}.
pub fn first_error_message(body: &str) -> String {
    match serde_json::from_str::<Value>(body) {
        Ok(v) => v
            .pointer("/error/error_message")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| body.chars().take(200).collect()),
        Err(_) => body.chars().take(200).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Contoh respons diambil apa adanya dari dokumentasi API UpCloud 1.3.
    const ACCOUNT: &str =
        r#"{"account":{"credits":9972.2324,"username":"upclouduser","vat_number":null}}"#;
    const ZONES: &str = r#"{"zones":{"zone":[
        {"description":"Frankfurt #1","id":"de-fra1","public":"yes"},
        {"description":"Fake private cloud","id":"fi-fake-priv-cloud","parent_zone":"fi-hel1","public":"no"},
        {"description":"Singapore #1","id":"sg-sin1","public":"yes"}]}}"#;
    const PLANS: &str = r#"{"plans":{"plan":[
        {"core_number":1,"memory_amount":2048,"name":"1xCPU-2GB","public_traffic_out":2048,"storage_size":50,"storage_tier":"maxiops"},
        {"core_number":32,"memory_amount":393216,"name":"GPU-32xCPU-384GB-2xL40S","public_traffic_out":44,"gpu_amount":2,"storage_size":0,"storage_tier":null,"gpu_model":"NVIDIA L40S"}]}}"#;
    const STORAGES: &str = r#"{"storages":{"storage":[
        {"access":"public","encrypted":"no","labels":[],"license":0,"size":5,"state":"online","template_type":"cloud-init","title":"Ubuntu Server 22.04 LTS (Jammy Jellyfish)","type":"template","uuid":"01000000-0000-4000-8000-000030220200"},
        {"access":"public","encrypted":"no","labels":[],"license":0.05,"size":30,"state":"online","title":"Windows Server 2022 (FI-Hel1)","type":"template","uuid":"01000000-0000-4000-8000-000030220400","zone":"fi-hel1"},
        {"access":"private","encrypted":"no","labels":[],"license":0,"size":10,"state":"online","title":"Windows Server 2019 (punya saya)","type":"template","uuid":"01eff7ad-168e-413e-83b0-054f6a28fa23","zone":"uk-lon1"}]}}"#;

    #[test]
    fn token_shape() {
        assert!(looks_like_token("ucat_01DQE3AJDEBFEKECFM558TGH2F"));
        assert!(!looks_like_token(""));
        assert!(!looks_like_token("passwordbiasa"));
        assert!(!looks_like_token("ucat_pendek"));
    }

    #[test]
    fn parses_account() {
        let v: Value = serde_json::from_str(ACCOUNT).unwrap();
        let a = v.pointer("/account").unwrap();
        assert_eq!(a.get("credits").unwrap().as_f64().unwrap(), 9972.2324);
        assert_eq!(a.get("username").unwrap().as_str().unwrap(), "upclouduser");
    }

    #[test]
    fn zones_deserialize_and_filter_private() {
        let v: Value = serde_json::from_str(ZONES).unwrap();
        let list = v.pointer("/zones/zone").unwrap().as_array().unwrap();
        let mut zones: Vec<Zone> = serde_json::from_value(Value::Array(list.clone())).unwrap();
        zones.retain(Zone::is_public);
        let ids: Vec<&str> = zones.iter().map(|z| z.id.as_str()).collect();
        assert_eq!(ids, vec!["de-fra1", "sg-sin1"]);
        assert_eq!(zones[0].description, "Frankfurt #1");
    }

    #[test]
    fn plans_skip_gpu_and_sort_by_ram() {
        let v: Value = serde_json::from_str(PLANS).unwrap();
        let list = v.pointer("/plans/plan").unwrap().as_array().unwrap();
        let mut plans: Vec<Plan> = serde_json::from_value(Value::Array(list.clone())).unwrap();
        plans.retain(|p| p.gpu_model.is_none());
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].name, "1xCPU-2GB");
        assert_eq!(plans[0].ram_gb(), 2.0);
        assert!(plans[0].fits_windows_11(), "2 GB cukup untuk Windows 11");
    }

    #[test]
    fn templates_keep_public_only_and_flag_windows() {
        let v: Value = serde_json::from_str(STORAGES).unwrap();
        let list = v.pointer("/storages/storage").unwrap().as_array().unwrap();
        let all: Vec<Template> = serde_json::from_value(Value::Array(list.clone())).unwrap();
        let public: Vec<Template> = all
            .into_iter()
            .filter(|s| s.access.eq_ignore_ascii_case("public"))
            .collect();
        assert_eq!(public.len(), 2);
        let win: Vec<&Template> = public.iter().filter(|s| s.is_windows()).collect();
        assert_eq!(win.len(), 1);
        assert_eq!(
            win[0].license, 0.05,
            "lisensi Windows harus terbaca, bukan 0"
        );
    }

    #[test]
    fn error_message_extraction() {
        let body =
            r#"{"error":{"error_code":"UNAUTHORIZED","error_message":"Authentication failed"}}"#;
        assert_eq!(first_error_message(body), "Authentication failed");
        assert_eq!(first_error_message("<html>502</html>"), "<html>502</html>");
    }

    #[test]
    fn error_display_is_readable() {
        let e = Error::Auth("HTTP 401".into());
        assert!(e.to_string().starts_with("Token ditolak UpCloud"));
    }
}
