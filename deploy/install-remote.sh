#!/usr/bin/env bash
# Dijalankan DI VPS (43.133.131.46) oleh CI atau oleh deploy.sh lokal.
# Paket binary sudah ada di $1 (default /tmp/rdpinstall-x86_64-linux-musl.tar.gz)
set -euo pipefail

PKG="${1:-/tmp/rdpinstall-x86_64-linux-musl.tar.gz}"
APP=/opt/rdpinstall
USER_NAME=rdpinstall

id -u "$USER_NAME" >/dev/null 2>&1 || useradd --system --home "$APP" --shell /usr/sbin/nologin "$USER_NAME"

install -d -o "$USER_NAME" -g "$USER_NAME" -m 750 "$APP" "$APP/data" "$APP/keys"

STAMP="$(date +%Y%m%d%H%M%S)"
[ -x "$APP/rdpinstall" ] && cp -a "$APP/rdpinstall" "$APP/rdpinstall.bak.$STAMP"

tmp="$(mktemp -d)"
tar -xzf "$PKG" -C "$tmp"
install -o "$USER_NAME" -g "$USER_NAME" -m 750 "$tmp/rdpinstall" "$APP/rdpinstall"
rm -rf "$tmp"

if [ ! -f "$APP/.env" ]; then
  echo "!! $APP/.env belum ada — salin dari .env.example lalu isi, service belum dijalankan" >&2
  exit 2
fi
chown "$USER_NAME:$USER_NAME" "$APP/.env"
chmod 640 "$APP/.env"

install -m 644 /dev/stdin /etc/systemd/system/rdpinstall.service <<'UNIT'
[Unit]
Description=rdpinstall — web installer Windows/RDP
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=rdpinstall
Group=rdpinstall
WorkingDirectory=/opt/rdpinstall
EnvironmentFile=/opt/rdpinstall/.env
ExecStart=/opt/rdpinstall/rdpinstall
Restart=always
RestartSec=3
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/rdpinstall/data /opt/rdpinstall/keys
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
UNIT

systemctl daemon-reload
systemctl enable rdpinstall >/dev/null 2>&1 || true
systemctl restart rdpinstall
sleep 2
systemctl is-active rdpinstall
curl -fsS http://127.0.0.1:8791/healthz && echo " <- healthz ok"
