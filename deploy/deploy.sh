#!/usr/bin/env bash
# Deploy dari mesin dev ini ke VPS. Build lokal (gnu) sebagai fallback cepat;
# jalur normal adalah GitHub Actions yang mengirim binary musl.
set -euo pipefail

HOST="${HOST:-root@43.133.131.46}"
TARGET="${TARGET:-x86_64-unknown-linux-musl}"

cd "$(dirname "$0")/.."

echo "== build ($TARGET) =="
cargo build --release --target "$TARGET"

BIN="target/$TARGET/release/rdpinstall"
PKG="/tmp/rdpinstall-$TARGET.tar.gz"
tar -czf "$PKG" -C "$(dirname "$BIN")" rdpinstall

echo "== kirim ke $HOST =="
scp "$PKG" "$HOST:/tmp/"
scp deploy/install-remote.sh "$HOST:/tmp/rdpinstall-install-remote.sh"
ssh "$HOST" "bash /tmp/rdpinstall-install-remote.sh /tmp/$(basename "$PKG")"
