//! Further sources besides albums: other people in the photo and people albums are shared with.

use crate::immich_api::ImmichClient;
use crate::smart_albums::{recipient_album_ids, Recipient, SmartAlbums};
use crate::web::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use futures_util::stream::{self, StreamExt};
use serde::Serialize;
use std::collections::HashMap;

/// People albums are shared with, from Immich Smart Albums.
#[derive(Serialize)]
pub struct RecipientsResponse {
    /// `None` when Smart Albums is not configured: the choice is not offered.
    pub recipients: Option<Vec<Recipient>>,
    /// Why the list is empty when Smart Albums could not be asked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipients_error: Option<String>,
}

/// Get the people albums are shared with.
pub async fn get_recipients() -> Json<RecipientsResponse> {
    let client = match SmartAlbums::from_env() {
        Ok(Some(client)) => client,
        Ok(None) => {
            return Json(RecipientsResponse {
                recipients: None,
                recipients_error: None,
            });
        }
        Err(e) => {
            return Json(RecipientsResponse {
                recipients: Some(Vec::new()),
                recipients_error: Some(e.to_string()),
            });
        }
    };
    match client.people().await {
        Ok(people) => Json(RecipientsResponse {
            recipients: Some(people),
            recipients_error: None,
        }),
        Err(e) => {
            tracing::warn!("Smart Albums people failed: {}", e);
            Json(RecipientsResponse {
                recipients: Some(Vec::new()),
                recipients_error: Some(e.to_string()),
            })
        }
    }
}

/// Number of photos that show both the person and each other named person.
///
/// Counted in parallel; a person whose count fails is left out of the answer.
pub async fn get_co_counts(
    State(state): State<AppState>,
    Path(person_id): Path<String>,
) -> Result<Json<HashMap<String, u32>>, (StatusCode, String)> {
    let config = state.config.read().await;
    let client = ImmichClient::new(&config.api).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create client: {}", e),
        )
    })?;
    drop(config);

    let others: Vec<String> = client
        .get_people()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .into_iter()
        .filter(|p| p.id != person_id && p.name.as_deref().is_some_and(|n| !n.is_empty()))
        .map(|p| p.id)
        .collect();

    const COUNT_CONCURRENCY: usize = 8;
    let counts = stream::iter(others)
        .map(|other| {
            let client = &client;
            let pair = vec![person_id.clone(), other.clone()];
            async move { (other, client.count_assets_with_people(&pair).await) }
        })
        .buffer_unordered(COUNT_CONCURRENCY)
        .filter_map(|(other, count)| async move { count.ok().map(|c| (other, c)) })
        .collect::<HashMap<String, u32>>()
        .await;

    Ok(Json(counts))
}

/// The albums shared with the chosen people. Asks Smart Albums only when someone is chosen.
pub async fn resolve_recipient_albums(
    recipient_ids: &[String],
) -> Result<Vec<String>, (StatusCode, String)> {
    if recipient_ids.is_empty() {
        return Ok(Vec::new());
    }
    let client = SmartAlbums::from_env()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "\"Shared with\" is not configured (SMARTALBUMS_URL, CONNECT_KEY)".to_string(),
            )
        })?;
    let people = client
        .people()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
    Ok(recipient_album_ids(&people, recipient_ids))
}

/// Comma-separated ids from a query parameter.
pub fn split_ids(list: Option<&str>) -> Vec<String> {
    list.unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

/// Album ids without duplicates, in their first order.
pub fn merge_ids(first: Vec<String>, more: Vec<String>) -> Vec<String> {
    let mut merged: Vec<String> = Vec::new();
    for id in first.into_iter().chain(more) {
        if !merged.contains(&id) {
            merged.push(id);
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_ids() {
        assert!(split_ids(None).is_empty());
        assert!(split_ids(Some(" , ")).is_empty());
        assert_eq!(split_ids(Some("a, b,,c")), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_merge_ids() {
        let merged = merge_ids(
            vec!["a".to_string(), "b".to_string()],
            vec!["b".to_string(), "c".to_string()],
        );
        assert_eq!(merged, vec!["a", "b", "c"]);
    }
}
