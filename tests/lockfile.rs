//! Fixture tests for the lockfile parsers.

use std::path::PathBuf;

use deprader::lockfile::parse_lockfile;
use deprader::model::Dependency;

fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
}

/// `(name, version, direct)` for each parsed dependency, in parse order.
fn relations(dependencies: &[Dependency]) -> Vec<(&str, Option<&str>, bool)> {
    dependencies
        .iter()
        .map(|d| (d.name.as_str(), d.version.as_deref(), d.direct))
        .collect()
}

#[test]
fn npm_marks_root_dependencies_direct_and_nested_installs_transitive() {
    let parsed = parse_lockfile(&fixture("npm-basic/package-lock.json")).unwrap();

    assert_eq!(
        relations(&parsed.dependencies),
        vec![
            ("express", Some("4.21.2"), true),
            // Same name as a direct dependency, but this nested copy is the
            // one express pulled in.
            ("lodash", Some("4.17.4"), false),
            ("expresss", Some("1.0.0"), true),
            ("lodash", Some("4.17.4"), true),
            ("zzz-unversioned-internal-fixture", None, false),
        ]
    );
}

#[test]
fn requirements_txt_dependencies_are_all_direct() {
    let parsed = parse_lockfile(&fixture("pypi-basic/requirements.txt")).unwrap();

    assert_eq!(
        relations(&parsed.dependencies),
        vec![
            ("flask", None, true),
            ("requests", Some("2.19.0"), true),
            ("reqeusts", Some("1.0.0"), true),
        ]
    );
    assert_eq!(parsed.warnings.len(), 1);
}

/// `(name, version, path)` for each parsed dependency, in parse order.
fn paths(dependencies: &[Dependency]) -> Vec<(&str, Option<&str>, String)> {
    dependencies
        .iter()
        .map(|d| (d.name.as_str(), d.version.as_deref(), d.path.join(" > ")))
        .collect()
}

#[test]
fn npm_paths_follow_the_shortest_chain_and_node_resolution() {
    let parsed = parse_lockfile(&fixture("npm-chain/package-lock.json")).unwrap();

    assert_eq!(
        paths(&parsed.dependencies),
        vec![
            (
                "deep-a",
                Some("1.0.0"),
                "my-app > tiny-helper > deep-a".to_string()
            ),
            (
                "deep-b",
                Some("1.0.0"),
                "my-app > tiny-helper > deep-a > deep-b".to_string()
            ),
            // Reachable via deep-b too, but webpack's chain is shorter.
            (
                "loader-kit",
                Some("1.0.0"),
                "my-app > webpack > loader-kit".to_string()
            ),
            // loader-kit gets its own nested copy of old-lib...
            (
                "old-lib",
                Some("1.0.0"),
                "my-app > webpack > loader-kit > old-lib".to_string()
            ),
            // ...while tiny-helper resolves the hoisted one.
            (
                "old-lib",
                Some("2.0.0"),
                "my-app > tiny-helper > old-lib".to_string()
            ),
            (
                "tiny-helper",
                Some("1.0.0"),
                "my-app > tiny-helper".to_string()
            ),
            ("webpack", Some("5.90.0"), "my-app > webpack".to_string()),
        ]
    );
}

#[test]
fn bun_lock_marks_root_workspace_dependencies_direct() {
    let parsed = parse_lockfile(&fixture("bun.lock")).unwrap();

    assert_eq!(parsed.ecosystem, deprader::model::Ecosystem::Npm);
    assert_eq!(
        relations(&parsed.dependencies),
        vec![
            ("@types/node", Some("20.11.30"), true),
            ("accepts", Some("1.3.8"), false),
            ("debug", Some("2.6.9"), false),
            // Nested `debug/ms` install, separate from the hoisted `ms`.
            ("ms", Some("2.0.0"), false),
            ("express", Some("4.21.2"), true),
            ("ms", Some("2.1.3"), false),
            ("typescript", Some("5.4.5"), true),
        ]
    );
}

#[test]
fn malformed_bun_lock_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bun.lock");
    std::fs::write(&path, "{ \"packages\": ").unwrap();

    assert!(parse_lockfile(&path).is_err());
}

#[test]
fn uv_lock_marks_project_dependencies_direct() {
    let parsed = parse_lockfile(&fixture("uv.lock")).unwrap();

    assert_eq!(parsed.ecosystem, deprader::model::Ecosystem::PyPI);
    assert_eq!(
        relations(&parsed.dependencies),
        vec![
            ("certifi", Some("2024.8.30"), false),
            ("iniconfig", Some("2.0.0"), false),
            ("pytest", Some("8.3.3"), true),
            ("requests", Some("2.32.3"), true),
            ("typing-extensions", Some("4.12.2"), true),
        ]
    );
}

#[test]
fn malformed_uv_lock_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("uv.lock");
    std::fs::write(&path, "[[package]\nname = ").unwrap();

    assert!(parse_lockfile(&path).is_err());
}
