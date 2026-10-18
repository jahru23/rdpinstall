# rdpinstall

Web installer Windows/RDP. Backend Rust (axum + tokio + sqlx/SQLite), frontend
server-rendered askama + htmx + CSS tulis tangan — tanpa npm, tanpa bundler.

Dua mode:

- **template** (default) — VPS dibuat UpCloud dari template Windows resmi; password
  Administrator dikembalikan di response create, jadi tidak perlu SSH key.
- **reinstall** (menyusul) — VPS Linux milik user sendiri, dirombak lewat
  `bin456789/reinstall` via SSH.

## Jalankan lokal

```bash
cp .env.example .env            # isi TELEGRAM_BOT_TOKEN + TELEGRAM_BOT_USERNAME
RDPINSTALL_DEV_LOGIN=1 cargo run # lalu klik "masuk sebagai dev"
```

Server: http://127.0.0.1:8791 · health: `/healthz`

Dev login hanya aktif kalau `RDPINSTALL_ENV != prod` **dan** `RDPINSTALL_DEV_LOGIN=1`.

## Struktur

```
src/
  main.rs        bootstrap, layer, listener
  config.rs      env + master key (0600)
  crypto.rs      AES-256-GCM seal/open, token acak
  db.rs          pool SQLite WAL + migrate
  auth.rs        verifikasi Login Widget + session cookie
  models.rs      row struct
  views.rs       template askama + filter rupiah/tanggal
  routes/        pages, telegram, assets
migrations/      skema (users, sessions, balances, ledger, payments, cloud_accounts, jobs, ...)
templates/       base/login/dashboard
static/style.css
deploy/          systemd unit, vhost nginx, skrip deploy
```

## Deploy

TLS diterminasi Cloudflare (mode Flexible), origin nginx hanya listen :80 dengan
allowlist IP Cloudflare dan menolak request tanpa header `CF-Connecting-IP` —
pola yang sama dengan vhost lain di VPS itu. Jadi tidak perlu certbot.

DNS: record `A install.jaystore.top -> 43.133.131.46`, proxied (awan oranye).


Jalur normal: push ke `main` → GitHub Actions build binary statis
`x86_64-unknown-linux-musl` → kirim ke VPS → `deploy/install-remote.sh` yang
memasang user sistem, `/opt/rdpinstall`, unit systemd, lalu cek `/healthz`.

Aktifkan auto-deploy dengan repository **variable** `DEPLOY_ENABLED=true` dan
**secret** `DEPLOY_HOST`, `DEPLOY_USER`, `DEPLOY_SSH_KEY`.

Fallback manual dari mesin dev:

```bash
rustup target add x86_64-unknown-linux-musl
./deploy/deploy.sh
```

## Catatan

- `RDPINSTALL_MASTER_KEY` mengenkripsi token cloud user dan password RDP di DB.
  Di produksi isi eksplisit; kalau tidak, kunci dibuat di `keys/master.key`.
- Password RDP otomatis dihapus setelah `RDPINSTALL_JOB_TTL_HOURS` (default 24 jam).
- Nginx vhost: `deploy/nginx-install.jaystore.top.conf` (buffering mati untuk SSE).
