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
    #[serde(default)]
    pub tier: Option<String>,
}

impl Template {
    /// Tier (maxiops/hdd) dari template, kalau API menyebutkannya.
    pub fn tier_hint(&self) -> Option<String> {
        self.tier.clone().filter(|t| !t.is_empty())
    }

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

// ---------------------------------------------------------------- servers

#[derive(Debug, Clone, Deserialize)]
pub struct ServerIp {
    #[serde(default)]
    pub access: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub family: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Server {
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub zone: String,
    #[serde(default)]
    pub plan: String,
    /// Hanya ada di respons pembuatan server Windows.
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub ip_addresses: Option<ServerIps>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ServerIps {
    #[serde(default)]
    pub ip_address: Vec<ServerIp>,
}

impl Server {
    pub fn public_ipv4(&self) -> Option<String> {
        self.ip_addresses
            .as_ref()?
            .ip_address
            .iter()
            .find(|i| {
                i.access.eq_ignore_ascii_case("public") && i.family.eq_ignore_ascii_case("IPv4")
            })
            .map(|i| i.address.clone())
            .filter(|a| !a.is_empty())
    }

    pub fn is_started(&self) -> bool {
        self.state.eq_ignore_ascii_case("started")
    }

    pub fn is_failed_state(&self) -> bool {
        matches!(
            self.state.to_ascii_lowercase().as_str(),
            "error" | "maintenance" | "unknown"
        )
    }
}

/// Permintaan pembuatan server: clone template publik + satu interface publik.
#[derive(Debug, Clone)]
pub struct CreateSpec {
    pub zone: String,
    pub title: String,
    pub hostname: String,
    pub plan: String,
    pub template_uuid: String,
    pub storage_title: String,
    pub storage_size_gb: i64,
    pub storage_tier: Option<String>,
}

/// Hostname UpCloud: huruf/angka/strip, maksimum 63, tanpa titik di ujung.
pub fn safe_hostname(raw: &str) -> String {
    let mut h: String = raw
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    while h.contains("--") {
        h = h.replace("--", "-");
    }
    let h = h.trim_matches('-').to_string();
    let mut out = if h.is_empty() {
        "win-rdp".to_string()
    } else {
        h
    };
    out.truncate(45);
    format!("{out}-{}", crate::crypto::random_token(2))
}

impl CreateSpec {
    pub fn to_body(&self) -> Value {
        let mut device = serde_json::json!({
            "action": "clone",
            "storage": self.template_uuid,
            "title": self.storage_title,
        });
        if self.storage_size_gb > 0 {
            device["size"] = serde_json::json!(self.storage_size_gb);
        }
        if let Some(tier) = &self.storage_tier {
            if !tier.is_empty() {
                device["tier"] = serde_json::json!(tier);
            }
        }
        serde_json::json!({
            "server": {
                "zone": self.zone,
                "title": self.title,
                "hostname": self.hostname,
                "plan": self.plan,
                "metadata": "yes",
                "storage_devices": { "storage_device": [device] },
                "networking": {
                    "interfaces": {
                        "interface": [
                            { "type": "public", "ip_addresses": { "ip_address": [ { "family": "IPv4" } ] } },
                            { "type": "utility", "ip_addresses": { "ip_address": [ { "family": "IPv4" } ] } }
                        ]
                    }
                }
            }
        })
    }
}

impl Client {
    async fn post(&self, path: &str, body: &Value) -> Result<Value, Error> {
        let url = format!("{}{}", self.base, path);
        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| Error::Transport(e.to_string()))?;

        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(Error::Auth(format!("HTTP {}", status.as_u16())));
        }
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: first_error_message(&text),
            });
        }
        serde_json::from_str::<Value>(&text).map_err(|e| Error::Decode(e.to_string()))
    }

    async fn delete(&self, path: &str) -> Result<(), Error> {
        let url = format!("{}{}", self.base, path);
        let res = self
            .http
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await
            .map_err(|e| Error::Transport(e.to_string()))?;
        if res.status().is_success() || res.status().as_u16() == 404 {
            return Ok(());
        }
        Err(Error::Api {
            status: res.status().as_u16(),
            message: first_error_message(&res.text().await.unwrap_or_default()),
        })
    }

    /// Membuat server dari template. Untuk template Windows, UpCloud
    /// mengembalikan password Administrator di respons ini — satu-satunya
    /// kesempatan mengambilnya.
    pub async fn create_server(&self, spec: &CreateSpec) -> Result<Server, Error> {
        let v = self.post("/server", &spec.to_body()).await?;
        let s = v
            .get("server")
            .ok_or_else(|| Error::Decode("respons tanpa objek server".into()))?;
        serde_json::from_value(s.clone()).map_err(|e| Error::Decode(e.to_string()))
    }

    pub async fn server(&self, uuid: &str) -> Result<Server, Error> {
        let v = self.get(&format!("/server/{uuid}")).await?;
        let s = v
            .get("server")
            .ok_or_else(|| Error::Decode("respons tanpa objek server".into()))?;
        serde_json::from_value(s.clone()).map_err(|e| Error::Decode(e.to_string()))
    }

    pub async fn delete_server(&self, uuid: &str) -> Result<(), Error> {
        self.delete(&format!("/server/{uuid}")).await
    }
}

