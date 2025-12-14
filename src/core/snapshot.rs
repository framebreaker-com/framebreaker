//! Snapshot generation and blind spot detection
//!
//! Key invariant: Snapshot only created when Proof is generated
//! This ensures 1-op-1 coupling between proofs and snapshots

use sha2::{Sha256, Digest};
use crate::types::{
    Turn, ConversationWindow,
    Proof, Snapshot, SeenContent, BlindSpot, BlindSpotCategory,
    HorizonItem, SnapshotResult, SnapshotReason,
};

/// Snapshot generator
#[derive(Debug, Default)]
pub struct SnapshotGenerator;

impl SnapshotGenerator {
    /// Create new generator
    pub fn new() -> Self {
        Self
    }
    
    /// Generate snapshot from a proof and conversation window
    /// 
    /// INVARIANT: Only call this AFTER proof generation succeeds
    pub fn generate(
        &self,
        proof: &Proof,
        window: &ConversationWindow,
        observers: Vec<String>,
    ) -> SnapshotResult {
        // Check preconditions
        if window.paired_turns().is_empty() {
            return SnapshotResult::failure(SnapshotReason::R302_SNAPSHOT_WINDOW_EMPTY);
        }
        
        let _pairs = window.paired_turns();
        let turns: Vec<&Turn> = window.turns().collect();
        
        // Extract seen content
        let seen = self.extract_seen(&turns);
        
        // Detect blind spots
        let blind_spots = self.detect_blind_spots(&turns);
        
        // Generate horizon items
        let horizon = self.generate_horizon(&turns, &blind_spots);
        
        // Hash the proof for linking
        let proof_hash = sha256(&proof.to_bytes());
        
        // Create snapshot ID with proof_hash prefix for easy linking
        let id = format!(
            "snap_{}_{:x}_{:08x}",
            chrono::Utc::now().format("%Y%m%d_%H%M%S"),
            u32::from_be_bytes(proof.payload.session_id[0..4].try_into().unwrap()),
            u32::from_be_bytes(proof_hash[0..4].try_into().unwrap()) // First 4 bytes of proof hash
        );
        
        let snapshot = Snapshot {
            id,
            timestamp_unix: chrono::Utc::now().timestamp(),
            session_id: proof.payload.session_id,
            proof_hash,
            r_final: proof.payload.r_final,
            dc_final: proof.payload.dc_final,
            lock_duration_secs: proof.payload.lock_duration_secs,
            seen,
            blind_spots,
            horizon,
            observers,
            turn_count: turns.len() as u32,
        };
        
        SnapshotResult::success(snapshot)
    }
    
