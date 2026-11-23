use std::path::PathBuf;

use clap::Args;
use deprader::repo_scan::{self, Result};

#[derive(Debug, Args)]
pub struct ScanArgs {
    /// Directory to inspect.
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Ignore pattern, relative to the scan root.
    #[arg(long)]
    exclude: Vec<String>,
    /// Fetch and verify the configured signed community IOC feed before scanning.
    #[arg(long, requires_all = ["rules_url", "rules_public_key"])]
    update_rules: bool,
    /// Direct HTTPS URL for the IOC JSON. The signature is served at URL.sig.
    #[arg(long, env = "DEPRADAR_RULES_URL")]
    rules_url: Option<String>,
    /// Base64-encoded trusted Ed25519 public key, obtained independently.
    #[arg(long, env = "DEPRADAR_RULES_PUBLIC_KEY")]
    rules_public_key: Option<String>,
}

pub fn scan(args: ScanArgs, configured_excludes: &[String]) -> Result<()> {
    let exclude = if args.exclude.is_empty() {
        configured_excludes
    } else {
        &args.exclude
    };
    let indicators = if args.update_rules {
        repo_scan::feed::fetch(
            args.rules_url.as_deref().ok_or("missing rules URL")?,
            args.rules_public_key
                .as_deref()
                .ok_or("missing rules public key")?,
        )?
    } else {
        Vec::new()
    };
    let report = repo_scan::scan(&args.path, exclude, &indicators)?;
    serde_json::to_writer_pretty(std::io::stdout().lock(), &report)?;
    println!();
    Ok(())
}

pub fn badge(
    path: &std::path::Path,
    receipt: Option<&std::path::Path>,
    public: bool,
) -> Result<()> {
    let now = repo_scan::now()?;
    let report = match std::fs::read(path) {
        Ok(bytes) => {
            let report: repo_scan::Report = serde_json::from_slice(&bytes)?;
            if public
                && !report.findings.is_empty()
                && report.scanned_at <= now
                && now - report.scanned_at < deprader::report::badge::MAX_AGE_SECONDS
            {
                let receipt = receipt.ok_or("a findings badge requires a disclosure receipt")?;
                check_embargo(&report_hash(&bytes), receipt, now, report.scanned_at)?;
            }
            Some(report)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    println!("{}", deprader::report::badge::render(report.as_ref(), now));
    Ok(())
}

#[derive(Debug, Args)]
pub struct BountyArgs {
    /// Private JSON scan report.
    report: PathBuf,
    /// Zero-based index of the manually confirmed finding.
    #[arg(long)]
    finding: usize,
    /// Receipt from repo-disclose.
    #[arg(long)]
    receipt: PathBuf,
    /// Public advisory reference.
    #[arg(long)]
    issue_ref: String,
    /// Router contract ID on Stellar testnet.
    #[arg(long)]
    router: String,
    /// Stellar CLI identity for signing and funding.
    #[arg(long)]
    source: String,
    /// Token contract ID.
    #[arg(long)]
    token: String,
    /// Amount in the token's smallest units.
    #[arg(long)]
    amount: std::num::NonZeroU64,
    /// Bounty expiry as Unix seconds.
    #[arg(long)]
    deadline: u64,
}

pub fn bounty(args: BountyArgs) -> Result<()> {
    let bytes = std::fs::read(&args.report)?;
    let report: repo_scan::Report = serde_json::from_slice(&bytes)?;
    if report.findings.get(args.finding).is_none() {
        return Err("the confirmed finding is absent from this report".into());
    }
    let hash = report_hash(&bytes);
    check_embargo(&hash, &args.receipt, repo_scan::now()?, report.scanned_at)?;
    let status = std::process::Command::new("stellar")
        .args(["contract", "invoke", "--network", "testnet"])
        .arg(format!("--id={}", args.router))
        .arg(format!("--source={}", args.source))
        .args(["--", "create_bounty_with_proof"])
        .arg(format!("--creator={}", args.source))
        .arg(format!("--issue_ref={}", args.issue_ref))
        .arg(format!("--proof_ref={hash}"))
        .arg(format!("--token={}", args.token))
        .arg(format!("--amount={}", args.amount))
        .arg(format!("--deadline={}", args.deadline))
        .status()?;
    if !status.success() {
        return Err(format!("Stellar contract invocation failed ({status})").into());
    }
    Ok(())
}

fn report_hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

const MIN_EMBARGO: u64 = 7 * 24 * 60 * 60;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Disclosure {
    report_sha256: String,
    privately_notified_at: u64,
    embargo_until: u64,
}

pub fn record_disclosure(
    report: &std::path::Path,
    receipt: &std::path::Path,
    embargo_until: u64,
) -> Result<()> {
    use std::io::Write;
    let bytes = std::fs::read(report)?;
    let report: repo_scan::Report = serde_json::from_slice(&bytes)?;
    let now = repo_scan::now()?;
    if report.findings.is_empty() || report.scanned_at > now {
        return Err("disclosure requires a completed scan with findings".into());
    }
    if embargo_until < now + MIN_EMBARGO {
        return Err("embargo must extend at least seven days from private notification".into());
    }
    let disclosure = Disclosure {
        report_sha256: report_hash(&bytes),
        privately_notified_at: now,
        embargo_until,
    };
    let data = serde_json::to_vec_pretty(&disclosure)?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(receipt)?.write_all(&data)?;
    Ok(())
}

fn check_embargo(hash: &str, receipt: &std::path::Path, now: u64, scanned_at: u64) -> Result<()> {
    let disclosure: Disclosure = serde_json::from_reader(std::fs::File::open(receipt)?)?;
    if disclosure.report_sha256 != hash {
        return Err("disclosure receipt belongs to a different report".into());
    }
    if disclosure.privately_notified_at < scanned_at
        || disclosure
            .embargo_until
            .saturating_sub(disclosure.privately_notified_at)
            < MIN_EMBARGO
    {
        return Err("invalid disclosure dates".into());
    }
    if now < disclosure.embargo_until {
        return Err(format!("embargo ends at Unix time {}", disclosure.embargo_until).into());
    }
    Ok(())
}

pub fn publish(report: &std::path::Path, receipt: &std::path::Path) -> Result<()> {
    use std::io::Write;
    let bytes = std::fs::read(report)?;
    let report: repo_scan::Report = serde_json::from_slice(&bytes)?;
    check_embargo(
        &report_hash(&bytes),
        receipt,
        repo_scan::now()?,
        report.scanned_at,
    )?;
    std::io::stdout().lock().write_all(&bytes)?;
    Ok(())
}
