use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

const KEY_LEN: usize = 32;

#[derive(Clone, Debug)]
pub struct Config {
    pub env: String,
    pub bind: String,
    pub base_url: String,
    pub db_path: String,
    pub bot_token: String,
    pub bot_username: String,
    pub master_key: [u8; KEY_LEN],
    pub session_ttl_hours: i64,
    pub job_ttl_hours: i64,
    pub price_per_install: i64,
    pub dev_login: bool,
    pub paygate_url: String,
    pub paygate_public_url: String,
    pub paygate_key: String,
    pub paygate_provider: String,
    pub cryptogate_url: String,
    pub cryptogate_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let env = var("RDPINSTALL_ENV", "dev");

        let bind = var("RDPINSTALL_BIND", "127.0.0.1:8791");
        let base_url = var("RDPINSTALL_BASE_URL", "http://127.0.0.1:8791");
        let db_path = var("RDPINSTALL_DB", "data/rdpinstall.db");

        let bot_token = var("TELEGRAM_BOT_TOKEN", "");
        let bot_username = var("TELEGRAM_BOT_USERNAME", "");

        let master_key = load_master_key()?;

        let dev_login = env != "prod" && var("RDPINSTALL_DEV_LOGIN", "0") == "1";

        Ok(Self {
            env,
            bind,
            base_url,
            db_path,
            bot_token,
            bot_username,
            master_key,
            session_ttl_hours: num("RDPINSTALL_SESSION_TTL_HOURS", 720),
            job_ttl_hours: num("RDPINSTALL_JOB_TTL_HOURS", 24),
            price_per_install: num("RDPINSTALL_PRICE", 5000),
            dev_login,
            paygate_url: var("PAYGATE_URL", "http://127.0.0.1:3001"),
            paygate_public_url: var("PAYGATE_PUBLIC_URL", "https://pg-v1.jaystore.top"),
            paygate_key: var("PAYGATE_API_KEY", ""),
            paygate_provider: var("PAYGATE_PROVIDER", crate::paygate::PROVIDER_DEFAULT),
            cryptogate_url: var("CRYPTOGATE_URL", "http://127.0.0.1:8899"),
            cryptogate_key: var("CRYPTOGATE_API_KEY", ""),
        })
    }

    pub fn is_prod(&self) -> bool {
        self.env == "prod"
    }
}

fn var(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn num(key: &str, default: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(default)
}

/// Master key for AES-GCM at rest: RDPINSTALL_MASTER_KEY (64 hex chars) or a
/// 0600 file. Generated on first run in dev so local work needs no setup.
fn load_master_key() -> Result<[u8; KEY_LEN]> {
    if let Ok(raw) = std::env::var("RDPINSTALL_MASTER_KEY") {
        let raw = raw.trim();
        if !raw.is_empty() {
            return parse_key(raw).context("RDPINSTALL_MASTER_KEY tidak valid");
        }
    }

    let path = PathBuf::from(var("RDPINSTALL_KEY_FILE", "keys/master.key"));
    if path.exists() {
        let raw =
            std::fs::read_to_string(&path).with_context(|| format!("baca {}", path.display()))?;
        return parse_key(raw.trim()).with_context(|| format!("{} tidak valid", path.display()));
    }

    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    let mut key = [0u8; KEY_LEN];
    crypto_fill(&mut key)?;
    write_private(&path, &hex::encode(key))?;
    tracing::warn!("master key baru dibuat: {}", path.display());
    Ok(key)
}

fn crypto_fill(buf: &mut [u8]) -> Result<()> {
    getrandom::fill(buf).map_err(|e| anyhow::anyhow!("gagal mengambil entropy dari OS: {e}"))?;
    Ok(())
}

fn parse_key(raw: &str) -> Result<[u8; KEY_LEN]> {
    let bytes = hex::decode(raw).context("harus hex")?;
    if bytes.len() != KEY_LEN {
        bail!(
            "panjang key harus {} byte ({} hex), dapat {}",
            KEY_LEN,
            KEY_LEN * 2,
            bytes.len()
        );
    }
    let mut key = [0u8; KEY_LEN];
    key.copy_from_slice(&bytes);
    Ok(key)
}

fn write_private(path: &Path, content: &str) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("tulis {}", path.display()))?;
    f.write_all(content.as_bytes())?;
    f.write_all(b"\n")?;
    Ok(())
}
