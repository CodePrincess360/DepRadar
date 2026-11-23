//! Turning a scan [`Report`](crate::pipeline::Report) into output.
//!
//! Every finding gets a [`Severity`] ([`severity_of`]) so output formats and
//! the CLI's exit code can rank it. Vulnerabilities take the level of their
//! OSV advisory ([`AdvisorySeverities`]); typosquats are rated by which
//! signal fired.
//!
//! - [`sarif`] — SARIF 2.1.0, for GitHub code scanning
//! - [`text`] — terminal output with color-coded severity labels
//! - [`markdown`] — Markdown format for GitHub issues and PR comments

use std::collections::HashMap;

pub use crate::cve::severity::Severity;

use crate::cve::osv_client::get_vulnerability;
use crate::cve::severity::assess;
use crate::model::{Finding, FindingKind};

pub mod badge;
pub mod markdown;
pub mod sarif;
pub mod text;

/// Level given to a vulnerability whose advisory has no usable severity
/// data, or couldn't be fetched. Erring high keeps unrated advisories from
/// slipping past a CI gate.
pub const UNRATED_ADVISORY: Severity = Severity::High;

/// Prefix OSV uses for advisories about known malware.
pub const MALICIOUS_ADVISORY_PREFIX: &str = "MAL-";

/// Label shown for a malicious-package finding.
pub const MALICIOUS_LABEL: &str = "malicious package";

/// Whether `advisory_id` is an OSV malware advisory (`MAL-`).
pub fn is_malicious_advisory(advisory_id: &str) -> bool {
    advisory_id.starts_with(MALICIOUS_ADVISORY_PREFIX)
}

/// Lower-case name of a severity level, as shown in output.
pub fn label(severity: Severity) -> &'static str {
    match severity {
        Severity::Low => "low",
        Severity::Medium => "medium",
        Severity::High => "high",
        Severity::Critical => "critical",
    }
}

/// Severity levels of OSV advisories, by advisory ID.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdvisorySeverities {
    levels: HashMap<String, Severity>,
}

impl AdvisorySeverities {
    /// Fetches the full OSV record for every advisory in `findings` and
    /// scores it with [`assess`]. Advisories that can't be fetched or carry
    /// no severity data are left out (so rated [`UNRATED_ADVISORY`]), each
    /// with a warning.
    pub fn fetch(findings: &[Finding]) -> (AdvisorySeverities, Vec<String>) {
        let mut severities = AdvisorySeverities::default();
        let mut warnings = Vec::new();
        for finding in findings {
            let FindingKind::Vulnerability { advisory_id } = &finding.kind else {
                continue;
            };
            if severities.levels.contains_key(advisory_id) {
                continue;
            }
            match get_vulnerability(advisory_id) {
                Ok(vuln) => match assess(&vuln) {
                    Some(score) => severities.insert(advisory_id, score.level),
                    None => warnings.push(format!(
                        "{advisory_id} has no severity data; rated {}",
                        label(UNRATED_ADVISORY)
                    )),
                },
                Err(err) => warnings.push(format!(
                    "couldn't fetch {advisory_id} to rate it ({err}); rated {}",
                    label(UNRATED_ADVISORY)
                )),
            }
        }
        (severities, warnings)
    }

    pub fn insert(&mut self, advisory_id: &str, severity: Severity) {
        self.levels.insert(advisory_id.to_string(), severity);
    }

    pub fn get(&self, advisory_id: &str) -> Option<Severity> {
        self.levels.get(advisory_id).copied()
    }
}

/// How serious `finding` is.
///
/// A homoglyph typosquat is critical: lookalike characters only end up in a
/// package name on purpose. A name that is merely a few edits from a popular
/// package is high, since it may be an honest near-collision. A vulnerability
/// takes its advisory's level, or [`UNRATED_ADVISORY`] if that's unknown. An
/// OSV `MAL-` advisory means the package is known malware, so it's always
/// critical.
pub fn severity_of(finding: &Finding, advisories: &AdvisorySeverities) -> Severity {
    match &finding.kind {
        FindingKind::Typosquat {
            homoglyph_match: Some(_),
            ..
        } => Severity::Critical,
        FindingKind::Typosquat { .. } => Severity::High,
        FindingKind::Deprecated { .. } | FindingKind::License { .. } => Severity::Low,
        FindingKind::Vulnerability { advisory_id } => {
            if is_malicious_advisory(advisory_id) {
                Severity::Critical
            } else {
                advisories.get(advisory_id).unwrap_or(UNRATED_ADVISORY)
            }
        }
    }
}