    /// Extract seen content from turns
    fn extract_seen(&self, turns: &[&Turn]) -> SeenContent {
        let all_text: String = turns.iter()
            .map(|t| t.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        
        let all_text_lower = all_text.to_lowercase();
        
        // Extract keywords (simple: words > 4 chars, appearing multiple times)
        let mut word_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for word in all_text_lower.split_whitespace() {
            let clean: String = word.chars().filter(|c| c.is_alphabetic()).collect();
            if clean.len() > 4 {
                *word_counts.entry(clean).or_insert(0) += 1;
            }
        }
        
        let keywords: Vec<String> = word_counts.into_iter()
            .filter(|(_, count)| *count >= 2)
            .map(|(word, _)| word)
            .take(10)
            .collect();
        
        // Detect themes (simplified heuristic)
        let mut themes = Vec::new();
        
        let theme_patterns = [
            ("peace", "Inner peace"),
            ("calm", "Calmness"),
            ("rust", "Rest/stillness"),
            ("stil", "Stillness"),
            ("helder", "Clarity"),
            ("clear", "Clarity"),
            ("samen", "Togetherness"),
            ("together", "Togetherness"),
            ("natuur", "Nature"),
            ("nature", "Nature"),
            ("sky", "Nature"),
            ("lucht", "Nature"),
        ];
        
        for (pattern, theme) in theme_patterns {
            if all_text_lower.contains(pattern) && !themes.contains(&theme.to_string()) {
                themes.push(theme.to_string());
            }
        }
        
        // Detect emotion
        let emotion = self.detect_emotion(&all_text_lower);
        
        SeenContent {
            themes,
            emotion,
            keywords,
            summary: None, // Could be LLM-generated in future
        }
    }
    
    /// Detect overall emotional tone
    fn detect_emotion(&self, text: &str) -> Option<String> {
        let positive = ["peace", "calm", "happy", "joy", "love", "beautiful", 
                       "vrede", "rustig", "blij", "mooi", "prachtig"];
        let negative = ["angry", "sad", "fear", "worry", "stress",
                       "boos", "verdrietig", "angst", "zorgen", "stress"];
        
        let pos_count = positive.iter().filter(|w| text.contains(*w)).count();
        let neg_count = negative.iter().filter(|w| text.contains(*w)).count();
        
        if pos_count > neg_count && pos_count > 0 {
            Some("Positive/peaceful".to_string())
        } else if neg_count > pos_count && neg_count > 0 {
            Some("Negative/tense".to_string())
        } else if pos_count > 0 && neg_count > 0 {
            Some("Mixed".to_string())
        } else {
            Some("Neutral".to_string())
        }
    }
    
    /// Detect blind spots based on what's missing
    fn detect_blind_spots(&self, turns: &[&Turn]) -> Vec<BlindSpot> {
        let all_text: String = turns.iter()
            .map(|t| t.text.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        
        let mut blind_spots = Vec::new();
        
        // Check for emotions not expressed
        let emotion_words = ["feel", "feeling", "emotion", "voel", "gevoel", "emotie"];
        if !emotion_words.iter().any(|w| all_text.contains(w)) {
            blind_spots.push(BlindSpot {
                description: "No emotions explicitly named".to_string(),
                category: BlindSpotCategory::EmotionUnexpressed,
                confidence: 0.7,
            });
        }
        
        // Check for body not mentioned
        let body_words = ["body", "physical", "sensation", "lichaam", "fysiek", "gevoel in"];
        if !body_words.iter().any(|w| all_text.contains(w)) {
            blind_spots.push(BlindSpot {
                description: "Body sensations not mentioned".to_string(),
                category: BlindSpotCategory::BodyUnmentioned,
                confidence: 0.6,
            });
        }
        
        // Check for future absent
        let future_words = ["will", "going to", "plan", "tomorrow", "future", 
                          "zal", "gaan", "plan", "morgen", "toekomst"];
        if !future_words.iter().any(|w| all_text.contains(w)) {
            blind_spots.push(BlindSpot {
                description: "Future/plans not discussed".to_string(),
                category: BlindSpotCategory::FutureAbsent,
                confidence: 0.5,
            });
        }
        
        // Check for past absent
        let past_words = ["was", "were", "used to", "before", "yesterday", "history",
                         "was", "waren", "vroeger", "gisteren", "geschiedenis"];
        if !past_words.iter().any(|w| all_text.contains(w)) {
            blind_spots.push(BlindSpot {
                description: "Past/history not referenced".to_string(),
                category: BlindSpotCategory::PastAbsent,
                confidence: 0.5,
            });
        }
        
        // Check for others absent
        let others_words = ["they", "them", "people", "friend", "family",
                          "zij", "hen", "mensen", "vriend", "familie"];
        if !others_words.iter().any(|w| all_text.contains(w)) {
            blind_spots.push(BlindSpot {
                description: "Other people not mentioned".to_string(),
                category: BlindSpotCategory::OthersAbsent,
                confidence: 0.4,
            });
        }
        
        // Check for uncertainty hidden
        let uncertainty_words = ["maybe", "perhaps", "not sure", "uncertain", "doubt",
                                "misschien", "wellicht", "weet niet", "onzeker", "twijfel"];
        if !uncertainty_words.iter().any(|w| all_text.contains(w)) {
            blind_spots.push(BlindSpot {
                description: "No uncertainty expressed (everything seems certain)".to_string(),
                category: BlindSpotCategory::UncertaintyHidden,
                confidence: 0.4,
            });
        }
        
        // NEW: Check for minimal self-reference (low average r)
        let avg_r: f64 = turns.iter().map(|t| t.r).sum::<f64>() / turns.len().max(1) as f64;
        if avg_r < 0.08 {
            blind_spots.push(BlindSpot {
                description: "Very low self-reference throughout".to_string(),
                category: BlindSpotCategory::MinimalSelfReference,
                confidence: 0.6,
            });
        }
        
        // NEW: Check for no collective identity
        let collective_words = ["we", "us", "our", "together", "wij", "ons", "samen"];
        if !collective_words.iter().any(|w| all_text.contains(w)) {
            blind_spots.push(BlindSpot {
                description: "No collective 'we' language".to_string(),
                category: BlindSpotCategory::NoCollectiveIdentity,
                confidence: 0.4,
            });
        }
        
        // NEW: Check for no humor
        let humor_words = ["haha", "lol", "funny", "joke", "laugh", "grappig", "lachen"];
        if !humor_words.iter().any(|w| all_text.contains(w)) {
            blind_spots.push(BlindSpot {
                description: "No humor or playfulness".to_string(),
                category: BlindSpotCategory::NoHumorPlayfulness,
                confidence: 0.3,
            });
        }
        
        // NEW: Check for silence dominant (few turns)
        if turns.len() < 4 {
            blind_spots.push(BlindSpot {
                description: "Very few exchanges (silence dominant)".to_string(),
                category: BlindSpotCategory::SilenceDominant,
                confidence: 0.5,
            });
        }
        
        // NEW: Check for high abstraction
        let abstract_words = ["being", "awareness", "consciousness", "truth", "reality",
                            "zijn", "bewustzijn", "waarheid", "werkelijkheid"];
        let concrete_words = ["table", "chair", "car", "house", "food", "water",
                            "tafel", "stoel", "auto", "huis", "eten", "water"];
        let has_abstract = abstract_words.iter().any(|w| all_text.contains(w));
        let has_concrete = concrete_words.iter().any(|w| all_text.contains(w));
        if has_abstract && !has_concrete {
            blind_spots.push(BlindSpot {
                description: "High abstraction without concrete examples".to_string(),
                category: BlindSpotCategory::HighAbstraction,
                confidence: 0.4,
            });
        }
        
        // NEW: Check for no sensory details
        let sensory_words = ["see", "hear", "smell", "taste", "touch", "feel", "sound", "color",
                           "zien", "horen", "ruiken", "proeven", "voelen", "geluid", "kleur"];
        if !sensory_words.iter().any(|w| all_text.contains(w)) {
            blind_spots.push(BlindSpot {
                description: "No sensory details mentioned".to_string(),
                category: BlindSpotCategory::NoSensoryDetail,
                confidence: 0.3,
            });
        }
        
        // NEW: Check for no meta-awareness
        let meta_words = ["this conversation", "we're talking", "I notice", "dit gesprek", 
                        "we praten", "ik merk"];
        if !meta_words.iter().any(|w| all_text.contains(w)) {
            blind_spots.push(BlindSpot {
                description: "No reflection on the conversation itself".to_string(),
                category: BlindSpotCategory::NoMetaAwareness,
                confidence: 0.3,
            });
        }
        
        blind_spots
    }
    
    /// Generate horizon items (questions at the edge)
    fn generate_horizon(&self, turns: &[&Turn], blind_spots: &[BlindSpot]) -> Vec<HorizonItem> {
        let mut horizon = Vec::new();
        
        // Generate horizon items from blind spots using the category's built-in question
        for bs in blind_spots {
            let question = bs.category.horizon_question();
            let trigger = match bs.category {
                BlindSpotCategory::EmotionUnexpressed => Some("Ask: How does this feel?"),
                BlindSpotCategory::BodyUnmentioned => Some("Ask: Where do you feel this in your body?"),
                BlindSpotCategory::FutureAbsent => Some("Ask: What happens next?"),
                BlindSpotCategory::PastAbsent => Some("Ask: Has this happened before?"),
                BlindSpotCategory::OthersAbsent => Some("Ask: Who else would care about this?"),
                BlindSpotCategory::ConflictAvoided => Some("Ask: What's the hard part?"),
                BlindSpotCategory::UncertaintyHidden => Some("Ask: What don't you know yet?"),
                BlindSpotCategory::MinimalSelfReference => Some("Explore: Rest in awareness without 'I'"),
                BlindSpotCategory::NoCollectiveIdentity => Some("Ask: How do we experience this together?"),
                BlindSpotCategory::NoHumorPlayfulness => Some("Try: Introduce lightness or play"),
                BlindSpotCategory::SilenceDominant => Some("Explore: Listen to what the silence says"),
                BlindSpotCategory::HighAbstraction => Some("Ask: What's a concrete example?"),
                BlindSpotCategory::NoSensoryDetail => Some("Ask: What do you see, hear, feel?"),
                BlindSpotCategory::NoMetaAwareness => Some("Notice: What is aware of this conversation?"),
            };
            
            horizon.push(HorizonItem {
                question: question.to_string(),
                reason: bs.description.clone(),
                potential_trigger: trigger.map(String::from),
            });
        }
        
        // Add generic horizon items based on content
        if turns.len() < 5 {
            horizon.push(HorizonItem {
                question: "What else wants to be said?".to_string(),
                reason: "Conversation was brief".to_string(),
                potential_trigger: Some("Continue the dialogue".to_string()),
            });
        }
        
        horizon
    }
}

/// Save snapshot to JSON file
pub fn save_snapshot(snapshot: &Snapshot, dir: &str) -> Result<String, SnapshotReason> {
    let filename = format!("{}/{}.json", dir, snapshot.id);
    
    let json = serde_json::to_string_pretty(snapshot)
        .map_err(|_| SnapshotReason::R303_SNAPSHOT_SERIALIZE_ERROR)?;
    
    std::fs::create_dir_all(dir)
        .map_err(|_| SnapshotReason::R304_SNAPSHOT_STORAGE_ERROR)?;
    
    std::fs::write(&filename, json)
        .map_err(|_| SnapshotReason::R304_SNAPSHOT_STORAGE_ERROR)?;
    
    Ok(filename)
}

/// Load snapshot from JSON file
pub fn load_snapshot(path: &str) -> Result<Snapshot, SnapshotReason> {
    let json = std::fs::read_to_string(path)
        .map_err(|_| SnapshotReason::R304_SNAPSHOT_STORAGE_ERROR)?;
    
    serde_json::from_str(&json)
        .map_err(|_| SnapshotReason::R303_SNAPSHOT_SERIALIZE_ERROR)
}

/// Load and validate snapshot (checks proof_hash is not empty)
pub fn load_and_validate_snapshot(path: &str) -> Result<Snapshot, SnapshotReason> {
    let snapshot = load_snapshot(path)?;
    
    // Validate: proof_hash should not be all zeros
    if snapshot.proof_hash == [0u8; 32] {
        return Err(SnapshotReason::R305_SNAPSHOT_INVALID_PROOF_LINK);
    }
    
    // Validate: session_id should not be all zeros
    if snapshot.session_id == [0u8; 16] {
        return Err(SnapshotReason::R305_SNAPSHOT_INVALID_PROOF_LINK);
    }
    
    // Validate: must have at least some content
    if snapshot.turn_count == 0 {
        return Err(SnapshotReason::R302_SNAPSHOT_WINDOW_EMPTY);
    }
    
    Ok(snapshot)
}

/// Validate snapshot against a proof (checks hash matches)
pub fn validate_snapshot_proof(snapshot: &Snapshot, proof: &Proof) -> bool {
    let expected_hash = sha256(&proof.to_bytes());
    snapshot.proof_hash == expected_hash && snapshot.session_id == proof.payload.session_id
}

/// SHA-256 helper
fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Proof, ProofPayload};
    
