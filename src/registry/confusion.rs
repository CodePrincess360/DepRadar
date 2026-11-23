//! Dependency-confusion check for private package names (#332).
//!
//! A project's internal packages are listed under `private_packages` in
//! `depradar.toml`. If one of those names is also published on the public
//! registry, an attacker may be squatting it: a misconfigured install can
//! pull the public copy instead of the private one. [`find_public_names`]
//! asks the public npm registry about each name and returns the ones that
//! exist there.

use crate::net::{NetError, PoliteClient};

use super::npm::{encode_package_name, NPM_REGISTRY_URL};

/// Returns the names in `private_packages` that exist on the public npm
/// registry, in the order given.
pub fn find_public_names(private_packages: &[String]) -> Result<Vec<String>, NetError> {
    find_public_names_with(&PoliteClient::default(), NPM_REGISTRY_URL, private_packages)
}

/// Like [`find_public_names`], with an explicit client and registry base
/// URL, e.g. a mock server in tests.
///
/// A `404` means the name is unclaimed. Any other non-success status, or a
/// failed request, is an error rather than a silent "not found".
pub fn find_public_names_with(
    client: &PoliteClient,
    base_url: &str,
    private_packages: &[String],
) -> Result<Vec<String>, NetError> {
    let mut public = Vec::new();
    for name in private_packages {
        let url = format!(
            "{}/{}",
            base_url.trim_end_matches('/'),
            encode_package_name(name)
        );
        let status = client.get(&url, false)?.status();
        if status.is_success() {
            public.push(name.clone());
        } else if status != reqwest::StatusCode::NOT_FOUND {
            return Err(NetError::Status(status));
        }
    }
    Ok(public)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn flags_a_private_name_that_exists_publicly() {
        let mut server = mockito::Server::new();
        let squatted = server
            .mock("GET", "/acme-internal-utils")
            .with_body(r#"{"name": "acme-internal-utils"}"#)
            .expect(1)
            .create();
        let unclaimed = server
            .mock("GET", "/@acme%2Fbilling")
            .with_status(404)
            .expect(1)
            .create();

        let public = find_public_names_with(
            &PoliteClient::default(),
            &server.url(),
            &names(&["acme-internal-utils", "@acme/billing"]),
        )
        .unwrap();

        assert_eq!(public, names(&["acme-internal-utils"]));
        squatted.assert();
        unclaimed.assert();
    }

    #[test]
    fn registry_errors_are_not_treated_as_unclaimed() {
        let mut server = mockito::Server::new();
        server
            .mock("GET", "/acme-internal-utils")
            .with_status(500)
            .create();

        let result = find_public_names_with(
            &PoliteClient::default(),
            &server.url(),
            &names(&["acme-internal-utils"]),
        );

        assert!(matches!(result, Err(NetError::Status(status)) if status == 500));
    }

    #[test]
    fn no_private_names_means_no_requests() {
        let public =
            find_public_names_with(&PoliteClient::default(), "http://127.0.0.1:9", &[]).unwrap();
        assert!(public.is_empty());
    }
}
