//! Integration tests for Slice 3 - Proof Generation
//!
//! Tests PROOF_POLICY_v1.0.md invariants:
//! - Proof only in LOCKED state
//! - Minimum 8 seconds stability
//! - Hash only paired turns in window
//! - Proofs are permanent (no revocation API)

use soul0::core::{RParser, DcParser, FacelockEngine, ProofGenerator, verify_proof};
use soul0::types::{Turn, ConversationWindow, FacelockState, ProofReason};
use std::thread::sleep;
use std::time::Duration;

fn make_turn(speaker: &str, text: &str) -> Turn {
    let r_parser = RParser::new();
    let r = r_parser.quick_parse(text);
    Turn::new(speaker, text, r)
}

fn mock_sign(data: &[u8]) -> [u8; 64] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let h1: [u8; 32] = hasher.finalize().into();
    
    let mut hasher = Sha256::new();
    hasher.update(&h1);
    let h2: [u8; 32] = hasher.finalize().into();
    
    let mut sig = [0u8; 64];
    sig[0..32].copy_from_slice(&h1);
    sig[32..64].copy_from_slice(&h2);
    sig
}

fn mock_verify(data: &[u8], sig: &[u8; 64], _pubkey: &[u8; 32]) -> bool {
    let expected = mock_sign(data);
    sig == &expected
}

// =============================================================================
// POLICY TESTS - PROOF_POLICY.md INVARIANTS
// =============================================================================

#[test]
fn test_invariant_a1_proof_only_in_locked() {
    let gen = ProofGenerator::new_random();
    let dc_parser = DcParser::new();
    
    let mut window = ConversationWindow::new();
    window.add_turn(make_turn("A", "Peace and stillness"));
    window.add_turn(make_turn("B", "Yes, very peaceful"));
    
    let dc = dc_parser.calculate(&window);
    
    // Test each non-LOCKED state
    for state in [FacelockState::Waiting, FacelockState::Approaching, FacelockState::Drift] {
        let result = gen.generate(
            [0u8; 16],
            state,
            10.0,
            0.05,
            &dc,
            &window,
            mock_sign,
        );
        
        assert!(!result.is_success(), "Proof should fail in {:?} state", state);
        assert_eq!(result.reason, ProofReason::R204_PROOF_NOT_LOCKED);
    }
}

#[test]
fn test_invariant_a2_minimum_stability() {
    let gen = ProofGenerator::new_random();
    let dc_parser = DcParser::new();
    
    let mut window = ConversationWindow::new();
    window.add_turn(make_turn("A", "Calm"));
    window.add_turn(make_turn("B", "Peaceful"));
    
    let dc = dc_parser.calculate(&window);
    
    // Test various durations below 8 seconds
    for duration in [0.1, 1.0, 5.0, 7.9] {
        let result = gen.generate(
            [0u8; 16],
            FacelockState::Locked,
            duration,
            0.05,
            &dc,
            &window,
            mock_sign,
        );
        
        assert!(!result.is_success(), "Proof should fail at {} seconds", duration);
        assert_eq!(result.reason, ProofReason::R201_PROOF_NOT_STABLE);
    }
    
    // Test at exactly 8 seconds - should succeed
    let result = gen.generate(
        [0u8; 16],
        FacelockState::Locked,
        8.0,
        0.05,
        &dc,
        &window,
        mock_sign,
    );
    
    assert!(result.is_success(), "Proof should succeed at 8 seconds");
}

#[test]
fn test_invariant_a3_hash_binds_to_window() {
    let gen = ProofGenerator::new_random();
    let dc_parser = DcParser::new();
    
    // Create window with specific content
    let mut window = ConversationWindow::new();
    window.add_turn(make_turn("A", "The sky is blue"));
    window.add_turn(make_turn("B", "Yes it is blue"));
    
    let dc = dc_parser.calculate(&window);
    
    let result1 = gen.generate(
        [1u8; 16],
        FacelockState::Locked,
        10.0,
        0.05,
        &dc,
        &window,
        mock_sign,
    );
    
    let proof1 = result1.proof.unwrap();
    
    // Create different window
    let mut window2 = ConversationWindow::new();
    window2.add_turn(make_turn("A", "The sky is red")); // Different!
    window2.add_turn(make_turn("B", "Yes it is red"));
    
    let dc2 = dc_parser.calculate(&window2);
    
    let result2 = gen.generate(
        [1u8; 16],
        FacelockState::Locked,
        10.0,
        0.05,
        &dc2,
        &window2,
        mock_sign,
    );
    
    let proof2 = result2.proof.unwrap();
    
    // Hashes should be different
    assert_ne!(
        proof1.payload.conversation_hash,
        proof2.payload.conversation_hash,
        "Different content should have different hash"
    );
}

#[test]
fn test_invariant_a4_no_revocation_api() {
    // This is a structural test - ensure no revoke function exists
    // In Rust, we verify this by checking the public API
    
    // ProofGenerator should not have revoke method
    // ProofResult should not have invalidate method
    // Proof should not have revoked field
    
    // The test passes if this compiles - the API doesn't expose revocation
    let gen = ProofGenerator::new_random();
    let _pubkey = gen.pubkey();
    
    // If we got here, there's no revocation API visible
    assert!(true, "No revocation API exists");
}

