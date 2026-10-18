use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database: {0}")]
    Db(#[from] sqlx::Error),
    #[error("{0}")]
    Any(#[from] anyhow::Error),
    #[error("{0}")]
    Bad(String),
    #[error("harus login")]
    Unauthorized,
    #[error("tidak ditemukan")]
    NotFound,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn bad(msg: impl Into<String>) -> Self {
        AppError::Bad(msg.into())
    }

    fn status(&self) -> StatusCode {
        match self {
            AppError::Bad(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            tracing::error!("{self}");
        } else {
            tracing::debug!("{self}");
        }
        let msg = match self {
            AppError::Any(ref e) if status.is_server_error() => format!("{e:#}"),
            ref e => e.to_string(),
        };
        let body = format!(
            r#"<!doctype html><html lang="id"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{} · rdpinstall</title><link rel="stylesheet" href="/assets/index.css"></head>
<body style="background:#fbfaf7;font-family:sans-serif;color:#0f172a;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0;">
<div style="background:#fff;border:1px solid #e8e4dc;border-radius:12px;padding:32px;text-align:center;max-width:400px;box-shadow:0 4px 20px rgba(0,0,0,0.05);">
<p style="font-size:32px;font-weight:bold;margin:0 0 8px;color:#ea580c;">{}</p>
<p style="color:#52525b;font-size:14px;margin:0 0 20px;">{}</p>
<p><a style="background:#ea580c;color:#fff;text-decoration:none;font-weight:bold;padding:10px 18px;border-radius:8px;font-size:14px;" href="/">kembali ke halaman depan</a></p>
</div></body></html>"#,
            status.as_u16(),
            status.as_u16(),
            html_escape(&msg)
        );
        (status, Html(body)).into_response()
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
