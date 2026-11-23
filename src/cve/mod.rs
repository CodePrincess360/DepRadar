//! CVE / known-vulnerability matching.
//!
//! Starts with a client for the [OSV](https://osv.dev) API ([`osv_client`]),
//! which resolves a package name + ecosystem + version to the raw set of
//! known vulnerabilities affecting it. No filtering or scoring yet — that
//! belongs with the (not-yet-written) CVE scanner module.

pub mod cache;
pub mod osv_client;
pub mod remediation;
pub mod severity;

pub use osv_client::{fetch_vuln, OsvVulnerability};

/// Label used to mark OSV `MAL-` advisories as malicious-package findings.
pub const MALICIOUS_PACKAGE_LABEL: &str = "malicious package";

/// Returns `true` when the given OSV advisory ID denotes a known-malware
/// (`MAL-`) advisory rather than an ordinary vulnerability.
pub fn is_malicious_advisory(id: &str) -> bool {
    id.starts_with("MAL-")
}