    fn make_mock_proof() -> Proof {
        let payload = ProofPayload {
            version: 1,
            session_id: [1u8; 16],
            r_final: 0.07,
            dc_final: 0.05,
            lock_duration_secs: 10,
            window_start_unix: 0,
            paired_turn_count: 2,
            conversation_hash: [0u8; 32],
            node_pubkey: [0u8; 32],
            payload_hash: [0u8; 32],
        };
        Proof::new(payload, [0u8; 64])
    }
    
    fn make_window() -> ConversationWindow {
        let mut window = ConversationWindow::new();
        window.add_turn(Turn::new("A", "The sky is peaceful and calm", 0.05));
        window.add_turn(Turn::new("B", "Yes, very peaceful today", 0.05));
        window.add_turn(Turn::new("A", "I feel at peace", 0.06));
        window.add_turn(Turn::new("B", "Stillness everywhere", 0.04));
        window
    }
    
    #[test]
    fn test_snapshot_generation() {
        let gen = SnapshotGenerator::new();
        let proof = make_mock_proof();
        let window = make_window();
        
        let result = gen.generate(&proof, &window, vec!["A".to_string(), "B".to_string()]);
        
        assert!(result.is_success());
        let snapshot = result.snapshot.unwrap();
        
        assert_eq!(snapshot.session_id, [1u8; 16]);
        assert!((snapshot.r_final - 0.07).abs() < 0.001);
        assert_eq!(snapshot.observers.len(), 2);
        assert!(snapshot.turn_count >= 4);
    }
    
