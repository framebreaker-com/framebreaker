//! Snapshot types for memory capture
//!
//! Based on SNAPSHOTS_v1.0.md:
//! - Snapshot triggered by ProofGenerated (1-op-1 coupling)
//! - Contains: seen, blind_spots, horizon
//! - Compaction rules for aging data

use serde::{Deserialize, Serialize};

/// A complete snapshot of a Facelock moment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Unique identifier
    pub id: String,
    /// When this snapshot was created (Unix timestamp)
    pub timestamp_unix: i64,
    /// Session ID (matches proof)
    pub session_id: [u8; 16],
    /// Link to proof (hash)
    pub proof_hash: [u8; 32],
    /// Final r value
    pub r_final: f64,
    /// Final ΔC value
    pub dc_final: f64,
    /// How long LOCKED was sustained
    pub lock_duration_secs: u64,
    /// What was seen (themes, emotions, topics)
    pub seen: SeenContent,
    /// What was structurally invisible
    pub blind_spots: Vec<BlindSpot>,
    /// What was just out of reach
    pub horizon: Vec<HorizonItem>,
    /// Observers involved
    pub observers: Vec<String>,
    /// Number of turns in window
    pub turn_count: u32,
}

/// Content that was observed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeenContent {
    /// Main themes discussed
    pub themes: Vec<String>,
    /// Emotional tone detected
    pub emotion: Option<String>,
    /// Key words/topics
    pub keywords: Vec<String>,
    /// Summary of what was shared
    pub summary: Option<String>,
}

impl Default for SeenContent {
    fn default() -> Self {
        Self {
            themes: Vec::new(),
            emotion: None,
            keywords: Vec::new(),
            summary: None,
        }
    }
}

/// Something structurally invisible during the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindSpot {
    /// What was not visible
    pub description: String,
    /// Category of blind spot
    pub category: BlindSpotCategory,
    /// Confidence (0.0-1.0)
    pub confidence: f64,
}

/// Categories of blind spots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlindSpotCategory {
    // Original 7
    /// Emotions not expressed
    EmotionUnexpressed,
    /// Body sensations not mentioned
    BodyUnmentioned,
    /// Future not discussed
    FutureAbsent,
    /// Past not referenced
    PastAbsent,
    /// Other people not mentioned
    OthersAbsent,
    /// Conflict avoided
    ConflictAvoided,
    /// Uncertainty not acknowledged
    UncertaintyHidden,
    
    // New expanded categories
    /// Very low self-reference (r < 0.08)
    MinimalSelfReference,
    /// No "we/us/together" language
    NoCollectiveIdentity,
    /// No humor or playfulness
    NoHumorPlayfulness,
    /// Very few turns despite long lock
    SilenceDominant,
    /// High abstraction, no concrete examples
    HighAbstraction,
    /// No sensory details (sights, sounds, textures)
    NoSensoryDetail,
    /// No reflection on the conversation itself
    NoMetaAwareness,
}

impl BlindSpotCategory {
    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            Self::EmotionUnexpressed => "Emotions not expressed",
            Self::BodyUnmentioned => "Body sensations not mentioned",
            Self::FutureAbsent => "Future/plans not discussed",
            Self::PastAbsent => "Past/history not referenced",
            Self::OthersAbsent => "Other people not mentioned",
            Self::ConflictAvoided => "Potential conflict avoided",
            Self::UncertaintyHidden => "Uncertainty not acknowledged",
            Self::MinimalSelfReference => "Minimal self-reference (very low r)",
            Self::NoCollectiveIdentity => "No collective 'we' identity",
            Self::NoHumorPlayfulness => "No humor or playfulness",
            Self::SilenceDominant => "Silence dominant (few turns)",
            Self::HighAbstraction => "High abstraction, no concrete examples",
            Self::NoSensoryDetail => "No sensory details mentioned",
            Self::NoMetaAwareness => "No meta-awareness of the conversation",
        }
    }
    
    /// Get horizon question for this blind spot
    pub fn horizon_question(&self) -> &'static str {
        match self {
            Self::EmotionUnexpressed => "What emotion was present but unspoken?",
            Self::BodyUnmentioned => "What was felt in the body during this moment?",
            Self::FutureAbsent => "What possibilities lie beyond this moment?",
            Self::PastAbsent => "What from the past influenced this silence?",
            Self::OthersAbsent => "Who else is affected by this?",
            Self::ConflictAvoided => "What tension beneath the surface seeks resolution?",
            Self::UncertaintyHidden => "What are you unsure about?",
            Self::MinimalSelfReference => "What remains when the sense of 'I' dissolves?",
            Self::NoCollectiveIdentity => "How does 'we' experience this together?",
            Self::NoHumorPlayfulness => "What lightness or joy was present but unexpressed?",
            Self::SilenceDominant => "What speaks in the silence?",
            Self::HighAbstraction => "How does this truth appear in everyday life?",
            Self::NoSensoryDetail => "What sights, sounds, or textures accompanied this?",
            Self::NoMetaAwareness => "What is aware of this awareness?",
        }
    }
}

