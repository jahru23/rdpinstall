//! Mode "cloud lain": VPS Linux milik pelanggan dirombak jadi Windows lewat
//! SSH root + password, memakai skrip bin456789/reinstall.
//!
//! Urutannya sengaja: precheck dulu (kalau mesin tidak memenuhi syarat, lebih
//! baik menolak daripada meninggalkan mesin setengah rusak), baru push skrip
//! yang berjalan detached. Instalasi akan me-reboot mesin dan memutus SSH —
//! itu normal, bukan kegagalan.

use std::fmt;

pub const REINSTALL_URL: &str =
    "https://raw.githubusercontent.com/bin456789/reinstall/main/reinstall.sh";

/// Ditolak: reinstall tidak mungkin di virtualisasi kontainer.
const BLOCKED_VIRT: [&str; 6] = [
    "openvz",
    "lxc",
    "lxc-libvirt",
    "openvzve",
    "vserver",
    "docker",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    Win10Pro,
    Win10Ltsc2021,
    Win11Pro,
    Win11Ltsc2024,
    Server2016,
    Server2019,
    Server2022,
    Server2025,
}

impl Edition {
    pub const ALL: [Edition; 8] = [
        Edition::Server2022,
        Edition::Server2019,
        Edition::Server2025,
        Edition::Server2016,
        Edition::Win11Ltsc2024,
        Edition::Win11Pro,
        Edition::Win10Ltsc2021,
        Edition::Win10Pro,
    ];

    /// String yang diteruskan ke `--image-name`. Ejaan mengikuti parser di
    /// reinstall.sh (`windows [server] <versi> [r2] <edisi>`); huruf besar/kecil
    /// tidak masalah karena skrip menormalkannya sendiri.
    pub fn image_name(self) -> &'static str {
        match self {
            Edition::Win10Pro => "Windows 10 Pro",
            Edition::Win10Ltsc2021 => "Windows 10 Enterprise LTSC 2021",
            Edition::Win11Pro => "Windows 11 Pro",
            Edition::Win11Ltsc2024 => "Windows 11 Enterprise LTSC 2024",
            Edition::Server2016 => "Windows Server 2016 SERVERDATACENTER",
            Edition::Server2019 => "Windows Server 2019 SERVERDATACENTER",
            Edition::Server2022 => "Windows Server 2022 SERVERDATACENTER",
            Edition::Server2025 => "Windows Server 2025 SERVERDATACENTER",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Edition::Win10Pro => "w10pro",
            Edition::Win10Ltsc2021 => "w10ltsc21",
            Edition::Win11Pro => "w11pro",
            Edition::Win11Ltsc2024 => "w11ltsc24",
            Edition::Server2016 => "srv2016",
            Edition::Server2019 => "srv2019",
            Edition::Server2022 => "srv2022",
            Edition::Server2025 => "srv2025",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Edition::Win10Pro => "Windows 10 Pro",
            Edition::Win10Ltsc2021 => "Windows 10 LTSC 2021",
            Edition::Win11Pro => "Windows 11 Pro",
            Edition::Win11Ltsc2024 => "Windows 11 LTSC 2024",
            Edition::Server2016 => "Server 2016",
            Edition::Server2019 => "Server 2019",
            Edition::Server2022 => "Server 2022",
            Edition::Server2025 => "Server 2025",
        }
    }

    pub fn note(self) -> &'static str {
        match self {
            Edition::Win10Pro => "paling dikenal, banyak panduan",
            Edition::Win10Ltsc2021 => "tanpa bloat, jarang update",
            Edition::Win11Pro => "butuh RAM 2 GB ke atas",
            Edition::Win11Ltsc2024 => "paling bersih untuk RDP",
            Edition::Server2016 => "paling ringan",
            Edition::Server2019 => "pilihan umum untuk RDP",
            Edition::Server2022 => "rekomendasi server",
            Edition::Server2025 => "terbaru, butuh RAM 2 GB ke atas",
        }
    }

    /// Kebutuhan minimum menurut README reinstall (1 GB, 2 GB untuk Win11/2025).
    pub fn min_ram_gb(self) -> u32 {
        match self {
            Edition::Win11Pro | Edition::Win11Ltsc2024 | Edition::Server2025 => 2,
            _ => 1,
        }
    }

    pub fn from_key(key: &str) -> Option<Edition> {
        Edition::ALL.iter().copied().find(|e| e.key() == key)
    }
}