    #[test]
    fn test_seen_extraction() {
        let gen = SnapshotGenerator::new();
        let proof = make_mock_proof();
        let window = make_window();
        
        let result = gen.generate(&proof, &window, vec![]);
        let snapshot = result.snapshot.unwrap();
        
        // Should detect peace/calm themes
        assert!(
            snapshot.seen.themes.iter().any(|t| t.contains("peace") || t.contains("Calm")),
            "Should detect peace/calm themes: {:?}",
            snapshot.seen.themes
        );
        
        // Should detect positive emotion
        assert!(
            snapshot.seen.emotion.as_ref().map(|e| e.contains("Positive")).unwrap_or(false),
            "Should detect positive emotion: {:?}",
            snapshot.seen.emotion
        );
    }
    
    #[test]
    fn test_blind_spot_detection() {
        let gen = SnapshotGenerator::new();
        let proof = make_mock_proof();
        
        // Create window WITHOUT body/future/past mentions
        let mut window = ConversationWindow::new();
        window.add_turn(Turn::new("A", "Peace", 0.05));
        window.add_turn(Turn::new("B", "Calm", 0.05));
        
        let result = gen.generate(&proof, &window, vec![]);
        let snapshot = result.snapshot.unwrap();
        
        // Should detect blind spots
        assert!(!snapshot.blind_spots.is_empty(), "Should detect blind spots");
        
        // Should have body blind spot
        assert!(
            snapshot.blind_spots.iter().any(|bs| bs.category == BlindSpotCategory::BodyUnmentioned),
            "Should detect body not mentioned"
        );
    }
    
