use serde::Serialize;

use crate::models::Job;

/// Satu titik pada rel fase.
#[derive(Debug, Clone, Serialize)]
pub struct RailStep {
    pub label: &'static str,
    pub state: &'static str,
}

/// Empat tahap perjalanan instalasi. Diturunkan dari status + fase yang
/// tersimpan, jadi yang ditampilkan selalu keadaan sebenarnya.
pub fn rail_for(job: &Job) -> Vec<RailStep> {
    use crate::models::Stage;
    let idx = match job.stage() {
        Stage::Antre | Stage::Diperiksa => 0,
        Stage::Mengunduh => 1,
        Stage::Reboot => 2,
        Stage::Windows => 3,
        Stage::Selesai => 4,
        Stage::Gagal => 99,
    };
    const LABELS: [&str; 4] = ["diperiksa", "unduh", "reboot", "Windows"];
    LABELS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let state = if idx == 99 {
                if i == 0 {
                    "bad"
                } else {
                    "future"
                }
            } else if i < idx {
                "past"
            } else if i == idx {
                "active"
            } else {
                "future"
            };
            RailStep { label, state }
        })
        .collect()
}

pub fn rupiah(n: i64) -> String {
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(ch);
    }
    if n < 0 {
        format!("-Rp{out}")
    } else {
        format!("Rp{out}")
    }
}

pub fn dt(ts: i64) -> String {
    if ts <= 0 {
        return "-".into();
    }
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|d| {
            let wib = d + chrono::Duration::hours(7);
            wib.format("%d %b %Y %H:%M").to_string()
        })
        .unwrap_or_else(|| "-".into())
}

/// Jarak waktu ringkas untuk baris log.
pub fn ago(ts: i64) -> String {
    let now = crate::db::now();
    let d = (now - ts).max(0);
    if d < 60 {
        format!("{d} detik lalu")
    } else if d < 3600 {
        format!("{} menit lalu", d / 60)
    } else if d < 86400 {
        format!("{} jam lalu", d / 3600)
    } else {
        format!("{} hari lalu", d / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Job, Stage};

    fn job(status: &str, phase: &str) -> Job {
        Job {
            id: 1,
            user_id: 1,
            mode: "reinstall".into(),
            provider: "ssh".into(),
            status: status.into(),
            phase: phase.into(),
            progress: 0,
            os_label: "Server 2022".into(),
            zone: String::new(),
            plan: String::new(),
            host: "203.0.113.9".into(),
            rdp_port: 3389,
            rdp_user: "Administrator".into(),
            provider_ref: String::new(),
            ssh_port: 22,
            ssh_user: "root".into(),
            guest: 0,
            voucher_code: String::new(),
            edition: "srv2022".into(),
            lang: "en-us".into(),
            error: String::new(),
            created_at: 0,
            updated_at: 0,
            done_at: 0,
            expires_at: 0,
            wiped: 0,
        }
    }

    #[test]
    fn rail_follows_the_phase() {
        let steps = rail_for(&job("running", "menghubungi VPS lewat SSH"));
        assert_eq!(steps[0].state, "active");
        assert_eq!(steps[1].state, "future");

        let steps = rail_for(&job("running", "VPS mengunduh image Windows"));
        assert_eq!(steps[0].state, "past");
        assert_eq!(steps[1].state, "active");

        let steps = rail_for(&job("running", "VPS reboot, memasang Windows"));
        assert_eq!(steps[1].state, "past");
        assert_eq!(steps[2].state, "active");

        let steps = rail_for(&job("running", "Windows hidup, memastikan RDP stabil"));
        assert_eq!(steps[3].state, "active");

        let steps = rail_for(&job("done", "RDP siap"));
        assert!(steps.iter().all(|s| s.state == "past"));
    }

    #[test]
    fn failed_job_marks_the_first_step_as_bad() {
        let steps = rail_for(&job("failed", "gagal"));
        assert_eq!(steps[0].state, "bad");
        assert_eq!(steps[3].state, "future");
    }

    #[test]
    fn stage_detection_matches_stored_phase() {
        assert_eq!(job("queued", "menunggu").stage(), Stage::Antre);
        assert_eq!(job("running", "memeriksa mesin").stage(), Stage::Diperiksa);
        assert_eq!(job("done", "apa saja").stage(), Stage::Selesai);
        assert_eq!(job("failed", "apa saja").stage(), Stage::Gagal);
    }

    #[test]
    fn rupiah_uses_indonesian_separators() {
        assert_eq!(rupiah(0), "Rp0");
        assert_eq!(rupiah(5_000), "Rp5.000");
        assert_eq!(rupiah(1_234_567), "Rp1.234.567");
        assert_eq!(rupiah(-5_000), "-Rp5.000");
    }
}