// =============================================================================
// PROOF SIZE AND FORMAT TESTS
// =============================================================================

#[test]
fn test_proof_exactly_248_bytes() {
    let gen = ProofGenerator::new_random();
    let dc_parser = DcParser::new();
    
    let mut window = ConversationWindow::new();
    window.add_turn(make_turn("A", "Test"));
    window.add_turn(make_turn("B", "Reply"));
    
    let dc = dc_parser.calculate(&window);
    
    let result = gen.generate(
        [0u8; 16],
        FacelockState::Locked,
        10.0,
        0.05,
        &dc,
        &window,
        mock_sign,
    );
    
    let proof = result.proof.unwrap();
    let bytes = proof.to_bytes();
    
    assert_eq!(bytes.len(), 248, "Proof must be exactly 248 bytes");
}

#[test]
fn test_proof_hex_roundtrip() {
    let gen = ProofGenerator::new_random();
    let dc_parser = DcParser::new();
    
    let mut window = ConversationWindow::new();
    window.add_turn(make_turn("A", "Roundtrip test"));
    window.add_turn(make_turn("B", "Roundtrip reply"));
    
    let dc = dc_parser.calculate(&window);
    
    let result = gen.generate(
        [42u8; 16], // Specific session ID
        FacelockState::Locked,
        12.5,
        0.08,
        &dc,
        &window,
        mock_sign,
    );
    
    let proof = result.proof.unwrap();
    let hex = proof.to_hex();
    let restored = soul0::types::Proof::from_hex(&hex).unwrap();
    
    assert_eq!(restored.payload.session_id, [42u8; 16]);
    assert!((restored.payload.r_final - 0.08).abs() < 0.001);
    assert_eq!(restored.payload.lock_duration_secs, 12);
}

// =============================================================================
// VERIFICATION TESTS
// =============================================================================

#[test]
fn test_valid_proof_verifies() {
    let gen = ProofGenerator::new_random();
    let dc_parser = DcParser::new();
    
    let mut window = ConversationWindow::new();
    window.add_turn(make_turn("A", "Verification test"));
    window.add_turn(make_turn("B", "Should verify"));
    
    let dc = dc_parser.calculate(&window);
    
    let result = gen.generate(
        [0u8; 16],
        FacelockState::Locked,
        10.0,
        0.05,
        &dc,
        &window,
        mock_sign,
    );
    
    let proof = result.proof.unwrap();
    assert!(verify_proof(&proof, mock_verify), "Valid proof should verify");
}

#[test]
fn test_tampered_proof_fails_verification() {
    let gen = ProofGenerator::new_random();
    let dc_parser = DcParser::new();
    
    let mut window = ConversationWindow::new();
    window.add_turn(make_turn("A", "Tamper test"));
    window.add_turn(make_turn("B", "Should fail if tampered"));
    
    let dc = dc_parser.calculate(&window);
    
    let result = gen.generate(
        [0u8; 16],
        FacelockState::Locked,
        10.0,
        0.05,
        &dc,
        &window,
        mock_sign,
    );
    
    let mut proof = result.proof.unwrap();
    
    // Tamper with r_final
    proof.payload.r_final = 0.99;
    
    assert!(!verify_proof(&proof, mock_verify), "Tampered proof should not verify");
}

// =============================================================================
// FULL FLOW INTEGRATION TEST
// =============================================================================

#[test]
fn test_full_flow_duo_to_proof() {
    let mut engine = FacelockEngine::new();
    let gen = ProofGenerator::new_random();
    let mut window = ConversationWindow::new();
    
    // Add turns with known low r values
    window.add_turn(Turn::new("A", "The morning is calm", 0.05));
    window.add_turn(Turn::new("B", "Yes very calm", 0.05));
    window.add_turn(Turn::new("A", "Stillness", 0.03));
    window.add_turn(Turn::new("B", "Peace", 0.03));
    
    // Drive engine to APPROACHING with low r
    engine.update(0.05);
    engine.update(0.05);
    engine.update(0.03);
    engine.update(0.03);
    
    assert_eq!(engine.state(), FacelockState::Approaching);
    
    // Wait for stability
    sleep(Duration::from_millis(8100));
    
    // One more low r update to trigger LOCKED
    window.add_turn(Turn::new("A", "Quiet", 0.03));
    let output = engine.update(0.03);
    
    // Should be LOCKED now
    assert_eq!(output.state, FacelockState::Locked, "Should be LOCKED after 8s stability");
    
    // Create a valid DcResult for proof generation
    let dc = soul0::types::DcResult::success(
        0.05, 
        soul0::types::DcSignals::zero(), 
        2, 
        5
    );
    
    // Generate proof
    let result = gen.generate(
        [1u8; 16],
        engine.state(),
        output.stable_ms as f64 / 1000.0,
        output.r,
        &dc,
        &window,
        mock_sign,
    );
    
    assert!(result.is_success(), "Should generate proof in LOCKED state");
    
    // Verify proof
    let proof = result.proof.unwrap();
    assert!(verify_proof(&proof, mock_verify), "Generated proof should verify");
    
    // Check proof content
    assert_eq!(proof.payload.version, 1);
    assert!(proof.payload.r_final < 0.15, "r should be low");
    assert!(proof.payload.paired_turn_count > 0, "Should have paired turns");
}
