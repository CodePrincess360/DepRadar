//! Bun text lockfile (`bun.lock`) parser.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::ParseError;
use crate::model::{Dependency, Ecosystem};

/// The parts of a `bun.lock` we read: the workspace manifests (to tell
/// direct dependencies apart) and the `packages` map. Each package value is
/// an array whose first element is the `name@version` it resolved to.
#[derive(Debug, Deserialize)]
struct BunLock {
    #[serde(default)]
    workspaces: BTreeMap<String, BunWorkspace>,
    #[serde(default)]
    packages: BTreeMap<String, Vec<serde_json::Value>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BunWorkspace {
    #[serde(default)]
    dependencies: BTreeMap<String, String>,
    #[serde(default)]
    dev_dependencies: BTreeMap<String, String>,
    #[serde(default)]
    optional_dependencies: BTreeMap<String, String>,
    #[serde(default)]
    peer_dependencies: BTreeMap<String, String>,
}

/// Parses a Bun `bun.lock` into its locked dependencies.
///
/// `bun.lock` is JSON with trailing commas, which are stripped before
/// parsing. Every `packages` entry whose first element is a registry
/// `name@version` is returned; workspace, git, file and other non-registry
/// entries (versions containing `:`) are skipped. A package is `direct` when
/// its key is its bare name (not a nested `parent/name` install) and the root
/// workspace (`""`) declares it, and `dev` when the root declares it only in
/// `devDependencies`. Transitive packages are never marked `dev`, since the
/// lockfile doesn't record it for them.
pub fn parse_bun_lock(contents: &str) -> Result<Vec<Dependency>, ParseError> {
    let lock: BunLock = serde_json::from_str(&strip_trailing_commas(contents))?;
    let root = lock.workspaces.get("");

    Ok(lock
        .packages
        .iter()
        .filter_map(|(key, entry)| {
            let (name, version) = split_spec(entry.first()?.as_str()?)?;
            let top_level = key == name;
            let (direct, dev) = match root {
                Some(root) if top_level => {
                    let runtime = root.dependencies.contains_key(name)
                        || root.optional_dependencies.contains_key(name)
                        || root.peer_dependencies.contains_key(name);
                    let dev_only = !runtime && root.dev_dependencies.contains_key(name);
                    (runtime || dev_only, dev_only)
                }
                _ => (false, false),
            };
            Some(Dependency {
                name: name.to_string(),
                version: Some(version.to_string()),
                ecosystem: Ecosystem::Npm,
                direct,
                dev,
                path: Vec::new(),
            })
        })
        .collect())
}

/// Splits a `name@version` spec, keeping the leading `@` of a scope. Returns
/// `None` for non-registry versions such as `workspace:packages/lib` or
/// `github:user/repo#sha`.
fn split_spec(spec: &str) -> Option<(&str, &str)> {
    let at = spec.get(1..)?.find('@')? + 1;
    let (name, version) = (&spec[..at], &spec[at + 1..]);
    (!name.is_empty() && !version.is_empty() && !version.contains(':')).then_some((name, version))
}

/// Removes commas that directly precede a closing `}` or `]` (ignoring
/// whitespace), outside of string literals.
fn strip_trailing_commas(contents: &str) -> String {
    let mut out = String::with_capacity(contents.len());
    let mut in_string = false;
    let mut escaped = false;
    for (i, c) in contents.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
        } else if c == ',' {
            let next = contents[i + 1..].trim_start().chars().next();
            if matches!(next, Some('}' | ']')) {
                continue;
            }
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUN_LOCK: &str = include_str!("../../tests/fixtures/bun.lock");

    fn find<'a>(deps: &'a [Dependency], name: &str, version: &str) -> &'a Dependency {
        deps.iter()
            .find(|d| d.name == name && d.version.as_deref() == Some(version))
            .unwrap_or_else(|| panic!("{name}@{version} not found in {deps:?}"))
    }

    #[test]
    fn parses_packages_with_direct_and_dev_flags() {
        let deps = parse_bun_lock(BUN_LOCK).unwrap();
        assert_eq!(deps.len(), 7);
        assert!(deps.iter().all(|d| d.ecosystem == Ecosystem::Npm));

        let express = find(&deps, "express", "4.21.2");
        assert!(express.direct);
        assert!(!express.dev);

        let types_node = find(&deps, "@types/node", "20.11.30");
        assert!(types_node.direct);
        assert!(types_node.dev);

        assert!(find(&deps, "typescript", "5.4.5").dev);

        let accepts = find(&deps, "accepts", "1.3.8");
        assert!(!accepts.direct);
        assert!(!accepts.dev);

        // Nested install under `debug/ms` is a separate, transitive copy.
        assert!(!find(&deps, "ms", "2.0.0").direct);
        assert!(!find(&deps, "ms", "2.1.3").direct);
    }

    #[test]
    fn skips_workspace_entries() {
        let deps = parse_bun_lock(BUN_LOCK).unwrap();
        assert!(deps.iter().all(|d| d.name != "local-lib"));
    }

    #[test]
    fn split_spec_handles_scopes_and_non_registry_versions() {
        assert_eq!(split_spec("express@4.21.2"), Some(("express", "4.21.2")));
        assert_eq!(
            split_spec("@types/node@20.11.30"),
            Some(("@types/node", "20.11.30"))
        );
        assert_eq!(split_spec("local-lib@workspace:packages/local-lib"), None);
        assert_eq!(split_spec("dep@github:user/repo#abc123"), None);
        assert_eq!(split_spec("no-version"), None);
    }

    #[test]
    fn trailing_commas_outside_strings_are_removed() {
        assert_eq!(
            strip_trailing_commas(r#"{"a": [1, 2,], "b": ",}",}"#),
            r#"{"a": [1, 2], "b": ",}"}"#
        );
    }

    #[test]
    fn missing_sections_are_empty() {
        assert!(parse_bun_lock(r#"{"lockfileVersion": 1}"#)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn malformed_input_is_an_error() {
        assert!(parse_bun_lock("not json").is_err());
        assert!(parse_bun_lock(r#"{"packages": {"express": "express@4.21.2"}}"#).is_err());
    }
}
