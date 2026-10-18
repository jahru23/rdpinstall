//! Akun cloud: user menempel API token miliknya sendiri, kita uji dulu ke
//! UpCloud, lalu simpan terenkripsi. Token tidak pernah ditampilkan kembali.

use axum::extract::{Form, Path, State};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::auth::CurrentUser;
use crate::crypto;
use crate::db::now;
use crate::error::{AppError, AppResult};
use crate::models::{CloudAccount, User};
use crate::routes::pages::render;
use crate::upcloud;
use crate::views::CloudPage;
use crate::SharedState;

const MAX_ACCOUNTS_PER_USER: i64 = 5;

#[derive(Debug, Deserialize)]
pub struct AddForm {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub token: String,
}

pub async fn index(State(state): State<SharedState>, user: crate::auth::MaybeUser) -> Response {
    let crate::auth::MaybeUser(Some(user)) = user else {
        return Redirect::to("/login").into_response();
    };
    match build_page(&state, user, "", "").await {
        Ok(r) => r,
        Err(e) => e.into_response(),
    }
}

pub async fn add(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Form(form): Form<AddForm>,
) -> Response {
    let token = form.token.trim().to_string();
    let label = form.label.trim().to_string();

    if !upcloud::looks_like_token(&token) {
        return err_page(
            &state,
            user,
            "Token UpCloud harus diawali `ucat_` (dibuat di Control Panel → Account → API Tokens).",
        )
        .await;
    }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cloud_accounts WHERE user_id = ?")
        .bind(user.id)
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    if count >= MAX_ACCOUNTS_PER_USER {
        return err_page(
            &state,
            user,
            "Batas akun cloud tercapai. Hapus salah satu dulu.",
        )
        .await;
    }

    // Uji token sebelum disimpan: sekaligus mengambil sisa kredit.
    let client = match upcloud::Client::new(&token) {
        Ok(c) => c,
        Err(e) => return err_page(&state, user, &e.to_string()).await,
    };
    let account = match client.account().await {
        Ok(a) => a,
        Err(e) => return err_page(&state, user, &e.to_string()).await,
    };

    let sealed = match crypto::seal(&state.cfg.master_key, &token) {
        Ok(s) => s,
        Err(e) => return AppError::Any(e).into_response(),
    };
    let ts = now();
    let ins = sqlx::query(
        "INSERT INTO cloud_accounts(user_id, provider, label, token_enc, status, credit, last_checked, last_error, created_at)
         VALUES(?, 'upcloud', ?, ?, 'ok', ?, ?, '', ?)",
    )
    .bind(user.id)
    .bind(if label.is_empty() { "UpCloud".to_string() } else { label })
    .bind(&sealed)
    .bind(account.credits_label())
    .bind(ts)
    .bind(ts)
    .execute(&state.pool)
    .await;

    match ins {
        Ok(r) => {
            audit(
                &state,
                user.id,
                "cloud_add",
                &format!("upcloud#{}", r.last_insert_rowid()),
            )
            .await;
            tracing::info!(
                "akun cloud ditambahkan user {} (kredit {:.2})",
                user.id,
                account.credits
            );
            let notice = format!("Token valid. Sisa kredit {:.2}.", account.credits);
            match build_page(&state, user, "", &notice).await {
                Ok(r) => r,
                Err(e) => e.into_response(),
            }
        }
        Err(e) => AppError::Db(e).into_response(),
    }
}

pub async fn check(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> Response {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT token_enc, label FROM cloud_accounts WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await;

    let (token_enc, _label) = match row {
        Ok(Some(r)) => r,
        Ok(None) => return AppError::NotFound.into_response(),
        Err(e) => return AppError::Db(e).into_response(),
    };

    let token = match crypto::open(&state.cfg.master_key, &token_enc) {
        Ok(t) => t,
        Err(e) => return AppError::Any(e).into_response(),
    };

    let (status, credit, err) = match upcloud::Client::new(&token) {
        Ok(client) => match client.account().await {
            Ok(a) => ("ok".to_string(), a.credits_label(), String::new()),
            Err(e) => ("invalid".to_string(), String::new(), e.to_string()),
        },
        Err(e) => ("invalid".to_string(), String::new(), e.to_string()),
    };

    let _ = sqlx::query(
        "UPDATE cloud_accounts SET status = ?, credit = ?, last_error = ?, last_checked = ? WHERE id = ? AND user_id = ?",
    )
    .bind(&status)
    .bind(&credit)
    .bind(&err)
    .bind(now())
    .bind(id)
    .bind(user.id)
    .execute(&state.pool)
    .await;

    audit(
        &state,
        user.id,
        "cloud_check",
        &format!("upcloud#{id} -> {status}"),
    )
    .await;

    let notice = if status == "ok" {
        format!("Akun #{id} masih valid ({credit}).")
    } else {
        format!("Akun #{id} gagal diuji: {err}")
    };
    match build_page(&state, user, "", &notice).await {
        Ok(r) => r,
        Err(e) => e.into_response(),
    }
}

pub async fn delete(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> Response {
    let res = sqlx::query("DELETE FROM cloud_accounts WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user.id)
        .execute(&state.pool)
        .await;
    match res {
        Ok(_) => {
            audit(&state, user.id, "cloud_delete", &format!("upcloud#{id}")).await;
            Redirect::to("/cloud").into_response()
        }
        Err(e) => AppError::Db(e).into_response(),
    }
}

async fn audit(state: &SharedState, user_id: i64, action: &str, detail: &str) {
    let _ = sqlx::query(
        "INSERT INTO audit_log(user_id, action, detail, ip, created_at) VALUES(?,?,?,'',?)",
    )
    .bind(user_id)
    .bind(action)
    .bind(detail)
    .bind(now())
    .execute(&state.pool)
    .await;
}

async fn accounts(state: &SharedState, user_id: i64) -> AppResult<Vec<CloudAccount>> {
    let rows = sqlx::query_as::<_, CloudAccount>(
        "SELECT id, user_id, provider, label, status, credit, last_checked, last_error, created_at
         FROM cloud_accounts WHERE user_id = ? ORDER BY id DESC",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(rows)
}

async fn build_page(
    state: &SharedState,
    user: User,
    error: &str,
    notice: &str,
) -> AppResult<Response> {
    let accounts = accounts(state, user.id).await?;
    let page = CloudPage {
        user,
        accounts,
        error: error.to_string(),
        notice: notice.to_string(),
        max_accounts: MAX_ACCOUNTS_PER_USER,
    };
    Ok(render(&page))
}

async fn err_page(state: &SharedState, user: User, msg: &str) -> Response {
    match build_page(state, user, msg, "").await {
        Ok(r) => r,
        Err(e) => e.into_response(),
    }
}
