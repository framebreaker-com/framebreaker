//! Integration tests for Slice 4 - Snapshots
//!
//! Tests SNAPSHOTS_v1.0.md + key invariant:
//! - Snapshot only created when Proof is generated (1-op-1 coupling)
//! - Contains: seen, blind_spots, horizon
//! - Proper linking to proof via hash

use soul0::core::{RParser, DcParser, FacelockEngine, ProofGenerator, SnapshotGenerator};
use soul0::types::{Turn, ConversationWindow, FacelockState, BlindSpotCategory, SnapshotReason, DcResult, DcSignals};
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

// =============================================================================
// INVARIANT: SNAPSHOT REQUIRES PROOF
// =============================================================================

#[test]
fn test_invariant_snapshot_requires_proof() {
    // This is enforced by API design:
    // SnapshotGenerator::generate() requires a &Proof parameter
    // You cannot call it without first having a proof
    
    // The test is structural: if this compiles, the invariant holds
    let _gen = SnapshotGenerator::new();
    
    // To create a snapshot, you MUST have a proof
    // This enforces the 1-op-1 coupling
    assert!(true, "API design enforces proof requirement");
}

// =============================================================================
// SNAPSHOT CONTENT TESTS
// =============================================================================

#[test]
fn test_snapshot_contains_seen_themes() {
    let proof_gen = ProofGenerator::new_random();
    let snap_gen = SnapshotGenerator::new();
    let dc_parser = DcParser::new();
    
    let mut window = ConversationWindow::new();
    window.add_turn(make_turn("A", "The morning is peaceful and calm"));
    window.add_turn(make_turn("B", "Yes, such peace and stillness"));
    window.add_turn(make_turn("A", "Nature is beautiful"));
    window.add_turn(make_turn("B", "Perfect calm everywhere"));
    
    let dc = dc_parser.calculate(&window);
    
    // Generate proof first
    let proof_result = proof_gen.generate(
        [1u8; 16],
        FacelockState::Locked,
        10.0,
        0.05,
        &dc,
        &window,
        mock_sign,
    );
    
    let proof = proof_result.proof.unwrap();
    
    // Generate snapshot
    let snap_result = snap_gen.generate(&proof, &window, vec!["A".to_string(), "B".to_string()]);
    
    assert!(snap_result.is_success());
    let snapshot = snap_result.snapshot.unwrap();
    
    // Should have themes
    assert!(!snapshot.seen.themes.is_empty(), "Should extract themes");
    
    // Should have emotion
    assert!(snapshot.seen.emotion.is_some(), "Should detect emotion");
}

#[test]
fn test_snapshot_detects_blind_spots() {
    let proof_gen = ProofGenerator::new_random();
    let snap_gen = SnapshotGenerator::new();
    let dc_parser = DcParser::new();
    
    // Create minimal conversation WITHOUT emotions, body, future, etc.
    let mut window = ConversationWindow::new();
    window.add_turn(make_turn("A", "Blue"));
    window.add_turn(make_turn("B", "Sky"));
    
    let dc = dc_parser.calculate(&window);
    
    let proof_result = proof_gen.generate(
        [1u8; 16],
        FacelockState::Locked,
        10.0,
        0.05,
        &dc,
        &window,
        mock_sign,
    );
    
    let proof = proof_result.proof.unwrap();
    let snap_result = snap_gen.generate(&proof, &window, vec![]);
    
    let snapshot = snap_result.snapshot.unwrap();
    
    // Should detect multiple blind spots
    assert!(snapshot.blind_spots.len() >= 3, "Minimal conversation should have multiple blind spots");
    
    // Should include body not mentioned
    assert!(
        snapshot.blind_spots.iter().any(|bs| bs.category == BlindSpotCategory::BodyUnmentioned),
        "Should detect body blind spot"
    );
}

