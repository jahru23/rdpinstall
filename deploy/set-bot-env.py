#!/usr/bin/env python3
"""Set TELEGRAM_BOT_TOKEN / TELEGRAM_BOT_USERNAME di /opt/rdpinstall/.env.

Nilai diambil dari environment, jadi tidak pernah muncul sebagai argumen proses.
"""
import grp
import os
import pathlib
import pwd
import re
import stat

ENVF = pathlib.Path("/opt/rdpinstall/.env")
token = os.environ.get("TELEGRAM_BOT_TOKEN", "").strip()
username = os.environ.get("TELEGRAM_BOT_USERNAME", "").strip().lstrip("@")

if not ENVF.exists():
    raise SystemExit(f"tidak ada: {ENVF}")
if not token or not username:
    raise SystemExit("TELEGRAM_BOT_TOKEN / TELEGRAM_BOT_USERNAME kosong")

s = ENVF.read_text()
s = re.sub(r"(?m)^TELEGRAM_BOT_TOKEN=.*$", "TELEGRAM_BOT_TOKEN=" + token, s)
s = re.sub(r"(?m)^TELEGRAM_BOT_USERNAME=.*$", "TELEGRAM_BOT_USERNAME=" + username, s)
ENVF.write_text(s)

uid = pwd.getpwnam("rdpinstall").pw_uid
gid = grp.getgrnam("rdpinstall").gr_gid
os.chown(ENVF, uid, gid)
os.chmod(ENVF, stat.S_IRUSR | stat.S_IWUSR | stat.S_IRGRP)

# ringkasan tanpa membocorkan nilai
print(f"env diperbarui: bot=@{username} token_len={len(token)}")
