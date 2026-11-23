//! Standalone SVG assets for README hosting.

use crate::repo_scan::Report;

pub const MAX_AGE_SECONDS: u64 = 14 * 24 * 60 * 60;

pub fn render(report: Option<&Report>, now: u64) -> String {
    let (state, color) = match report {
        Some(report) if report.scanned_at <= now && now - report.scanned_at < MAX_AGE_SECONDS => {
            if report.findings.is_empty() && report.oversized_files == 0 {
                ("clean", "#237b39")
            } else if !report.findings.is_empty() {
                ("findings", "#b02d2d")
            } else {
                ("stale", "#666")
            }
        }
        _ => ("stale", "#666"),
    };
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="204" height="22" role="img" aria-label="Scanned by DepRadar: {state}">
<title>Scanned by DepRadar: {state}</title>
<rect width="204" height="22" rx="3" fill="{color}"/>
<path fill="#343434" d="M3 0h135v22H3a3 3 0 0 1-3-3V3a3 3 0 0 1 3-3"/>
<g fill="#fff" text-anchor="middle" font-family="Verdana,sans-serif" font-size="11">
<text x="69" y="15">Scanned by DepRadar</text><text x="171" y="15">{state}</text>
</g></svg>"##
    )
}
