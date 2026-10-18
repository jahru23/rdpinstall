use serde::Deserialize;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: i64,
    pub tg_id: Option<i64>,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub photo_url: String,
    pub banned: i64,
    pub created_at: i64,
    pub last_seen_at: i64,
}

impl User {
    pub fn display_name(&self) -> String {
        let mut n = self.first_name.clone();
        if !self.last_name.is_empty() {
            if !n.is_empty() {
                n.push(' ');
            }
            n.push_str(&self.last_name);
        }
        if n.is_empty() {
            if !self.username.is_empty() {
                n = format!("@{}", self.username);
            } else {
                n = format!("user#{}", self.id);
            }
        }
        n
    }

    pub fn handle(&self) -> String {
        if self.username.is_empty() {
            String::new()
        } else {
            format!("@{}", self.username)
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Job {
    pub id: i64,
    pub user_id: i64,
    pub mode: String,
    pub provider: String,
    pub status: String,
    pub phase: String,
    pub progress: i64,
    pub os_label: String,
    pub zone: String,
    pub plan: String,
    pub host: String,
    pub rdp_port: i64,
    pub rdp_user: String,
    pub provider_ref: String,
    pub error: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub done_at: i64,
    pub expires_at: i64,
    pub wiped: i64,
}

impl Job {
    pub fn status_class(&self) -> &'static str {
        match self.status.as_str() {
            "done" => "ok",
            "failed" | "cancelled" => "bad",
            "running" | "queued" => "live",
            _ => "idle",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status.as_str(), "queued" | "running")
    }
}

/// Satu invoice topup.
///
/// Catatan penamaan: kolom DB `qr_string` menyimpan **URL PNG QRIS** dari
/// gateway (bukan payload QR mentah) — dokumen PayGate melarang menggambar QR
/// sendiri dari string, jadi kami memakai gambar yang diterbitkan gateway.
#[derive(Debug, Clone, FromRow)]
pub struct Payment {
    pub id: i64,
    pub user_id: i64,
    pub gateway: String,
    #[sqlx(rename = "ref")]
    pub reference: String,
    pub gateway_id: String,
    pub amount: i64,
    pub total_amount: i64,
    pub status: String,
    pub qr_string: String,
    pub pay_url: String,
    pub txn_id: String,
    pub created_at: i64,
    pub paid_at: i64,
    pub expires_at: i64,
}

impl Payment {
    pub fn status_class(&self) -> &'static str {
        match self.status.as_str() {
            "paid" => "ok",
            "pending" => "live",
            "expired" | "cancelled" | "failed" => "bad",
            _ => "unchecked",
        }
    }

    pub fn is_pending(&self) -> bool {
        self.status == "pending"
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct JobEvent {
    pub ts: i64,
    pub phase: String,
    pub message: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct CloudAccount {
    pub id: i64,
    pub user_id: i64,
    pub provider: String,
    pub label: String,
    pub status: String,
    pub credit: String,
    pub last_checked: i64,
    pub last_error: String,
    pub created_at: i64,
}

impl CloudAccount {
    pub fn status_class(&self) -> &'static str {
        match self.status.as_str() {
            "ok" => "ok",
            "invalid" => "bad",
            _ => "unchecked",
        }
    }
}

/// Payload Telegram Login Widget kirim sebagai query string.
#[derive(Debug, Clone, Deserialize)]
pub struct TelegramAuth {
    pub id: i64,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub photo_url: String,
    pub auth_date: i64,
    #[serde(default)]
    pub hash: String,
}

impl TelegramAuth {
    /// Field yang ikut ditandatangani (semua kecuali `hash`).
    pub fn signed_fields(&self) -> Vec<(String, String)> {
        vec![
            ("id".into(), self.id.to_string()),
            ("first_name".into(), self.first_name.clone()),
            ("last_name".into(), self.last_name.clone()),
            ("username".into(), self.username.clone()),
            ("photo_url".into(), self.photo_url.clone()),
            ("auth_date".into(), self.auth_date.to_string()),
        ]
        .into_iter()
        .filter(|(_, v)| !v.is_empty())
        .collect()
    }
}