impl fmt::Display for Edition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    ZhCn,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en-us",
            Lang::ZhCn => "zh-cn",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::ZhCn => "Chinese Simplified",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        match code {
            "en-us" | "en" => Some(Lang::En),
            "zh-cn" | "cn" => Some(Lang::ZhCn),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstallSpec {
    pub image_name: String,
    pub lang: String,
    pub username: String,
    pub password: String,
    pub rdp_port: u16,
    pub ssh_port: u16,
    pub allow_ping: bool,
}

/// Perintah reinstall.sh. Password TIDAK ikut di sini: pada bootstrap ia
/// dipasang lewat variabel lingkungan lalu dirujuk sebagai $RDP_PW, supaya
/// tidak muncul di daftar proses maupun riwayat perintah.
pub fn build_command(spec: &InstallSpec) -> String {
    let mut parts: Vec<String> = vec![
        "bash jay-installer.sh windows".into(),
        format!("--image-name={}", shell_quote(&spec.image_name)),
        format!("--lang={}", shell_quote(&spec.lang)),
        format!("--username={}", shell_quote(&spec.username)),
        "--password=\"$RDP_PW\"".into(),
        format!("--rdp-port={}", spec.rdp_port),
    ];
    if spec.allow_ping {
        parts.push("--allow-ping".into());
    }
    if spec.ssh_port != 22 {
        parts.push(format!("--ssh-port={}", spec.ssh_port));
    }
    parts.join(" ")
}

/// Skrip yang dijalankan di VPS pelanggan: unduh installer, jalankan
/// detached dengan output ke berkas log, lalu cetak penanda supaya kita tahu
/// prosesnya benar-benar mulai.
pub fn build_bootstrap(spec: &InstallSpec) -> String {
    let cmd = build_command(spec);
    let pw = shell_single_quote(&spec.password);
    let cmd_q = shell_single_quote(&cmd);
    format!(
        r#"set -e
cd /root 2>/dev/null || cd /tmp
if ! command -v curl >/dev/null 2>&1; then
  (command -v apt-get >/dev/null 2>&1 && apt-get update -qq && apt-get install -y -qq curl) \
  || (command -v yum >/dev/null 2>&1 && yum install -y -q curl) \
  || (command -v dnf >/dev/null 2>&1 && dnf install -y -q curl) || true
fi
if command -v curl >/dev/null 2>&1; then
  curl -fsSL -o jay-installer.sh {url}
else
  wget -qO jay-installer.sh {url}
fi
chmod +x jay-installer.sh
export RDP_PW={pw}
nohup bash -c {cmd_q} >/root/rdp-install.log 2>&1 &
echo "LAUNCHED $!"
"#,
        url = REINSTALL_URL,
        pw = pw,
        cmd_q = cmd_q,
    )
}

/// Untuk tombol pemulihan kalau instalasi meninggalkan mesin tidak bisa boot.
pub fn reset_command() -> &'static str {
    "bash jay-installer.sh reset"
}

pub fn shell_quote(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-/:@+ ".contains(c))
    {
        format!("\"{s}\"")
    } else {
        shell_single_quote(s)
    }
}

pub fn shell_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Sembunyikan nilai password di setiap baris log yang kita tampilkan.
pub fn redact(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(pos) = rest.find("--password") {
        let (head, tail) = rest.split_at(pos);
        out.push_str(head);
        out.push_str("--password ***");
        // lewati nilai setelah '=' atau spasi
        let after = &tail["--password".len()..];
        let skip = match after.find([' ', '=']) {
            // nilai diawali spasi: lompati nilai, sisakan spasi pemisahnya
            Some(0) => after[1..].find(' ').map(|p| p + 1).unwrap_or(after.len()),
            // nilai ditulis setelah '='
            Some(p) => p,
            None => 0,
        };
        rest = &after[skip..];
    }
    out.push_str(rest);
    out
}

// ------------------------------------------------------------------ precheck

#[derive(Debug, Clone, Default)]
pub struct Precheck {
    pub virt: String,
    pub os: String,
    pub kernel: String,
    pub arch: String,
    pub whoami: String,
    pub bootmode: String,
    pub ram_gb: f64,
    pub disk_gb: f64,
    pub has_curl: bool,
}

#[derive(Debug, Clone)]
pub struct PrecheckError(pub String);

