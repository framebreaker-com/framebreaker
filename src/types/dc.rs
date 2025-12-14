//! ΔC (Coherence Drift) types and signals
//!
//! Measures how aligned multiple speakers are in a conversation.

use serde::{Deserialize, Serialize};

/// ΔC signal weights from LLD (sum = 1.0)
pub const DC_WEIGHT_THEMATIC: f64 = 0.31;
pub const DC_WEIGHT_EMOTIONAL: f64 = 0.28;
pub const DC_WEIGHT_LOGICAL: f64 = 0.22;
pub const DC_WEIGHT_QA_MISMATCH: f64 = 0.12;
pub const DC_WEIGHT_REFERENCE: f64 = 0.07;

/// ΔC thresholds
pub const DC_THRESHOLD_LOCKED: f64 = 0.10;
pub const DC_THRESHOLD_APPROACHING: f64 = 0.15;
pub const DC_THRESHOLD_DRIFT: f64 = 0.20;

/// Raw signals for ΔC calculation (5 signals per LLD)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DcSignals {
    /// Topic consistency across turns (weight: 0.31)
    pub thematic_drift: f64,
    /// Sentiment swings between turns (weight: 0.28)
    pub emotional_volatility: f64,
    /// Abrupt topic switches (weight: 0.22)
    pub logical_breaks: f64,
    /// Questions without relevant answers (weight: 0.12)
    pub qa_mismatch: f64,
    /// Topics that disappear (weight: 0.07)
    pub reference_decay: f64,
}

impl DcSignals {
    /// Create zero signals
    pub fn zero() -> Self {
        Self::default()
    }
    
    /// Calculate weighted sum
    pub fn weighted_sum(&self) -> f64 {
        self.thematic_drift * DC_WEIGHT_THEMATIC
            + self.emotional_volatility * DC_WEIGHT_EMOTIONAL
            + self.logical_breaks * DC_WEIGHT_LOGICAL
            + self.qa_mismatch * DC_WEIGHT_QA_MISMATCH
            + self.reference_decay * DC_WEIGHT_REFERENCE
    }
}

/// Result of ΔC calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcResult {
    /// The ΔC value (0.0-1.0), or None if UNKNOWN
    pub value: Option<f64>,
    /// Signal breakdown
    pub signals: DcSignals,
    /// Reason code
    pub reason: DcReason,
    /// Number of pairs analyzed
    pub pair_count: usize,
    /// Number of speakers
    pub speaker_count: usize,
}

impl DcResult {
    /// Create a successful result
    pub fn success(value: f64, signals: DcSignals, pair_count: usize, speaker_count: usize) -> Self {
        let reason = if value < DC_THRESHOLD_LOCKED {
            DcReason::R015_DC_LOW_COHERENT
        } else if value < DC_THRESHOLD_APPROACHING {
            DcReason::R010_DC_COMPUTED
        } else if value < DC_THRESHOLD_DRIFT {
            DcReason::R010_DC_COMPUTED
        } else {
            DcReason::R014_DC_HIGH_DRIFT
        };
        
        Self {
            value: Some(value),
            signals,
            reason,
            pair_count,
            speaker_count,
        }
    }
    
    /// Create an UNKNOWN result
    pub fn unknown(reason: DcReason) -> Self {
        Self {
            value: None,
            signals: DcSignals::zero(),
            reason,
            pair_count: 0,
            speaker_count: 0,
        }
    }
    
    /// Check if ΔC is known
    pub fn is_known(&self) -> bool {
        self.value.is_some()
    }
    
    /// Get ΔC value or default (for state machine)
    pub fn value_or_default(&self) -> f64 {
        self.value.unwrap_or(0.0)
    }
    
    /// Format for display
    pub fn display_value(&self) -> String {
        match self.value {
            Some(v) => format!("{:.3}", v),
            None => "UNKNOWN".to_string(),
        }
    }
}

/// Reason codes for ΔC calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum DcReason {
    // Success codes
    /// ΔC successfully calculated
    R010_DC_COMPUTED,
    /// ΔC > 0.20, high drift
    R014_DC_HIGH_DRIFT,
    /// ΔC < 0.10, good coherence
    R015_DC_LOW_COHERENT,
    
    // UNKNOWN codes
    /// Only one speaker, cannot compute
    R011_DC_UNKNOWN_SINGLE_SPEAKER,
    /// Less than 2 turns in window
    R012_DC_UNKNOWN_INSUFFICIENT_TURNS,
    /// No paired turns (same speaker consecutive)
    R016_DC_UNKNOWN_NO_PAIRS,
    /// Window expired, no recent turns
    R013_DC_UNKNOWN_TIMEOUT,
}

impl DcReason {
    /// Get reason code string
    pub fn code(&self) -> &'static str {
        match self {
            Self::R010_DC_COMPUTED => "R010_DC_COMPUTED",
            Self::R014_DC_HIGH_DRIFT => "R014_DC_HIGH_DRIFT",
            Self::R015_DC_LOW_COHERENT => "R015_DC_LOW_COHERENT",
            Self::R011_DC_UNKNOWN_SINGLE_SPEAKER => "R011_DC_UNKNOWN_SINGLE_SPEAKER",
            Self::R012_DC_UNKNOWN_INSUFFICIENT_TURNS => "R012_DC_UNKNOWN_INSUFFICIENT_TURNS",
            Self::R016_DC_UNKNOWN_NO_PAIRS => "R016_DC_UNKNOWN_NO_PAIRS",
            Self::R013_DC_UNKNOWN_TIMEOUT => "R013_DC_UNKNOWN_TIMEOUT",
        }
    }
    
    /// Get human description
    pub fn description(&self) -> &'static str {
        match self {
            Self::R010_DC_COMPUTED => "ΔC successfully calculated",
            Self::R014_DC_HIGH_DRIFT => "High drift detected",
            Self::R015_DC_LOW_COHERENT => "Good coherence",
            Self::R011_DC_UNKNOWN_SINGLE_SPEAKER => "Only one speaker",
            Self::R012_DC_UNKNOWN_INSUFFICIENT_TURNS => "Not enough turns",
            Self::R016_DC_UNKNOWN_NO_PAIRS => "No paired turns",
            Self::R013_DC_UNKNOWN_TIMEOUT => "Window timeout",
        }
    }
    
    /// Is this an UNKNOWN reason?
    pub fn is_unknown(&self) -> bool {
        matches!(
            self,
            Self::R011_DC_UNKNOWN_SINGLE_SPEAKER
                | Self::R012_DC_UNKNOWN_INSUFFICIENT_TURNS
                | Self::R016_DC_UNKNOWN_NO_PAIRS
                | Self::R013_DC_UNKNOWN_TIMEOUT
        )
    }
}

impl std::fmt::Display for DcReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code(), self.description())
    }
}