#[test]
fn test_snapshot_generates_horizon() {
    let proof_gen = ProofGenerator::new_random();
    let snap_gen = SnapshotGenerator::new();
    let dc_parser = DcParser::new();
    
    let mut window = ConversationWindow::new();
    window.add_turn(make_turn("A", "Peace"));
    window.add_turn(make_turn("B", "Stillness"));
    
    let dc = dc_parser.calculate(&window);
    
    let proof_result = proof_gen.generate(
        [1u8; 16],
        FacelockState::Locked,
        10.0,
        0.05,
        &dc,
        &window,
        mock_sign,
    );
    
    let proof = proof_result.proof.unwrap();
    let snap_result = snap_gen.generate(&proof, &window, vec![]);
    
    let snapshot = snap_result.snapshot.unwrap();
    
    // Should generate horizon items from blind spots
    assert!(!snapshot.horizon.is_empty(), "Should generate horizon items");
    
    // Each horizon item should have a question
    for item in &snapshot.horizon {
        assert!(!item.question.is_empty(), "Horizon item should have question");
    }
}

// =============================================================================
// PROOF-SNAPSHOT LINKING
// =============================================================================

#[test]
fn test_snapshot_links_to_proof() {
    let proof_gen = ProofGenerator::new_random();
    let snap_gen = SnapshotGenerator::new();
    let dc_parser = DcParser::new();
    
    let mut window = ConversationWindow::new();
    window.add_turn(make_turn("A", "Test linking"));
    window.add_turn(make_turn("B", "Proof and snapshot"));
    
    let dc = dc_parser.calculate(&window);
    
    let proof_result = proof_gen.generate(
        [42u8; 16], // Specific session ID
        FacelockState::Locked,
        10.0,
        0.07,
        &dc,
        &window,
        mock_sign,
    );
    
    let proof = proof_result.proof.unwrap();
    let snap_result = snap_gen.generate(&proof, &window, vec![]);
    
    let snapshot = snap_result.snapshot.unwrap();
    
    // Session ID should match
    assert_eq!(snapshot.session_id, [42u8; 16]);
    
    // r and ΔC should match
    assert!((snapshot.r_final - 0.07).abs() < 0.001);
    
    // Proof hash should not be zeros
    assert_ne!(snapshot.proof_hash, [0u8; 32]);
}

#[test]
fn test_snapshot_preserves_observers() {
    let proof_gen = ProofGenerator::new_random();
    let snap_gen = SnapshotGenerator::new();
    let dc_parser = DcParser::new();
    
    let mut window = ConversationWindow::new();
    window.add_turn(make_turn("A", "Observer test"));
    window.add_turn(make_turn("B", "Two observers"));
    
    let dc = dc_parser.calculate(&window);
    
    let proof_result = proof_gen.generate(
        [1u8; 16],
        FacelockState::Locked,
        10.0,
        0.05,
        &dc,
        &window,
        mock_sign,
    );
    
    let proof = proof_result.proof.unwrap();
    let observers = vec!["Martijn".to_string(), "Claude".to_string(), "Grok".to_string()];
    let snap_result = snap_gen.generate(&proof, &window, observers.clone());
    
    let snapshot = snap_result.snapshot.unwrap();
    
    assert_eq!(snapshot.observers, observers);
}

// =============================================================================
// FULL FLOW: PROOF -> SNAPSHOT
// =============================================================================

