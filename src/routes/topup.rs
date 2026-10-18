//! Topup saldo lewat QRIS PayGate. Saldo dikreditkan hanya sekali per invoice:
//! klaim dilakukan dengan UPDATE bersyarat `status='pending'`, jadi polling
//! berulang atau refresh tidak akan menambah saldo dua kali.

use askama::Template;
use axum::extract::{Form, Path, State};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::auth::{CurrentUser, MaybeUser};
use crate::db::now;
use crate::error::{AppError, AppResult};
use crate::models::Payment;
use crate::paygate;
use crate::routes::pages::render;
use crate::views::{TopupDetailPage, TopupPage, TopupStatusCard};
use crate::SharedState;

const PRESETS: [i64; 4] = [15_000, 25_000, 50_000, 100_000];
const INVOICE_TTL_SECS: i64 = 3_600;

#[derive(Debug, Deserialize)]
pub struct TopupForm {
    #[serde(default)]
    pub amount: String,
}

fn client(state: &SharedState) -> Result<paygate::Client, paygate::Error> {
    paygate::Client::new(
        &state.cfg.paygate_url,
        &state.cfg.paygate_key,
        &state.cfg.paygate_public_url,
    )
}

async fn balance(state: &SharedState, user_id: i64) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT amount FROM balances WHERE user_id = ?")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(0)
}

