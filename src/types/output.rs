//! Output structures for terminal display

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::types::{FacelockState, ReasonCode};

/// Output structure for each state update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateOutput {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Current r value
    pub r: f64,
    /// Current state
    pub state: FacelockState,
    /// How long stable (milliseconds)
    pub stable_ms: u64,
    /// Reason for current state
    pub reason: ReasonCode,
    /// Is proof available?
    pub proof_available: bool,
}

impl StateOutput {
    /// Create new output
    pub fn new(
        r: f64, 
        state: FacelockState, 
        stable_ms: u64, 
        reason: ReasonCode
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            r,
            state,
            stable_ms,
            reason,
            proof_available: state == FacelockState::Locked,
        }
    }
    
    /// Format for terminal display (with colors)
    pub fn to_terminal_string(&self) -> String {
        let color = self.state.color_code();
        let reset = FacelockState::color_reset();
        let emoji = self.state.emoji();
        
        format!(
            "{}{} r={:.3} | state={} | stable={:.1}s | {}{}",
            color,
            emoji,
            self.r,
            self.state,
            self.stable_ms as f64 / 1000.0,
            self.reason.code(),
            reset
        )
    }
    
    /// Format for parseable output (no colors)
    pub fn to_parseable_string(&self) -> String {
        format!(
            "r={:.3} | state={} | stable={:.1}s | reason={}",
            self.r,
            self.state,
            self.stable_ms as f64 / 1000.0,
            self.reason.code()
        )
    }
}
