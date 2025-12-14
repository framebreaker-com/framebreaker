//! Facelock state definitions

use serde::{Deserialize, Serialize};

/// The four possible states of a Facelock session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FacelockState {
    /// Initial state, not enough data yet
    Waiting,
    /// Moving toward alignment, r is dropping
    Approaching,
    /// Full alignment achieved, proof available
    Locked,
    /// Alignment lost, r exceeded threshold
    Drift,
}

impl FacelockState {
    /// Get ANSI color code for terminal display
    pub fn color_code(&self) -> &'static str {
        match self {
            FacelockState::Waiting => "\x1b[90m",     // Gray
            FacelockState::Approaching => "\x1b[33m", // Orange/Yellow
            FacelockState::Locked => "\x1b[32m",      // Green
            FacelockState::Drift => "\x1b[31m",       // Red
        }
    }
    
    /// Reset ANSI color
    pub fn color_reset() -> &'static str {
        "\x1b[0m"
    }
    
    /// Get emoji for state
    pub fn emoji(&self) -> &'static str {
        match self {
            FacelockState::Waiting => "⏳",
            FacelockState::Approaching => "🔶",
            FacelockState::Locked => "🔒",
            FacelockState::Drift => "🔴",
        }
    }
}

impl std::fmt::Display for FacelockState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            FacelockState::Waiting => "WAITING",
            FacelockState::Approaching => "APPROACHING",
            FacelockState::Locked => "LOCKED",
            FacelockState::Drift => "DRIFT",
        };
        write!(f, "{}", name)
    }
}
