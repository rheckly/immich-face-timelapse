//! `run.json` in each output folder: what a run was made from (person, sources,
//! period and its processing/video settings), so the history can show it and
//! the browser can take the settings over. Never holds the Immich address or key.

use std::path::Path;

/// File name inside the output folder.
pub const FILE: &str = "run.json";

/// Write run.json; a failure is only logged (the run itself does not depend on it).
pub async fn write(folder: &Path, value: &serde_json::Value) {
    let path = folder.join(FILE);
    let text = match serde_json::to_string_pretty(value) {
        Ok(t) => t,
        Err(e) => return tracing::warn!("run.json not written: {}", e),
    };
    if let Err(e) = tokio::fs::write(&path, text).await {
        tracing::warn!("run.json not written to {}: {}", path.display(), e);
    }
}

/// Read run.json, `None` for folders from before it existed.
pub async fn read(folder: &Path) -> Option<serde_json::Value> {
    let text = tokio::fs::read_to_string(folder.join(FILE)).await.ok()?;
    serde_json::from_str(&text).ok()
}

/// Change run.json in place (nothing happens when it does not exist).
pub async fn update(folder: &Path, change: impl FnOnce(&mut serde_json::Value)) {
    if let Some(mut run) = read(folder).await {
        if run.is_object() {
            change(&mut run);
            write(folder, &run).await;
        }
    }
}
