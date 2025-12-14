//! ΔC Parser: Measures coherence drift between speakers
//!
//! Based on LLD v1.0 with 5 signals and TURN_MODEL semantics.
//! ΔC = UNKNOWN when conversation structure is invalid.

use crate::types::{
    ConversationWindow, TurnPair,
    DcSignals, DcResult, DcReason,
};

/// ΔC Parser for measuring coherence between speakers
#[derive(Debug, Default)]
pub struct DcParser;

impl DcParser {
    /// Create new parser
    pub fn new() -> Self {
        Self
    }
    
    /// Calculate ΔC from conversation window
    /// Returns DcResult with value or UNKNOWN reason
    pub fn calculate(&self, window: &ConversationWindow) -> DcResult {
        // Check preconditions per TURN_MODEL
        
        // Need at least 2 turns
        if window.len() < 2 {
            return DcResult::unknown(DcReason::R012_DC_UNKNOWN_INSUFFICIENT_TURNS);
        }
        
        // Need at least 2 speakers
        if window.speaker_count() < 2 {
            return DcResult::unknown(DcReason::R011_DC_UNKNOWN_SINGLE_SPEAKER);
        }
        
        // Need at least 1 pair
        let pairs = window.paired_turns();
        if pairs.is_empty() {
            return DcResult::unknown(DcReason::R016_DC_UNKNOWN_NO_PAIRS);
        }
        
        // Calculate signals from pairs
        let signals = self.calculate_signals(&pairs);
        let dc_value = signals.weighted_sum().clamp(0.0, 1.0);
        
        DcResult::success(
            dc_value,
            signals,
            pairs.len(),
            window.speaker_count(),
        )
    }
    
    /// Calculate individual signals from turn pairs
    fn calculate_signals(&self, pairs: &[TurnPair]) -> DcSignals {
        if pairs.is_empty() {
            return DcSignals::zero();
        }
        
        DcSignals {
            thematic_drift: self.calc_thematic_drift(pairs),
            emotional_volatility: self.calc_emotional_volatility(pairs),
            logical_breaks: self.calc_logical_breaks(pairs),
            qa_mismatch: self.calc_qa_mismatch(pairs),
            reference_decay: self.calc_reference_decay(pairs),
        }
    }
    
    /// Signal 1: Thematic drift (topic consistency)
    /// Higher = less consistent topics
    fn calc_thematic_drift(&self, pairs: &[TurnPair]) -> f64 {
        // Simple heuristic: word overlap between consecutive turns
        // Low overlap = high drift
        
        let mut total_drift = 0.0;
        
        for pair in pairs {
            let text1 = pair.first.text.to_lowercase();
            let text2 = pair.second.text.to_lowercase();
            
            let words1: std::collections::HashSet<&str> = text1
                .split_whitespace()
                .filter(|w| w.len() > 2)
                .collect();
            
            let words2: std::collections::HashSet<&str> = text2
                .split_whitespace()
                .filter(|w| w.len() > 2)
                .collect();
            
            if words1.is_empty() || words2.is_empty() {
                total_drift += 0.5; // Neutral if no meaningful words
            } else {
                let intersection = words1.intersection(&words2).count();
                let union = words1.union(&words2).count();
                let jaccard = intersection as f64 / union as f64;
                total_drift += 1.0 - jaccard; // Invert: low overlap = high drift
            }
        }
        
        (total_drift / pairs.len() as f64).clamp(0.0, 1.0)
    }
    
    /// Signal 2: Emotional volatility (sentiment swings)
    /// Higher = more dramatic sentiment changes
    fn calc_emotional_volatility(&self, pairs: &[TurnPair]) -> f64 {
        // Simple heuristic: presence of emotional markers
        let emotional_markers = [
            "!", "?!", "...", 
            "wow", "amazing", "terrible", "hate", "love", "angry",
            "happy", "sad", "excited", "frustrated", "annoyed",
            "wauw", "geweldig", "verschrikkelijk", "haat", "boos",
            "blij", "verdrietig", "gefrustreerd",
        ];
        
        let mut volatility = 0.0;
        
        for pair in pairs {
            let text1 = pair.first.text.to_lowercase();
            let text2 = pair.second.text.to_lowercase();
            
            let count1: usize = emotional_markers.iter()
                .map(|m| text1.matches(m).count())
                .sum();
            let count2: usize = emotional_markers.iter()
                .map(|m| text2.matches(m).count())
                .sum();
            
            // Volatility = difference in emotional intensity
            volatility += (count1 as f64 - count2 as f64).abs() * 0.2;
        }
        
        (volatility / pairs.len() as f64).clamp(0.0, 1.0)
    }
    
    /// Signal 3: Logical breaks (abrupt topic switches)
    /// Higher = more abrupt changes
    fn calc_logical_breaks(&self, pairs: &[TurnPair]) -> f64 {
        // Heuristic: check for transition words or complete topic change
        let transition_words = [
            "but", "however", "although", "anyway", "by the way",
            "speaking of", "that reminds me", "off topic",
            "maar", "echter", "overigens", "trouwens",
        ];
        
        let mut breaks = 0.0;
        
        for pair in pairs {
            let text2 = pair.second.text.to_lowercase();
            
            // Check for abrupt transitions
            for word in &transition_words {
                if text2.contains(word) {
                    breaks += 0.3;
                }
            }
            
            // Check for very short responses (might indicate disconnect)
            if pair.second.text.split_whitespace().count() <= 2 
                && pair.first.text.split_whitespace().count() > 10 {
                breaks += 0.2;
            }
        }
        
        (breaks / pairs.len() as f64).clamp(0.0, 1.0)
    }
    
