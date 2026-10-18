#!/usr/bin/env bash
# Klik-through UI & API: halaman tamu, validasi voucer, dashboard, instalasi, topup, admin panel lengkap.
set -uo pipefail

DIR=/tmp/rdpi-ui
PORT=8796
BIN="${BIN:-/root/rdpinstall/target/debug/rdpinstall}"
FAILED=0

rm -rf "$DIR"; mkdir -p "$DIR"

RDPINSTALL_ENV=dev RDPINSTALL_DEV_LOGIN=1 \
TELEGRAM_BOT_USERNAME=InstallXJayBot TELEGRAM_BOT_TOKEN=dummy \
RDPINSTALL_DB="$DIR/t.db" RDPINSTALL_KEY_FILE="$DIR/master.key" \
RDPINSTALL_BIND="127.0.0.1:$PORT" RDPINSTALL_ADMIN_TG_IDS=1 \
"$BIN" > "$DIR/log" 2>&1 &
PID=$!
trap 'kill $PID 2>/dev/null' EXIT

check() { # check <nama> <harapan> <dapat>
  if [ "$2" = "$3" ]; then
    echo "  OK   $1 ($3)"
  else
    echo "  GAGAL $1: harap '$2', dapat '$3'"
    FAILED=$((FAILED + 1))
  fi
}
has() { # has <nama> <berkas> <pola>
  if grep -Fq "$3" "$2"; then echo "  OK   $1"; else echo "  GAGAL $1: '$3' tidak ada"; FAILED=$((FAILED + 1)); fi
}

for _ in $(seq 1 25); do curl -sf -o /dev/null "http://127.0.0.1:$PORT/healthz" && break; sleep 1; done

CJ="$DIR/cj"
B="http://127.0.0.1:$PORT"

echo "1. halaman tamu SPA"
cd "$DIR"
check "GET /" 200 "$(curl -s -o guest.html -w '%{http_code}' "$B/")"
has "judul tamu" guest.html "VPS Linux Anda, dijadikan Windows"
has "field voucer" guest.html 'name="voucher"'
has "bundle assets" guest.html 'assets/index'

echo "2. validasi voucer endpoint"
# Voucer acak harus ditolak
resp=$(curl -s "$B/api/voucher/validate?code=RDP-FAKE-9999")
echo "$resp" | grep -q '"valid":false' && echo "  OK   voucer palsu ditolak" || { echo "  GAGAL voucer palsu tidak ditolak: $resp"; FAILED=$((FAILED+1)); }

echo "3. login dev"
curl -s -o /dev/null -c "$CJ" -X POST "$B/auth/dev"
check "cookie sesi" 1 "$(grep -c rdpi_sid "$CJ")"

echo "4. dashboard API"
code=$(curl -s -b "$CJ" -H "Accept: application/json" "$B/dashboard" -o "$DIR/dash.json" -w "%{http_code}")
check "GET /dashboard JSON" 200 "$code"
has "balance" "$DIR/dash.json" '"balance":0'

echo "5. topup API"
code=$(curl -s -b "$CJ" -H "Accept: application/json" "$B/topup" -o "$DIR/topup.json" -w "%{http_code}")
check "GET /topup JSON" 200 "$code"
has "presets" "$DIR/topup.json" '"presets":[15000,25000,50000,100000]'

echo "6. admin panel lengkap API"
code=$(curl -s -b "$CJ" -H "Accept: application/json" "$B/admin/overview" -o "$DIR/overview.json" -w "%{http_code}")
check "GET /admin/overview JSON" 200 "$code"
has "stats overview" "$DIR/overview.json" '"stats"'

code=$(curl -s -b "$CJ" -H "Accept: application/json" "$B/admin/jobs" -o "$DIR/admin_jobs.json" -w "%{http_code}")
check "GET /admin/jobs JSON" 200 "$code"
has "jobs array" "$DIR/admin_jobs.json" '"jobs":[]'

code=$(curl -s -b "$CJ" -H "Accept: application/json" "$B/admin/users" -o "$DIR/admin_users.json" -w "%{http_code}")
check "GET /admin/users JSON" 200 "$code"
has "users array" "$DIR/admin_users.json" '"users"'

code=$(curl -s -b "$CJ" -H "Accept: application/json" "$B/admin/payments" -o "$DIR/admin_payments.json" -w "%{http_code}")
check "GET /admin/payments JSON" 200 "$code"
has "payments array" "$DIR/admin_payments.json" '"payments":[]'