/// Extra label for `finding`, if it needs one beyond its severity.
///
/// A vulnerability from an OSV `MAL-` advisory is labelled
/// [`MALICIOUS_LABEL`]; everything else has no extra label.
pub fn finding_label(finding: &Finding) -> Option<&'static str> {
    match &finding.kind {
        FindingKind::Vulnerability { advisory_id } if is_malicious_advisory(advisory_id) => {
            Some(MALICIOUS_LABEL)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Ecosystem;

    fn finding(kind: FindingKind) -> Finding {
        Finding {
            package: "pkg".to_string(),
            version: Some("1.0.0".to_string()),
            ecosystem: Ecosystem::Npm,
            direct: true,
            path: Vec::new(),
            kind,
        }
    }

    #[test]
    fn rates_typosquats_by_signal() {
        let none = AdvisorySeverities::default();
        let homoglyph = finding(FindingKind::Typosquat {
            distance_match: Some(("react".to_string(), 1)),
            homoglyph_match: Some("react".to_string()),
        });
        let distance = finding(FindingKind::Typosquat {
            distance_match: Some(("express".to_string(), 1)),
            homoglyph_match: None,
        });
        assert_eq!(severity_of(&homoglyph, &none), Severity::Critical);
        assert_eq!(severity_of(&distance, &none), Severity::High);
    }

    #[test]
    fn rates_vulnerabilities_by_advisory() {
        let mut advisories = AdvisorySeverities::default();
        advisories.insert("GHSA-low", Severity::Low);
        let rated = finding(FindingKind::Vulnerability {
            advisory_id: "GHSA-low".to_string(),
        });
        let unrated = finding(FindingKind::Vulnerability {
            advisory_id: "GHSA-unknown".to_string(),
        });
        assert_eq!(severity_of(&rated, &advisories), Severity::Low);
        assert_eq!(severity_of(&unrated, &advisories), UNRATED_ADVISORY);
    }

    #[test]
    fn rates_malicious_advisories_critical() {
        let mut advisories = AdvisorySeverities::default();
        // Even if the advisory was rated low, malware wins.
        advisories.insert("MAL-2024-1", Severity::Low);
        let malicious = finding(FindingKind::Vulnerability {
            advisory_id: "MAL-2024-1".to_string(),
        });
        let unrated_malicious = finding(FindingKind::Vulnerability {
            advisory_id: "MAL-2024-2".to_string(),
        });
        assert_eq!(severity_of(&malicious, &advisories), Severity::Critical);
        assert_eq!(
            severity_of(&unrated_malicious, &advisories),
            Severity::Critical
        );
        assert_eq!(finding_label(&malicious), Some(MALICIOUS_LABEL));
        assert_eq!(finding_label(&unrated_malicious), Some(MALICIOUS_LABEL));
    }

    #[test]
    fn non_malicious_findings_have_no_extra_label() {
        let advisories = AdvisorySeverities::default();
        let vuln = finding(FindingKind::Vulnerability {
            advisory_id: "GHSA-unknown".to_string(),
        });
        let typosquat = finding(FindingKind::Typosquat {
            distance_match: Some(("react".to_string(), 1)),
            homoglyph_match: Some("react".to_string()),
        });
        assert_eq!(finding_label(&vuln), None);
        assert_eq!(finding_label(&typosquat), None);
        assert!(!is_malicious_advisory("GHSA-unknown"));
        assert!(is_malicious_advisory("MAL-2024-1"));
        assert_eq!(severity_of(&vuln, &advisories), UNRATED_ADVISORY);
    }
}
