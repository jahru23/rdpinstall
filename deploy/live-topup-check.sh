#!/usr/bin/env bash
# Uji jalur topup di VPS dengan instance sementara (DB terpisah, port 8794).
# Membuat invoice QRIS SUNGGUHAN di PayGate, memeriksa QR-nya bisa diambil,
# memastikan saldo TIDAK bertambah selama masih pending, lalu membatalkannya.
set -uo pipefail

APP=/opt/rdpinstall
DIR=/tmp/rdpi-live
PORT=8794
BIN="$APP/rdpinstall"

rm -rf "$DIR"; mkdir -p "$DIR"

# kunci PayGate dari .env produksi (tidak dicetak)
set -a
. "$APP/.env"
set +a

RDPINSTALL_ENV=dev RDPINSTALL_DEV_LOGIN=1 TELEGRAM_BOT_USERNAME=x TELEGRAM_BOT_TOKEN=x \
RDPINSTALL_DB="$DIR/t.db" RDPINSTALL_KEY_FILE="$DIR/master.key" \
RDPINSTALL_BIND="127.0.0.1:$PORT" \
"$BIN" > "$DIR/log" 2>&1 &
PID=$!
trap 'kill $PID 2>/dev/null; rm -rf "$DIR"' EXIT

for _ in $(seq 1 30); do curl -sf -o /dev/null "http://127.0.0.1:$PORT/healthz" && break; sleep 1; done

CJ="$DIR/cj.txt"
curl -s -o /dev/null -X POST -c "$CJ" "http://127.0.0.1:$PORT/auth/dev"

echo "== POST /topup 15000 =="
LOC=$(curl -s -o /dev/null -w '%{redirect_url}' -b "$CJ" -X POST -d 'amount=15000' "http://127.0.0.1:$PORT/topup")
echo "redirect: $LOC"
REF="${LOC##*/}"

echo "== halaman QR =="
curl -s -b "$CJ" "$LOC" -o "$DIR/page.html" -w 'status=%{http_code} bytes=%{size_download}\n'
QRURL=$(grep -o 'src="[^"]*qr[^"]*"' "$DIR/page.html" | head -1 | sed 's/src="//;s/"$//')
PAYURL=$(grep -o 'href="https://[^"]*"' "$DIR/page.html" | grep -v jaystore.top | head -1 | sed 's/href="//;s/"$//')
echo "QR img   : ${QRURL:-<tidak ada>}"
echo "pay link : ${PAYURL:-<tidak ada>}"
grep -oE 'Bayar tepat <b>[^<]*|saldo yang masuk[^<]*' "$DIR/page.html" | head -2

echo "== QR bisa diambil? =="
if [ -n "$QRURL" ]; then
  curl -s -o "$DIR/qr.png" -w 'http=%{http_code} type=%{content_type} bytes=%{size_download}\n' "$QRURL"
  file "$DIR/qr.png" 2>/dev/null | head -1
fi

echo "== status (harus pending, saldo tetap 0) =="
curl -s -b "$CJ" "http://127.0.0.1:$PORT/topup/$REF/status" -o "$DIR/st.html" -w 'status=%{http_code}\n'
grep -oE 'badge [a-z]+">[a-z]+' "$DIR/st.html" | head -2
sqlite3 "$DIR/t.db" "select 'payments', status, amount, total_amount, txn_id from payments;" 2>/dev/null || python3 -c "
import sqlite3;print(list(sqlite3.connect('$DIR/t.db').execute('select status, amount, total_amount, txn_id from payments')))"
echo -n "saldo: "; python3 -c "
import sqlite3;print(list(sqlite3.connect('$DIR/t.db').execute('select amount from balances where user_id=1')))"
echo -n "ledger topup: "; python3 -c "
import sqlite3;print(list(sqlite3.connect('$DIR/t.db').execute(\"select count(*) from ledger where kind='topup'\")))"

echo "== batalkan =="
curl -s -o /dev/null -w 'cancel=%{http_code}\n' -b "$CJ" -X POST "http://127.0.0.1:$PORT/topup/$REF/cancel"
python3 -c "
import sqlite3;print('status di DB:', list(sqlite3.connect('$DIR/t.db').execute('select status from payments')))"

echo "== status gateway langsung =="
curl -s -H "X-API-Key: $PAYGATE_API_KEY" "${PAYGATE_URL:-http://127.0.0.1:3001}/v1/invoices/$(python3 -c "
import sqlite3;print(list(sqlite3.connect('$DIR/t.db').execute('select gateway_id from payments'))[0][0])")" \
  | python3 -c "import sys,json;d=json.load(sys.stdin);i=d['data']['invoice'];print({k:i.get(k) for k in ('id','status','base_amount','service_fee','amount')})"
