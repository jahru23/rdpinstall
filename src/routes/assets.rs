use axum::http::header;
use axum::response::{IntoResponse, Response};

const CSS: &str = include_str!("../../static/style.css");
const FAVICON: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><rect width="32" height="32" rx="7" fill="#0b0b0d"/><path d="M8 22V10h6.4a4 4 0 0 1 0 8H11v4H8Zm3-6.6h3.2a1.4 1.4 0 0 0 0-2.8H11v2.8Z" fill="#e5484d"/></svg>"##;

pub async fn css() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        CSS,
    )
        .into_response()
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
