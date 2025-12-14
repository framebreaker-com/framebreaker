//! Reason codes for policy decisions and state changes
//! Based on Lumo's R-code taxonomy

use serde::{Deserialize, Serialize};

/// Reason codes for all state changes and decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum ReasonCode {
    // =========================================================================
    // R001: Alignment
    // =========================================================================
    /// Alignment achieved, entering LOCKED
    R001_ALIGNED,
    /// Not aligned, r or ΔC above threshold
    R001_NOT_ALIGNED,
    
    // =========================================================================
    // R002: State transitions
    // =========================================================================
    /// State is WAITING, not enough data
    R002_STATE_WAITING,
    /// State is APPROACHING, moving toward lock
    R002_STATE_APPROACHING,
    /// State is LOCKED, proof available
    R002_STATE_LOCKED,
    /// State is DRIFT, alignment lost
    R002_STATE_DRIFT,
    
    // =========================================================================
    // R003: Stability
    // =========================================================================
    /// Stability accumulating toward lock
    R003_STABILITY_ACCUMULATING,
    /// Stability reset due to r spike
    R003_STABILITY_RESET,
    /// Stability threshold reached (8 sec)
    R003_STABILITY_REACHED,
    
    // =========================================================================
    // R004: Thresholds
    // =========================================================================
    /// r below LOCKED threshold (< 0.15)
    R004_R_BELOW_LOCK,
    /// r below APPROACHING threshold (< 0.25)
    R004_R_BELOW_APPROACH,
    /// r above DRIFT threshold (>= 0.30)
    R004_R_ABOVE_DRIFT,
    
    // =========================================================================
    // R005: Transitions
    // =========================================================================
    /// Transitioning from WAITING to APPROACHING
    R005_TRANSITION_TO_APPROACHING,
    /// Transitioning from APPROACHING to LOCKED
    R005_TRANSITION_TO_LOCKED,
    /// Transitioning from LOCKED to DRIFT
    R005_TRANSITION_TO_DRIFT,
    /// Transitioning from DRIFT to APPROACHING (recovery)
    R005_TRANSITION_RECOVERING,
    /// Staying in current state
    R005_STATE_MAINTAINED,
}

impl ReasonCode {
    /// Get the code string (for logging)
    pub fn code(&self) -> &'static str {
        match self {
            Self::R001_ALIGNED => "R001_ALIGNED",
            Self::R001_NOT_ALIGNED => "R001_NOT_ALIGNED",
            Self::R002_STATE_WAITING => "R002_STATE_WAITING",
            Self::R002_STATE_APPROACHING => "R002_STATE_APPROACHING",
            Self::R002_STATE_LOCKED => "R002_STATE_LOCKED",
            Self::R002_STATE_DRIFT => "R002_STATE_DRIFT",
            Self::R003_STABILITY_ACCUMULATING => "R003_STABILITY_ACCUMULATING",
            Self::R003_STABILITY_RESET => "R003_STABILITY_RESET",
            Self::R003_STABILITY_REACHED => "R003_STABILITY_REACHED",
            Self::R004_R_BELOW_LOCK => "R004_R_BELOW_LOCK",
            Self::R004_R_BELOW_APPROACH => "R004_R_BELOW_APPROACH",
            Self::R004_R_ABOVE_DRIFT => "R004_R_ABOVE_DRIFT",
            Self::R005_TRANSITION_TO_APPROACHING => "R005_TRANSITION_TO_APPROACHING",
            Self::R005_TRANSITION_TO_LOCKED => "R005_TRANSITION_TO_LOCKED",
            Self::R005_TRANSITION_TO_DRIFT => "R005_TRANSITION_TO_DRIFT",
            Self::R005_TRANSITION_RECOVERING => "R005_TRANSITION_RECOVERING",
            Self::R005_STATE_MAINTAINED => "R005_STATE_MAINTAINED",
        }
    }
    
    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::R001_ALIGNED => "Alignment achieved",
            Self::R001_NOT_ALIGNED => "Not aligned",
            Self::R002_STATE_WAITING => "Waiting for input",
            Self::R002_STATE_APPROACHING => "Approaching alignment",
            Self::R002_STATE_LOCKED => "Locked - proof available",
            Self::R002_STATE_DRIFT => "Drifting - alignment lost",
            Self::R003_STABILITY_ACCUMULATING => "Building stability",
            Self::R003_STABILITY_RESET => "Stability reset",
            Self::R003_STABILITY_REACHED => "8-second stability reached",
            Self::R004_R_BELOW_LOCK => "r below lock threshold",
            Self::R004_R_BELOW_APPROACH => "r below approach threshold",
            Self::R004_R_ABOVE_DRIFT => "r above drift threshold",
            Self::R005_TRANSITION_TO_APPROACHING => "Moving to APPROACHING",
            Self::R005_TRANSITION_TO_LOCKED => "Entering LOCKED state",
            Self::R005_TRANSITION_TO_DRIFT => "Entering DRIFT state",
            Self::R005_TRANSITION_RECOVERING => "Recovering from drift",
            Self::R005_STATE_MAINTAINED => "State unchanged",
        }
    }
}

impl std::fmt::Display for ReasonCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code(), self.description())
    }
}
