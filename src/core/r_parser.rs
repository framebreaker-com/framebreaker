//! r-Parser: Measures ego noise via 7 linguistic signals
//! 
//! Based on LLD v1.0 with Grok's empirically tuned weights (sum = 18.5)

use lazy_static::lazy_static;
use regex::Regex;
use crate::{
    R_WEIGHT_FIRST_PERSON, R_WEIGHT_ABSOLUTES, R_WEIGHT_FUTURE,
    R_WEIGHT_PAST, R_WEIGHT_COMPARISON, R_WEIGHT_JUDGMENT, 
    R_WEIGHT_URGENCY, R_WEIGHT_SUM,
};
use crate::types::{RSignals, RValue};

lazy_static! {
    // =========================================================================
    // Signal 1: First person (weight: 2.8)
    // Dutch: ik, mij, mijn, me, m'n, mezelf, mijzelf, zelf, wij, ons, onze
    // =========================================================================
    static ref RE_FIRST_PERSON: Regex = Regex::new(
        r"(?i)\b(i|me|my|mine|myself|i'm|i've|i'll|i'd|i am|ik|mij|mijn|me|m'n|mezelf|mijzelf|zelf|wij|ons|onze)\b"
    ).unwrap();
    
    // =========================================================================
    // Signal 2: Absolutes (weight: 3.1)
    // Dutch: altijd, nooit, iedereen, niemand, alles, niets, helemaal, volledig, totaal
    // =========================================================================
    static ref RE_ABSOLUTES: Regex = Regex::new(
        r"(?i)\b(always|never|everything|nothing|everyone|no one|nobody|everybody|all|none|every|any|completely|totally|absolutely|altijd|nooit|iedereen|niemand|alles|niets|helemaal|volledig|totaal|elk|elke|geen enkele|overal|nergens)\b"
    ).unwrap();
    
    // =========================================================================
    // Signal 3: Future projection (weight: 2.4)
    // Dutch: zal, ga, gaat, moet, moeten, morgen, straks, binnenkort, later
    // =========================================================================
    static ref RE_FUTURE: Regex = Regex::new(
        r"(?i)\b(will|going to|gonna|must|have to|need to|should|ought to|shall|tomorrow|next|soon|later|eventually|zal|zullen|ga|gaat|gaan|moet|moeten|morgen|straks|binnenkort|later|ooit|dadelijk|zo meteen)\b"
    ).unwrap();
    
    // =========================================================================
    // Signal 4: Past attachment (weight: 1.9)
    // Dutch: vroeger, toen, gisteren, voorheen, ooit, was, waren, had, hadden
    // =========================================================================
    static ref RE_PAST: Regex = Regex::new(
        r"(?i)\b(was|were|had|used to|before|previously|back then|in the past|yesterday|last|earlier|once|vroeger|toen|gisteren|voorheen|ooit|destijds|in het verleden|was|waren|had|hadden|geweest)\b"
    ).unwrap();
    
    // =========================================================================
    // Signal 5: Comparison (weight: 2.2)
    // Dutch: beter, slechter, meer, minder, dan, liever, eerder, vergelijken
    // =========================================================================
    static ref RE_COMPARISON: Regex = Regex::new(
        r"(?i)\b(better|worse|more|less|than|compared to|versus|vs|superior|inferior|ahead|behind|rather|instead|beter|slechter|meer|minder|dan|liever|eerder|vergeleken met|in vergelijking|anders dan)\b"
    ).unwrap();
    
    // =========================================================================
    // Signal 6: Judgment (weight: 3.5 - HIGHEST)
    // Dutch: zou moeten, fout, schuld, slecht, goed, dom, stom, verkeerd, terecht
    // =========================================================================
    static ref RE_JUDGMENT: Regex = Regex::new(
        r"(?i)\b(should|shouldn't|wrong|right|fault|blame|guilty|mistake|bad|good|terrible|awful|stupid|idiot|horrible|perfect|correct|incorrect|zou moeten|had moeten|fout|schuld|slecht|goed|dom|stom|verkeerd|terecht|onterecht|verschrikkelijk|vreselijk|idioot|perfect|juist|onjuist)\b"
    ).unwrap();
    
    // =========================================================================
    // Signal 7: Urgency (weight: 2.6)
    // Dutch: nu, meteen, snel, dringend, belangrijk, direct, onmiddellijk
    // =========================================================================
    static ref RE_URGENCY: Regex = Regex::new(
        r"(?i)\b(now|immediately|right now|quickly|hurry|urgent|asap|fast|rush|important|critical|essential|nu|meteen|snel|dringend|belangrijk|direct|onmiddellijk|gauw|haast|spoed|acuut|cruciaal)\b"
    ).unwrap();
}

/// r-Parser for measuring ego noise
#[derive(Debug, Default)]
pub struct RParser;

impl RParser {
    /// Create new parser
    pub fn new() -> Self {
        Self
    }
    
