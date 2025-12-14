//! Signal structures for r-parser

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Raw signals extracted from text (7 signals per LLD)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RSignals {
    /// Density of I/me/my/mine (weight: 2.8)
    pub first_person: f64,
    /// always/never/everything/nothing (weight: 3.1)
    pub absolutes: f64,
    /// will/going to/must/should (weight: 2.4)
    pub future_projection: f64,
    /// was/had/used to/before (weight: 1.9)
    pub past_attachment: f64,
    /// better/worse/more/less than (weight: 2.2)
    pub comparison: f64,
    /// should/wrong/fault/blame (weight: 3.5 - HIGHEST)
    pub judgment: f64,
    /// now/immediately/quickly/hurry (weight: 2.6)
    pub urgency: f64,
    /// Debug: language pattern hits (EN vs NL)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_hits: Option<LanguageHits>,
}

/// Debug info for language pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageHits {
    pub english: u32,
    pub dutch: u32,
}

impl RSignals {
    /// Create zero signals
    pub fn zero() -> Self {
        Self {
            first_person: 0.0,
            absolutes: 0.0,
            future_projection: 0.0,
            past_attachment: 0.0,
            comparison: 0.0,
            judgment: 0.0,
            urgency: 0.0,
            language_hits: None,
        }
    }
}

/// Computed r value with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RValue {
    /// Final r value: 0.0-1.0
    pub value: f64,
    /// Raw signals used to compute r
    pub signals: RSignals,
    /// Confidence based on text length (0.0-1.0)
    pub confidence: f64,
    /// When this was computed
    pub timestamp: DateTime<Utc>,
    /// Word count of input
    pub word_count: usize,
}

impl RValue {
    /// Create a new RValue
    pub fn new(value: f64, signals: RSignals, confidence: f64, word_count: usize) -> Self {
        Self {
            value,
            signals,
            confidence,
            timestamp: Utc::now(),
            word_count,
        }
    }
}