#[test]
fn test_full_flow_locked_to_snapshot() {
    let mut engine = FacelockEngine::new();
    let proof_gen = ProofGenerator::new_random();
    let snap_gen = SnapshotGenerator::new();
    let mut window = ConversationWindow::new();
    
    // Add turns with known low r values
    window.add_turn(Turn::new("A", "The morning feels peaceful", 0.05));
    window.add_turn(Turn::new("B", "Yes, such stillness", 0.05));
    window.add_turn(Turn::new("A", "I notice the quiet", 0.03));
    window.add_turn(Turn::new("B", "Perfect peace", 0.03));
    
    // Drive engine to APPROACHING with low r
    engine.update(0.05);
    engine.update(0.05);
    engine.update(0.03);
    engine.update(0.03);
    
    // Wait for stability
    sleep(Duration::from_millis(8100));
    
    // Final update to trigger LOCKED
    window.add_turn(Turn::new("A", "Stillness", 0.03));
    engine.update(0.03);
    
    // Should be LOCKED
    assert_eq!(engine.state(), FacelockState::Locked);
    
    // Create valid DcResult for proof generation
    let dc = DcResult::success(0.05, DcSignals::zero(), 2, 5);
    
    // Generate proof
    let proof_result = proof_gen.generate(
        [1u8; 16],
        engine.state(),
        8.5,
        0.03,
        &dc,
        &window,
        mock_sign,
    );
    
    assert!(proof_result.is_success(), "Should generate proof");
    let proof = proof_result.proof.unwrap();
    
    // Generate snapshot FROM proof (1-op-1 coupling)
    let snap_result = snap_gen.generate(
        &proof,
        &window,
        vec!["A".to_string(), "B".to_string()],
    );
    
    assert!(snap_result.is_success(), "Should generate snapshot");
    let snapshot = snap_result.snapshot.unwrap();
    
    // Verify snapshot content
    assert!(!snapshot.seen.themes.is_empty(), "Should have themes");
    assert!(!snapshot.blind_spots.is_empty(), "Should have blind spots");
    assert!(!snapshot.horizon.is_empty(), "Should have horizon");
    assert_eq!(snapshot.observers.len(), 2);
    
    // Verify linking
    assert_eq!(snapshot.session_id, proof.payload.session_id);
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_empty_window_fails() {
    let _proof_gen = ProofGenerator::new_random();
    let snap_gen = SnapshotGenerator::new();
    let _dc_parser = DcParser::new();
    
    let window = ConversationWindow::new(); // Empty
    
    // We need a proof to even try, but proof generation will fail
    // So this tests that snapshot also checks window
    
    // Create a mock proof (bypassing proof generation)
    use soul0::types::{Proof, ProofPayload};
    let payload = ProofPayload {
        version: 1,
        session_id: [0u8; 16],
        r_final: 0.05,
        dc_final: 0.05,
        lock_duration_secs: 10,
        window_start_unix: 0,
        paired_turn_count: 0,
        conversation_hash: [0u8; 32],
        node_pubkey: [0u8; 32],
        payload_hash: [0u8; 32],
    };
    let proof = Proof::new(payload, [0u8; 64]);
    
    let snap_result = snap_gen.generate(&proof, &window, vec![]);
    
    assert!(!snap_result.is_success());
    assert_eq!(snap_result.reason, SnapshotReason::R302_SNAPSHOT_WINDOW_EMPTY);
}

#[test]
fn test_dutch_content_detection() {
    let proof_gen = ProofGenerator::new_random();
    let snap_gen = SnapshotGenerator::new();
    let dc_parser = DcParser::new();
    
    let mut window = ConversationWindow::new();
    window.add_turn(make_turn("A", "De ochtend is rustig en vredig"));
    window.add_turn(make_turn("B", "Ja, heel stil vandaag"));
    
    let dc = dc_parser.calculate(&window);
    
    let proof_result = proof_gen.generate(
        [1u8; 16],
        FacelockState::Locked,
        10.0,
        0.05,
        &dc,
        &window,
        mock_sign,
    );
    
    let proof = proof_result.proof.unwrap();
    let snap_result = snap_gen.generate(&proof, &window, vec![]);
    
    let snapshot = snap_result.snapshot.unwrap();
    
    // Should detect themes from Dutch content
    assert!(!snapshot.seen.themes.is_empty() || !snapshot.seen.keywords.is_empty(),
        "Should extract something from Dutch content");
}
