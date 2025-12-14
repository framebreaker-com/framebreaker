//! Proof generation and verification
//!
//! Based on PROOF_POLICY_v1.0.md:
//! - Proof only in LOCKED state
//! - Minimum 8 seconds stability
//! - Hash only paired turns in window
//! - Ed25519 signature

use sha2::{Sha256, Digest};
use crate::LOCKED_MIN_DURATION_SECS;
use crate::types::{
    FacelockState, TurnPair, ConversationWindow,
    DcResult, Proof, ProofPayload, ProofResult, ProofReason,
};

/// Proof generator
#[derive(Debug)]
pub struct ProofGenerator {
    /// Node's keypair (simplified - in production use proper key management)
    node_pubkey: [u8; 32],
}

impl ProofGenerator {
    /// Create new generator with a public key
    pub fn new(node_pubkey: [u8; 32]) -> Self {
        Self { node_pubkey }
    }
    
    /// Create generator with random key (for testing)
    pub fn new_random() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let mut key = [0u8; 32];
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        key[0..16].copy_from_slice(&nanos.to_le_bytes());
        key[16..32].copy_from_slice(&nanos.to_be_bytes());
        Self { node_pubkey: key }
    }
    
    /// Check if proof can be generated (policy check only)
    pub fn can_generate(
        &self,
        state: FacelockState,
        locked_duration_secs: f64,
        dc_result: &DcResult,
        window: &ConversationWindow,
    ) -> Result<(), ProofReason> {
        // P1: Must be in LOCKED state
        if state != FacelockState::Locked {
            return Err(ProofReason::R204_PROOF_NOT_LOCKED);
        }
        
        // P1: Must be stable for minimum duration
        if locked_duration_secs < LOCKED_MIN_DURATION_SECS {
            return Err(ProofReason::R201_PROOF_NOT_STABLE);
        }
        
        // P2: ΔC must be known
        if !dc_result.is_known() {
            return Err(ProofReason::R202_PROOF_DC_UNKNOWN);
        }
        
        // P2: Must have paired turns
        if window.paired_turns().is_empty() {
            return Err(ProofReason::R203_PROOF_WINDOW_EMPTY);
        }
        
        Ok(())
    }
    
    /// Generate a proof
    pub fn generate(
        &self,
        session_id: [u8; 16],
        state: FacelockState,
        locked_duration_secs: f64,
        r_final: f64,
        dc_result: &DcResult,
        window: &ConversationWindow,
        sign_fn: impl Fn(&[u8]) -> [u8; 64],
    ) -> ProofResult {
        // Check policy
        if let Err(reason) = self.can_generate(state, locked_duration_secs, dc_result, window) {
            return ProofResult::failure(reason);
        }
        
        let pairs = window.paired_turns();
        let dc_final = dc_result.value.unwrap_or(0.0);
        
        // Calculate conversation hash (only paired turns in window)
        let conversation_hash = hash_paired_turns(&pairs);
        
        // Get window start time
        let window_start_unix = pairs.first()
            .and_then(|p| p.first.timestamp)
            .map(|_| chrono::Utc::now().timestamp()) // Simplified
            .unwrap_or(0);
        
        // Build payload (without final hash)
        let mut payload = ProofPayload {
            version: 1,
            session_id,
            r_final,
            dc_final,
            lock_duration_secs: locked_duration_secs as u64,
            window_start_unix,
            paired_turn_count: pairs.len() as u32,
            conversation_hash,
            node_pubkey: self.node_pubkey,
            payload_hash: [0u8; 32], // Will be filled
        };
        
        // Calculate payload hash (excluding the hash field itself)
        // Layout: version(2) + session_id(16) + r(8) + dc(8) + lock_dur(8) + window_start(8) + pairs(4) + conv_hash(32) + pubkey(32) = 118
        let partial_bytes = payload.to_bytes();
        let payload_hash = sha256(&partial_bytes[0..118]);
        payload.payload_hash = payload_hash;
        
        // Sign the complete payload
        let payload_bytes = payload.to_bytes();
        let signature = sign_fn(&payload_bytes);
        
        // Create proof
        let proof = Proof::new(payload, signature);
        
        ProofResult::success(proof)
    }
    
    /// Get node public key
    pub fn pubkey(&self) -> &[u8; 32] {
        &self.node_pubkey
    }
}

