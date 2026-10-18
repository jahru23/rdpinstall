//! Klien SSH (russh) untuk mode "cloud lain".
//!
//! Hanya yang dibutuhkan: autentikasi password, jalankan satu perintah, dan
//! cek port TCP. Kunci host pelanggan diterima pada pemakaian pertama, seperti
//! SSH biasa, dan sidik jarinya dicatat ke log supaya bisa diperiksa.

use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, Handle};
use russh::keys::PublicKeyOrCertificate;
use russh::{ChannelMsg, Disconnect};

#[derive(Debug)]
pub enum Error {
    Connect(String),
    Auth,
    Exec(String),
    Timeout(u64),
    Russh(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Connect(m) => write!(f, "Tidak bisa terhubung ke VPS: {m}"),
            Error::Auth => f.write_str("Username atau password SSH salah."),
            Error::Exec(m) => write!(f, "Perintah gagal dijalankan: {m}"),
            Error::Timeout(s) => write!(f, "Perintah tidak selesai dalam {s} detik."),
            Error::Russh(m) => write!(f, "Kesalahan SSH: {m}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Default)]
pub struct Output {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
    pub code: u32,
}

pub struct Handler;

impl client::Handler for Handler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        // Diterima (trust on first use, sama seperti ssh pada umumnya), tapi
        // sidik jarinya dicatat agar bisa dicocokkan pelanggan bila perlu.
        if let PublicKeyOrCertificate::PublicKey { key, .. } = server_public_key {
            tracing::info!(
                "ssh host key {}",
                key.fingerprint(russh::keys::HashAlg::Sha256)
            );
        }
        Ok(true)
    }
}

pub async fn connect(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
    timeout: Duration,
) -> Result<Handle<Handler>, Error> {
    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(30)),
        ..Default::default()
    });
    let mut handle = tokio::time::timeout(timeout, client::connect(config, (host, port), Handler))
        .await
        .map_err(|_| Error::Timeout(timeout.as_secs()))?
        .map_err(|e| Error::Connect(e.to_string()))?;

    let res = handle
        .authenticate_password(user, password)
        .await
        .map_err(|e| Error::Russh(e.to_string()))?;
    if !res.success() {
        return Err(Error::Auth);
    }
    Ok(handle)
}

/// Jalankan satu perintah, kumpulkan stdout+stderr, hormati batas waktu.
pub async fn exec(handle: &Handle<Handler>, cmd: &str, timeout: Duration) -> Result<Output, Error> {
    let fut = async {
        let mut channel = handle
            .channel_open_session()
            .await
            .map_err(|e| Error::Russh(e.to_string()))?;
        channel
            .exec(true, cmd)
            .await
            .map_err(|e| Error::Exec(e.to_string()))?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut code = None;
        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { ref data } => stdout.extend_from_slice(data),
                ChannelMsg::ExtendedData { ref data, .. } => stderr.extend_from_slice(data),
                ChannelMsg::ExitStatus { exit_status } => code = Some(exit_status),
                _ => {}
            }
        }
        Ok::<Output, Error>(Output {
            ok: code.unwrap_or(1) == 0,
            stdout: String::from_utf8_lossy(&stdout).to_string(),
            stderr: String::from_utf8_lossy(&stderr).to_string(),
            code: code.unwrap_or(1),
        })
    };

    tokio::time::timeout(timeout, fut)
        .await
        .map_err(|_| Error::Timeout(timeout.as_secs()))?
}

pub async fn disconnect(handle: Handle<Handler>) {
    let _ = handle
        .disconnect(Disconnect::ByApplication, "selesai", "")
        .await;
}

/// Cek port TCP. Dipakai untuk memantau: SSH terbuka = masih Linux/mengunduh,
/// SSH tertutup = sedang me-reboot (normal), RDP terbuka = Windows siap.
pub async fn tcp_open(host: &str, port: u16, timeout: Duration) -> bool {
    matches!(
        tokio::time::timeout(timeout, tokio::net::TcpStream::connect((host, port))).await,
        Ok(Ok(_))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_messages_are_readable() {
        assert!(Error::Auth.to_string().contains("password SSH salah"));
        assert!(Error::Timeout(60).to_string().contains("60 detik"));
    }

    #[tokio::test]
    async fn tcp_open_is_false_for_closed_port() {
        // port tinggi yang hampir pasti kosong di loopback
        assert!(!tcp_open("127.0.0.1", 59999, Duration::from_millis(300)).await);
    }

    #[tokio::test]
    async fn tcp_open_is_true_for_open_port() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let _ = listener.accept().await;
        });
        assert!(tcp_open("127.0.0.1", port, Duration::from_secs(2)).await);
    }
}
