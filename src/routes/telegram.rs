use askama::Template;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::CookieJar;

use crate::auth;
use crate::models::TelegramAuth;
use crate::views::LoginPage;
use crate::SharedState;

fn bad_login(state: &SharedState, msg: &str) -> Response {
    let page = LoginPage {
        bot_username: state.cfg.bot_username.clone(),
        error: msg.to_string(),
        dev_login: state.cfg.dev_login,
    };
    let body = page
        .render()
        .unwrap_or_else(|_| "<p>login error</p>".to_string());
    (
        axum::http::StatusCode::UNAUTHORIZED,
        axum::response::Html(body),
    )
        .into_response()
}

/// Telegram Login Widget redirect: /auth/telegram?id=..&hash=..
pub async fn callback(
    State(state): State<SharedState>,
    jar: CookieJar,
    headers: HeaderMap,
    Query(tg): Query<TelegramAuth>,
) -> Response {
    if !auth::verify_telegram(&state.cfg.bot_token, &tg.signed_fields(), &tg.hash) {
        tracing::warn!("login Telegram ditolak: hash tidak cocok (tg_id={})", tg.id);
        return bad_login(&state, "Verifikasi Telegram gagal. Coba lagi.");
    }
    if !auth::auth_date_fresh(tg.auth_date) {
        return bad_login(&state, "Sesi Telegram kedaluwarsa. Ulangi login.");
    }

    let user = match auth::upsert_user(&state, &tg).await {
        Ok(u) => u,
        Err(e) => return e.into_response(),
    };
    if user.banned != 0 {
        return bad_login(&state, "Akun ini diblokir.");
    }

    let ip = auth::client_ip(&headers);
    let ua = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let token = match auth::create_session(&state, user.id, &ip, ua).await {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    tracing::info!("login ok: user {} ({})", user.id, user.handle());

    (
        jar.add(auth::session_cookie(&state, &token)),
        Redirect::to("/"),
    )
        .into_response()
}

/// Login cepat tanpa Telegram — hanya aktif kalau RDPINSTALL_DEV_LOGIN=1 & bukan prod.
pub async fn dev_login(State(state): State<SharedState>, jar: CookieJar) -> Response {
    if !state.cfg.dev_login {
        return Redirect::to("/login").into_response();
    }
    let tg = TelegramAuth {
        id: 1,
        first_name: "Dev".into(),
        last_name: "Local".into(),
        username: "devlocal".into(),
        photo_url: String::new(),
        auth_date: crate::db::now(),
        hash: String::new(),
    };
    let user = match auth::upsert_user(&state, &tg).await {
        Ok(u) => u,
        Err(e) => return e.into_response(),
    };
    let token = match auth::create_session(&state, user.id, "127.0.0.1", "dev").await {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    (
        jar.add(auth::session_cookie(&state, &token)),
        Redirect::to("/"),
    )
        .into_response()
}

pub async fn logout(State(state): State<SharedState>, jar: CookieJar) -> Response {
    if let Some(c) = jar.get(auth::SESSION_COOKIE) {
        let _ = auth::destroy_session(&state, c.value()).await;
    }
    (jar.add(auth::clear_cookie()), Redirect::to("/login")).into_response()
}
