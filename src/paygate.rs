//! Klien tipis PayGate (gateway yang sudah jalan di host yang sama).
//!
//! Topup QRIS: buat invoice, tampilkan QR, lalu nilai status lewat polling.
//! Yang dikreditkan ke saldo adalah `base_amount`; `amount` (base + fee gateway)
//! hanya untuk ditampilkan ke pelanggan.

use std::time::Duration;

use serde::Deserialize;

pub const PROVIDER_QRIS: &str = "sumopod";
/// SumoPod menolak invoice dengan total di bawah Rp10.000.
pub const MIN_BASE_AMOUNT: i64 = 10_000;
pub const MAX_BASE_AMOUNT: i64 = 5_000_000;

#[derive(Debug)]
pub enum Error {
    Transport(String),
    Api { status: u16, message: String },
    Decode(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Transport(m) => write!(f, "Tidak bisa menghubungi PayGate: {m}"),
            Error::Api { status, message } => write!(f, "PayGate menolak ({status}): {message}"),
            Error::Decode(m) => write!(f, "Jawaban PayGate tidak terbaca: {m}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Invoice {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub reference_id: String,
    #[serde(default)]
    pub base_amount: i64,
    #[serde(default)]
    pub service_fee: i64,
    #[serde(default)]
    pub amount: i64,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub qris: String,
    #[serde(default)]
    pub qris_id: String,
    #[serde(default)]
    pub payment_link_url: String,
    #[serde(default)]
    pub transaction_id: Option<String>,
    #[serde(default)]
    pub paid_at: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
}

impl Invoice {
    pub fn is_paid(&self) -> bool {
        self.status.eq_ignore_ascii_case("paid")
    }

    pub fn is_final(&self) -> bool {
        matches!(
            self.status.to_ascii_lowercase().as_str(),
            "paid" | "expired" | "cancelled" | "failed"
        )
    }

    pub fn credit_amount(&self) -> i64 {
        if self.base_amount > 0 {
            self.base_amount
        } else {
            self.amount
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Created {
    pub invoice: Invoice,
    /// URL PNG QRIS yang sudah bisa langsung dipasang di <img src>.
    pub qr_png_url: String,
    pub qr_url: String,
    pub pay_url: String,
    pub raw: serde_json::Value,
}

pub fn valid_amount(base: i64) -> Result<(), String> {
    if base < MIN_BASE_AMOUNT {
        return Err(format!(
            "Nominal minimal Rp{} (biaya gateway membuat total di bawah batas SumoPod).",
            MIN_BASE_AMOUNT
        ));
    }
    if base > MAX_BASE_AMOUNT {
        return Err(format!("Nominal maksimal Rp{MAX_BASE_AMOUNT} per topup."));
    }
    Ok(())
}

pub struct Client {
    http: reqwest::Client,
    base: String,
    key: String,
    public_base: String,
}

impl Client {
    pub fn new(base: &str, key: &str, public_base: &str) -> Result<Self, Error> {
        if key.trim().is_empty() {
            return Err(Error::Api {
                status: 0,
                message: "PAYGATE_API_KEY belum diset".into(),
            });
        }
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(25))
            .user_agent(concat!("rdpinstall/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| Error::Transport(e.to_string()))?;
        Ok(Self {
            http,
            base: base.trim_end_matches('/').to_string(),
            key: key.trim().to_string(),
            public_base: public_base.trim_end_matches('/').to_string(),
        })
    }

    /// PayGate membangun URL QR dari host yang dipakai memanggilnya. Karena kita
    /// memanggilnya lewat 127.0.0.1, URL yang kembali menunjuk ke localhost dan
    /// tidak bisa dibuka browser user — jadi host loopback diganti dengan alamat
    /// publik gateway, sementara path + query dipertahankan.
    fn absolute(&self, url: &str) -> String {
        if url.is_empty() {
            return String::new();
        }
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return format!("{}{}", self.public_base, ensure_slash(url));
        }
        let rest = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
            .unwrap_or("");
        match rest.split_once('/') {
            Some((host, path)) if is_loopback(host) => {
                format!("{}/{}", self.public_base, path)
            }
            None if is_loopback(rest) => self.public_base.clone(),
            _ => url.to_string(),
        }
    }

    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Error> {
        let url = format!("{}{}", self.base, path);
        let mut req = self
            .http
            .request(method, &url)
            .header("X-API-Key", &self.key)
            .header("Accept", "application/json");
        if let Some(b) = body {
            req = req.json(&b);
        }
        let res = req
            .send()
            .await
            .map_err(|e| Error::Transport(e.to_string()))?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: first_error(&text),
            });
        }
        serde_json::from_str(&text).map_err(|e| Error::Decode(e.to_string()))
    }

    pub async fn create_invoice(
        &self,
        base_amount: i64,
        reference_id: &str,
        expires_in: i64,
    ) -> Result<Created, Error> {
        let body = serde_json::json!({
            "amount": base_amount,
            "provider": PROVIDER_QRIS,
            "reference_id": reference_id,
            "expires_in": expires_in,
        });
        let v = self
            .request(reqwest::Method::POST, "/v1/invoices", Some(body))
            .await?;

        let data = v.get("data").cloned().unwrap_or_default();
        let invoice: Invoice = data
            .get("invoice")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| Error::Decode(e.to_string()))?
            .ok_or_else(|| Error::Decode("respons tanpa data.invoice".into()))?;

        let qr_png_url = data
            .get("qris_png_url")
            .and_then(|v| v.as_str())
            .map(|s| self.absolute(s))
            .unwrap_or_default();
        let qr_url = data
            .get("qris_url")
            .and_then(|v| v.as_str())
            .map(|s| self.absolute(s))
            .unwrap_or_default();
        let pay_url = data
            .pointer("/urls/payment_url")
            .or_else(|| data.get("payment_url"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| invoice.payment_link_url.clone());

        Ok(Created {
            invoice,
            qr_png_url,
            qr_url,
            pay_url,
            raw: v,
        })
    }

    pub async fn invoice(&self, invoice_id: &str) -> Result<Invoice, Error> {
        let v = self
            .request(
                reqwest::Method::GET,
                &format!("/v1/invoices/{invoice_id}"),
                None,
            )
            .await?;
        let inv = v
            .pointer("/data/invoice")
            .cloned()
            .ok_or_else(|| Error::Decode("respons tanpa data.invoice".into()))?;
        serde_json::from_value(inv).map_err(|e| Error::Decode(e.to_string()))
    }

    pub async fn cancel_invoice(&self, invoice_id: &str) -> Result<(), Error> {
        self.request(
            reqwest::Method::POST,
            &format!("/v1/invoices/{invoice_id}/cancel"),
            None,
        )
        .await
        .map(|_| ())
    }
}

fn ensure_slash(url: &str) -> String {
    if url.starts_with('/') {
        url.to_string()
    } else {
        format!("/{url}")
    }
}

fn is_loopback(host_port: &str) -> bool {
    let host = host_port.split(':').next().unwrap_or(host_port);
    matches!(
        host,
        "127.0.0.1" | "localhost" | "0.0.0.0" | "[::1]" | "::1"
    )
}

fn first_error(body: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(v) => {
            let msg = v
                .pointer("/error/message")
                .or_else(|| v.pointer("/error/code"))
                .and_then(|x| x.as_str());
            match msg {
                Some(m) => m.to_string(),
                None => body.chars().take(200).collect(),
            }
        }
        Err(_) => body.chars().take(200).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Contoh respons diambil dari docs/INTEGRATION.md PayGate.
    const CREATED: &str = r#"{
      "success": true,
      "data": {
        "invoice": {
          "id": "inv_a1b2c3d4e5f6g7h8",
          "provider": "sumopod",
          "reference_id": "rdpi-1-abcd",
          "base_amount": 15000,
          "service_fee": 405,
          "amount": 15405,
          "status": "pending",
          "qris": "00020101021226610014ID.CO.QRIS.WWW",
          "qris_id": "PG-rdpi-1-abcd-1",
          "payment_link_url": "https://pay.sumopod.com/p/pay_abc"
        },
        "qris_url": "/qr/PG-rdpi-1-abcd-1",
        "qris_png_url": "/qr/PG-rdpi-1-abcd-1?format=png"
      }
    }"#;

    const PAID: &str = r#"{
      "success": true,
      "data": {
        "invoice": {
          "id": "inv_a1b2c3d4e5f6g7h8",
          "amount": 15405,
          "base_amount": 15000,
          "service_fee": 405,
          "status": "paid",
          "transaction_id": "pay_123",
          "issuer": "SUMOPOD",
          "paid_at": "2026-07-21T08:00:00.000Z"
        }
      }
    }"#;

