#!/usr/bin/env python3
"""Buat API key PayGate khusus rdpinstall, lalu simpan ke /opt/rdpinstall/.env.

Dijalankan DI VPS. Nilai kunci tidak pernah dicetak — hanya panjang + prefix.
Idempoten: kalau sudah ada kunci bernama `rdpinstall`, tidak membuat yang baru.
"""
import json
import os
import pathlib
import re
import subprocess
import sys
import urllib.request

PAYGATE_ENV = "/var/www/paygate/.env"
TARGET_ENV = "/opt/rdpinstall/.env"
KEY_NAME = "rdpinstall"
BASE = "http://127.0.0.1:3001"


def env_value(path: str, key: str) -> str:
    for line in pathlib.Path(path).read_text().splitlines():
        line = line.strip()
        if line.startswith(f"{key}="):
            return line.split("=", 1)[1].strip().strip('"').strip("'")
    return ""


def call(method: str, path: str, api_key: str, body=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(f"{BASE}{path}", data=data, method=method)
    req.add_header("X-API-Key", api_key)
    req.add_header("Content-Type", "application/json")
    with urllib.request.urlopen(req, timeout=20) as r:
        return json.load(r)


admin = env_value(PAYGATE_ENV, "API_KEY")
if not admin:
    sys.exit("API_KEY PayGate tidak ditemukan")

# sudah ada kunci dengan nama itu? Kalau ada tapi secret-nya tidak kita pegang,
# cabut dulu supaya tidak menumpuk kunci aktif yang tidak terpakai.
existing = call("GET", "/v1/api-keys", admin)
keys = ((existing.get("data") or {}).get("keys")) or []
for k in keys:
    if k.get("name") == KEY_NAME and not k.get("revoked_at"):
        call("DELETE", f"/v1/api-keys/{k['id']}", admin)
        print(f"kunci lama '{KEY_NAME}' ({k['id']}) dicabut, membuat yang baru")

res = call("POST", "/v1/api-keys", admin, {"name": KEY_NAME})
data = res.get("data") or {}
# bentuk respons: { success, data: { key: {...metadata}, secret: "gw_...", warning } }
key = data.get("secret")
if not key:
    print("respons tidak memuat secret:", json.dumps(res)[:300])
    sys.exit(1)

text = pathlib.Path(TARGET_ENV).read_text()
if re.search(r"(?m)^PAYGATE_API_KEY=", text):
    text = re.sub(r"(?m)^PAYGATE_API_KEY=.*$", "PAYGATE_API_KEY=" + key, text)
else:
    text = text.rstrip() + f"\nPAYGATE_API_KEY={key}\n"
pathlib.Path(TARGET_ENV).write_text(text)
os.chmod(TARGET_ENV, 0o640)
subprocess.run(["chown", "rdpinstall:rdpinstall", TARGET_ENV], check=False)

print(f"kunci '{KEY_NAME}' dibuat: prefix {key[:12]}… panjang {len(key)}")
