//! Validasi masukan yang dipakai jalur tamu maupun jalur berakun.

use crate::error::AppError;
use crate::reinstall::Edition;

pub fn is_private_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.octets()[0] == 0
                || (v4.octets()[0] == 100 && v4.octets()[1] >= 64 && v4.octets()[1] <= 127)
        }
        std::net::IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified() || v6.is_unique_local(),
    }
}

pub fn host(raw: &str) -> Result<String, AppError> {
    let s = raw.trim().trim_matches(|c: char| c == ',' || c == ';');
    if s.is_empty() || s.len() > 253 {
        return Err(AppError::bad("Alamat VPS tidak valid."));
    }
    if let Ok(ip) = s.parse::<std::net::IpAddr>() {
        if is_private_ip(&ip) {
            return Err(AppError::bad(
                "Masukkan IP publik VPS. Alamat privat tidak bisa dipakai.",
            ));
        }
        return Ok(s.to_string());
    }
    let ok = s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    if !ok || s.starts_with('-') || s.ends_with('-') || s.ends_with('.') || s.starts_with('.') {
        return Err(AppError::bad("Alamat VPS tidak valid."));
    }
    let s_lower = s.to_lowercase();
    if s_lower == "localhost"
        || s_lower.ends_with(".localhost")
        || s_lower.ends_with(".local")
        || s_lower.ends_with(".internal")
        || !s_lower.contains('.')
    {
        return Err(AppError::bad(
            "Masukkan alamat IP publik atau domain publik yang valid.",
        ));
    }
    Ok(s_lower)
}

pub fn port(raw: &str, default: u16) -> u16 {
    raw.trim()
        .parse::<u16>()
        .ok()
        .filter(|p| *p > 0)
        .unwrap_or(default)
}

pub fn ssh_user(raw: &str) -> Result<String, AppError> {
    let s = raw.trim();
    if s.is_empty() {
        return Ok("root".to_string());
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
        || s.len() > 32
    {
        return Err(AppError::bad(
            "Username SSH hanya huruf, angka, titik, garis bawah, dan strip.",
        ));
    }
    Ok(s.to_string())
}

pub fn rdp_user(raw: &str) -> Result<String, AppError> {
    let s = raw.trim();
    if s.is_empty() {
        return Ok("Administrator".to_string());
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
        || s.len() > 20
    {
        return Err(AppError::bad(
            "Username Windows maksimal 20 karakter tanpa spasi.",
        ));
    }
    Ok(s.to_string())
}

/// Windows menolak password lemah. Lebih baik ketahuan sekarang daripada
/// setelah mesin pelanggan dirombak.
pub fn windows_password(pw: &str) -> Result<(), AppError> {
    if pw.chars().count() < 8 {
        return Err(AppError::bad("Password Windows minimal 8 karakter."));
    }
    let mut kinds = 0;
    if pw.chars().any(|c| c.is_ascii_lowercase()) {
        kinds += 1;
    }
    if pw.chars().any(|c| c.is_ascii_uppercase()) {
        kinds += 1;
    }
    if pw.chars().any(|c| c.is_ascii_digit()) {
        kinds += 1;
    }
    if pw.chars().any(|c| !c.is_alphanumeric()) {
        kinds += 1;
    }
    if kinds < 3 {
        return Err(AppError::bad(
            "Password Windows perlu minimal 3 dari 4: huruf kecil, huruf besar, angka, simbol.",
        ));
    }
    Ok(())
}

pub fn edition(key: &str) -> Edition {
    Edition::from_key(key).unwrap_or(Edition::Server2022)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_public_and_rejects_private_hosts() {
        assert_eq!(host("103.150.60.10").unwrap(), "103.150.60.10");
        assert_eq!(host(" vps.contoh.com ").unwrap(), "vps.contoh.com");
        assert!(host("192.168.1.10").is_err());
        assert!(host("10.0.0.5").is_err());
        assert!(host("127.0.0.1").is_err());
        assert!(host("::1").is_err());
        assert!(host("fd00::1").is_err());
        assert!(host("").is_err());
        assert!(host("-aneh").is_err());
        assert!(host("spasi ada").is_err());
        assert!(host("localhost").is_err());
        assert!(host("app.localhost").is_err());
        assert!(host("server.local").is_err());
    }

    #[test]
    fn ports_fall_back_to_defaults() {
        assert_eq!(port("2222", 22), 2222);
        assert_eq!(port("", 22), 22);
        assert_eq!(port("0", 22), 22);
        assert_eq!(port("bukan angka", 3389), 3389);
    }

    #[test]
    fn windows_password_rules() {
        assert!(windows_password("Kata1!Sandi").is_ok());
        assert!(
            windows_password("pendek1").is_err(),
            "7 karakter terlalu pendek"
        );
        assert!(
            windows_password("pendek1!").is_ok(),
            "8 karakter + 3 jenis sudah sah"
        );
        assert!(windows_password("semuakecil").is_err());
        assert!(windows_password("HurufBesarSaja").is_err());
    }

    #[test]
    fn usernames_are_sane() {
        assert_eq!(ssh_user("").unwrap(), "root");
        assert_eq!(rdp_user("").unwrap(), "Administrator");
        assert!(ssh_user("root;rm -rf /").is_err());
        assert!(rdp_user("nama dengan spasi").is_err());
    }
}
