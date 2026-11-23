//! Gradle (Maven) `gradle.lockfile` parser.

use super::ParseError;
use crate::model::{Dependency, Ecosystem};

/// Parses a Gradle `gradle.lockfile` into its locked dependencies.
///
/// Each entry is a `group:artifact:version=configuration,...` line; the
/// package name is `group:artifact`, as Maven and OSV name it. Comment lines
/// and the `empty=...` line listing configurations with no dependencies are
/// skipped. An entry is `dev` when every configuration it's locked in is a
/// test configuration (`testCompileClasspath`, `testRuntimeClasspath`, ...).
/// `gradle.lockfile` doesn't record which dependencies the build declares
/// itself, so `direct` is always `false`.
///
/// Any other line that doesn't have this shape is an error.
pub fn parse_gradle_lockfile(contents: &str) -> Result<Vec<Dependency>, ParseError> {
    let mut deps = Vec::new();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("empty=") {
            continue;
        }
        let malformed = || ParseError::Malformed(format!("invalid gradle.lockfile line: {line}"));
        let (coordinates, configurations) = line.split_once('=').ok_or_else(malformed)?;
        let parts: Vec<&str> = coordinates.split(':').collect();
        let [group, artifact, version] = parts[..] else {
            return Err(malformed());
        };
        if group.is_empty() || artifact.is_empty() || version.is_empty() {
            return Err(malformed());
        }
        let dev = configurations
            .split(',')
            .all(|configuration| configuration.trim().starts_with("test"));
        deps.push(Dependency {
            name: format!("{group}:{artifact}"),
            version: Some(version.to_string()),
            ecosystem: Ecosystem::Maven,
            direct: false,
            dev,
            path: Vec::new(),
        });
    }
    Ok(deps)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRADLE_LOCKFILE: &str = include_str!("../../tests/fixtures/gradle-basic/gradle.lockfile");

    fn find<'a>(deps: &'a [Dependency], name: &str) -> &'a Dependency {
        deps.iter()
            .find(|d| d.name == name)
            .unwrap_or_else(|| panic!("{name} not found in {deps:?}"))
    }

    #[test]
    fn parses_the_fixture() {
        let deps = parse_gradle_lockfile(GRADLE_LOCKFILE).unwrap();
        assert_eq!(deps.len(), 4);
        assert!(deps.iter().all(|d| d.ecosystem == Ecosystem::Maven));
        assert!(deps.iter().all(|d| !d.direct));

        let guava = find(&deps, "com.google.guava:guava");
        assert_eq!(guava.version.as_deref(), Some("32.1.2-jre"));
        assert!(!guava.dev);

        let failureaccess = find(&deps, "com.google.guava:failureaccess");
        assert_eq!(failureaccess.version.as_deref(), Some("1.0.1"));
        assert!(!failureaccess.dev);

        let slf4j = find(&deps, "org.slf4j:slf4j-api");
        assert_eq!(slf4j.version.as_deref(), Some("2.0.9"));
        assert!(!slf4j.dev);

        let junit = find(&deps, "junit:junit");
        assert_eq!(junit.version.as_deref(), Some("4.13.2"));
        assert!(junit.dev);
    }

    #[test]
    fn malformed_lines_are_an_error() {
        let inputs = [
            "com.google.guava:guava:32.1.2-jre\n",
            "com.google.guava:32.1.2-jre=compileClasspath\n",
            "a:b:c:d=compileClasspath\n",
            ":guava:32.1.2-jre=compileClasspath\n",
        ];
        for input in inputs {
            assert!(parse_gradle_lockfile(input).is_err(), "{input:?}");
        }
    }

    #[test]
    fn comments_and_empty_line_have_no_dependencies() {
        let lockfile = "# comment\nempty=annotationProcessor\n";
        assert!(parse_gradle_lockfile(lockfile).unwrap().is_empty());
    }
}