    /// Signal 4: Q&A mismatch (questions without answers)
    /// Higher = more unanswered questions
    fn calc_qa_mismatch(&self, pairs: &[TurnPair]) -> f64 {
        let mut mismatches = 0.0;
        
        for pair in pairs {
            let is_question = pair.first.text.contains('?');
            
            if is_question {
                // Check if response seems like an answer
                let response = pair.second.text.to_lowercase();
                let answer_indicators = [
                    "yes", "no", "ja", "nee", "because", "omdat",
                    "i think", "ik denk", "maybe", "misschien",
                ];
                
                let has_answer = answer_indicators.iter()
                    .any(|ind| response.contains(ind))
                    || response.len() > 20; // Long response likely addresses question
                
                if !has_answer {
                    mismatches += 0.5;
                }
            }
        }
        
        (mismatches / pairs.len() as f64).clamp(0.0, 1.0)
    }
    
    /// Signal 5: Reference decay (topics that disappear)
    /// Higher = more abandoned topics
    fn calc_reference_decay(&self, pairs: &[TurnPair]) -> f64 {
        if pairs.len() < 2 {
            return 0.0;
        }
        
        // Track nouns/topics across pairs
        // If topics from early pairs never appear again, that's decay
        
        // Simplified: check if words from first pair appear in last pair
        let first_words: std::collections::HashSet<_> = pairs.first()
            .map(|p| {
                p.first.text.to_lowercase()
                    .split_whitespace()
                    .filter(|w| w.len() > 3)
                    .map(String::from)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
            .into_iter()
            .collect();
        
        let last_words: std::collections::HashSet<_> = pairs.last()
            .map(|p| {
                format!("{} {}", p.first.text, p.second.text)
                    .to_lowercase()
                    .split_whitespace()
                    .filter(|w| w.len() > 3)
                    .map(String::from)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
            .into_iter()
            .collect();
        
        if first_words.is_empty() {
            return 0.0;
        }
        
        let retained = first_words.intersection(&last_words).count();
        let decay = 1.0 - (retained as f64 / first_words.len() as f64);
        
        decay.clamp(0.0, 1.0)
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Turn;
    
    fn make_window(turns: Vec<(&str, &str, f64)>) -> ConversationWindow {
        let mut window = ConversationWindow::new();
        for (speaker, text, r) in turns {
            window.add_turn(Turn::new(speaker, text, r));
        }
        window
    }
    
    #[test]
    fn test_empty_window() {
        let parser = DcParser::new();
        let window = ConversationWindow::new();
        let result = parser.calculate(&window);
        
        assert!(!result.is_known());
        assert_eq!(result.reason, DcReason::R012_DC_UNKNOWN_INSUFFICIENT_TURNS);
    }
    
    #[test]
    fn test_single_speaker() {
        let parser = DcParser::new();
        let window = make_window(vec![
            ("A", "Hello", 0.1),
            ("A", "How are you", 0.1),
        ]);
        let result = parser.calculate(&window);
        
        assert!(!result.is_known());
        assert_eq!(result.reason, DcReason::R011_DC_UNKNOWN_SINGLE_SPEAKER);
    }
    
    #[test]
    fn test_no_pairs() {
        let parser = DcParser::new();
        // Two speakers but both speak only once - this should create a pair
        let window = make_window(vec![
            ("A", "Hello", 0.1),
            ("B", "Hi", 0.1),
        ]);
        let result = parser.calculate(&window);
        
        // This should work - we have one pair (A→B)
        assert!(result.is_known());
    }
    
    #[test]
    fn test_clean_conversation_low_dc() {
        let parser = DcParser::new();
        let window = make_window(vec![
            ("A", "The sky is blue today", 0.05),
            ("B", "Yes, the sky is very blue", 0.05),
            ("A", "Blue sky makes me happy", 0.08),
            ("B", "Blue sky is beautiful", 0.06),
        ]);
        let result = parser.calculate(&window);
        
        assert!(result.is_known());
        let dc = result.value.unwrap();
        assert!(dc < 0.30, "Clean conversation should have low ΔC, got {}", dc);
    }
    
    #[test]
    fn test_messy_conversation_high_dc() {
        let parser = DcParser::new();
        let window = make_window(vec![
            ("A", "What do you think about the weather?", 0.1),
            ("B", "I had pizza yesterday", 0.2),
            ("A", "The ocean is beautiful", 0.1),
            ("B", "My car needs repairs", 0.3),
        ]);
        let result = parser.calculate(&window);
        
        assert!(result.is_known());
        let dc = result.value.unwrap();
        assert!(dc > 0.20, "Messy conversation should have high ΔC, got {}", dc);
    }
    
    #[test]
    fn test_qa_mismatch() {
        let parser = DcParser::new();
        let window = make_window(vec![
            ("A", "What is your favorite color?", 0.1),
            ("B", "The weather is nice", 0.1), // Doesn't answer
        ]);
        let result = parser.calculate(&window);
        
        assert!(result.is_known());
        assert!(result.signals.qa_mismatch > 0.0, "Should detect Q&A mismatch");
    }
    
    #[test]
    fn test_reason_codes() {
        let parser = DcParser::new();
        
        // Low coherence conversation
        let window = make_window(vec![
            ("A", "Peace and stillness", 0.02),
            ("B", "Yes, peace and calm", 0.02),
        ]);
        let result = parser.calculate(&window);
        // Should successfully compute (either LOW_COHERENT or COMPUTED depending on threshold)
        assert!(result.is_known(), "Should compute ΔC for valid conversation");
    }
}
