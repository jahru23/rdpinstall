#!/usr/bin/env bash
# Klik-through UI: halaman tamu, voucer, dashboard, instalasi, topup, admin.
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
  if grep -q "$3" "$2"; then echo "  OK   $1"; else echo "  GAGAL $1: '$3' tidak ada"; FAILED=$((FAILED + 1)); fi
}

for _ in $(seq 1 25); do curl -sf -o /dev/null "http://127.0.0.1:$PORT/healthz" && break; sleep 1; done

CJ="$DIR/cj"
B="http://127.0.0.1:$PORT"
code() { curl -s -o "$DIR/last.html" -w "%{http_code}" -b "$CJ" "$B$1"; }

echo "1. halaman tamu"
cd "$DIR"
check "GET /" 200 "$(curl -s -o guest.html -w '%{http_code}' "$B/")"
has "judul tamu" guest.html "VPS Linux Anda, dijadikan Windows"
has "field voucer" guest.html 'name="voucher"'
has "field host" guest.html 'name="host"'
has "field password SSH" guest.html 'name="ssh_pass"'
has "field password Windows" guest.html 'name="rdp_pass"'
has "pilihan edisi" guest.html 'name="edition"'
has "stylesheet" guest.html 'href="/static/style.css"'

echo "2. masuk"
check "GET /masuk" 200 "$(code /masuk)"
has "widget Telegram" "$DIR/last.html" 'data-telegram-login="InstallXJayBot"'

echo "3. login dev"
curl -s -o /dev/null -c "$CJ" -X POST "$B/auth/dev"
check "cookie sesi" 1 "$(grep -c rdpi_sid "$CJ")"

echo "4. dashboard"
check "GET /dashboard" 200 "$(code /dashboard)"
has "kartu saldo" "$DIR/last.html" "Saldo"
has "keadaan kosong" "$DIR/last.html" "Belum ada instalasi"

echo "5. wizard instalasi"
check "GET /instalasi" 200 "$(code /instalasi)"
has "form host" "$DIR/last.html" 'name="host"'
has "form edisi" "$DIR/last.html" 'name="edition"'

echo "6. topup"
check "GET /topup" 200 "$(code /topup)"
has "nominal cepat" "$DIR/last.html" "Rp15.000"
has "field nominal" "$DIR/last.html" 'name="amount"'

echo "7. admin voucer"
check "GET /admin/vouchers" 200 "$(code /admin/vouchers)"
has "keadaan kosong" "$DIR/last.html" "Belum ada voucer"

echo "8. cetak voucer"
curl -s -b "$CJ" -X POST -d "count=1&note=uji-ci" "$B/admin/vouchers" -o "$DIR/v.html" -w "" 
CODE=$(grep -oE "RDP-[A-Z0-9]{4}-[A-Z0-9]{4}" "$DIR/v.html" | head -1)
[ -n "$CODE" ] && echo "  OK   voucer dibuat: $CODE" || { echo "  GAGAL voucer tidak dibuat"; FAILED=$((FAILED+1)); }
check "baris voucer di DB" "('$CODE', 0, 0)" "$(python3 -c "
import sqlite3;print(list(sqlite3.connect('$DIR/t.db').execute('select code,used_at,revoked from vouchers'))[0])")"

echo "9. tamu menolak alamat privat, voucer tidak hangus"
curl -s -b "$CJ" -X POST -d "voucher=$CODE&host=192.168.1.5&ssh_pass=x&rdp_pass=Kata1!Sandi" "$B/" -o "$DIR/priv.html" -w ""
has "pesan tolak" "$DIR/priv.html" "IP publik VPS"
check "voucer utuh" "[(0,)]" "$(python3 -c "
import sqlite3;print(list(sqlite3.connect('$DIR/t.db').execute('select used_at from vouchers')))")"

echo "10. tamu dengan alamat publik"
curl -s -b "$CJ" -X POST -d "voucher=$CODE&host=203.0.113.9&ssh_user=root&ssh_pass=salah&rdp_user=Administrator&rdp_pass=Kata1!Sandi&edition=srv2022&lang=en-us" "$B/" -o /dev/null -w ""
TOKEN=$(python3 -c "
import sqlite3;print(list(sqlite3.connect('$DIR/t.db').execute('select access_token from jobs order by id desc limit 1'))[0][0])")
[ -n "$TOKEN" ] && echo "  OK   token halaman tamu dibuat" || { echo "  GAGAL token kosong"; FAILED=$((FAILED+1)); }
V=$(python3 -c "
import sqlite3;r=list(sqlite3.connect('$DIR/t.db').execute('select used_at,used_host from vouchers'))[0];print('dipakai' if r[0] and r[1]=='203.0.113.9' else 'tidak')")
check "voucer terpakai" "dipakai" "$V"

echo "11. halaman progres tamu"
check "GET /g/{token}" 200 "$(curl -s -o "$DIR/g.html" -w '%{http_code}' "$B/g/$TOKEN")"
has "rel fase" "$DIR/g.html" 'class="rail"'
has "titik aktif" "$DIR/g.html" 'class="step active"'
check "token ngawur 404" 404 "$(curl -s -o /dev/null -w '%{http_code}' "$B/g/tidakada")"

echo "12. worker gagal dengan sebab yang jelas"
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

echo "13. rute terproteksi dan 404"
check "anon /dashboard" "303" "$(curl -s -o /dev/null -w '%{http_code}' "$B/dashboard")"
check "anon /instalasi" "303" "$(curl -s -o /dev/null -w '%{http_code}' "$B/instalasi")"
check "anon /admin/vouchers" "303" "$(curl -s -o /dev/null -w '%{http_code}' "$B/admin/vouchers")"
check "halaman tidak ada" 404 "$(curl -s -o /dev/null -w '%{http_code}' "$B/tidak-ada")"

echo
if [ "$FAILED" -eq 0 ]; then
  echo "SEMUA PEMERIKSAAN LOLOS"
else
  echo "$FAILED PEMERIKSAAN GAGAL"
fi
echo "== log server =="; tail -8 "$DIR/log"
exit $([ "$FAILED" -eq 0 ] && echo 0 || echo 1)