impl fmt::Display for PrecheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Skrip precheck. Tidak mengubah apa pun di mesin target.
pub const PRECHECK_SCRIPT: &str = r#"set -e
echo "VIRT=$(systemd-detect-virt 2>/dev/null || echo unknown)"
echo "OS=$((. /etc/os-release 2>/dev/null && echo "$PRETTY_NAME") || uname -s)"
echo "KERNEL=$(uname -r)"
echo "ARCH=$(uname -m)"
echo "WHOAMI=$(id -un)"
echo "BOOTMODE=$([ -d /sys/firmware/efi ] && echo efi || echo bios)"
echo "RAMKB=$(awk '/MemTotal/{print $2}' /proc/meminfo)"
ROOTDEV=$(lsblk -ndo NAME,TYPE 2>/dev/null | awk '$2=="disk"{print "/dev/"$1; exit}')
echo "ROOTDEV=$ROOTDEV"
echo "DISKBYTES=$(lsblk -ndbo SIZE "$ROOTDEV" 2>/dev/null | head -1)"
command -v curl >/dev/null 2>&1 && echo "HAVECURL=yes" || echo "HAVECURL=no"
"#;

pub fn parse_precheck(out: &str) -> Precheck {
    let mut p = Precheck {
        virt: "unknown".into(),
        has_curl: false,
        ..Default::default()
    };
    for line in out.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let v = v.trim();
        match k.trim() {
            "VIRT" => p.virt = v.to_lowercase(),
            "OS" => p.os = v.to_string(),
            "KERNEL" => p.kernel = v.to_string(),
            "ARCH" => p.arch = v.to_string(),
            "WHOAMI" => p.whoami = v.to_string(),
            "BOOTMODE" => p.bootmode = v.to_string(),
            "RAMKB" => p.ram_gb = v.parse::<f64>().unwrap_or(0.0) / 1024.0 / 1024.0,
            "DISKBYTES" => p.disk_gb = v.parse::<f64>().unwrap_or(0.0) / 1024.0 / 1024.0 / 1024.0,
            "HAVECURL" => p.has_curl = v.eq_ignore_ascii_case("yes"),
            _ => {}
        }
    }
    p
}

