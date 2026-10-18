//! Secrets at rest: AES-256-GCM with a master key held outside the DB.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{anyhow, bail, Result};

const PREFIX: &str = "v1";
const NONCE_LEN: usize = 12;

fn aes(master: &[u8; 32]) -> Result<Aes256Gcm> {
    Aes256Gcm::new_from_slice(master).map_err(|_| anyhow!("panjang kunci AES tidak valid"))
}

fn nonce(bytes: &[u8]) -> Result<Nonce<<Aes256Gcm as aes_gcm::aead::AeadCore>::NonceSize>> {
    Nonce::try_from(bytes).map_err(|_| anyhow!("nonce tidak valid"))
}

/// Returns `v1:<nonce_hex>:<ciphertext_hex>`.
pub fn seal(master: &[u8; 32], plaintext: &str) -> Result<String> {
    let mut nonce_bytes = [0u8; NONCE_LEN];
    getrandom::fill(&mut nonce_bytes).map_err(|e| anyhow!("entropy OS gagal: {e}"))?;
    let nonce = nonce(&nonce_bytes)?;
    let ct = aes(master)?
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|_| anyhow!("enkripsi gagal"))?;
    Ok(format!(
        "{PREFIX}:{}:{}",
        hex::encode(nonce_bytes),
        hex::encode(ct)
    ))
}

pub fn open(master: &[u8; 32], sealed: &str) -> Result<String> {
    let mut parts = sealed.splitn(3, ':');
    let (ver, nonce_hex, ct_hex) = match (parts.next(), parts.next(), parts.next()) {
        (Some(a), Some(b), Some(c)) => (a, b, c),
        _ => bail!("format ciphertext tidak dikenal"),
    };
    if ver != PREFIX {
        bail!("versi ciphertext tidak didukung: {ver}");
    }
    let nonce_bytes = hex::decode(nonce_hex)?;
    if nonce_bytes.len() != NONCE_LEN {
        bail!("nonce tidak valid");
    }
    let ct = hex::decode(ct_hex)?;
    let pt = aes(master)?
        .decrypt(&nonce(&nonce_bytes)?, ct.as_ref())
        .map_err(|_| anyhow!("dekripsi gagal (key salah atau data rusak)"))?;
    String::from_utf8(pt).map_err(|_| anyhow!("plaintext bukan utf-8"))
}

/// URL-safe random token (session ids, gateway references).
pub fn random_token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    // OS entropy; a failure here is fatal for security, so panic loudly.
    getrandom::fill(&mut buf).expect("entropy OS tidak tersedia");
    hex::encode(buf)
}

/// Kode acak untuk voucer: tanpa huruf/angka yang mudah tertukar (0/O, 1/I/L).
pub fn random_code(len: usize) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
    let mut buf = vec![0u8; len];
    getrandom::fill(&mut buf).expect("entropy OS tidak tersedia");
    buf.iter()
        .map(|b| ALPHABET[*b as usize % ALPHABET.len()] as char)
        .collect()
}

pub fn mask(secret: &str) -> String {
    let n = secret.chars().count();
    if n <= 8 {
        return "****".into();
    }
    let head: String = secret.chars().take(4).collect();
    let tail: String = secret.chars().skip(n - 4).collect();
    format!("{head}...{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    #[test]
    fn seal_open_roundtrip() {
        let k = key(7);
        let secret = "ucat_01DQE3AJDEBFEKECFM558TGH2F";
        let sealed = seal(&k, secret).unwrap();
        assert!(sealed.starts_with("v1:"));
        assert!(!sealed.contains(secret));
        assert_eq!(open(&k, &sealed).unwrap(), secret);
    }

    #[test]
    fn nonce_is_not_reused() {
        let k = key(1);
        let a = seal(&k, "sama").unwrap();
        let b = seal(&k, "sama").unwrap();
        assert_ne!(a, b, "nonce harus acak tiap panggilan");
        assert_eq!(open(&k, &a).unwrap(), open(&k, &b).unwrap());
    }

    #[test]
    fn wrong_key_and_tampering_fail() {
        let sealed = seal(&key(2), "rahasia").unwrap();
        assert!(open(&key(3), &sealed).is_err());

        // rusak satu byte ciphertext -> harus gagal, bukan mengembalikan sampah
        let mut parts: Vec<String> = sealed.split(':').map(|s| s.to_string()).collect();
        let mut ct = hex::decode(&parts[2]).unwrap();
        ct[0] ^= 0xff;
        parts[2] = hex::encode(ct);
        assert!(open(&key(2), &parts.join(":")).is_err());

        assert!(open(&key(2), "bukan-ciphertext").is_err());
        assert!(open(&key(2), "v9:00:00").is_err());
    }

    #[test]
    fn random_token_is_unique_and_hex() {
        let a = random_token(32);
        let b = random_token(32);
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn voucher_codes_avoid_ambiguous_characters() {
        let c = random_code(8);
        assert_eq!(c.len(), 8);
        assert!(c
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit()));
        for bad in ['0', 'O', '1', 'I', 'L'] {
            assert!(!c.contains(bad), "kode {c} memuat karakter ambigu {bad}");
        }
        assert_ne!(random_code(8), random_code(8));
    }

    #[test]
    fn mask_hides_middle() {
        assert_eq!(mask("ucat_01DQE3AJDEBFEKECFM558TGH2F"), "ucat...GH2F");
        assert_eq!(mask("abc"), "****");
    }
}