    fn parse(body: &str) -> Invoice {
        let v: serde_json::Value = serde_json::from_str(body).unwrap();
        serde_json::from_value(v.pointer("/data/invoice").unwrap().clone()).unwrap()
    }

    #[test]
    fn parses_created_invoice() {
        let inv = parse(CREATED);
        assert_eq!(inv.id, "inv_a1b2c3d4e5f6g7h8");
        assert_eq!(inv.base_amount, 15000);
        assert_eq!(inv.amount, 15405);
        assert_eq!(inv.service_fee, 405);
        assert_eq!(inv.status, "pending");
        assert!(!inv.is_final());
        assert_eq!(
            inv.credit_amount(),
            15000,
            "yang dikreditkan base_amount, bukan total"
        );
    }

    #[test]
    fn parses_paid_invoice_and_detects_final() {
        let inv = parse(PAID);
        assert!(inv.is_paid());
        assert!(inv.is_final());
        assert_eq!(inv.transaction_id.as_deref(), Some("pay_123"));
    }

    #[test]
    fn credit_amount_falls_back_to_total_when_base_missing() {
        let inv = Invoice {
            amount: 12000,
            status: "paid".into(),
            ..Default::default()
        };
        assert_eq!(inv.credit_amount(), 12000);
    }

    #[test]
    fn amount_bounds() {
        assert!(valid_amount(10_000).is_ok());
        assert!(valid_amount(50_000).is_ok());
        assert!(
            valid_amount(9_999).is_err(),
            "SumoPod menolak di bawah 10rb"
        );
        assert!(valid_amount(5_000_001).is_err());
    }