#[cfg(test)]
mod server_tests {
    use super::*;

    const CREATED: &str = r#"{"server":{
        "boot_order":"disk","core_number":"2","firewall":"on","hostname":"win-rdp-ab12",
        "ip_addresses":{"ip_address":[
            {"access":"utility","address":"10.0.0.0","family":"IPv4"},
            {"access":"public","address":"94.237.10.20","family":"IPv4"},
            {"access":"public","address":"2a01:4f8::1","family":"IPv6"}]},
        "license":0.05,"memory_amount":"4096","password":"Aa1!zzzzzzzzzz","plan":"2xCPU-4GB",
        "state":"started","title":"Windows RDP","uuid":"00c78863-db86-44ea-af70-d6edc4d162bf",
        "zone":"sg-sin1"}}"#;

    #[test]
    fn parses_created_server_with_windows_password() {
        let v: Value = serde_json::from_str(CREATED).unwrap();
        let s: Server = serde_json::from_value(v.get("server").unwrap().clone()).unwrap();
        assert_eq!(s.uuid, "00c78863-db86-44ea-af70-d6edc4d162bf");
        assert!(s.is_started());
        assert_eq!(s.password.as_deref(), Some("Aa1!zzzzzzzzzz"));
        assert_eq!(
            s.public_ipv4().as_deref(),
            Some("94.237.10.20"),
            "harus IPv4 publik, bukan utility/IPv6"
        );
        assert!(!s.is_failed_state());
    }

    #[test]
    fn server_without_password_or_ips_is_tolerated() {
        let body = r#"{"server":{"uuid":"x","state":"maintenance"}}"#;
        let v: Value = serde_json::from_str(body).unwrap();
        let s: Server = serde_json::from_value(v.get("server").unwrap().clone()).unwrap();
        assert!(s.password.is_none());
        assert!(s.public_ipv4().is_none());
        assert!(s.is_failed_state());
    }

    #[test]
    fn create_body_matches_documented_shape() {
        let spec = CreateSpec {
            zone: "sg-sin1".into(),
            title: "Windows RDP".into(),
            hostname: "win-rdp-ab12".into(),
            plan: "2xCPU-4GB".into(),
            template_uuid: "01000000-0000-4000-8000-000030220400".into(),
            storage_title: "Windows Server 2022".into(),
            storage_size_gb: 40,
            storage_tier: Some("maxiops".into()),
        };
        let b = spec.to_body();
        assert_eq!(b["server"]["zone"], "sg-sin1");
        assert_eq!(b["server"]["plan"], "2xCPU-4GB");
        let dev = &b["server"]["storage_devices"]["storage_device"][0];
        assert_eq!(dev["action"], "clone");
        assert_eq!(dev["storage"], "01000000-0000-4000-8000-000030220400");
        assert_eq!(dev["size"], 40);
        assert_eq!(dev["tier"], "maxiops");
        let ifaces = b["server"]["networking"]["interfaces"]["interface"]
            .as_array()
            .unwrap();
        assert_eq!(ifaces[0]["type"], "public");
        assert_eq!(ifaces[0]["ip_addresses"]["ip_address"][0]["family"], "IPv4");
        // tanpa login_user: UpCloud membuat password Administrator sendiri
        assert!(b["server"].get("login_user").is_none());
    }

    #[test]
    fn create_body_without_size_or_tier() {
        let spec = CreateSpec {
            zone: "fi-hel1".into(),
            title: "t".into(),
            hostname: "h".into(),
            plan: "1xCPU-2GB".into(),
            template_uuid: "u".into(),
            storage_title: "s".into(),
            storage_size_gb: 0,
            storage_tier: None,
        };
        let b = spec.to_body();
        assert!(b["server"]["storage_devices"]["storage_device"][0]
            .get("size")
            .is_none());
        assert!(b["server"]["storage_devices"]["storage_device"][0]
            .get("tier")
            .is_none());
    }

    #[test]
    fn hostname_is_sanitized() {
        let h = safe_hostname("Windows RDP #1 (SG)");
        assert!(h.starts_with("windows-rdp-1-sg-"), "dapat: {h}");
        assert!(h.len() <= 48);
        assert!(h.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
        assert_ne!(
            safe_hostname("x"),
            safe_hostname("x"),
            "harus unik per panggilan"
        );
    }
}
