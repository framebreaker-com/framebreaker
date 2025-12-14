//! Turn model for ΔC calculation
//!
//! Implements TURN_MODEL_v1.0.md:
//! - Turn = one speaker's contribution
//! - Pair = two consecutive turns from different speakers
//! - Window = 30 seconds sliding

use std::collections::{VecDeque, HashMap, HashSet};
use std::time::{Instant, Duration};
use serde::{Deserialize, Serialize};

/// Window duration for ΔC calculation
pub const WINDOW_DURATION_SECS: u64 = 30;

/// Maximum turns per speaker in window
pub const MAX_TURNS_PER_SPEAKER: usize = 10;

/// A single turn from one speaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    /// Speaker identifier (e.g., "A", "B", or name)
    pub speaker: String,
    /// The text content
    pub text: String,
    /// When this turn was created (not serialized)
    #[serde(skip)]
    pub timestamp: Option<Instant>,
    /// Computed r for this turn
    pub r: f64,
}

impl Turn {
    /// Create a new turn with current timestamp
    pub fn new(speaker: impl Into<String>, text: impl Into<String>, r: f64) -> Self {
        Self {
            speaker: speaker.into(),
            text: text.into(),
            timestamp: Some(Instant::now()),
            r,
        }
    }
    
    /// Get age in milliseconds
    pub fn age_ms(&self) -> u64 {
        self.timestamp
            .map(|t| Instant::now().duration_since(t).as_millis() as u64)
            .unwrap_or(0)
    }
}

/// A pair of consecutive turns from different speakers
#[derive(Debug, Clone)]
pub struct TurnPair {
    pub first: Turn,
    pub second: Turn,
}

/// Conversation window - sliding window for ΔC calculation
#[derive(Debug)]
pub struct ConversationWindow {
    turns: VecDeque<Turn>,
    window_duration: Duration,
}

impl Default for ConversationWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationWindow {
    /// Create new window with default duration (30s)
    pub fn new() -> Self {
        Self {
            turns: VecDeque::new(),
            window_duration: Duration::from_secs(WINDOW_DURATION_SECS),
        }
    }
    
    /// Create window with custom duration
    pub fn with_duration(secs: u64) -> Self {
        Self {
            turns: VecDeque::new(),
            window_duration: Duration::from_secs(secs),
        }
    }
    
    /// Add a turn and prune old turns
    pub fn add_turn(&mut self, turn: Turn) {
        self.turns.push_back(turn);
        self.prune();
    }
    
