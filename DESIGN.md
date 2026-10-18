# DESIGN.md

Arah desain untuk rdpinstall. Ditulis dari jawaban pemilik produk (2026-10-18),
bukan dikarang agen. Aturan antislop dipakai sebagai filter di atas arah ini.

## Identitas

Alat kerja, bukan halaman pemasaran. Yang datang ke sini sedang menyiapkan mesin
dan butuh memastikan angkanya benar, jadi desainnya tenang, padat, dan tidak
menghibur diri.

Ciri yang dipegang: **monokrom + satu aksen merah**. Tidak meniru layout produk
lain (Linear, Vercel, Stripe). Nuansanya boleh rapi dan bersih seperti alat
developer pada umumnya, tapi susunannya milik produk ini.

## Dial

`Dial: ENERGY 1 / RHYTHM 2 / MOTION 1`

- **ENERGY 1**: tenang. Tidak ada yang berteriak, tidak ada gradient, tidak ada glow.
- **RHYTHM 2**: bagian-bagiannya berbeda bentuk sesuai isinya. Halaman tamu
  (form) beda dari dashboard (kartu ringkasan) dan beda dari halaman job
  (rel fase). Bukan satu pola yang diulang.
- **MOTION 1**: hanya keadaan hover, focus, dan pembaruan progres yang memang
  bergerak karena datanya berubah. Tidak ada animasi masuk.

## Warna

| Peran | Nilai | Alasan |
|---|---|---|
| Latar | `#f7f7f5` (kertas hangat, bukan putih murni) | putih murni menyilaukan untuk layar lama; hangat sedikit lebih enak dibaca |
| Permukaan (kartu) | `#ffffff` | kartu harus terbaca sebagai lapisan di atas latar |
| Garis | `#e2e2de` | pemisah, bukan hiasan |
| Teks utama | `#17171a` | kontras 16:1, jauh di atas ambang WCAG AA |
| Teks sekunder | `#5c5c63` | kontras 7:1, tetap aman untuk teks kecil |
| Teks redup | `#8a8a92` | hanya untuk label huruf kecil, kontras 4,6:1 |
| Aksen | `#c8281f` | satu-satunya warna berani. Dipakai untuk aksi utama, status hidup, dan penanda fase aktif |
| Aksen redup | `#8f1c15` | untuk keadaan hover/tekan pada aksen |
| Baik | `#1c7a4a` | status selesai |
| Gagal | `#a8261d` | status gagal, sengaja dekat dengan aksen supaya keluarga warna tetap kecil |

Palet inti: monokrom + satu aksen. Hijau/merah status adalah keadaan, bukan
warna merek. Tidak ada biru.

## Tipografi

- Teks: tumpukan sans sistem (`-apple-system, Segoe UI, Roboto, Helvetica`).
  Alasan: nol unduhan, tampil asli di tiap perangkat, dan tidak memakai font
  bawaan agen (Inter/Geist) yang jadi penanda hasil AI.
- Data mesin (IP, port, username, sidik jari, baris log fase): monospace sistem.
  Alasan: angka-angka ini yang paling sering salah dibaca dan disalin; lebar
  tetap membuatnya bisa dicocokkan kolom demi kolom. Monospace **tidak** dipakai
  untuk judul.
- Judul halaman 22-26 px, bobot 600. Tidak ada huruf besar berjarak lebar.

## Bentuk dan jarak

- Radius 8 px untuk kartu dan tombol, 5 px untuk label kecil. Tidak ada bentuk pil.
- Garis 1 px sebagai pemisah; bayangan hanya satu tingkat untuk kartu yang
  memang mengambang di atas latar, tidak untuk setiap elemen.
- Jarak antar bagian 20-28 px; di dalam kartu 12-16 px.

## Ciri yang diulang (motif identitas)

**Rel fase.** Perjalanan instalasi (SSH masuk, unduh, reboot, Windows hidup)
digambar sebagai garis bertahap horizontal dengan empat titik: titik yang sudah
lewat berisi aksen, titik aktif berdenyut halus, titik berikutnya kosong. Ini
bukan daftar "how it works" berikon bulat bernomor; ini penunjuk posisi mesin
pelanggan, muncul di halaman job dan di halaman tamu.

## Aturan isi

- Bahasa Indonesia, kalimat pendek, tanpa tanda pisah panjang.
- Tidak ada emoji di web.
- Tidak ada angka atau klaim yang tidak punya sumber. Jumlah install yang
  ditampilkan hanya kalau dihitung dari database.
- Setiap tombol punya tujuan nyata. Yang belum jadi tidak ditampilkan.
- Tiga keadaan wajib ada di setiap tampilan data: kosong, sedang memuat, gagal.

## Yang sengaja tidak dipakai

Gradient, glow, kaca/blur, grid latar, ikon percikan/bintang, lencana pil di atas
judul, animasi masuk pada setiap elemen, monospace untuk judul, dan tumpukan
kartu fitur yang seragam.
