# DESIGN.md

Arah desain untuk rdpinstall v2 (React 19 + Tailwind CSS).
Disusun langsung dari arahan pemilik produk (2026-10-18):
- Tema terang (Light theme)
- Dominan White Bone (latar & kartu bernuansa tulang putih/hangat)
- Orange button with shadow (tombol utama oranye menyala dengan elevasi bayangan)
- Little bit black (aksen hitam pekat untuk teks judul, border kontras, dan badge)
- Multi-Step Wizard untuk instalasi tamu

## Dial Antislop

`Dial: ENERGY 2 / RHYTHM 2 / MOTION 2`

- **ENERGY 2**: Dinamis dan percaya diri. Tombol oranye bertenaga dengan bayangan lembut, kontras teks hitam tajam, border bertekstur.
- **RHYTHM 2**: Multi-Step Wizard dengan transisi antar tahap (1. Voucer -> 2. Kredensial VPS -> 3. Pilihan Windows -> 4. Review & Deploy), kartu ringkasan bervariasi.
- **MOTION 2**: Transisi langkah halus, progress stepper interaktif, feedback animasi tombol copy, pulse ring pada node rel fase yang aktif.

## Palet Warna

| Peran | Nilai | Karakteristik |
|---|---|---|
| Latar Halaman (Bone) | `#fbfaf7` | Putih gading / tulang hangat, bersih, tidak menyilaukan mata |
| Kartu / Permukaan | `#ffffff` | Putih bersih berpadu halus di atas background bone |
| Latar Sub-panel / Input | `#f4f1ea` | Sedikit lebih gelap dari latar utama untuk kontras form |
| Border Halus | `#e8e4dc` | Garis 1px pembatas lembut |
| Border Kuat / Black Accent | `#18181b` | Aksen hitam pekat untuk kontras dan elemen penting |
| Teks Utama | `#0f172a` | Hitam slate pekat (kontras 15:1 pada latar bone) |
| Teks Sekunder / Muted | `#52525b` | Abu-abu netral untuk label dan penjelasan |
| Tombol Utama (Orange) | `linear-gradient(135deg, #f97316 0%, #ea580c 100%)` | Oranye segar dan bertenaga |
| Bayangan Tombol | `0 4px 14px rgba(234, 88, 12, 0.32)` | Shadow oranye hangat khas produk modern |
| Tombol Hover | `linear-gradient(135deg, #fb923c 0%, #f97316 100%)` | Sedikit lebih terang saat disentuh kursor |
| Status Sukses / Siap | `#15803d` (hijau emerald lembut) | Latar `#ecfdf5`, border `#a7f3d0` |
| Status Gagal / Peringatan | `#dc2626` (merah tegas) | Latar `#fef2f2`, border `#fecaca` |

## Tipografi

- Font Display & Body: Inter / System Sans-serif modern (`-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto`).
- Angka, Alamat Mesin, Token & Port: Monospace (`ui-monospace, "SF Mono", Menlo, Consolas`).

## Struktur Multi-Step Wizard (Halaman Utama)

1. **Langkah 1 (Voucer)**: Input kode voucer sekali pakai (`RDP-XXXX-XXXX`), verifikasi cepat, dan penjelasan singkat.
2. **Langkah 2 (Mesin VPS)**: Input IP publik / host, port SSH (default 22), user (default root), dan password SSH.
3. **Langkah 3 (Konfigurasi Windows)**: Pilihan edisi Windows (10/11/Server) dengan catatan kebutuhan RAM, pilihan bahasa, user RDP, dan password Windows yang memenuhi syarat.
4. **Langkah 4 (Review & Pasang)**: Ringkasan konfigurasi sebelum deploy, tombol eksekusi oranye besar dengan konfirmasi.

## Rel Fase Instalasi

Visual progress stepper 4 tahap:
`Diperiksa (SSH & KVM Check)` -> `Unduh Image Windows` -> `Reboot Mesin` -> `Windows & RDP Siap`
Indikator titik aktif berdenyut halus, titik selesai bertanda centang, dan titik tertunda berwarna netral.
