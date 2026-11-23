//! Signed IOC feeds. Signatures cover the exact bytes parsed as indicators.

use std::{collections::HashSet, io::Read, time::Duration};

use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::{blocking::Client, redirect::Policy, Url};
use ring::signature::{UnparsedPublicKey, ED25519};

use super::{Indicator, Result};

const MAX_PAYLOAD: u64 = 256 * 1024;

pub fn fetch(url: &str, public_key: &str) -> Result<Vec<Indicator>> {
    let payload_url = Url::parse(url)?;
    if payload_url.scheme() != "https"
        || payload_url.query().is_some()
        || payload_url.fragment().is_some()
        || !payload_url.username().is_empty()
        || payload_url.password().is_some()
    {
        return Err(
            "rule feed must be a direct HTTPS URL without credentials, query, or fragment".into(),
        );
    }
    let client = Client::builder()
        .https_only(true)
        .redirect(Policy::none())
        .timeout(Duration::from_secs(20))
        .build()?;
    fetch_from(&client, payload_url, public_key)
}

fn fetch_from(client: &Client, payload_url: Url, public_key: &str) -> Result<Vec<Indicator>> {
    let signature_url = Url::parse(&format!("{payload_url}.sig"))?;
    let payload = download(client, payload_url, MAX_PAYLOAD)?;
    let signature = download(client, signature_url, 128)?;
    verify(
        &payload,
        std::str::from_utf8(&signature)?.trim(),
        public_key,
    )
}

fn download(client: &Client, url: Url, limit: u64) -> Result<Vec<u8>> {
    let response = client.get(url).send()?.error_for_status()?;
    if !response.status().is_success() {
        return Err("rule feed must return a successful response without redirection".into());
    }
    let mut bytes = Vec::new();
    response.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err("rule feed response exceeds its size limit".into());
    }
    Ok(bytes)
}

/// Verify Ed25519 first, then validate the data-only rule schema.
pub fn verify(payload: &[u8], signature: &str, public_key: &str) -> Result<Vec<Indicator>> {
    let key = STANDARD.decode(public_key)?;
    let signature = STANDARD.decode(signature)?;
    UnparsedPublicKey::new(&ED25519, key)
        .verify(payload, &signature)
        .map_err(|_| "IOC feed signature verification failed")?;
    let indicators: Vec<Indicator> = serde_json::from_slice(payload)?;
    let mut ids = HashSet::new();
    if indicators.len() > 512
        || indicators.iter().any(|rule| {
            !rule.id.starts_with("ioc/")
                || rule.id.len() > 96
                || !rule
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_'))
                || !ids.insert(&rule.id)
                || !(8..=512).contains(&rule.marker.len())
                || rule.marker.contains(['\r', '\n'])
        })
    {
        return Err("invalid IOC rule set".into());
    }
    Ok(indicators)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::{Ed25519KeyPair, KeyPair};

    #[test]
    fn downloaded_rules_are_rejected_when_the_signed_bytes_change() {
        let mut server = mockito::Server::new();
        let payload = r#"[{"id":"ioc/fixture/test","marker":"DEFANGED_FIXTURE"}]"#;
        let key = Ed25519KeyPair::from_seed_unchecked(&[37; 32]).unwrap();
        let public_key = STANDARD.encode(key.public_key());
        let signature = server
            .mock("GET", "/rules.json.sig")
            .with_body(STANDARD.encode(key.sign(payload.as_bytes())))
            .expect(2)
            .create();
        let rules = server
            .mock("GET", "/rules.json")
            .with_body(payload)
            .create();
        let client = Client::new();
        let url = Url::parse(&format!("{}/rules.json", server.url())).unwrap();
        assert_eq!(
            fetch_from(&client, url.clone(), &public_key).unwrap().len(),
            1
        );
        rules.assert();
        rules.remove();
        let tampered = server
            .mock("GET", "/rules.json")
            .with_body(format!("{payload} "))
            .create();
        assert!(fetch_from(&client, url, &public_key)
            .unwrap_err()
            .to_string()
            .contains("signature verification failed"));
        tampered.assert();
        signature.assert();
    }
}
