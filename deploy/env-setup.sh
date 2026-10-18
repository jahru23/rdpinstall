#!/usr/bin/env bash
# Menyiapkan /opt/rdpinstall/.env pada VPS. Idempoten: tidak menimpa kalau sudah ada.
set -euo pipefail

APP=/opt/rdpinstall
ENVF=$APP/.env

install -d -o rdpinstall -g rdpinstall -m 750 "$APP" "$APP/data" "$APP/keys"

if [ -f "$ENVF" ]; then
  echo "sudah ada: $ENVF (dibiarkan)"
  exit 0
fi

KEY="$(openssl rand -hex 32)"
BOT_TOKEN="${BOT_TOKEN:-}"
BOT_USERNAME="${BOT_USERNAME:-}"

cat > "$ENVF" <<EOF
RDPINSTALL_ENV=prod
RDPINSTALL_BIND=127.0.0.1:8791
RDPINSTALL_BASE_URL=https://install.jaystore.top
RDPINSTALL_DB=data/rdpinstall.db

RDPINSTALL_MASTER_KEY=$KEY
RDPINSTALL_KEY_FILE=keys/master.key

TELEGRAM_BOT_TOKEN=$BOT_TOKEN
TELEGRAM_BOT_USERNAME=$BOT_USERNAME

RDPINSTALL_SESSION_TTL_HOURS=720
RDPINSTALL_JOB_TTL_HOURS=24
RDPINSTALL_PRICE=5000
RDPINSTALL_DEV_LOGIN=0

PAYGATE_URL=http://127.0.0.1:3001
PAYGATE_PUBLIC_URL=https://pg-v1.jaystore.top
PAYGATE_API_KEY=
CRYPTOGATE_URL=http://127.0.0.1:8899
CRYPTOGATE_API_KEY=

RUST_LOG=rdpinstall=info,tower_http=warn
EOF

chown rdpinstall:rdpinstall "$ENVF"
chmod 640 "$ENVF"
echo "dibuat: $ENVF (master key 32 byte, mode 640)"