echo "7. cetak voucer & validasi sukses"
curl -s -b "$CJ" -X POST -d "count=1&note=uji-ci" "$B/admin/vouchers" -o /dev/null
curl -s -b "$CJ" -H "Accept: application/json" "$B/admin/overview" -o "$DIR/overview2.json"
CODE=$(python3 -c "import json; print(json.load(open('$DIR/overview2.json'))['vouchers'][0]['code'])" 2>/dev/null || echo "")
[ -n "$CODE" ] && echo "  OK   voucer dibuat: $CODE" || { echo "  GAGAL voucer tidak dibuat"; FAILED=$((FAILED+1)); }

# Validasi kode voucer asli yang baru dibuat
resp=$(curl -s "$B/api/voucher/validate?code=$CODE")
echo "$resp" | grep -q '"valid":true' && echo "  OK   voucer asli tervalidasi sukses" || { echo "  GAGAL validasi voucer asli: $resp"; FAILED=$((FAILED+1)); }

echo "8. tamu menolak alamat privat, voucer tidak hangus"
resp=$(curl -s -b "$CJ" -H "Accept: application/json" -X POST -d "voucher=$CODE&host=192.168.1.5&ssh_pass=x&rdp_pass=Kata1!Sandi" "$B/")
echo "$resp" | grep -q "IP publik VPS" && echo "  OK   pesan tolak privat" || { echo "  GAGAL pesan tolak"; FAILED=$((FAILED+1)); }

echo "9. tamu dengan alamat publik"
resp=$(curl -s -b "$CJ" -H "Accept: application/json" -X POST -d "voucher=$CODE&host=203.0.113.9&ssh_user=root&ssh_pass=salah&rdp_user=Administrator&rdp_pass=Kata1!Sandi&edition=srv2022&lang=en-us" "$B/")
TOKEN=$(python3 -c "import json, sys; d=json.loads('''$resp'''); print(d.get('token',''))" 2>/dev/null || echo "")
[ -n "$TOKEN" ] && echo "  OK   token halaman tamu dibuat: $TOKEN" || { echo "  GAGAL token kosong: $resp"; FAILED=$((FAILED+1)); }

# Voucer sekarang harus terdeteksi sudah digunakan dan menyertakan token pemulihan
resp=$(curl -s "$B/api/voucher/validate?code=$CODE")
echo "$resp" | grep -q '"already_used":true' && echo "  OK   voucer terpakai ditolak & token pemulihan tersedia" || { echo "  GAGAL status terpakai: $resp"; FAILED=$((FAILED+1)); }

echo "10. admin ubah saldo user"
curl -s -b "$CJ" -X POST -d "amount=25000&mode=set&note=uji-admin" "$B/admin/users/1/balance" -o "$DIR/adj.json"
has "adj success" "$DIR/adj.json" '"success":true'
check "saldo terupdate di DB" "[(25000,)]" "$(python3 -c "
import sqlite3;print(list(sqlite3.connect('$DIR/t.db').execute('select amount from balances where user_id=1')))")"

echo "11. login admin username & password"
# Password salah
bad_code=$(curl -s -H "Accept: application/json" -X POST -d "username=admin&password=salah" -o /dev/null -w "%{http_code}" "$B/admin/login")
check "admin password salah 401" 401 "$bad_code"

# Password benar
admin_cj="$DIR/admin_cj"
good_code=$(curl -s -c "$admin_cj" -H "Accept: application/json" -X POST -d "username=admin&password=admin12345" -o "$DIR/admin_login.json" -w "%{http_code}" "$B/admin/login")
check "admin password benar 200" 200 "$good_code"
has "login sukses" "$DIR/admin_login.json" '"success":true'
check "cookie admin" 1 "$(grep -c rdpi_sid "$admin_cj")"

echo "12. rute terproteksi anonim"
check "anon /dashboard" "303" "$(curl -s -o /dev/null -w '%{http_code}' "$B/dashboard")"
check "anon /admin/overview JSON" 401 "$(curl -s -H "Accept: application/json" -o /dev/null -w '%{http_code}' "$B/admin/overview")"
check "anon /admin HTML (SPA login view)" 200 "$(curl -s -o /dev/null -w '%{http_code}' "$B/admin")"

echo
if [ "$FAILED" -eq 0 ]; then
  echo "SEMUA PEMERIKSAAN LOLOS"
else
  echo "$FAILED PEMERIKSAAN GAGAL"
fi
exit $([ "$FAILED" -eq 0 ] && echo 0 || echo 1)