/// Tolak lebih awal dengan alasan yang bisa dibaca manusia. Batas RAM/disk
/// mengikuti README reinstall; Windows 11 dan Server 2025 butuh 2 GB.
pub fn validate(p: &Precheck, edition: Edition) -> Result<(), PrecheckError> {
    if BLOCKED_VIRT.contains(&p.virt.as_str()) || p.virt.contains("openvz") || p.virt == "lxc" {
        return Err(PrecheckError(format!(
            "VPS ini memakai virtualisasi {} (kontainer). Reinstall hanya bisa di KVM.",
            if p.virt.is_empty() {
                "tidak dikenal"
            } else {
                &p.virt
            }
        )));
    }
    if !p.whoami.is_empty() && p.whoami != "root" {
        return Err(PrecheckError(format!(
            "Masuk sebagai '{}', bukan root. Reinstall butuh akses root.",
            p.whoami
        )));
    }
    let min_ram = edition.min_ram_gb() as f64;
    if p.ram_gb > 0.0 && p.ram_gb + 0.05 < min_ram {
        return Err(PrecheckError(format!(
            "RAM {:.2} GB, sedangkan {} butuh minimal {} GB.",
            p.ram_gb,
            edition.label(),
            min_ram
        )));
    }
    if p.disk_gb > 0.0 && p.disk_gb < 25.0 {
        return Err(PrecheckError(format!(
            "Disk {:.1} GB, minimal 25 GB untuk memasang Windows.",
            p.disk_gb
        )));
    }
    if p.arch != "x86_64" && p.arch != "aarch64" && !p.arch.is_empty() {
        return Err(PrecheckError(format!(
            "Arsitektur {} belum didukung (hanya x86_64 dan aarch64).",
            p.arch
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> InstallSpec {
        InstallSpec {
            image_name: Edition::Server2022.image_name().into(),
            lang: "en-us".into(),
            username: "JayAdmin".into(),
            password: "Kata1!SandiXy".into(),
            rdp_port: 3389,
            ssh_port: 22,
            allow_ping: false,
        }
    }

    #[test]
    fn command_matches_documented_shape() {
        let c = build_command(&spec());
        assert!(c.starts_with("bash jay-installer.sh windows "));
        assert!(c.contains(r#"--image-name="Windows Server 2022 SERVERDATACENTER""#));
        assert!(c.contains("--lang=\"en-us\""));
        assert!(c.contains("--username=\"JayAdmin\""));
        assert!(c.contains("--rdp-port=3389"));
        assert!(
            !c.contains("--ssh-port"),
            "port 22 default tidak perlu ditulis"
        );
        assert!(!c.contains("--allow-ping"));
    }

    #[test]
    fn password_never_appears_in_the_command() {
        let c = build_command(&spec());
        assert!(
            !c.contains("Kata1!SandiXy"),
            "password tidak boleh di argv: {c}"
        );
        assert!(c.contains("--password=\"$RDP_PW\""));
    }

    #[test]
    fn bootstrap_passes_password_via_environment_and_is_detached() {
        let b = build_bootstrap(&spec());
        assert!(b.contains("export RDP_PW='Kata1!SandiXy'"));
        assert!(
            b.contains("nohup bash -c "),
            "harus detached, instalasi me-reboot mesin"
        );
        assert!(b.contains("chmod +x jay-installer.sh"));
        assert!(b.contains(REINSTALL_URL));
        assert!(b.contains("LAUNCHED"));
    }

    #[test]
    fn bootstrap_survives_password_with_quote() {
        let mut s = spec();
        s.password = "Pw'dengan'kutip".into();
        let b = build_bootstrap(&s);
        assert!(
            b.contains(r"'Pw'\''dengan'\''kutip'"),
            "kutip harus di-escape"
        );
    }

    #[test]
    fn ssh_port_and_ping_appear_when_requested() {
        let mut s = spec();
        s.ssh_port = 2222;
        s.allow_ping = true;
        let c = build_command(&s);
        assert!(c.contains("--ssh-port=2222"));
        assert!(c.contains("--allow-ping"));
    }

    #[test]
    fn parses_precheck_output() {
        let out = "VIRT=kvm\nOS=Debian GNU/Linux 11 (bullseye)\nKERNEL=5.10.0-8-amd64\nARCH=x86_64\nWHOAMI=root\nBOOTMODE=bios\nRAMKB=2048000\nROOTDEV=/dev/vda\nDISKBYTES=53687091200\nHAVECURL=yes\n";
        let p = parse_precheck(out);
        assert_eq!(p.virt, "kvm");
        assert_eq!(p.arch, "x86_64");
        assert_eq!(p.whoami, "root");
        assert!(p.ram_gb > 1.9 && p.ram_gb < 2.0);
        assert!(p.disk_gb > 49.0 && p.disk_gb < 51.0);
        assert!(p.has_curl);
        assert!(validate(&p, Edition::Server2022).is_ok());
    }

    #[test]
    fn rejects_containers_and_small_machines() {
        let mut p = parse_precheck(
            "VIRT=lxc\nWHOAMI=root\nRAMKB=1048576\nDISKBYTES=53687091200\nARCH=x86_64",
        );
        assert!(
            validate(&p, Edition::Server2022).is_err(),
            "LXC tidak bisa di-reinstall"
        );

        p.virt = "kvm".into();
        p.ram_gb = 1.0;
        assert!(
            validate(&p, Edition::Server2022).is_ok(),
            "1 GB cukup untuk Server 2022"
        );
        assert!(
            validate(&p, Edition::Win11Pro).is_err(),
            "Windows 11 butuh 2 GB"
        );

        p.ram_gb = 4.0;
        p.disk_gb = 20.0;
        assert!(
            validate(&p, Edition::Server2022).is_err(),
            "disk < 25 GB ditolak"
        );

        p.disk_gb = 50.0;
        p.whoami = "ubuntu".into();
        assert!(
            validate(&p, Edition::Server2022).is_err(),
            "bukan root ditolak"
        );
    }

    #[test]
    fn redacts_password_values_from_logs() {
        assert_eq!(
            redact("bash reinstall.sh windows --password KitaRahasia --rdp-port=3389"),
            "bash reinstall.sh windows --password *** --rdp-port=3389"
        );
        assert_eq!(
            redact("--password=KitaRahasia --lang=en-us"),
            "--password *** --lang=en-us"
        );
        assert_eq!(
            redact("tidak ada rahasia di sini"),
            "tidak ada rahasia di sini"
        );
    }

    #[test]
    fn every_edition_has_distinct_key_and_image_name() {
        let mut keys: Vec<&str> = Edition::ALL.iter().map(|e| e.key()).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), Edition::ALL.len());
        for e in Edition::ALL {
            assert!(e.image_name().to_lowercase().starts_with("windows"));
            assert_eq!(Edition::from_key(e.key()), Some(e));
        }
    }
}