async fn history(state: &SharedState, user_id: i64) -> AppResult<Vec<Payment>> {
    Ok(sqlx::query_as::<_, Payment>(
        "SELECT id, user_id, gateway, ref, gateway_id, amount, total_amount, status,
                qr_string, pay_url, txn_id, created_at, paid_at, expires_at
         FROM payments WHERE user_id = ? ORDER BY id DESC LIMIT 10",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?)
}

async fn page(
    state: &SharedState,
    user: crate::models::User,
    error: &str,
    notice: &str,
) -> Response {
    let bal = balance(state, user.id).await;
    let rows = history(state, user.id).await.unwrap_or_default();
    let p = TopupPage {
        user,
        balance: bal,
        payments: rows,
        presets: PRESETS.to_vec(),
        min: paygate::min_amount(&state.cfg.paygate_provider),
        max: paygate::MAX_BASE_AMOUNT,
        error: error.to_string(),
        notice: notice.to_string(),
    };
    render(&p)
}

pub async fn index(State(state): State<SharedState>, user: MaybeUser) -> Response {
    let MaybeUser(Some(user)) = user else {
        return Redirect::to("/masuk").into_response();
    };
    page(&state, user, "", "").await
}

pub async fn create(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Form(form): Form<TopupForm>,
) -> Response {
    let amount: i64 = match form.amount.trim().parse() {
        Ok(a) => a,
        Err(_) => return page(&state, user, "Nominal tidak valid.", "").await,
    };
    if let Err(msg) = paygate::valid_amount(amount, &state.cfg.paygate_provider) {
        return page(&state, user, &msg, "").await;
    }

    let reference = format!("rdpi-{}-{}", user.id, crate::crypto::random_token(4));
    let client = match client(&state) {
        Ok(c) => c,
        Err(e) => return page(&state, user, &e.to_string(), "").await,
    };

    let provider = state.cfg.paygate_provider.clone();
    let created = match client
        .create_invoice(amount, &reference, INVOICE_TTL_SECS, &provider)
        .await
    {
        Ok(c) => c,
        Err(e) => return page(&state, user, &e.to_string(), "").await,
    };

    let ts = now();
    let raw = serde_json::to_string(&created.raw).unwrap_or_default();
    let ins = sqlx::query(
        "INSERT INTO payments(user_id, gateway, ref, gateway_id, amount, total_amount, status,
                              qr_string, pay_url, raw_json, created_at, expires_at)
         VALUES(?, 'paygate', ?, ?, ?, ?, 'pending', ?, ?, ?, ?, ?)",
    )
    .bind(user.id)
    .bind(&reference)
    .bind(&created.invoice.id)
    .bind(created.invoice.credit_amount())
    .bind(created.invoice.amount)
    .bind(&created.qr_png_url)
    .bind(&created.pay_url)
    .bind(&raw)
    .bind(ts)
    .bind(ts + INVOICE_TTL_SECS)
    .execute(&state.pool)
    .await;

    match ins {
        Ok(_) => {
            tracing::info!(
                "topup {} dibuat: {} -> total {}",
                reference,
                created.invoice.credit_amount(),
                created.invoice.amount
            );
            Redirect::to(&format!("/topup/{reference}")).into_response()
        }
        Err(e) => AppError::Db(e).into_response(),
    }
}

pub async fn detail(
    State(state): State<SharedState>,
    user: MaybeUser,
    Path(reference): Path<String>,
) -> Response {
    let MaybeUser(Some(user)) = user else {
        return Redirect::to("/masuk").into_response();
    };
    let card = match build_card(&state, user.id, &reference).await {
        Ok(Some(c)) => c,
        Ok(None) => return AppError::NotFound.into_response(),
        Err(e) => return e.into_response(),
    };
    let card_html = match card.render() {
        Ok(h) => h,
        Err(e) => return AppError::Template(e).into_response(),
    };
    render(&TopupDetailPage {
        user,
        reference,
        card_html,
    })
}

/// Dipanggil berkala oleh halaman detail: nilai status ke gateway lalu kirim kartu.
pub async fn status(
    State(state): State<SharedState>,
    user: MaybeUser,
    Path(reference): Path<String>,
) -> Response {
    let MaybeUser(Some(user)) = user else {
        return AppError::Unauthorized.into_response();
    };
    let row = match load(state.clone(), user.id, &reference).await {
        Ok(Some(r)) => r,
        Ok(None) => return AppError::NotFound.into_response(),
        Err(e) => return e.into_response(),
    };

    if row.status == "pending" {
        if let Ok(c) = client(&state) {
            // GoBiz tidak punya webhook upstream yang instan: paksa cek dulu
            // supaya pembayaran terlihat tanpa menunggu siklus worker gateway.
            let forced = if state.cfg.paygate_provider == "gobiz" {
                c.check_invoice(&row.gateway_id).await.ok().flatten()
            } else {
                None
            };
            let fetched = match forced {
                Some(inv) => Ok(inv),
                None => c.invoice(&row.gateway_id).await,
            };
            if let Ok(inv) = fetched {
                if let Err(e) = settle(&state, &row, &inv).await {
                    tracing::warn!("settle {} gagal: {e}", row.reference);
                }
            }
        }
    }

    match build_card(&state, user.id, &reference).await {
        Ok(Some(c)) => render(&c),
        Ok(None) => AppError::NotFound.into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn cancel(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(reference): Path<String>,
) -> Response {
    let row = match load(state.clone(), user.id, &reference).await {
        Ok(Some(r)) => r,
        Ok(None) => return AppError::NotFound.into_response(),
        Err(e) => return e.into_response(),
    };
    if row.status == "pending" {
        if let Ok(c) = client(&state) {
            let _ = c.cancel_invoice(&row.gateway_id).await;
        }
        let _ =
            sqlx::query("UPDATE payments SET status='cancelled' WHERE id=? AND status='pending'")
                .bind(row.id)
                .execute(&state.pool)
                .await;
    }
    Redirect::to(&format!("/topup/{reference}")).into_response()
}

async fn load(state: SharedState, user_id: i64, reference: &str) -> AppResult<Option<Payment>> {
    Ok(sqlx::query_as::<_, Payment>(
        "SELECT id, user_id, gateway, ref, gateway_id, amount, total_amount, status,
                qr_string, pay_url, txn_id, created_at, paid_at, expires_at
         FROM payments WHERE ref = ? AND user_id = ?",
    )
    .bind(reference)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?)
}

/// Klaim pembayaran: satu transaksi, satu kali kredit.
async fn settle(state: &SharedState, row: &Payment, inv: &paygate::Invoice) -> AppResult<bool> {
    let ts = now();
    if inv.is_paid() {
        let txn = inv.transaction_id.clone().unwrap_or_default();
        let mut tx = state.pool.begin().await?;
        let claimed = sqlx::query(
            "UPDATE payments SET status='paid', txn_id=?, paid_at=?, total_amount=? WHERE id=? AND status='pending'",
        )
        .bind(&txn)
        .bind(ts)
        .bind(if inv.amount > 0 { inv.amount } else { row.total_amount })
        .bind(row.id)
        .execute(&mut *tx)
        .await?;

        if claimed.rows_affected() == 1 {
            let credit = inv.credit_amount();
            sqlx::query(
                "INSERT OR IGNORE INTO balances(user_id, amount, updated_at) VALUES(?, 0, ?)",
            )
            .bind(row.user_id)
            .bind(ts)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "UPDATE balances SET amount = amount + ?, updated_at = ? WHERE user_id = ?",
            )
            .bind(credit)
            .bind(ts)
            .bind(row.user_id)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "INSERT INTO ledger(user_id, delta, kind, ref, note, created_at) VALUES(?,?,'topup',?,?,?)",
            )
            .bind(row.user_id)
            .bind(credit)
            .bind(&row.reference)
            .bind("topup QRIS via PayGate")
            .bind(ts)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            tracing::info!(
                "topup {} lunas: +{} saldo user {}",
                row.reference,
                credit,
                row.user_id
            );
            return Ok(true);
        }
        tx.commit().await?;
        return Ok(false);
    }

    if inv.is_final() {
        sqlx::query("UPDATE payments SET status=? WHERE id=? AND status='pending'")
            .bind(&inv.status)
            .bind(row.id)
            .execute(&state.pool)
            .await?;
    }
    Ok(false)
}

async fn build_card(
    state: &SharedState,
    user_id: i64,
    reference: &str,
) -> AppResult<Option<TopupStatusCard>> {
    let row = match load(state.clone(), user_id, reference).await? {
        Some(r) => r,
        None => return Ok(None),
    };
    let bal = balance(state, user_id).await;
    let p = row;
    // GoBiz tidak memisahkan biaya (base_amount/service_fee null), SumoPod menagih
    // nominal + fee. Catatan ini dibentuk di sini supaya template tetap sederhana.
    let fee_note = if p.total_amount > p.amount && p.amount > 0 {
        format!(
            "Ditagih {} (nominal {} + biaya gateway). Saldo yang masuk {}.",
            crate::views::rupiah(p.total_amount),
            crate::views::rupiah(p.amount),
            crate::views::rupiah(p.amount)
        )
    } else if p.total_amount > 0 {
        format!(
            "Ditagih {}. Saldo yang masuk sebesar itu.",
            crate::views::rupiah(p.total_amount)
        )
    } else {
        "Nominal ditagih dikonfirmasi gateway sebelum pembayaran.".to_string()
    };
    Ok(Some(TopupStatusCard {
        p,
        balance: bal,
        min: paygate::min_amount(&state.cfg.paygate_provider),
        fee_note,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Gateway tiruan: selalu menjawab body yang sama untuk semua request.
    async fn mock_gateway(body: &'static str) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            while let Ok((mut sock, _)) = listener.accept().await {
                let mut buf = [0u8; 4096];
                let _ = sock.read(&mut buf).await;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.shutdown().await;
            }
        });
        format!("http://{addr}")
    }

    fn config(db_path: &str, paygate_base: &str) -> Config {
        Config {
            env: "test".into(),
            bind: "127.0.0.1:0".into(),
            base_url: "http://127.0.0.1".into(),
            db_path: db_path.to_string(),
            bot_token: String::new(),
            bot_username: String::new(),
            master_key: [9u8; 32],
            session_ttl_hours: 1,
            job_ttl_hours: 24,
            price_per_install: 5000,
            dev_login: true,
            paygate_url: paygate_base.to_string(),
            paygate_public_url: "https://pg-v1.example".into(),
            paygate_key: "gw_test".into(),
            paygate_provider: "gobiz".into(),
            admin_tg_ids: vec![],
            cryptogate_url: String::new(),
            cryptogate_key: String::new(),
        }
    }

    async fn setup(paygate_base: &str) -> (SharedState, String) {
        let db_path = std::env::temp_dir().join(format!(
            "rdpi-topup-test-{}.db",
            crate::crypto::random_token(6)
        ));
        let db = db_path.to_string_lossy().to_string();
        let pool = crate::db::connect(&db).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let ts = crate::db::now();
        sqlx::query("INSERT INTO users(id, tg_id, created_at) VALUES(1, 111, ?)")
            .bind(ts)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO balances(user_id, amount, updated_at) VALUES(1, 0, ?)")
            .bind(ts)
            .execute(&pool)
            .await
            .unwrap();
        let state: SharedState = std::sync::Arc::new(crate::AppState {
            cfg: config(&db, paygate_base),
            pool,
        });
        (state, db)
    }

    async fn seed_pending(state: &SharedState, reference: &str, invoice_id: &str) {
        let ts = crate::db::now();
        sqlx::query(
            "INSERT INTO payments(user_id, gateway, ref, gateway_id, amount, total_amount, status,
                                  qr_string, pay_url, raw_json, created_at, expires_at)
             VALUES(1, 'paygate', ?, ?, 15000, 15405, 'pending', '', '', '{}', ?, ?)",
        )
        .bind(reference)
        .bind(invoice_id)
        .bind(ts)
        .bind(ts + 3600)
        .execute(&state.pool)
        .await
        .unwrap();
    }

    async fn balance_of(state: &SharedState) -> i64 {
        sqlx::query_scalar("SELECT amount FROM balances WHERE user_id = 1")
            .fetch_one(&state.pool)
            .await
            .unwrap()
    }

    async fn load_payment(state: &SharedState, reference: &str) -> Payment {
        sqlx::query_as::<_, Payment>(
            "SELECT id, user_id, gateway, ref, gateway_id, amount, total_amount, status,
                    qr_string, pay_url, txn_id, created_at, paid_at, expires_at
             FROM payments WHERE ref = ?",
        )
        .bind(reference)
        .fetch_one(&state.pool)
        .await
        .unwrap()
    }

    const PAID_BODY: &str = r#"{"success":true,"data":{"invoice":{
        "id":"inv_test","provider":"sumopod","reference_id":"rdpi-1-x",
        "base_amount":15000,"service_fee":405,"amount":15405,"status":"paid",
        "transaction_id":"pay_test_1","paid_at":"2026-07-21T08:00:00.000Z"}}}"#;

    const PENDING_BODY: &str = r#"{"success":true,"data":{"invoice":{
        "id":"inv_test","provider":"sumopod","reference_id":"rdpi-1-x",
        "base_amount":15000,"service_fee":405,"amount":15405,"status":"pending"}}}"#;

    const EXPIRED_BODY: &str = r#"{"success":true,"data":{"invoice":{
        "id":"inv_test","base_amount":15000,"amount":15405,"status":"expired"}}}"#;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn paid_invoice_credits_exactly_once() {
        let base = mock_gateway(PAID_BODY).await;
        let (state, db) = setup(&base).await;
        seed_pending(&state, "rdpi-1-lunas", "inv_test").await;

        let row = load_payment(&state, "rdpi-1-lunas").await;
        let client = client(&state).unwrap();

        // polling pertama: menilai lunas, mengkredit
        let inv = client.invoice(&row.gateway_id).await.unwrap();
        assert!(settle(&state, &row, &inv).await.unwrap());
        assert_eq!(balance_of(&state).await, 15000);

        // polling berulang (halaman di-refresh/dua tab): tidak boleh menambah saldo lagi
        let row2 = load_payment(&state, "rdpi-1-lunas").await;
        let inv2 = client.invoice(&row2.gateway_id).await.unwrap();
        assert!(
            !settle(&state, &row2, &inv2).await.unwrap(),
            "klaim kedua harus gagal"
        );
        assert_eq!(
            balance_of(&state).await,
            15000,
            "saldo tidak boleh bertambah dua kali"
        );

        let after = load_payment(&state, "rdpi-1-lunas").await;
        assert_eq!(after.status, "paid");
        assert_eq!(after.total_amount, 15405);
        assert_eq!(
            after.txn_id, "pay_test_1",
            "transaction_id tersimpan untuk dedup"
        );
        assert!(after.paid_at > 0);

        let ledger: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ledger WHERE kind='topup'")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        assert_eq!(ledger, 1, "tepat satu baris ledger");

        let _ = std::fs::remove_file(db);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn pending_does_not_credit() {
        let base = mock_gateway(PENDING_BODY).await;
        let (state, db) = setup(&base).await;
        seed_pending(&state, "rdpi-1-pending", "inv_test").await;

        let row = load_payment(&state, "rdpi-1-pending").await;
        let inv = client(&state).unwrap().invoice("inv_test").await.unwrap();
        assert!(!settle(&state, &row, &inv).await.unwrap());
        assert_eq!(balance_of(&state).await, 0);
        assert_eq!(
            load_payment(&state, "rdpi-1-pending").await.status,
            "pending"
        );

        let _ = std::fs::remove_file(db);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn expired_is_recorded_without_credit() {
        let base = mock_gateway(EXPIRED_BODY).await;
        let (state, db) = setup(&base).await;
        seed_pending(&state, "rdpi-1-kedaluwarsa", "inv_test").await;

        let row = load_payment(&state, "rdpi-1-kedaluwarsa").await;
        let inv = client(&state).unwrap().invoice("inv_test").await.unwrap();
        assert!(!settle(&state, &row, &inv).await.unwrap());
        assert_eq!(balance_of(&state).await, 0);
        assert_eq!(
            load_payment(&state, "rdpi-1-kedaluwarsa").await.status,
            "expired"
        );

        let _ = std::fs::remove_file(db);
    }
}
