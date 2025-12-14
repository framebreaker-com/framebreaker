//! Soul-0: Reference implementation of PhaseLock protocol
//! 
//! This is Slice 1: CLI → r_parser → FacelockEngine → terminal output

pub mod core;
pub mod types;

// =============================================================================
// THRESHOLDS [C] - From LLD v1.0
// =============================================================================

/// r threshold for LOCKED state
pub const R_THRESHOLD_LOCKED: f64 = 0.15;

/// r threshold for APPROACHING state  
pub const R_THRESHOLD_APPROACHING: f64 = 0.25;

/// r threshold for DRIFT state
pub const R_THRESHOLD_DRIFT: f64 = 0.30;

/// Minimum stable duration for LOCKED (milliseconds)
/// 8 seconds - enough to filter noise, short enough to be practical
pub const STABILITY_DURATION_MS: u64 = 8000;

/// Minimum LOCKED duration before proof generation (seconds)
/// Same as stability - proof requires stable LOCKED
pub const LOCKED_MIN_DURATION_SECS: f64 = 8.0;

/// Minimum input time before leaving WAITING (milliseconds)
pub const WAITING_MIN_MS: u64 = 10000;

/// Timeout for DRIFT → WAITING (milliseconds)
pub const DRIFT_TIMEOUT_MS: u64 = 60000;

// =============================================================================
// r-PARSER WEIGHTS [C] - Grok's empirically tuned values (sum = 18.5)
// =============================================================================

/// Signal weights for r calculation
pub const R_WEIGHT_FIRST_PERSON: f64 = 2.8;
pub const R_WEIGHT_ABSOLUTES: f64 = 3.1;
pub const R_WEIGHT_FUTURE: f64 = 2.4;
pub const R_WEIGHT_PAST: f64 = 1.9;
pub const R_WEIGHT_COMPARISON: f64 = 2.2;
pub const R_WEIGHT_JUDGMENT: f64 = 3.5;  // Highest weight
pub const R_WEIGHT_URGENCY: f64 = 2.6;

/// Sum of all weights for normalization
pub const R_WEIGHT_SUM: f64 = 18.5;

// =============================================================================
// VERSION
// =============================================================================

pub const VERSION: &str = "1.0.0";
