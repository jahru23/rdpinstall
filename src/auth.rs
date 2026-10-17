//! Telegram Login Widget verification + cookie sessions.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use hmac::{Hmac, KeyInit, Mac};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::db::now;
use crate::error::{AppError, AppResult};
use crate::models::{TelegramAuth, User};
use crate::SharedState;

pub const SESSION_COOKIE: &str = "rdpi_sid";
const MAX_AUTH_AGE_SECS: i64 = 86_400;

/// Verifies the widget payload against `SHA256(bot_token)` as HMAC key.
pub fn verify_telegram(bot_token: &str, fields: &[(String, String)], hash: &str) -> bool {
    if bot_token.is_empty() || hash.is_empty() {
        return false;
    }
    let mut sorted = fields.to_vec();
    sorted.sort();
    let data_check_string = sorted
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("\n");

    let secret = Sha256::digest(bot_token.as_bytes());
    let mut mac = match Hmac::<Sha256>::new_from_slice(&secret) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(data_check_string.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());

    expected
        .as_bytes()
        .ct_eq(hash.to_lowercase().as_bytes())
        .into()
}

pub fn auth_date_fresh(auth_date: i64) -> bool {
    let age = now() - auth_date;
    (-300..=MAX_AUTH_AGE_SECS).contains(&age)
}

pub async fn upsert_user(state: &SharedState, tg: &TelegramAuth) -> AppResult<User> {
    let ts = now();
    sqlx::query(
        "INSERT INTO users(tg_id, username, first_name, last_name, photo_url, created_at, last_seen_at)
         VALUES(?,?,?,?,?,?,?)
         ON CONFLICT(tg_id) DO UPDATE SET
           username=excluded.username, first_name=excluded.first_name,
           last_name=excluded.last_name, photo_url=excluded.photo_url,
           last_seen_at=excluded.last_seen_at",
    )
    .bind(tg.id)
    .bind(&tg.username)
    .bind(&tg.first_name)
    .bind(&tg.last_name)
    .bind(&tg.photo_url)
    .bind(ts)
    .bind(ts)
    .execute(&state.pool)
    .await?;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE tg_id = ?")
        .bind(tg.id)
        .fetch_one(&state.pool)
        .await?;

    sqlx::query("INSERT OR IGNORE INTO balances(user_id, amount, updated_at) VALUES(?, 0, ?)")
        .bind(user.id)
        .bind(ts)
        .execute(&state.pool)
        .await?;

    Ok(user)
}

pub async fn create_session(
    state: &SharedState,
    user_id: i64,
    ip: &str,
    ua: &str,
) -> AppResult<String> {
    let token = crate::crypto::random_token(32);
    let ts = now();
    let expires = ts + state.cfg.session_ttl_hours * 3600;
    sqlx::query(
        "INSERT INTO sessions(token, user_id, created_at, expires_at, ip, ua) VALUES(?,?,?,?,?,?)",
    )
    .bind(&token)
    .bind(user_id)
    .bind(ts)
    .bind(expires)
    .bind(ip)
    .bind(&ua[..ua.len().min(200)])
    .execute(&state.pool)
    .await?;
    Ok(token)
}

pub async fn destroy_session(state: &SharedState, token: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM sessions WHERE token = ?")
        .bind(token)
        .execute(&state.pool)
        .await?;
    Ok(())
}

pub async fn user_for_session(state: &SharedState, token: &str) -> AppResult<Option<User>> {
    let row = sqlx::query_as::<_, User>(
        "SELECT u.* FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token = ? AND s.expires_at > ? AND u.banned = 0",
    )
    .bind(token)
    .bind(now())
    .fetch_optional(&state.pool)
    .await?;
    Ok(row)
}

pub fn session_cookie(state: &SharedState, token: &str) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, token.to_string()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(state.cfg.is_prod())
        .max_age(time::Duration::hours(state.cfg.session_ttl_hours))
        .build()
}

pub fn clear_cookie() -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(0))
        .build()
}

pub fn token_from(parts: &Parts) -> Option<String> {
    CookieJar::from_headers(&parts.headers)
        .get(SESSION_COOKIE)
        .map(|c| c.value().to_string())
        .filter(|v| !v.is_empty())
}

/// Extractor: route hanya untuk user login (kalau tidak -> 401).
pub struct CurrentUser(pub User);

impl FromRequestParts<SharedState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &SharedState,
    ) -> Result<Self, Self::Rejection> {
        let token = token_from(parts).ok_or(AppError::Unauthorized)?;
        let user = user_for_session(state, &token)
            .await?
            .ok_or(AppError::Unauthorized)?;
        Ok(CurrentUser(user))
    }
}

/// Extractor: user login opsional (halaman publik).
pub struct MaybeUser(pub Option<User>);

impl FromRequestParts<SharedState> for MaybeUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &SharedState,
    ) -> Result<Self, Self::Rejection> {
        let user = match token_from(parts) {
            Some(token) => user_for_session(state, &token).await.unwrap_or(None),
            None => None,
        };
        Ok(MaybeUser(user))
    }
}

pub fn client_ip(headers: &axum::http::HeaderMap) -> String {
    headers
        .get("cf-connecting-ip")
        .or_else(|| headers.get("x-real-ip"))
        .or_else(|| headers.get("x-forwarded-for"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or("").trim().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "123456:TESTTOKEN";
    // Dihitung independen dengan hmac-sha256 (python), bukan oleh kode ini:
    //   dcs = "auth_date=1730000000\nfirst_name=Jay\nid=12345\nusername=jayx"
    //   key = sha256(TOKEN)
    const VALID_HASH: &str = "9f92b70eddbad9942e81910ae434ef2627e2fa5dc516649419184cf631543281";

    fn fields() -> Vec<(String, String)> {
        vec![
            ("id".into(), "12345".into()),
            ("first_name".into(), "Jay".into()),
            ("username".into(), "jayx".into()),
            ("auth_date".into(), "1730000000".into()),
        ]
    }

    #[test]
    fn accepts_valid_signature() {
        assert!(verify_telegram(TOKEN, &fields(), VALID_HASH));
        assert!(verify_telegram(
            TOKEN,
            &fields(),
            &VALID_HASH.to_uppercase()
        ));
    }

    #[test]
    fn rejects_tampered_payload() {
        let mut extra = fields();
        extra.push(("photo_url".into(), "https://example.invalid/x.jpg".into()));
        assert!(!verify_telegram(TOKEN, &extra, VALID_HASH));

        let mut changed = fields();
        changed[1] = ("first_name".into(), "Someone".into());
        assert!(!verify_telegram(TOKEN, &changed, VALID_HASH));

        assert!(!verify_telegram("999:OTHER", &fields(), VALID_HASH));
        assert!(!verify_telegram(TOKEN, &fields(), ""));
        assert!(!verify_telegram("", &fields(), VALID_HASH));
    }

    #[test]
    fn field_order_does_not_matter() {
        let mut shuffled = fields();
        shuffled.reverse();
        assert!(verify_telegram(TOKEN, &shuffled, VALID_HASH));
    }

    #[test]
    fn auth_date_window() {
        let now = crate::db::now();
        assert!(auth_date_fresh(now));
        assert!(auth_date_fresh(now - 3600));
        assert!(!auth_date_fresh(now - 90_000));
        assert!(!auth_date_fresh(now + 3600));
    }
}
