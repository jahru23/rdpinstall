#!/usr/bin/env bash
# Klik-through UI & API: halaman tamu, voucer, dashboard, instalasi, topup, admin.
# Dipakai sebagai pemeriksaan di CI maupun saat kerja lokal. Keluar dengan
# status 1 kalau ada pemeriksaan yang gagal, supaya CI benar-benar menahan.
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
has "field host" guest.html 'name="host"'
has "field password SSH" guest.html 'name="ssh_pass"'
has "field password Windows" guest.html 'name="rdp_pass"'
has "pilihan edisi" guest.html 'name="edition"'
has "bundle assets" guest.html 'assets/index'

echo "2. masuk"
check "GET /masuk" 200 "$(curl -s -o /dev/null -w '%{http_code}' "$B/masuk")"

echo "3. login dev"
curl -s -o /dev/null -c "$CJ" -X POST "$B/auth/dev"
check "cookie sesi" 1 "$(grep -c rdpi_sid "$CJ")"

echo "4. dashboard API"
code=$(curl -s -b "$CJ" -H "Accept: application/json" "$B/dashboard" -o "$DIR/dash.json" -w "%{http_code}")
check "GET /dashboard JSON" 200 "$code"
has "balance" "$DIR/dash.json" '"balance":0'
has "jobs array" "$DIR/dash.json" '"jobs":[]'

echo "5. topup API"
code=$(curl -s -b "$CJ" -H "Accept: application/json" "$B/topup" -o "$DIR/topup.json" -w "%{http_code}")
check "GET /topup JSON" 200 "$code"
has "presets" "$DIR/topup.json" '"presets":[15000,25000,50000,100000]'

echo "6. admin voucer API"
code=$(curl -s -b "$CJ" -H "Accept: application/json" "$B/admin/vouchers" -o "$DIR/admin.json" -w "%{http_code}")
check "GET /admin/vouchers JSON" 200 "$code"
has "vouchers list" "$DIR/admin.json" '"vouchers":[]'

echo "7. cetak voucer"
curl -s -b "$CJ" -X POST -d "count=1&note=uji-ci" "$B/admin/vouchers" -o /dev/null
curl -s -b "$CJ" -H "Accept: application/json" "$B/admin/vouchers" -o "$DIR/admin2.json"
CODE=$(python3 -c "import json; print(json.load(open('$DIR/admin2.json'))['vouchers'][0]['code'])" 2>/dev/null || echo "")
[ -n "$CODE" ] && echo "  OK   voucer dibuat: $CODE" || { echo "  GAGAL voucer tidak dibuat"; FAILED=$((FAILED+1)); }
check "baris voucer di DB" "('$CODE', 0, 0)" "$(python3 -c "
import sqlite3;print(list(sqlite3.connect('$DIR/t.db').execute('select code,used_at,revoked from vouchers'))[0])")"

echo "8. tamu menolak alamat privat, voucer tidak hangus"
resp=$(curl -s -b "$CJ" -H "Accept: application/json" -X POST -d "voucher=$CODE&host=192.168.1.5&ssh_pass=x&rdp_pass=Kata1!Sandi" "$B/")
echo "$resp" | grep -q "IP publik VPS" && echo "  OK   pesan tolak privat" || { echo "  GAGAL pesan tolak"; FAILED=$((FAILED+1)); }
check "voucer utuh" "[(0,)]" "$(python3 -c "
import sqlite3;print(list(sqlite3.connect('$DIR/t.db').execute('select used_at from vouchers')))")"

echo "9. tamu dengan alamat publik"
resp=$(curl -s -b "$CJ" -H "Accept: application/json" -X POST -d "voucher=$CODE&host=203.0.113.9&ssh_user=root&ssh_pass=salah&rdp_user=Administrator&rdp_pass=Kata1!Sandi&edition=srv2022&lang=en-us" "$B/")
TOKEN=$(python3 -c "import json, sys; d=json.loads('''$resp'''); print(d.get('token',''))" 2>/dev/null || echo "")
[ -n "$TOKEN" ] && echo "  OK   token halaman tamu dibuat: $TOKEN" || { echo "  GAGAL token kosong: $resp"; FAILED=$((FAILED+1)); }
V=$(python3 -c "
import sqlite3;r=list(sqlite3.connect('$DIR/t.db').execute('select used_at,used_host from vouchers'))[0];print('dipakai' if r[0] and r[1]=='203.0.113.9' else 'tidak')")
check "voucer terpakai" "dipakai" "$V"

echo "10. halaman progres tamu JSON"
check "GET /g/{token} JSON" 200 "$(curl -s -H "Accept: application/json" -o "$DIR/g.json" -w '%{http_code}' "$B/g/$TOKEN")"
has "rel fase" "$DIR/g.json" '"rail"'
has "status job" "$DIR/g.json" '"os_label":"Server 2022"'
check "token ngawur 404" 404 "$(curl -s -H "Accept: application/json" -o /dev/null -w '%{http_code}' "$B/g/tidakada")"

echo "11. worker gagal dengan sebab yang jelas"
ST=""
for i in $(seq 1 12); do
  ST=$(python3 -c "
import sqlite3;print(list(sqlite3.connect('$DIR/t.db').execute('select status from jobs order by id desc limit 1'))[0][0])")
  [ "$ST" = "failed" ] && break
  sleep 5
done
check "job gagal (SSH ke alamat uji)" "failed" "$ST"
EXP=$(python3 -c "
import sqlite3,time
print('ya' if list(sqlite3.connect('$DIR/t.db').execute('select expires_at from jobs order by id desc limit 1'))[0][0] > time.time() else 'tidak')")
check "job gagal punya masa kedaluwarsa" "ya" "$EXP"

echo "12. rute terproteksi anonim"
check "anon /dashboard" "303" "$(curl -s -o /dev/null -w '%{http_code}' "$B/dashboard")"
check "anon /instalasi" "303" "$(curl -s -o /dev/null -w '%{http_code}' "$B/instalasi")"
check "anon /admin/vouchers" "303" "$(curl -s -o /dev/null -w '%{http_code}' "$B/admin/vouchers")"
check "API tidak ada" 404 "$(curl -s -o /dev/null -w '%{http_code}' "$B/api/tidak-ada")"

echo
if [ "$FAILED" -eq 0 ]; then
  echo "SEMUA PEMERIKSAAN LOLOS"
else
  echo "$FAILED PEMERIKSAAN GAGAL"
fi
exit $([ "$FAILED" -eq 0 ] && echo 0 || echo 1)