    #[test]
    fn test_horizon_generation() {
        let gen = SnapshotGenerator::new();
        let proof = make_mock_proof();
        let window = make_window();
        
        let result = gen.generate(&proof, &window, vec![]);
        let snapshot = result.snapshot.unwrap();
        
        // Should have horizon items
        assert!(!snapshot.horizon.is_empty(), "Should generate horizon items");
        
        // Each horizon item should have a question
        for item in &snapshot.horizon {
            assert!(!item.question.is_empty());
        }
    }
    
    #[test]
    fn test_empty_window_fails() {
        let gen = SnapshotGenerator::new();
        let proof = make_mock_proof();
        let window = ConversationWindow::new(); // Empty
        
        let result = gen.generate(&proof, &window, vec![]);
        
        assert!(!result.is_success());
        assert_eq!(result.reason, SnapshotReason::R302_SNAPSHOT_WINDOW_EMPTY);
    }
    
    #[test]
    fn test_snapshot_serialization() {
        let gen = SnapshotGenerator::new();
        let proof = make_mock_proof();
        let window = make_window();
        
        let result = gen.generate(&proof, &window, vec!["A".to_string()]);
        let snapshot = result.snapshot.unwrap();
        
        // Should serialize to JSON
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("seen"));
        assert!(json.contains("blind_spots"));
        assert!(json.contains("horizon"));
        
        // Should deserialize back
        let restored: Snapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, snapshot.id);
    }
    
    #[test]
    fn test_proof_hash_linking() {
        let gen = SnapshotGenerator::new();
        let proof = make_mock_proof();
        let window = make_window();
        
        let result = gen.generate(&proof, &window, vec![]);
        let snapshot = result.snapshot.unwrap();
        
        // Proof hash should not be all zeros
        assert_ne!(snapshot.proof_hash, [0u8; 32], "Proof hash should be computed");
    }
}
