//! Core types for Soul-0

mod state;
mod signals;
mod output;
mod reason;
mod turn;
mod dc;
mod proof;
mod snapshot;

pub use state::FacelockState;
pub use signals::{RSignals, RValue, LanguageHits};
pub use output::StateOutput;
pub use reason::ReasonCode;
pub use turn::{Turn, TurnPair, ConversationWindow, WINDOW_DURATION_SECS, MAX_TURNS_PER_SPEAKER};
pub use dc::{DcSignals, DcResult, DcReason, DC_THRESHOLD_LOCKED, DC_THRESHOLD_APPROACHING, DC_THRESHOLD_DRIFT};
pub use proof::{Proof, ProofPayload, ProofResult, ProofReason};
pub use snapshot::{Snapshot, SeenContent, BlindSpot, BlindSpotCategory, HorizonItem, SnapshotResult, SnapshotReason, CompactionSummary};
