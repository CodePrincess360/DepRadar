//! Go modules `go.sum` parser.

use std::collections::BTreeSet;

use super::ParseError;
use crate::model::{Dependency, Ecosystem};

/// Parses a Go `go.sum` into its module dependencies.
///
/// Each line is `module version hash`. A module whose source was downloaded
/// has a line with its plain version (`v1.2.3`); a `v1.2.3/go.mod` line only
/// checksums the module's `go.mod`, which Go also records for versions it
/// read while resolving the build list but doesn't build. Only modules with
/// a plain-version line are returned, each `(module, version)` pair once.
/// `go.sum` doesn't record which modules the project requires itself, so
/// `direct` and `dev` are always `false`.
///
/// A line that isn't three fields with a `v`-prefixed version is an error.
pub fn parse_go_sum(contents: &str) -> Result<Vec<Dependency>, ParseError> {
    let mut seen = BTreeSet::new();
    let mut deps = Vec::new();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        let [module, version, _hash] = fields[..] else {
            return Err(malformed(line));
        };
        if !version.starts_with('v') {
            return Err(malformed(line));
        }
        if version.ends_with("/go.mod") {
            continue;
        }
        if seen.insert((module, version)) {
            deps.push(Dependency {
                name: module.to_string(),
                version: Some(version.to_string()),
                ecosystem: Ecosystem::Go,
                direct: false,
                dev: false,
                path: Vec::new(),
            });
        }
    }
    Ok(deps)
}

fn malformed(line: &str) -> ParseError {
    ParseError::Malformed(format!("invalid go.sum line: {line}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GO_SUM: &str = include_str!("../../tests/fixtures/go-basic/go.sum");

    fn find<'a>(deps: &'a [Dependency], name: &str) -> &'a Dependency {
        deps.iter()
            .find(|d| d.name == name)
            .unwrap_or_else(|| panic!("{name} not found in {deps:?}"))
    }

    #[test]
    fn parses_the_fixture() {
        let deps = parse_go_sum(GO_SUM).unwrap();
        assert_eq!(deps.len(), 3);
        assert!(deps.iter().all(|d| d.ecosystem == Ecosystem::Go));
        assert!(deps.iter().all(|d| !d.direct && !d.dev));

        assert_eq!(
            find(&deps, "github.com/google/uuid").version.as_deref(),
            Some("v1.6.0")
        );
        assert_eq!(
            find(&deps, "golang.org/x/text").version.as_deref(),
            Some("v0.14.0")
        );
        assert_eq!(
            find(&deps, "gopkg.in/yaml.v3").version.as_deref(),
            Some("v3.0.1")
        );
        // Only its go.mod was checksummed, so it isn't part of the build.
        assert!(!deps.iter().any(|d| d.name == "github.com/kr/pretty"));
    }

    #[test]
    fn malformed_lines_are_an_error() {
        let inputs = [
            "github.com/google/uuid v1.6.0\n",
            "github.com/google/uuid 1.6.0 h1:NIvaJDMOsjHA8n1jAhLSgzrAzy1Hgr+hNrb57e+94F0=\n",
            "github.com/google/uuid v1.6.0 h1:abc= extra\n",
        ];
        for input in inputs {
            assert!(parse_go_sum(input).is_err(), "{input:?}");
        }
    }

    #[test]
    fn empty_go_sum_has_no_dependencies() {
        assert!(parse_go_sum("\n").unwrap().is_empty());
    }
}
