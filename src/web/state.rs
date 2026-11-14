//! Application state for the web server.

use crate::config::Config;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, RwLock as StdRwLock};
use tokio::sync::{broadcast, RwLock};
use tokio_util::sync::CancellationToken;

/// Job status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    Idle,
    Running,
    Cancelling,
    CompilingVideo,
    Completed,
    Cancelled,
    Error(String),
}

impl JobStatus {
    /// Returns true if this is a terminal status (job has finished).
    /// Terminal statuses should not be overwritten by non-terminal statuses.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            JobStatus::Completed | JobStatus::Cancelled | JobStatus::Error(_)
        )
    }

    /// Returns true while a job holds the job slot: running, making the video
    /// or still cancelling. No other job may start until it is terminal.
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            JobStatus::Running | JobStatus::Cancelling | JobStatus::CompilingVideo
        )
    }

    /// Get the status as a string identifier for API responses.
    pub fn as_str(&self) -> &str {
        match self {
            JobStatus::Idle => "idle",
            JobStatus::Running => "running",
            JobStatus::Cancelling => "cancelling",
            JobStatus::CompilingVideo => "compiling_video",
            JobStatus::Completed => "completed",
            JobStatus::Cancelled => "cancelled",
            JobStatus::Error(_) => "error",
        }
    }
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Detailed statistics for skipped images.
///
/// Uses a HashMap to support dynamic skip reasons from pipeline steps.
#[derive(Debug, Clone, Default)]
pub struct SkipStats {
    /// Map of skip reason ID to count.
    counts: HashMap<String, u32>,
    /// Number of images kept (passed all filters).
    kept: u32,
}

impl SkipStats {
    /// Create new empty skip stats.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the count for a specific reason.
    pub fn get(&self, reason: &str) -> u32 {
        self.counts.get(reason).copied().unwrap_or(0)
    }

    /// Set the count for a specific reason.
    pub fn set(&mut self, reason: impl Into<String>, count: u32) {
        self.counts.insert(reason.into(), count);
    }

    /// Get all counts as a reference to the underlying HashMap.
    pub fn counts(&self) -> &HashMap<String, u32> {
        &self.counts
    }

    /// Number of images kept (passed all filters).
    pub fn kept(&self) -> u32 {
        self.kept
    }

    /// Set the kept count.
    pub fn set_kept(&mut self, count: u32) {
        self.kept = count;
    }

    /// Total number of skipped images.
    pub fn total(&self) -> u32 {
        self.counts.values().sum()
    }
}

/// Atomic version of SkipStats for thread-safe concurrent updates.
///
/// Uses a HashMap with atomic counters to support dynamic skip reasons
/// from pipeline steps. New reasons are added on first increment.
#[derive(Debug, Default)]
pub struct AtomicSkipStats {
    /// Map of skip reason to atomic counter.
    /// Uses std::sync::RwLock for synchronous access (required for HashMap mutations).
    counts: StdRwLock<HashMap<String, Arc<AtomicU32>>>,
    /// Number of images kept (passed all filters).
    kept: AtomicU32,
}

