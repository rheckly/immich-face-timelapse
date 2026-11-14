//! Connection to Immich Smart Albums: the people albums are shared with.
//!
//! Configured with the same environment variables as Mosaik:
//! - `SMARTALBUMS_URL` - base URL of Smart Albums, e.g. `http://smartalbums:8080`
//! - `CONNECT_KEY` - key sent as `X-Connect-Key`
//!
//! Without both variables the "Shared with" choice is not offered.

use crate::error::{Error, Result};
use reqwest::Client;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;
use std::time::Duration;

/// Environment variable with the Smart Albums URL.
pub const URL_ENV: &str = "SMARTALBUMS_URL";
/// Environment variable with the Smart Albums connect key.
pub const KEY_ENV: &str = "CONNECT_KEY";

/// How long to wait for Smart Albums.
const TIMEOUT: Duration = Duration::from_secs(20);

/// An album shared with a person.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SharedAlbum {
    #[serde(deserialize_with = "id_text")]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub count: u32,
}

/// A person albums are shared with.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Recipient {
    #[serde(deserialize_with = "id_text")]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub albums: Vec<SharedAlbum>,
}

#[derive(Deserialize)]
struct PeopleResponse {
    people: Vec<Recipient>,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: Option<String>,
}

/// Ids may come as text or as numbers.
fn id_text<'de, D: Deserializer<'de>>(deserializer: D) -> std::result::Result<String, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Id {
        Text(String),
        Number(i64),
    }
    Ok(match Id::deserialize(deserializer)? {
        Id::Text(text) => text,
        Id::Number(number) => number.to_string(),
    })
}

/// Client for the Smart Albums connect API.
pub struct SmartAlbums {
    client: Client,
    url: String,
    key: String,
}

impl SmartAlbums {
    /// Create a client for the given URL and key.
    pub fn new(url: &str, key: &str) -> Result<Self> {
        Ok(Self {
            client: Client::builder().timeout(TIMEOUT).build()?,
            url: url.trim_end_matches('/').to_string(),
            key: key.to_string(),
        })
    }

    /// Client from `SMARTALBUMS_URL` and `CONNECT_KEY`, or `None` when either is missing.
    pub fn from_env() -> Result<Option<Self>> {
        let read = |name: &str| {
            std::env::var(name)
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        match (read(URL_ENV), read(KEY_ENV)) {
            (Some(url), Some(key)) => Self::new(&url, &key).map(Some),
            _ => Ok(None),
        }
    }

    /// People with the albums shared with them.
    pub async fn people(&self) -> Result<Vec<Recipient>> {
        let response = self
            .client
            .get(format!("{}/api/connect/people", self.url))
            .header("X-Connect-Key", &self.key)
            .send()
            .await
            .map_err(|e| Error::SmartAlbums(format!("Smart Albums not reachable: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let message = response
                .json::<ErrorResponse>()
                .await
                .ok()
                .and_then(|r| r.error)
                .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
            return Err(Error::SmartAlbums(format!("Smart Albums: {message}")));
        }

        let body: PeopleResponse = response
            .json()
            .await
            .map_err(|e| Error::SmartAlbums(format!("Smart Albums: unexpected answer ({e})")))?;
        Ok(body.people)
    }
}

/// The album ids shared with the chosen people, without duplicates.
pub fn recipient_album_ids(recipients: &[Recipient], chosen: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    recipients
        .iter()
        .filter(|r| chosen.contains(&r.id))
        .flat_map(|r| r.albums.iter())
        .filter(|a| seen.insert(a.id.clone()))
        .map(|a| a.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Recipient> {
        let json = r#"{"people":[
            {"id":1,"name":"Grandma","albums":[{"id":"a1","name":"Summer","count":10},{"id":"a2","name":"Winter","count":3}]},
            {"id":"p2","name":"Uncle","albums":[{"id":"a2","name":"Winter","count":3},{"id":"a3","name":"Trip"}]},
            {"id":"p3","name":"Nobody"}
        ]}"#;
        let response: PeopleResponse = serde_json::from_str(json).unwrap();
        response.people
    }

    #[test]
    fn test_people_response() {
        let people = sample();
        assert_eq!(people[0].id, "1");
        assert_eq!(people[1].albums[1].count, 0);
        assert!(people[2].albums.is_empty());
    }

    #[test]
    fn test_recipient_album_ids() {
        let people = sample();
        let chosen = vec!["1".to_string(), "p2".to_string()];
        assert_eq!(
            recipient_album_ids(&people, &chosen),
            vec!["a1", "a2", "a3"]
        );
        assert!(recipient_album_ids(&people, &["p3".to_string()]).is_empty());
        assert!(recipient_album_ids(&people, &[]).is_empty());
    }

    #[test]
    fn test_new_trims_trailing_slash() {
        let client = SmartAlbums::new("http://smartalbums:8080/", "key").unwrap();
        assert_eq!(client.url, "http://smartalbums:8080");
    }
}
