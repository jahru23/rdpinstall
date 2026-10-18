#!/usr/bin/env bash
# Uji klik-through UI baru: halaman tamu, voucer, dashboard, instalasi, topup, admin.
# Bukti untuk R-35: setiap elemen yang diklik dicatat hasilnya.
set -uo pipefail
DIR=/tmp/rdpi-ui
PORT=8796
BIN=/root/rdpinstall/target/debug/rdpinstall
rm -rf "$DIR"; mkdir -p "$DIR"

RDPINSTALL_ENV=dev RDPINSTALL_DEV_LOGIN=1 \
TELEGRAM_BOT_USERNAME=InstallXJayBot TELEGRAM_BOT_TOKEN=dummy \
RDPINSTALL_DB="$DIR/t.db" RDPINSTALL_KEY_FILE="$DIR/master.key" \
RDPINSTALL_BIND="127.0.0.1:$PORT" RDPINSTALL_ADMIN_TG_IDS=1 \
"$BIN" > "$DIR/log" 2>&1 &
PID=$!
trap 'kill $PID 2>/dev/null' EXIT

for _ in $(seq 1 25); do curl -sf -o /dev/null "http://127.0.0.1:$PORT/healthz" && break; sleep 1; done

CJ="$DIR/cj"
B="http://127.0.0.1:$PORT"
get() { curl -s -b "$CJ" "$B$1" -o "$DIR/last.html" -w "%{http_code}"; }

echo "1) GET / halaman tamu"
code=$(curl -s "$B/" -o "$DIR/guest.html" -w "%{http_code}")
echo "   status=$code"
grep -oE "VPS Linux Anda, dijadikan Windows|Kode voucer|Alamat VPS|Password SSH|nama=\"edition\"|Minimal 8 karakter" "$DIR/guest.html" | sort -u | sed 's/^/   ada: /'
echo -n "   stylesheet terpasang: "; grep -c 'href="/static/style.css"' "$DIR/guest.html"

echo "2) GET /masuk"
code=$(curl -s "$B/masuk" -o "$DIR/login.html" -w "%{http_code}")
echo "   status=$code"; grep -o 'data-telegram-login="[^"]*"' "$DIR/login.html" | sed 's/^/   /'

echo "3) POST /auth/dev (login dev)"
curl -s -o /dev/null -c "$CJ" -X POST "$B/auth/dev"
echo -n "   cookie tersimpan: "; grep -c rdpi_sid "$CJ"

echo "4) GET /dashboard"
echo "   status=$(get /dashboard)"; grep -oE "Saldo|Sekali pasang|Riwayat|Belum ada instalasi" "$DIR/last.html" | sort -u | sed 's/^/   ada: /'

echo "5) GET /instalasi"
echo "   status=$(get /instalasi)"; grep -oE "Pasang Windows di VPS Anda|name=\"edition\"|name=\"host\"|name=\"ssh_pass\"|name=\"rdp_pass\"" "$DIR/last.html" | sort -u | sed 's/^/   ada: /'

echo "6) GET /topup"
echo "   status=$(get /topup)"; grep -oE "Isi saldo|Rp5\.000|Rp15\.000|name=\"amount\"" "$DIR/last.html" | sort -u | sed 's/^/   ada: /'

echo "7) GET /admin/vouchers"
echo "   status=$(get /admin/vouchers)"; grep -oE "Belum ada voucer|Cetak" "$DIR/last.html" | sort -u | sed 's/^/   ada: /'

echo "8) POST /admin/vouchers (cetak 1 voucer)"
curl -s -b "$CJ" -X POST -d "count=1&note=uji-klik" "$B/admin/vouchers" -o "$DIR/v.html" -w "   status=%{http_code}\n"
CODE=$(grep -oE "RDP-[A-Z0-9]{4}-[A-Z0-9]{4}" "$DIR/v.html" | head -1)
echo "   voucer dibuat: ${CODE:-<tidak ada>}"
echo -n "   baris di DB: "; python3 -c "
import sqlite3;print(list(sqlite3.connect('$DIR/t.db').execute('select code,used_at,revoked from vouchers')))"

echo "9) POST / tamu dengan voucer + alamat privat (harus ditolak)"
curl -s -b "$CJ" -X POST -d "voucher=$CODE&host=192.168.1.5&ssh_pass=x&rdp_pass=Kata1!Sandi" "$B/" -o "$DIR/priv.html" -w "   status=%{http_code}\n"
grep -oE "IP publik VPS[^<]*" "$DIR/priv.html" | head -1 | sed 's/^/   pesan: /'
echo -n "   voucer masih utuh: "; python3 -c "
import sqlite3;print(list(sqlite3.connect('$DIR/t.db').execute('select used_at from vouchers')))"

echo "10) POST / tamu dengan alamat publik (job dibuat, SSH akan gagal)"
curl -s -b "$CJ" -X POST -d "voucher=$CODE&host=203.0.113.9&ssh_user=root&ssh_pass=salah&rdp_user=Administrator&rdp_pass=Kata1!Sandi&edition=srv2022&lang=en-us" "$B/" -o /dev/null -w "   status=%{http_code} redirect=%{redirect_url}\n"
TOKEN=$(python3 -c "
import sqlite3;print(list(sqlite3.connect('$DIR/t.db').execute('select access_token from jobs order by id desc limit 1'))[0][0])")
echo "   token halaman tamu: ${TOKEN:0:12}..."
echo -n "   voucer terpakai: "; python3 -c "
import sqlite3;print(list(sqlite3.connect('$DIR/t.db').execute('select used_at,used_host,used_job_id from vouchers')))"

echo "11) GET /g/{token}"
code=$(curl -s "$B/g/$TOKEN" -o "$DIR/g.html" -w "%{http_code}")
echo "   status=$code"
grep -oE "rel fase|class=\"rail\"|class=\"step [a-z]+\"|Job #1|diperiksa|unduh|reboot" "$DIR/g.html" | sort | uniq -c | sed 's/^/   /'
echo "12) token ngawur"
echo "   status=$(curl -s -o /dev/null -w "%{http_code}" "$B/g/tidakada")"

echo "13) tunggu worker gagal (SSH ke alamat uji)"
for i in $(seq 1 12); do
  st=$(python3 -c "
import sqlite3;print(list(sqlite3.connect('$DIR/t.db').execute(\"select status,phase,error from jobs order by id desc limit 1\"))[0])")
  case "$st" in *failed*) echo "   [$i] $st"; break;; esac
  echo "   [$i] $st"; sleep 5
done
echo -n "   kredensial (password tersimpan terenkripsi sampai janitor membersihkan): "; python3 -c "
import sqlite3
r=list(sqlite3.connect('$DIR/t.db').execute('select ssh_pass_enc,rdp_pass_enc,status from jobs order by id desc limit 1'))[0]
print('ya' if r[0] and r[1] else 'tidak', '| status', r[2])"

echo "14) 404"
echo "   status=$(curl -s -o /dev/null -w "%{http_code}" "$B/halaman-tidak-ada")"
echo "15) anonim ke /dashboard"
echo "   status=$(curl -s -o /dev/null -w "%{http_code} %{redirect_url}" "$B/dashboard")"

echo "== log server (10 baris) =="; tail -10 "$DIR/log"