impl AtomicSkipStats {
    /// Create a new AtomicSkipStats with no counters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment the kept counter. Returns the new count.
    pub fn increment_kept(&self) -> u32 {
        self.kept.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Increment a counter for the given reason string.
    ///
    /// If the reason doesn't exist yet, it's created with an initial count of 1.
    /// Returns the new count value.
    pub fn increment(&self, reason: &str) -> u32 {
        // First, try to get existing counter with read lock
        {
            let counts = self.counts.read().unwrap();
            if let Some(counter) = counts.get(reason) {
                return counter.fetch_add(1, Ordering::SeqCst) + 1;
            }
        }

        // Counter doesn't exist, need write lock to create it
        let mut counts = self.counts.write().unwrap();

        // Double-check after acquiring write lock (another thread may have created it)
        if let Some(counter) = counts.get(reason) {
            return counter.fetch_add(1, Ordering::SeqCst) + 1;
        }

        // Create new counter starting at 1
        let counter = Arc::new(AtomicU32::new(1));
        counts.insert(reason.to_string(), counter);
        1
    }

    /// Get a snapshot of the current stats as a non-atomic SkipStats.
    pub fn snapshot(&self) -> SkipStats {
        let counts = self.counts.read().unwrap();
        let mut stats = SkipStats::new();
        for (reason, counter) in counts.iter() {
            stats.set(reason.clone(), counter.load(Ordering::SeqCst));
        }
        stats.set_kept(self.kept.load(Ordering::SeqCst));
        stats
    }

    /// Reset all counters to zero.
    pub fn reset(&self) {
        let mut counts = self.counts.write().unwrap();
        counts.clear();
        self.kept.store(0, Ordering::SeqCst);
    }
}

/// Progress information for a running job.
#[derive(Debug, Clone)]
pub struct Progress {
    pub status: JobStatus,
    pub completed: u32,
    pub total: u32,
    pub message: Option<String>,
    pub skip_stats: SkipStats,
    /// The person being processed (for display when resuming)
    pub person_id: Option<String>,
    pub person_name: Option<String>,
    /// The album filters applied (empty = no filter)
    pub album_ids: Vec<String>,
    pub album_names: Vec<String>,
    /// Output folder of the current/last run (versioned, e.g. `anna_2026-11-12_1430`)
    pub output_folder: Option<String>,
    /// When the video compilation started (Unix time in milliseconds)
    pub video_started_at: Option<u64>,
    /// How long the video compilation took, once it is done (milliseconds)
    pub video_millis: Option<u64>,
    /// Length of the finished video in seconds
    pub video_secs: Option<f64>,
    /// The photos are prepared and wait for the user to pick them before the video is made
    pub awaiting_review: bool,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            status: JobStatus::Idle,
            completed: 0,
            total: 0,
            message: None,
            skip_stats: SkipStats::default(),
            person_id: None,
            person_name: None,
            album_ids: vec![],
            album_names: vec![],
            output_folder: None,
            video_started_at: None,
            video_millis: None,
            video_secs: None,
            awaiting_review: false,
        }
    }
}

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// Current configuration.
    pub config: Arc<RwLock<Config>>,

    /// Current job progress.
    pub progress: Arc<RwLock<Progress>>,

    /// Channel for broadcasting progress updates.
    pub progress_tx: broadcast::Sender<Progress>,

    /// Cancellation token for the current job, with the job's id so that a
    /// finishing job only clears its own token, never that of a newer job.
    pub cancel_token: Arc<RwLock<Option<(u64, CancellationToken)>>>,

    /// Source of the job ids.
    next_job_id: Arc<AtomicU64>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let (progress_tx, _) = broadcast::channel(100);

        Self {
            config: Arc::new(RwLock::new(config)),
            progress: Arc::new(RwLock::new(Progress::default())),
            progress_tx,
            cancel_token: Arc::new(RwLock::new(None)),
            next_job_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Update progress and broadcast to all listeners.
    ///
    /// Note: This method prevents non-terminal statuses (Running, CompilingVideo)
    /// from overwriting terminal statuses (Completed, Cancelled, Error). This
    /// avoids race conditions where fire-and-forget progress updates arrive after
    /// the job has finished.
    pub async fn update_progress(&self, progress: Progress) {
        let mut current = self.progress.write().await;

        // Don't allow non-terminal statuses to overwrite terminal statuses.
        // This prevents race conditions from fire-and-forget progress callbacks.
        if current.status.is_terminal() && !progress.status.is_terminal() {
            tracing::debug!(
                "Ignoring progress update: current status {:?} is terminal, incoming {:?} is not",
                current.status,
                progress.status
            );
            return;
        }

        *current = progress.clone();
        let _ = self.progress_tx.send(progress);
    }

    /// Reset progress for a new job, bypassing the terminal state check.
    ///
    /// Use this when starting a new job to clear any previous terminal state
    /// (Completed, Cancelled, Error) that would otherwise block progress updates.
    pub async fn reset_progress(&self, progress: Progress) {
        let mut current = self.progress.write().await;
        *current = progress.clone();
        let _ = self.progress_tx.send(progress);
    }

    /// Request cancellation of the current job.
    pub async fn request_cancel(&self) -> bool {
        let cancel_token = self.cancel_token.read().await;
        if let Some((_, token)) = cancel_token.as_ref() {
            token.cancel();
            // Update progress to show cancelling state immediately.
            // Only change to Cancelling if in an active (non-terminal) state.
            let mut progress = self.progress.write().await;
            if progress.status == JobStatus::Running || progress.status == JobStatus::CompilingVideo
            {
                progress.status = JobStatus::Cancelling;
                progress.message = Some("Cancelling...".to_string());
            }
            true
        } else {
            false
        }
    }

    /// Create a new cancellation token for a job; returns the job's id (for
    /// [`Self::clear_cancel_token`]) and the token.
    pub async fn create_cancel_token(&self) -> (u64, CancellationToken) {
        let id = self.next_job_id.fetch_add(1, Ordering::SeqCst);
        let token = CancellationToken::new();
        *self.cancel_token.write().await = Some((id, token.clone()));
        (id, token)
    }

    /// Clear the cancellation token when the job `id` completes. Does nothing
    /// when a newer job has its own token by then.
    pub async fn clear_cancel_token(&self, id: u64) {
        let mut slot = self.cancel_token.write().await;
        if slot.as_ref().is_some_and(|(current, _)| *current == id) {
            *slot = None;
        }
    }

    /// Check if a job is currently running and return an error if so.
    ///
    /// Use this at the start of handlers that cannot run while a job is in progress.
    pub async fn ensure_no_job_running(&self) -> Result<(), JobRunningError> {
        let progress = self.progress.read().await;
        if progress.status.is_active() {
            Err(JobRunningError)
        } else {
            Ok(())
        }
    }
}