/// Something just out of reach
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonItem {
    /// Question or topic at the edge
    pub question: String,
    /// Why it's on the horizon (not in view)
    pub reason: String,
    /// Could become visible with...
    pub potential_trigger: Option<String>,
}

/// Result of snapshot creation
#[derive(Debug, Clone)]
pub struct SnapshotResult {
    /// The snapshot if successful
    pub snapshot: Option<Snapshot>,
    /// Reason code
    pub reason: SnapshotReason,
}

impl SnapshotResult {
    /// Create success result
    pub fn success(snapshot: Snapshot) -> Self {
        Self {
            snapshot: Some(snapshot),
            reason: SnapshotReason::R300_SNAPSHOT_CREATED,
        }
    }
    
    /// Create failure result
    pub fn failure(reason: SnapshotReason) -> Self {
        Self {
            snapshot: None,
            reason,
        }
    }
    
    /// Check if successful
    pub fn is_success(&self) -> bool {
        self.snapshot.is_some()
    }
}

/// Reason codes for snapshot creation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum SnapshotReason {
    /// Snapshot successfully created
    R300_SNAPSHOT_CREATED,
    /// No proof available (snapshot requires proof)
    R301_SNAPSHOT_NO_PROOF,
    /// Window empty
    R302_SNAPSHOT_WINDOW_EMPTY,
    /// Serialization error
    R303_SNAPSHOT_SERIALIZE_ERROR,
    /// Storage error
    R304_SNAPSHOT_STORAGE_ERROR,
    /// Invalid proof link (corrupt or tampered)
    R305_SNAPSHOT_INVALID_PROOF_LINK,
}

impl SnapshotReason {
    /// Get code string
    pub fn code(&self) -> &'static str {
        match self {
            Self::R300_SNAPSHOT_CREATED => "R300_SNAPSHOT_CREATED",
            Self::R301_SNAPSHOT_NO_PROOF => "R301_SNAPSHOT_NO_PROOF",
            Self::R302_SNAPSHOT_WINDOW_EMPTY => "R302_SNAPSHOT_WINDOW_EMPTY",
            Self::R303_SNAPSHOT_SERIALIZE_ERROR => "R303_SNAPSHOT_SERIALIZE_ERROR",
            Self::R304_SNAPSHOT_STORAGE_ERROR => "R304_SNAPSHOT_STORAGE_ERROR",
            Self::R305_SNAPSHOT_INVALID_PROOF_LINK => "R305_SNAPSHOT_INVALID_PROOF_LINK",
        }
    }
    
    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            Self::R300_SNAPSHOT_CREATED => "Snapshot successfully created",
            Self::R301_SNAPSHOT_NO_PROOF => "No proof available (required)",
            Self::R302_SNAPSHOT_WINDOW_EMPTY => "No turns in window",
            Self::R303_SNAPSHOT_SERIALIZE_ERROR => "Failed to serialize snapshot",
            Self::R304_SNAPSHOT_STORAGE_ERROR => "Failed to store snapshot",
            Self::R305_SNAPSHOT_INVALID_PROOF_LINK => "Invalid proof link (corrupt or tampered)",
        }
    }
}

impl std::fmt::Display for SnapshotReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code(), self.description())
    }
}

/// Compaction summary (for aging snapshots)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionSummary {
    /// Period covered
    pub period_start: i64,
    pub period_end: i64,
    /// Number of snapshots compacted
    pub snapshot_count: u32,
    /// Average r over period
    pub avg_r: f64,
    /// Average ΔC over period
    pub avg_dc: f64,
    /// Recurring themes
    pub recurring_themes: Vec<String>,
    /// Blind spots that became visible
    pub resolved_blind_spots: Vec<String>,
    /// New patterns detected
    pub patterns: Vec<String>,
}
