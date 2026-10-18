use axum::{
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "web/dist/"]
pub struct FrontendAssets;

const FAVICON: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><rect width="32" height="32" rx="7" fill="#ea580c"/><path d="M8 22V10h6.4a4 4 0 0 1 0 8H11v4H8Zm3-6.6h3.2a1.4 1.4 0 0 0 0-2.8H11v2.8Z" fill="#ffffff"/></svg>"##;

pub async fn static_handler(uri: Uri) -> Response {
    let raw_path = uri.path().trim_start_matches('/');
    let target = if raw_path.is_empty() {
        "index.html"
    } else {
        raw_path
    };

    if target == "favicon.svg" {
        return (
            [
                (header::CONTENT_TYPE, "image/svg+xml"),
                (header::CACHE_CONTROL, "public, max-age=86400"),
            ],
            FAVICON,
        )
            .into_response();
    }

    if let Some(file) = FrontendAssets::get(target) {
        let mime = mime_guess::from_path(target).first_or_octet_stream();
        return (
            [
                (header::CONTENT_TYPE, mime.as_ref()),
                (header::CACHE_CONTROL, "public, max-age=31536000"),
            ],
            file.data,
        )
            .into_response();
    }

    if raw_path.starts_with("api/") {
        return (StatusCode::NOT_FOUND, "API endpoint not found").into_response();
    }

    // SPA fallback: any page route serves index.html
    if let Some(index) = FrontendAssets::get("index.html") {
        return (
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            index.data,
        )
            .into_response();
    }

    (StatusCode::NOT_FOUND, "404 Not Found").into_response()
}

pub async fn favicon() -> Response {
    (
        [
            (header::CONTENT_TYPE, "image/svg+xml"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        FAVICON,
    )
        .into_response()
}
