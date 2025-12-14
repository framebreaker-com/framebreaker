//! Core modules for Soul-0

pub mod r_parser;
pub mod facelock;
pub mod dc_parser;
pub mod proof;
pub mod snapshot;
pub mod api;

pub use r_parser::RParser;
pub use facelock::FacelockEngine;
pub use dc_parser::DcParser;
pub use proof::{ProofGenerator, verify_proof, hash_paired_turns};
pub use snapshot::{SnapshotGenerator, save_snapshot, load_snapshot, load_and_validate_snapshot, validate_snapshot_proof};
pub use api::{create_router, run_server};