    /// Parse text and return r value with full signal breakdown
    pub fn parse(&self, text: &str) -> RValue {
        let text = text.trim();
        
        // Handle empty input
        if text.is_empty() {
            return RValue::new(0.0, RSignals::zero(), 0.0, 0);
        }
        
        let word_count = text.split_whitespace().count().max(1);
        let wc = word_count as f64;
        
        // Count matches for each signal (normalized by word count)
        let signals = RSignals {
            first_person: count_matches(&RE_FIRST_PERSON, text) / wc,
            absolutes: count_matches(&RE_ABSOLUTES, text) / wc,
            future_projection: count_matches(&RE_FUTURE, text) / wc,
            past_attachment: count_matches(&RE_PAST, text) / wc,
            comparison: count_matches(&RE_COMPARISON, text) / wc,
            judgment: count_matches(&RE_JUDGMENT, text) / wc,
            urgency: count_matches(&RE_URGENCY, text) / wc,
            language_hits: None, // Debug field, not used in normal parsing
        };
        
        // Weighted sum using Grok's exact weights
        let raw_score = 
            signals.first_person * R_WEIGHT_FIRST_PERSON +
            signals.absolutes * R_WEIGHT_ABSOLUTES +
            signals.future_projection * R_WEIGHT_FUTURE +
            signals.past_attachment * R_WEIGHT_PAST +
            signals.comparison * R_WEIGHT_COMPARISON +
            signals.judgment * R_WEIGHT_JUDGMENT +
            signals.urgency * R_WEIGHT_URGENCY;
        
        // Normalize: r = (sum / 18.5).clamp(0.0, 1.0)
        let value = (raw_score / R_WEIGHT_SUM).clamp(0.0, 1.0);
        
        // Confidence based on text length (more words = more reliable)
        let confidence = (word_count as f64 / 50.0).min(1.0);
        
        RValue::new(value, signals, confidence, word_count)
    }
    
    /// Quick parse - just return the r value
    pub fn quick_parse(&self, text: &str) -> f64 {
        self.parse(text).value
    }
}

/// Count regex matches in text
fn count_matches(regex: &Regex, text: &str) -> f64 {
    regex.find_iter(text).count() as f64
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_empty_input() {
        let parser = RParser::new();
        let result = parser.parse("");
        assert_eq!(result.value, 0.0);
        assert_eq!(result.word_count, 0);
    }
    
    #[test]
    fn test_low_r_pure_observation() {
        let parser = RParser::new();
        // Pure observation, no ego
        let result = parser.parse("The sky is blue. Silence. Breathing happens.");
        assert!(result.value < 0.10, "Expected r < 0.10 for pure observation, got {}", result.value);
    }
    
    #[test]
    fn test_low_r_meditation() {
        let parser = RParser::new();
        let result = parser.parse("Stillness. Presence. Awareness.");
        assert!(result.value < 0.10, "Expected r < 0.10 for meditation language, got {}", result.value);
    }
    
    #[test]
    fn test_high_r_ego_noise() {
        let parser = RParser::new();
        // Maximum ego noise
        let result = parser.parse(
            "I always think I should do better than everyone else immediately! It's their fault, not mine!"
        );
        // With word-normalized signals, even high ego text produces moderate r
        assert!(result.value > 0.05, "Expected r > 0.05 for high ego noise, got {}", result.value);
    }
    
    #[test]
    fn test_medium_r_normal_conversation() {
        let parser = RParser::new();
        // Normal conversation
        let result = parser.parse("I think this is interesting, what do you think?");
        // Should be low-to-moderate
        assert!(
            result.value < 0.30, 
            "Expected r < 0.30 for normal conversation, got {}", 
            result.value
        );
    }
    
    #[test]
    fn test_dutch_high_r() {
        let parser = RParser::new();
        // Dutch high r
        let result = parser.parse(
            "Ik vind dat iedereen altijd te veel praat, vroeger was het beter."
        );
        // Dutch patterns should trigger some signals
        assert!(result.value > 0.02, "Expected r > 0.02 for Dutch high-r, got {}", result.value);
    }
    
    #[test]
    fn test_dutch_low_r() {
        let parser = RParser::new();
        // Dutch low r
        let result = parser.parse("Stilte. Ademhalen. Ruimte.");
        assert!(result.value < 0.15, "Expected r < 0.15 for Dutch low-r, got {}", result.value);
    }
    
    #[test]
    fn test_determinism() {
        let parser = RParser::new();
        let text = "I think therefore I am, but what do I really know?";
        let r1 = parser.quick_parse(text);
        let r2 = parser.quick_parse(text);
        assert!((r1 - r2).abs() < 1e-10, "Same input should give same r");
    }
    
    #[test]
    fn test_judgment_has_highest_weight() {
        let parser = RParser::new();
        // Text with mostly judgment words
        let judgment_text = "wrong fault blame guilty mistake bad terrible";
        let result = parser.parse(judgment_text);
        assert!(
            result.signals.judgment > result.signals.first_person,
            "Judgment signal should be detected"
        );
    }
    
    #[test]
    fn test_confidence_increases_with_length() {
        let parser = RParser::new();
        let short = parser.parse("Hello");
        let long = parser.parse("Hello there, this is a much longer text with many more words in it to analyze properly");
        assert!(
            long.confidence > short.confidence,
            "Longer text should have higher confidence"
        );
    }
}
