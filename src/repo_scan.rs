//! Read-only repository scanning. Findings omit source text so a report can
//! be shared privately without copying potentially sensitive file contents.

use std::{
    fs::File,
    io::Read,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use ignore::{overrides::OverrideBuilder, WalkBuilder};
use serde::{Deserialize, Serialize};

use crate::report::Severity;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_FINDINGS: usize = 10_000;

#[derive(Debug, Serialize, Deserialize)]
pub struct Finding {
    pub rule_id: String,
    pub severity: Severity,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    pub scanned_at: u64,
    pub files_scanned: usize,
    pub oversized_files: usize,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Indicator {
    pub id: String,
    pub marker: String,
}

pub fn now() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

pub fn scan(root: &Path, exclude: &[String], indicators: &[Indicator]) -> Result<Report> {
    if !root.is_dir() {
        return Err("scan path must be a directory".into());
    }

    let mut overrides = OverrideBuilder::new(root);
    for pattern in exclude {
        overrides.add(&format!("!{pattern}"))?;
    }
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .require_git(false)
        .overrides(overrides.build()?)
        .filter_entry(|entry| {
            entry.depth() == 0
                || ![".git", "node_modules", "target"]
                    .iter()
                    .any(|name| entry.file_name() == *name)
        })
        .build();

    let mut report = Report {
        scanned_at: now()?,
        files_scanned: 0,
        oversized_files: 0,
        findings: Vec::new(),
    };
    for entry in walker {
        let entry = entry?;
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let path = entry.path().strip_prefix(root)?;
        let mut contents = Vec::new();
        File::open(entry.path())?
            .take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut contents)?;
        if contents.len() as u64 > MAX_FILE_BYTES {
            report.oversized_files += 1;
            continue;
        }
        report.files_scanned += 1;
        let Ok(source) = std::str::from_utf8(&contents) else {
            continue;
        };
        let name = entry.file_name().to_string_lossy();
        let file = path
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        for (line, content) in source.lines().enumerate() {
            let mut record = |rule_id: &str| {
                report.findings.push(Finding {
                    rule_id: rule_id.to_owned(),
                    severity: Severity::Medium,
                    file: file.clone(),
                    line: line + 1,
                })
            };
            if ["postcss.config.", "tailwind.config.", "vite.config."]
                .iter()
                .any(|prefix| name.starts_with(prefix))
                && has_hidden_suffix(content)
            {
                record("repo-scan/padded-build-config");
            }
            for indicator in indicators {
                if content.contains(&indicator.marker) {
                    record(&indicator.id);
                }
            }
            if report.findings.len() > MAX_FINDINGS {
                return Err("repository scan exceeded the findings limit".into());
            }
        }
    }
    report
        .findings
        .sort_by(|a, b| (&a.file, a.line, &a.rule_id).cmp(&(&b.file, b.line, &b.rule_id)));
    Ok(report)
}

fn has_hidden_suffix(line: &str) -> bool {
    let mut seen_text = false;
    let mut spaces = 0;
    for byte in line.bytes() {
        if matches!(byte, b' ' | b'\t') {
            spaces += 1;
        } else {
            if seen_text && spaces >= 180 {
                return true;
            }
            seen_text = true;
            spaces = 0;
        }
    }
    false
}

pub mod feed;
