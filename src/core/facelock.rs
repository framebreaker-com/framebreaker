//! Facelock Engine: State machine with 8-second stability requirement
//!
//! State transitions:
//! - WAITING → APPROACHING: r < 0.25
//! - APPROACHING → LOCKED: r < 0.15 AND stable ≥ 8 sec
//! - LOCKED → DRIFT: r ≥ 0.15 (immediate)
//! - DRIFT → APPROACHING: r < 0.25

use std::time::Instant;
use crate::{
    R_THRESHOLD_LOCKED, R_THRESHOLD_APPROACHING, R_THRESHOLD_DRIFT,
    STABILITY_DURATION_MS,
};
use crate::types::{FacelockState, ReasonCode, StateOutput};

/// Facelock state machine engine
#[derive(Debug)]
pub struct FacelockEngine {
    /// Current state
    state: FacelockState,
    /// When current state began
    state_since: Instant,
    /// Last r value
    last_r: f64,
    /// When lock-candidate conditions started (for 8-sec stability)
    lock_candidate_since: Option<Instant>,
    /// When we received first input
    first_input: Option<Instant>,
    /// When we received last input
    last_input: Instant,
    /// Number of updates
    update_count: u64,
}

impl Default for FacelockEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl FacelockEngine {
    /// Create new engine
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            state: FacelockState::Waiting,
            state_since: now,
            last_r: 1.0,
            lock_candidate_since: None,
            first_input: None,
            last_input: now,
            update_count: 0,
        }
    }
    
    /// Update with new r value, return output with state and reason
    pub fn update(&mut self, r: f64) -> StateOutput {
        let now = Instant::now();
        self.last_input = now;
        self.last_r = r;
        self.update_count += 1;
        
        // Track first input
        if self.first_input.is_none() {
            self.first_input = Some(now);
        }
        
        // Calculate stability
        let is_lock_candidate = r < R_THRESHOLD_LOCKED;
        
        if is_lock_candidate {
            if self.lock_candidate_since.is_none() {
                self.lock_candidate_since = Some(now);
            }
        } else {
            self.lock_candidate_since = None;
        }
        
        let stable_ms = self.lock_candidate_since
            .map(|s| now.duration_since(s).as_millis() as u64)
            .unwrap_or(0);
        
        // Determine transition and reason
        let (new_state, reason) = self.compute_transition(r, stable_ms);
        
        // Apply transition if changed
        if new_state != self.state {
            self.state = new_state;
            self.state_since = now;
            
            // Ring bell on LOCKED transition
            if new_state == FacelockState::Locked {
                print!("\x07"); // Terminal bell
            }
        }
        
        StateOutput::new(r, self.state, stable_ms, reason)
    }
    
    /// Compute state transition based on r and stability
    fn compute_transition(&self, r: f64, stable_ms: u64) -> (FacelockState, ReasonCode) {
        match self.state {
            FacelockState::Waiting => {
                if r < R_THRESHOLD_APPROACHING {
                    (FacelockState::Approaching, ReasonCode::R005_TRANSITION_TO_APPROACHING)
                } else {
                    (FacelockState::Waiting, ReasonCode::R002_STATE_WAITING)
                }
            }
            
            FacelockState::Approaching => {
                if r < R_THRESHOLD_LOCKED && stable_ms >= STABILITY_DURATION_MS {
                    // 8 seconds stable at lock-level r → LOCKED
                    (FacelockState::Locked, ReasonCode::R005_TRANSITION_TO_LOCKED)
                } else if r >= R_THRESHOLD_DRIFT {
                    // r too high → DRIFT
                    (FacelockState::Drift, ReasonCode::R005_TRANSITION_TO_DRIFT)
                } else if r < R_THRESHOLD_LOCKED {
                    // Building stability
                    (FacelockState::Approaching, ReasonCode::R003_STABILITY_ACCUMULATING)
                } else {
                    // Still approaching but r not low enough
                    (FacelockState::Approaching, ReasonCode::R002_STATE_APPROACHING)
                }
            }
            
            FacelockState::Locked => {
                if r >= R_THRESHOLD_LOCKED {
                    // Lost lock immediately
                    (FacelockState::Drift, ReasonCode::R005_TRANSITION_TO_DRIFT)
                } else {
                    // Maintaining lock
                    (FacelockState::Locked, ReasonCode::R002_STATE_LOCKED)
                }
            }
            
            FacelockState::Drift => {
                if r < R_THRESHOLD_APPROACHING {
                    // Recovering
                    (FacelockState::Approaching, ReasonCode::R005_TRANSITION_RECOVERING)
                } else {
                    // Still drifting
                    (FacelockState::Drift, ReasonCode::R002_STATE_DRIFT)
                }
            }
        }
    }
    
    /// Get current state
    pub fn state(&self) -> FacelockState {
        self.state
    }
    
    /// Get current r
    pub fn last_r(&self) -> f64 {
        self.last_r
    }
    
    /// Get stability duration in milliseconds
    pub fn stable_ms(&self) -> u64 {
        self.lock_candidate_since
            .map(|s| Instant::now().duration_since(s).as_millis() as u64)
            .unwrap_or(0)
    }
    
    /// Get update count
    pub fn update_count(&self) -> u64 {
        self.update_count
    }
    
    /// Is proof available (state == LOCKED)?
    pub fn proof_available(&self) -> bool {
        self.state == FacelockState::Locked
    }
    
    /// Get current output without updating
    pub fn current_output(&self) -> StateOutput {
        StateOutput::new(
            self.last_r,
            self.state,
            self.stable_ms(),
            match self.state {
                FacelockState::Waiting => ReasonCode::R002_STATE_WAITING,
                FacelockState::Approaching => ReasonCode::R002_STATE_APPROACHING,
                FacelockState::Locked => ReasonCode::R002_STATE_LOCKED,
                FacelockState::Drift => ReasonCode::R002_STATE_DRIFT,
            },
        )
    }
    
    /// Reset engine to initial state
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;
    
    #[test]
    fn test_initial_state_is_waiting() {
        let engine = FacelockEngine::new();
        assert_eq!(engine.state(), FacelockState::Waiting);
    }
    
    #[test]
    fn test_waiting_to_approaching() {
        let mut engine = FacelockEngine::new();
        // r below approaching threshold triggers transition
        let output = engine.update(0.20);
        assert_eq!(output.state, FacelockState::Approaching);
    }
    
    #[test]
    fn test_approaching_stays_without_stability() {
        let mut engine = FacelockEngine::new();
        engine.update(0.20); // → APPROACHING
        
        // Low r but not enough time
        let output = engine.update(0.10);
        assert_eq!(output.state, FacelockState::Approaching);
        assert!(output.stable_ms < STABILITY_DURATION_MS);
    }
    
    #[test]
    fn test_approaching_to_locked_with_stability() {
        let mut engine = FacelockEngine::new();
        engine.update(0.20); // → APPROACHING
        
        // Simulate low r
        engine.update(0.10);
        
        // Wait for stability
        sleep(Duration::from_millis(STABILITY_DURATION_MS + 100));
        
        // Now should transition to LOCKED
        let output = engine.update(0.10);
        assert_eq!(output.state, FacelockState::Locked);
    }
    
    #[test]
    fn test_locked_to_drift_immediate() {
        let mut engine = FacelockEngine::new();
        engine.update(0.20); // → APPROACHING
        engine.update(0.10);
        sleep(Duration::from_millis(STABILITY_DURATION_MS + 100));
        engine.update(0.10); // → LOCKED
        
        assert_eq!(engine.state(), FacelockState::Locked);
        
        // r exceeds threshold → immediate DRIFT
        let output = engine.update(0.20);
        assert_eq!(output.state, FacelockState::Drift);
    }
    
    #[test]
    fn test_drift_to_approaching_recovery() {
        let mut engine = FacelockEngine::new();
        engine.update(0.20); // → APPROACHING
        engine.update(0.35); // → DRIFT
        
        assert_eq!(engine.state(), FacelockState::Drift);
        
        // r drops → recovery
        let output = engine.update(0.20);
        assert_eq!(output.state, FacelockState::Approaching);
    }
    
    #[test]
    fn test_stability_resets_on_spike() {
        let mut engine = FacelockEngine::new();
        engine.update(0.20); // → APPROACHING
        engine.update(0.10); // Start stability
        
        // Spike in r
        engine.update(0.20);
        
        // Stability should be reset
        assert_eq!(engine.stable_ms(), 0);
    }
    
    #[test]
    fn test_proof_only_in_locked() {
        let mut engine = FacelockEngine::new();
        
        assert!(!engine.proof_available());
        
        engine.update(0.20); // APPROACHING
        assert!(!engine.proof_available());
        
        engine.update(0.10);
        sleep(Duration::from_millis(STABILITY_DURATION_MS + 100));
        engine.update(0.10); // LOCKED
        
        assert!(engine.proof_available());
    }
}