    /// Prune turns outside window and enforce per-speaker limit
    fn prune(&mut self) {
        let now = Instant::now();
        
        // Remove turns older than window duration
        while let Some(front) = self.turns.front() {
            if let Some(ts) = front.timestamp {
                if now.duration_since(ts) > self.window_duration {
                    self.turns.pop_front();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        
        // Enforce per-speaker limit (keep most recent)
        let mut counts: HashMap<String, usize> = HashMap::new();
        let mut to_remove: Vec<usize> = Vec::new();
        
        // Count from back (newest) to front (oldest)
        for (i, turn) in self.turns.iter().enumerate().rev() {
            let count = counts.entry(turn.speaker.clone()).or_insert(0);
            *count += 1;
            if *count > MAX_TURNS_PER_SPEAKER {
                to_remove.push(i);
            }
        }
        
        // Remove excess turns (oldest first)
        for i in to_remove.into_iter().rev() {
            self.turns.remove(i);
        }
    }
    
    /// Get all turns (oldest first)
    pub fn turns(&self) -> impl Iterator<Item = &Turn> {
        self.turns.iter()
    }
    
    /// Get turn count
    pub fn len(&self) -> usize {
        self.turns.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.turns.is_empty()
    }
    
    /// Get unique speakers in window
    pub fn speakers(&self) -> HashSet<String> {
        self.turns.iter().map(|t| t.speaker.clone()).collect()
    }
    
    /// Get speaker count
    pub fn speaker_count(&self) -> usize {
        self.speakers().len()
    }
    
    /// Extract paired turns (consecutive turns from different speakers)
    pub fn paired_turns(&self) -> Vec<TurnPair> {
        let mut pairs = Vec::new();
        let turns: Vec<_> = self.turns.iter().collect();
        
        for window in turns.windows(2) {
            if window[0].speaker != window[1].speaker {
                pairs.push(TurnPair {
                    first: window[0].clone(),
                    second: window[1].clone(),
                });
            }
        }
        
        pairs
    }
    
    /// Check if we have enough data for ΔC calculation
    pub fn can_calculate_dc(&self) -> bool {
        self.speaker_count() >= 2 && !self.paired_turns().is_empty()
    }
    
    /// Get average r across all turns in window
    pub fn average_r(&self) -> f64 {
        if self.turns.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.turns.iter().map(|t| t.r).sum();
        sum / self.turns.len() as f64
    }
    
    /// Get average r per speaker
    pub fn average_r_per_speaker(&self) -> HashMap<String, f64> {
        let mut sums: HashMap<String, (f64, usize)> = HashMap::new();
        
        for turn in &self.turns {
            let entry = sums.entry(turn.speaker.clone()).or_insert((0.0, 0));
            entry.0 += turn.r;
            entry.1 += 1;
        }
        
        sums.into_iter()
            .map(|(k, (sum, count))| (k, sum / count as f64))
            .collect()
    }
    
    /// Clear all turns
    pub fn clear(&mut self) {
        self.turns.clear();
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
    fn test_new_turn() {
        let turn = Turn::new("A", "Hello", 0.1);
        assert_eq!(turn.speaker, "A");
        assert_eq!(turn.text, "Hello");
        assert!(turn.timestamp.is_some());
    }
    
    #[test]
    fn test_window_add_turn() {
        let mut window = ConversationWindow::new();
        assert!(window.is_empty());
        
        window.add_turn(Turn::new("A", "Hello", 0.1));
        assert_eq!(window.len(), 1);
        
        window.add_turn(Turn::new("B", "Hi", 0.1));
        assert_eq!(window.len(), 2);
    }
    
    #[test]
    fn test_speaker_count() {
        let mut window = ConversationWindow::new();
        
        window.add_turn(Turn::new("A", "One", 0.1));
        assert_eq!(window.speaker_count(), 1);
        
        window.add_turn(Turn::new("A", "Two", 0.1));
        assert_eq!(window.speaker_count(), 1);
        
        window.add_turn(Turn::new("B", "Three", 0.1));
        assert_eq!(window.speaker_count(), 2);
    }
    
    #[test]
    fn test_paired_turns() {
        let mut window = ConversationWindow::new();
        
        // Same speaker - no pairs
        window.add_turn(Turn::new("A", "One", 0.1));
        window.add_turn(Turn::new("A", "Two", 0.1));
        assert!(window.paired_turns().is_empty());
        
        // Different speaker - one pair
        window.add_turn(Turn::new("B", "Three", 0.1));
        assert_eq!(window.paired_turns().len(), 1);
        
        // Back to A - two pairs now
        window.add_turn(Turn::new("A", "Four", 0.1));
        assert_eq!(window.paired_turns().len(), 2);
    }
    
    #[test]
    fn test_can_calculate_dc() {
        let mut window = ConversationWindow::new();
        
        // Empty - no
        assert!(!window.can_calculate_dc());
        
        // One speaker - no
        window.add_turn(Turn::new("A", "One", 0.1));
        assert!(!window.can_calculate_dc());
        
        // Two speakers, one pair - yes
        window.add_turn(Turn::new("B", "Two", 0.1));
        assert!(window.can_calculate_dc());
    }
    
    #[test]
    fn test_window_prune_by_time() {
        let mut window = ConversationWindow::with_duration(1); // 1 second window
        
        window.add_turn(Turn::new("A", "Old", 0.1));
        assert_eq!(window.len(), 1);
        
        // Wait for expiry
        sleep(Duration::from_millis(1100));
        
        // Add new turn to trigger prune
        window.add_turn(Turn::new("B", "New", 0.1));
        
        // Old turn should be pruned
        assert_eq!(window.len(), 1);
        assert_eq!(window.turns().next().unwrap().text, "New");
    }
    
    #[test]
    fn test_average_r() {
        let mut window = ConversationWindow::new();
        
        window.add_turn(Turn::new("A", "One", 0.1));
        window.add_turn(Turn::new("B", "Two", 0.3));
        
        assert!((window.average_r() - 0.2).abs() < 0.001);
    }
}
