//! PHP (Packagist) `composer.lock` parser.

use serde::Deserialize;

use super::ParseError;
use crate::model::{Dependency, Ecosystem};

/// The parts of a `composer.lock` we read: its runtime and dev packages.
#[derive(Debug, Deserialize)]
struct ComposerLock {
    #[serde(default)]
    packages: Option<Vec<ComposerPackage>>,
    #[serde(default, rename = "packages-dev")]
    packages_dev: Option<Vec<ComposerPackage>>,
}

#[derive(Debug, Deserialize)]
struct ComposerPackage {
    name: String,
    version: String,
}

/// Parses a Composer `composer.lock` into its locked dependencies.
///
/// Every entry in `packages` is returned with its pinned version, and every
/// entry in `packages-dev` likewise with `dev: true`. `composer.lock`
/// doesn't record which packages the project requires itself, so `direct`
/// is always `false`.
pub fn parse_composer_lock(contents: &str) -> Result<Vec<Dependency>, ParseError> {
    let lock: ComposerLock = serde_json::from_str(contents)?;

    let packages = lock.packages.unwrap_or_default().into_iter();
    let packages_dev = lock.packages_dev.unwrap_or_default().into_iter();
    Ok(packages
        .map(|package| (package, false))
        .chain(packages_dev.map(|package| (package, true)))
        .map(|(package, dev)| Dependency {
            name: package.name,
            version: Some(package.version),
            ecosystem: Ecosystem::Packagist,
            direct: false,
            dev,
            path: Vec::new(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPOSER_LOCK: &str = include_str!("../../tests/fixtures/composer-basic/composer.lock");

    fn find<'a>(deps: &'a [Dependency], name: &str) -> &'a Dependency {
        deps.iter()
            .find(|d| d.name == name)
            .unwrap_or_else(|| panic!("{name} not found in {deps:?}"))
    }

    #[test]
    fn parses_the_fixture() {
        let deps = parse_composer_lock(COMPOSER_LOCK).unwrap();
        assert_eq!(deps.len(), 3);
        assert!(deps.iter().all(|d| d.ecosystem == Ecosystem::Packagist));
        assert!(deps.iter().all(|d| !d.direct));

        let log = find(&deps, "monolog/monolog");
        assert_eq!(log.version.as_deref(), Some("3.5.0"));
        assert!(!log.dev);

        let psr_log = find(&deps, "psr/log");
        assert_eq!(psr_log.version.as_deref(), Some("3.0.0"));
        assert!(!psr_log.dev);

        let phpunit = find(&deps, "phpunit/phpunit");
        assert_eq!(phpunit.version.as_deref(), Some("10.5.5"));
        assert!(phpunit.dev);
    }

    #[test]
    fn missing_or_null_package_lists_are_empty() {
        let lock = r#"{"packages": null}"#;
        assert!(parse_composer_lock(lock).unwrap().is_empty());
    }

    #[test]
    fn invalid_json_is_an_error() {
        let inputs = [r#"{"packages": [{"name": "psr/log"}]}"#, r#"{"packages": ["#];
        for input in inputs {
            assert!(parse_composer_lock(input).is_err(), "{input:?}");
        }
    }
}