/// Verify a proof
pub fn verify_proof(
    proof: &Proof,
    verify_fn: impl Fn(&[u8], &[u8; 64], &[u8; 32]) -> bool,
) -> bool {
    let payload_bytes = proof.payload.to_bytes();
    
    // Verify signature over complete payload
    if !verify_fn(&payload_bytes, &proof.signature, &proof.payload.node_pubkey) {
        return false;
    }
    
    // Verify payload hash (hash of bytes 0..118, excluding the hash field itself)
    // Layout: version(2) + session_id(16) + r(8) + dc(8) + lock_dur(8) + window_start(8) + pairs(4) + conv_hash(32) + pubkey(32) = 118
    let calculated_hash = sha256(&payload_bytes[0..118]);
    if calculated_hash != proof.payload.payload_hash {
        return false;
    }
    
    true
}

/// Hash paired turns (deterministic)
pub fn hash_paired_turns(pairs: &[TurnPair]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    
    for pair in pairs {
        // Hash first turn
        hasher.update(pair.first.speaker.as_bytes());
        hasher.update([0u8]); // Separator
        hasher.update(pair.first.text.as_bytes());
        hasher.update([0u8]);
        
        // Hash second turn
        hasher.update(pair.second.speaker.as_bytes());
        hasher.update([0u8]);
        hasher.update(pair.second.text.as_bytes());
        hasher.update([0u8]);
    }
    
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
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
    use crate::types::{Turn, DcSignals, DcReason};
    
    fn mock_sign(data: &[u8]) -> [u8; 64] {
        // Simple mock: hash the data twice
        let h1 = sha256(data);
        let h2 = sha256(&h1);
        let mut sig = [0u8; 64];
        sig[0..32].copy_from_slice(&h1);
        sig[32..64].copy_from_slice(&h2);
        sig
    }
    
    fn mock_verify(data: &[u8], sig: &[u8; 64], _pubkey: &[u8; 32]) -> bool {
        let expected = mock_sign(data);
        sig == &expected
    }
    
    fn make_window_with_pairs() -> ConversationWindow {
        let mut window = ConversationWindow::new();
        window.add_turn(Turn::new("A", "Hello", 0.1));
        window.add_turn(Turn::new("B", "Hi there", 0.1));
        window.add_turn(Turn::new("A", "How are you", 0.1));
        window.add_turn(Turn::new("B", "Good thanks", 0.1));
        window
    }
    
    fn make_dc_result(value: f64) -> DcResult {
        DcResult::success(value, DcSignals::zero(), 2, 2)
    }
    
    #[test]
    fn test_proof_denied_not_locked() {
        let gen = ProofGenerator::new_random();
        let window = make_window_with_pairs();
        let dc = make_dc_result(0.05);
        
        let result = gen.generate(
            [0u8; 16],
            FacelockState::Approaching, // Not locked!
            10.0,
            0.05,
            &dc,
            &window,
            mock_sign,
        );
        
        assert!(!result.is_success());
        assert_eq!(result.reason, ProofReason::R204_PROOF_NOT_LOCKED);
    }
    
    #[test]
    fn test_proof_denied_not_stable() {
        let gen = ProofGenerator::new_random();
        let window = make_window_with_pairs();
        let dc = make_dc_result(0.05);
        
        let result = gen.generate(
            [0u8; 16],
            FacelockState::Locked,
            5.0, // Only 5 seconds, need 8
            0.05,
            &dc,
            &window,
            mock_sign,
        );
        
        assert!(!result.is_success());
        assert_eq!(result.reason, ProofReason::R201_PROOF_NOT_STABLE);
    }
    
    #[test]
    fn test_proof_denied_dc_unknown() {
        let gen = ProofGenerator::new_random();
        let window = make_window_with_pairs();
        let dc = DcResult::unknown(DcReason::R011_DC_UNKNOWN_SINGLE_SPEAKER);
        
        let result = gen.generate(
            [0u8; 16],
            FacelockState::Locked,
            10.0,
            0.05,
            &dc,
            &window,
            mock_sign,
        );
        
        assert!(!result.is_success());
        assert_eq!(result.reason, ProofReason::R202_PROOF_DC_UNKNOWN);
    }
    
    #[test]
    fn test_proof_denied_empty_window() {
        let gen = ProofGenerator::new_random();
        let window = ConversationWindow::new(); // Empty
        let dc = make_dc_result(0.05);
        
        let result = gen.generate(
            [0u8; 16],
            FacelockState::Locked,
            10.0,
            0.05,
            &dc,
            &window,
            mock_sign,
        );
        
        assert!(!result.is_success());
        assert_eq!(result.reason, ProofReason::R203_PROOF_WINDOW_EMPTY);
    }
    
    #[test]
    fn test_proof_success() {
        let gen = ProofGenerator::new_random();
        let window = make_window_with_pairs();
        let dc = make_dc_result(0.05);
        
        let result = gen.generate(
            [1u8; 16], // Session ID
            FacelockState::Locked,
            10.0, // 10 seconds stable
            0.07, // Final r
            &dc,
            &window,
            mock_sign,
        );
        
        assert!(result.is_success());
        assert_eq!(result.reason, ProofReason::R200_PROOF_GENERATED);
        
        let proof = result.proof.unwrap();
        assert_eq!(proof.payload.version, 1);
        assert_eq!(proof.payload.session_id, [1u8; 16]);
        assert!((proof.payload.r_final - 0.07).abs() < 0.001);
        // 4 turns A-B-A-B produces 3 consecutive pairs (A-B, B-A, A-B)
        assert_eq!(proof.payload.paired_turn_count, 3);
    }
    
    #[test]
    fn test_proof_serialization() {
        let gen = ProofGenerator::new_random();
        let window = make_window_with_pairs();
        let dc = make_dc_result(0.05);
        
        let result = gen.generate(
            [1u8; 16],
            FacelockState::Locked,
            10.0,
            0.07,
            &dc,
            &window,
            mock_sign,
        );
        
        let proof = result.proof.unwrap();
        
        // Serialize
        let bytes = proof.to_bytes();
        assert_eq!(bytes.len(), 248);
        
        // Deserialize
        let restored = Proof::from_bytes(&bytes);
        assert_eq!(restored.payload.version, proof.payload.version);
        assert_eq!(restored.payload.session_id, proof.payload.session_id);
        assert_eq!(restored.signature, proof.signature);
    }
    
    #[test]
    fn test_proof_hex() {
        let gen = ProofGenerator::new_random();
        let window = make_window_with_pairs();
        let dc = make_dc_result(0.05);
        
        let result = gen.generate(
            [1u8; 16],
            FacelockState::Locked,
            10.0,
            0.07,
            &dc,
            &window,
            mock_sign,
        );
        
        let proof = result.proof.unwrap();
        let hex = proof.to_hex();
        
        assert_eq!(hex.len(), 496); // 248 bytes * 2 chars per byte
        
        let restored = Proof::from_hex(&hex).unwrap();
        assert_eq!(restored.payload.session_id, proof.payload.session_id);
    }
    
    #[test]
    fn test_proof_verification() {
        let gen = ProofGenerator::new_random();
        let window = make_window_with_pairs();
        let dc = make_dc_result(0.05);
        
        let result = gen.generate(
            [1u8; 16],
            FacelockState::Locked,
            10.0,
            0.07,
            &dc,
            &window,
            mock_sign,
        );
        
        assert!(result.is_success(), "Proof generation should succeed");
        let proof = result.proof.unwrap();
        
        // Verify the signature was created correctly
        let payload_bytes = proof.payload.to_bytes();
        let expected_sig = mock_sign(&payload_bytes);
        assert_eq!(proof.signature, expected_sig, "Signature should match");
        
        // Verify using verify_proof function
        assert!(verify_proof(&proof, mock_verify), "Proof should verify");
    }
    
    #[test]
    fn test_proof_tamper_detection() {
        let gen = ProofGenerator::new_random();
        let window = make_window_with_pairs();
        let dc = make_dc_result(0.05);
        
        let result = gen.generate(
            [1u8; 16],
            FacelockState::Locked,
            10.0,
            0.07,
            &dc,
            &window,
            mock_sign,
        );
        
        let mut proof = result.proof.unwrap();
        
        // Tamper with payload
        proof.payload.r_final = 0.99;
        
        // Should not verify
        assert!(!verify_proof(&proof, mock_verify));
    }
    
    #[test]
    fn test_hash_determinism() {
        let window = make_window_with_pairs();
        let pairs = window.paired_turns();
        
        let hash1 = hash_paired_turns(&pairs);
        let hash2 = hash_paired_turns(&pairs);
        
        assert_eq!(hash1, hash2, "Hash should be deterministic");
    }
    
    #[test]
    fn test_hash_different_content() {
        let mut window1 = ConversationWindow::new();
        window1.add_turn(Turn::new("A", "Hello", 0.1));
        window1.add_turn(Turn::new("B", "Hi", 0.1));
        
        let mut window2 = ConversationWindow::new();
        window2.add_turn(Turn::new("A", "Hello", 0.1));
        window2.add_turn(Turn::new("B", "Hey", 0.1)); // Different!
        
        let hash1 = hash_paired_turns(&window1.paired_turns());
        let hash2 = hash_paired_turns(&window2.paired_turns());
        
        assert_ne!(hash1, hash2, "Different content should have different hash");
    }
}