    #[test]
    fn loopback_host_from_gateway_is_rewritten_to_public_base() {
        let c = Client::new("http://127.0.0.1:3001", "k", "https://pg-v1.jaystore.top").unwrap();
        // inilah yang benar-benar dikembalikan PayGate saat dipanggil via localhost
        assert_eq!(
            c.absolute("http://127.0.0.1:3001/qr/PG-x-1?format=png"),
            "https://pg-v1.jaystore.top/qr/PG-x-1?format=png"
        );
        assert_eq!(
            c.absolute("http://localhost:3001/qr/abc"),
            "https://pg-v1.jaystore.top/qr/abc"
        );
        // host luar (mis. halaman checkout SumoPod) dibiarkan apa adanya
        assert_eq!(
            c.absolute("https://checkout.pymnt.app/payment-links/abc"),
            "https://checkout.pymnt.app/payment-links/abc"
        );
    }

    #[test]
    fn relative_qr_url_becomes_absolute() {
        let c = Client::new("http://127.0.0.1:3001", "k", "https://pg-v1.jaystore.top/").unwrap();
        assert_eq!(
            c.absolute("/qr/abc?format=png"),
            "https://pg-v1.jaystore.top/qr/abc?format=png"
        );
        assert_eq!(c.absolute("https://x/y.png"), "https://x/y.png");
        assert_eq!(c.absolute(""), "");
    }

    #[test]
    fn extracts_gateway_error_message() {
        let body =
            r#"{"success":false,"error":{"code":"MIN_AMOUNT","message":"minimal Rp10.000"}}"#;
        assert_eq!(first_error(body), "minimal Rp10.000");
    }
}
