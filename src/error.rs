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
    #[error("template: {0}")]
    Template(#[from] askama::Error),
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
<title>{} — rdpinstall</title><link rel="stylesheet" href="/static/style.css"></head>
<body class="err"><main class="wrap"><div class="card">
<p class="err-code">{}</p><p class="err-msg">{}</p>
<p><a class="btn ghost" href="/">kembali</a></p>
</div></main></body></html>"#,
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
