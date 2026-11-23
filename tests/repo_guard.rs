use std::{fs, path::PathBuf, process::Command};

use base64::{engine::general_purpose::STANDARD, Engine};
use deprader::repo_scan::{self, feed, Report};
use ring::signature::{Ed25519KeyPair, KeyPair};
use sha2::{Digest, Sha256};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_depradar"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/repo-scan")
        .join(name)
}

#[test]
fn scan_reports_findings_and_respects_ignore_rules() {
    let positive = cli()
        .arg("repo-scan")
        .arg(fixture("positive"))
        .output()
        .unwrap();
    assert!(positive.status.success(), "{:?}", positive.stderr);
    let report: Report = serde_json::from_slice(&positive.stdout).unwrap();
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].file, "postcss.config.js");
    assert_eq!(report.findings[0].line, 1);

    let temporary = tempfile::tempdir().unwrap();
    fs::copy(
        fixture("positive").join("postcss.config.js"),
        temporary.path().join("postcss.config.js"),
    )
    .unwrap();
    fs::write(temporary.path().join(".gitignore"), "postcss.config.js\n").unwrap();
    assert!(repo_scan::scan(temporary.path(), &[], &[])
        .unwrap()
        .findings
        .is_empty());
    assert!(repo_scan::scan(&fixture("negative"), &[], &[])
        .unwrap()
        .findings
        .is_empty());
}

#[test]
fn badge_distinguishes_clean_findings_and_stale() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("report.json");
    for (case, state) in [("positive", "findings"), ("negative", "clean")] {
        let report = repo_scan::scan(&fixture(case), &[], &[]).unwrap();
        fs::write(&path, serde_json::to_vec(&report).unwrap()).unwrap();
        let badge = cli().arg("repo-badge").arg(&path).output().unwrap();
        assert!(badge.status.success());
        assert!(String::from_utf8(badge.stdout)
            .unwrap()
            .contains(&format!("Scanned by DepRadar: {state}")));
    }
    let mut report = repo_scan::scan(&fixture("positive"), &[], &[]).unwrap();
    report.scanned_at = 1;
    fs::write(&path, serde_json::to_vec(&report).unwrap()).unwrap();
    let badge = cli().arg("repo-badge").arg(&path).output().unwrap();
    assert!(badge.status.success());
    assert!(String::from_utf8(badge.stdout)
        .unwrap()
        .contains("Scanned by DepRadar: stale"));
}

#[test]
fn publication_enforces_embargo_and_binds_receipt_to_report() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("report.json");
    let receipt = dir.path().join("receipt.json");
    let mut report = repo_scan::scan(&fixture("positive"), &[], &[]).unwrap();
    let bytes = serde_json::to_vec(&report).unwrap();
    fs::write(&path, &bytes).unwrap();
    let now = repo_scan::now().unwrap();
    let embargo = 7 * 24 * 60 * 60;
    let record = cli()
        .arg("repo-disclose")
        .arg(&path)
        .arg(&receipt)
        .arg("--embargo-until")
        .arg((now + embargo + 60).to_string())
        .output()
        .unwrap();
    assert!(record.status.success(), "{:?}", record.stderr);
    let publish = || {
        cli()
            .arg("repo-publish")
            .arg(&path)
            .arg(&receipt)
            .output()
            .unwrap()
    };
    let blocked = publish();
    assert!(!blocked.status.success());
    assert!(blocked.stdout.is_empty());
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("embargo ends"));
    let badge = cli()
        .arg("repo-badge")
        .arg(&path)
        .arg("--public")
        .arg("--receipt")
        .arg(&receipt)
        .output()
        .unwrap();
    assert!(!badge.status.success() && badge.stdout.is_empty());

    report.scanned_at = now - embargo - 60;
    let bytes = serde_json::to_vec(&report).unwrap();
    fs::write(&path, &bytes).unwrap();
    assert!(String::from_utf8_lossy(&publish().stderr).contains("different report"));
    write_expired_receipt(&receipt, &bytes, now);
    let released = publish();
    assert!(released.status.success());
    assert_eq!(released.stdout, bytes);
}

fn write_expired_receipt(path: &std::path::Path, bytes: &[u8], now: u64) {
    fs::write(
        path,
        serde_json::to_vec(&serde_json::json!({
            "report_sha256": format!("{:x}", Sha256::digest(bytes)),
            "privately_notified_at": now - 7 * 24 * 60 * 60 - 10,
            "embargo_until": now - 10,
        }))
        .unwrap(),
    )
    .unwrap();
}

#[cfg(unix)]
#[test]
fn bounty_passes_report_digest_to_stellar_and_rejects_missing_findings() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("report.json");
    let receipt = dir.path().join("receipt.json");
    let stellar = dir.path().join("stellar");
    fs::write(&stellar, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n").unwrap();
    fs::set_permissions(&stellar, fs::Permissions::from_mode(0o700)).unwrap();

    let now = repo_scan::now().unwrap();
    let mut report = repo_scan::scan(&fixture("positive"), &[], &[]).unwrap();
    report.scanned_at = now - 8 * 24 * 60 * 60;
    let bytes = serde_json::to_vec(&report).unwrap();
    fs::write(&path, &bytes).unwrap();
    write_expired_receipt(&receipt, &bytes, now);

    let open = |index| {
        cli()
            .arg("repo-bounty")
            .arg(&path)
            .args([
                "--finding",
                index,
                "--router",
                "router",
                "--source",
                "alice",
                "--issue-ref",
                "test",
                "--token",
                "token",
                "--amount",
                "10",
                "--deadline",
                "1",
            ])
            .arg("--receipt")
            .arg(&receipt)
            .env("PATH", dir.path())
            .output()
            .unwrap()
    };
    let opened = open("0");
    assert!(opened.status.success(), "{:?}", opened.stderr);
    assert!(String::from_utf8_lossy(&opened.stdout)
        .contains(&format!("--proof_ref={:x}", Sha256::digest(&bytes))));
    let denied = open("1");
    assert!(!denied.status.success() && denied.stdout.is_empty());
}

#[test]
fn signed_community_rules_detect_defanged_indicators_and_reject_tampering() {
    let key = Ed25519KeyPair::from_seed_unchecked(&[23; 32]).unwrap();
    let payload = br#"[{"id":"ioc/example/marker","marker":"DEPRADAR_DEFANGED_INDICATOR"}]"#;
    let signature = STANDARD.encode(key.sign(payload));
    let public_key = STANDARD.encode(key.public_key());
    let indicators = feed::verify(payload, &signature, &public_key).unwrap();
    let report = repo_scan::scan(&fixture("negative"), &[], &indicators).unwrap();
    assert_eq!(report.findings[0].rule_id, "ioc/example/marker");
    let mut modified = payload.to_vec();
    modified.push(b' ');
    assert!(feed::verify(&modified, &signature, &public_key)
        .unwrap_err()
        .to_string()
        .contains("signature verification failed"));
    assert!(feed::verify(payload, &signature, &STANDARD.encode([0; 32])).is_err());
}