/// Error returned when an operation cannot proceed because a job is running.
#[derive(Debug, Clone, Copy)]
pub struct JobRunningError;

impl std::fmt::Display for JobRunningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cannot perform this operation while a job is running")
    }
}

impl std::error::Error for JobRunningError {}

/// Validate that a path component (folder name or filename) is safe.
///
/// Returns an error if the name is empty or `.`, or contains traversal
/// sequences, separators or NUL.
pub fn validate_path_component(
    name: &str,
    component_type: &str,
) -> Result<(), PathValidationError> {
    // The name must stay a direct child of its folder: `DELETE /api/output/%2E`
    // must not remove the output folder itself.
    if name.trim().is_empty()
        || name.trim() == "."
        || name.contains("..")
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
    {
        Err(PathValidationError {
            component_type: component_type.to_string(),
        })
    } else {
        Ok(())
    }
}

/// Error returned when a path component fails validation.
#[derive(Debug, Clone)]
pub struct PathValidationError {
    pub component_type: String,
}

impl std::fmt::Display for PathValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid {}", self.component_type)
    }
}

impl std::error::Error for PathValidationError {}

#[cfg(test)]
mod job_slot_tests {
    use super::*;

    #[test]
    fn cancelling_still_holds_the_job_slot() {
        assert!(JobStatus::Running.is_active());
        assert!(JobStatus::Cancelling.is_active());
        assert!(JobStatus::CompilingVideo.is_active());
        assert!(!JobStatus::Idle.is_active());
        assert!(!JobStatus::Completed.is_active());
        assert!(!JobStatus::Cancelled.is_active());
        assert!(!JobStatus::Error("x".into()).is_active());
    }

    #[test]
    fn path_component_must_be_a_direct_child() {
        for bad in ["", " ", ".", " . ", "..", "../x", "a/b", "a\\b", "x\0"] {
            assert!(validate_path_component(bad, "folder").is_err(), "{bad:?}");
        }
        for ok in ["anna", "anna_2026-11-12_1430", "a.b", ".hidden"] {
            assert!(validate_path_component(ok, "folder").is_ok(), "{ok:?}");
        }
    }

    #[tokio::test]
    async fn start_is_refused_while_cancelling() {
        let state = AppState::new(Config::default());
        state.progress.write().await.status = JobStatus::Cancelling;
        assert!(state.ensure_no_job_running().await.is_err());
    }

    #[tokio::test]
    async fn an_old_job_does_not_clear_the_token_of_a_newer_one() {
        let state = AppState::new(Config::default());
        let (old, _) = state.create_cancel_token().await;
        let (new, new_token) = state.create_cancel_token().await;
        assert_ne!(old, new);

        // The old job finishes after the new one has started
        state.clear_cancel_token(old).await;
        assert!(state.request_cancel().await, "new job must stay cancellable");
        assert!(new_token.is_cancelled());

        state.clear_cancel_token(new).await;
        assert!(!state.request_cancel().await);
    }
}
