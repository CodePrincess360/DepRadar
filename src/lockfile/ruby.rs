//! Ruby (RubyGems) Bundler `Gemfile.lock` parser.

use std::collections::BTreeSet;

use super::ParseError;
use crate::model::{Dependency, Ecosystem};

/// Parses a Bundler `Gemfile.lock` into its locked dependencies.
///
/// The lockfile is made of unindented section headers (`GEM`, `GIT`, `PATH`,
/// `PLATFORMS`, `DEPENDENCIES`, `BUNDLED WITH`, ...) followed by indented
/// lines. Gems come from the `specs:` of the `GEM`, `GIT` and `PATH`
/// sections, written as `name (version)` at four spaces of indentation; the
/// six-space lines under them are that gem's own requirements and are
/// skipped. A platform suffix (`1.16.2-x86_64-linux`) is dropped from the
/// version, and each `(name, version)` pair is returned once. A gem is
/// `direct` when it is listed under `DEPENDENCIES`. `Gemfile.lock` doesn't
/// record Gemfile groups, so `dev` is always `false`.
///
/// Returns [`ParseError::Invalid`] for an unindented line that isn't a
/// section header, or a spec line that isn't `name (version)`.
pub fn parse_gemfile_lock(contents: &str) -> Result<Vec<Dependency>, ParseError> {
    let mut seen = BTreeSet::new();
    let mut specs = Vec::new();
    let mut declared = BTreeSet::new();
    let mut section = "";

    for (index, line) in contents.lines().enumerate() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            continue;
        }
        if !line.starts_with(' ') {
            if !line.chars().all(|c| c.is_ascii_uppercase() || c == ' ') {
                return Err(invalid(index, "expected a section header"));
            }
            section = line;
            continue;
        }

        let indent = line.len() - line.trim_start().len();
        let entry = line.trim_start();
        match section {
            "GEM" | "GIT" | "PATH" if indent == 4 => {
                let (name, version) =
                    split_spec(entry).ok_or_else(|| invalid(index, "expected `name (version)`"))?;
                if seen.insert((name.to_string(), version.to_string())) {
                    specs.push((name.to_string(), version.to_string()));
                }
            }
            "DEPENDENCIES" if indent == 2 => {
                // `rails (~> 7.1)`, `rspec`, or `my_gem!` for a GIT/PATH source.
                let name = entry.split([' ', '!']).next().unwrap_or_default();
                declared.insert(name.to_string());
            }
            _ => {}
        }
    }

    Ok(specs
        .into_iter()
        .map(|(name, version)| Dependency {
            direct: declared.contains(&name),
            name,
            version: Some(version),
            ecosystem: Ecosystem::RubyGems,
            dev: false,
            path: Vec::new(),
        })
        .collect())
}

/// Splits a spec line such as `nokogiri (1.16.2-x86_64-linux)` into
/// `("nokogiri", "1.16.2")`.
fn split_spec(entry: &str) -> Option<(&str, &str)> {
    let (name, rest) = entry.split_once(" (")?;
    let version = rest.strip_suffix(')')?;
    // Platform-specific gems append `-<platform>` to the version.
    let version = version.split('-').next()?;
    (!name.is_empty() && !version.is_empty()).then_some((name, version))
}

fn invalid(index: usize, reason: &str) -> ParseError {
    ParseError::Invalid(format!("Gemfile.lock line {}: {reason}", index + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GEMFILE_LOCK: &str = include_str!("../../tests/fixtures/Gemfile.lock");

    fn find<'a>(deps: &'a [Dependency], name: &str) -> &'a Dependency {
        deps.iter()
            .find(|d| d.name == name)
            .unwrap_or_else(|| panic!("{name} not found in {deps:?}"))
    }

    #[test]
    fn parses_specs_from_gem_git_and_path_sections() {
        let deps = parse_gemfile_lock(GEMFILE_LOCK).unwrap();
        assert_eq!(deps.len(), 7);
        assert!(deps
            .iter()
            .all(|d| d.ecosystem == Ecosystem::RubyGems && !d.dev));

        let rails = find(&deps, "rails");
        assert_eq!(rails.version.as_deref(), Some("7.1.3"));
        assert!(rails.direct);

        let rack = find(&deps, "rack");
        assert_eq!(rack.version.as_deref(), Some("3.0.9"));
        assert!(!rack.direct);

        // From GIT and PATH sources, declared with a trailing `!`.
        assert!(find(&deps, "my_engine").direct);
        assert!(find(&deps, "local_gem").direct);
    }

    #[test]
    fn platform_variants_collapse_to_one_entry() {
        let deps = parse_gemfile_lock(GEMFILE_LOCK).unwrap();
        let nokogiri: Vec<_> = deps.iter().filter(|d| d.name == "nokogiri").collect();
        assert_eq!(nokogiri.len(), 1);
        assert_eq!(nokogiri[0].version.as_deref(), Some("1.16.2"));
        assert!(nokogiri[0].direct);
    }

    #[test]
    fn empty_input_is_empty() {
        assert!(parse_gemfile_lock("").unwrap().is_empty());
    }

    #[test]
    fn malformed_input_is_an_error() {
        assert!(matches!(
            parse_gemfile_lock("not a lockfile"),
            Err(ParseError::Invalid(_))
        ));
        assert!(matches!(
            parse_gemfile_lock("GEM\n  specs:\n    rack 3.0.9\n"),
            Err(ParseError::Invalid(_))
        ));
    }
}
