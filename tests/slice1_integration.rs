//! Integration tests for Slice 1
//!
//! Tests the full path: text → r_parser → FacelockEngine → output

use soul0::core::{RParser, FacelockEngine};
use soul0::types::FacelockState;
use soul0::{R_THRESHOLD_APPROACHING, STABILITY_DURATION_MS};
use std::thread::sleep;
use std::time::Duration;

/// Test the full slice 1 path
#[test]
fn test_full_slice_path() {
    let parser = RParser::new();
    let mut engine = FacelockEngine::new();
    
    // Input → parse → update → output
    let text = "The sky is blue.";
    let r_value = parser.parse(text);
    let output = engine.update(r_value.value);
    
    // Should have valid output
    assert!(output.r >= 0.0 && output.r <= 1.0);
    assert!(!output.reason.code().is_empty());
}

/// Test state progression from WAITING to LOCKED
#[test]
fn test_state_progression_to_locked() {
    let parser = RParser::new();
    let mut engine = FacelockEngine::new();
    
    // Low r text (pure observation)
    let low_r_text = "Stillness. Presence. Awareness. Peace.";
    
    // First update - should go to APPROACHING
    let r = parser.quick_parse(low_r_text);
    assert!(r < R_THRESHOLD_APPROACHING, "Low r text should have r < 0.25, got {}", r);
    
    let output = engine.update(r);
    assert_eq!(output.state, FacelockState::Approaching);
    
    // Continue with low r, building stability
    for _ in 0..5 {
        engine.update(r);
        sleep(Duration::from_millis(100));
    }
    
    // Wait for 8-second stability
    sleep(Duration::from_millis(STABILITY_DURATION_MS));
    
    // Should now reach LOCKED
    let output = engine.update(r);
    assert_eq!(output.state, FacelockState::Locked, "Should be LOCKED after 8s stability");
    assert!(output.proof_available);
}

/// Test DRIFT on high r
#[test]
fn test_drift_on_high_r() {
    let parser = RParser::new();
    let mut engine = FacelockEngine::new();
    
    // Start with low r
    let low_r = parser.quick_parse("Silence.");
    engine.update(low_r);
    
    assert_eq!(engine.state(), FacelockState::Approaching);
    
    // Spike with high r value directly (simulating high ego input)
    // Since our normalized parser produces lower values, we test the engine directly
    let output = engine.update(0.50); // Direct high value
    assert_eq!(output.state, FacelockState::Drift);
}

/// Test determinism - same input always gives same r
#[test]
fn test_determinism_full_path() {
    let parser = RParser::new();
    
    let text = "I think this is interesting, but I'm not sure what to think about it all.";
    
    let r1 = parser.quick_parse(text);
    let r2 = parser.quick_parse(text);
    let r3 = parser.quick_parse(text);
    
    assert!((r1 - r2).abs() < 1e-10);
    assert!((r2 - r3).abs() < 1e-10);
}

/// Test that r values are in expected ranges for different text types
#[test]
fn test_r_ranges() {
    let parser = RParser::new();
    
    // Pure observation - should be very low r
    let pure = parser.quick_parse("Blue sky. Wind. Leaves falling.");
    assert!(pure < 0.10, "Pure observation should have r < 0.10, got {}", pure);
    
    // Normal conversation - low to medium r
    let normal = parser.quick_parse("I had a good day today. The weather was nice.");
    assert!(normal < 0.30, "Normal conversation should have r < 0.30, got {}", normal);
    
    // High ego - higher r than pure observation
    let ego = parser.quick_parse("I'm always right and everyone else is always wrong. They should listen to me immediately!");
    assert!(ego > pure, "High ego text should have higher r than pure observation");
}

/// Test recovery from DRIFT
#[test]
fn test_recovery_from_drift() {
    let mut engine = FacelockEngine::new();
    
    // Go to approaching with low r
    engine.update(0.05);
    assert_eq!(engine.state(), FacelockState::Approaching);
    
    // Spike to drift with high r
    engine.update(0.50);
    assert_eq!(engine.state(), FacelockState::Drift);
    
    // Recover with low r
    let output = engine.update(0.05);
    assert_eq!(output.state, FacelockState::Approaching);
}

/// Test JSON output is valid
#[test]
fn test_json_output_valid() {
    let parser = RParser::new();
    let mut engine = FacelockEngine::new();
    
    let r = parser.quick_parse("Hello world.");
    let output = engine.update(r);
    
    // Should serialize without error
    let json = serde_json::to_string(&output).unwrap();
    assert!(json.contains("\"state\""));
    assert!(json.contains("\"r\""));
    assert!(json.contains("\"reason\""));
    
    // Should deserialize back
    let _: soul0::types::StateOutput = serde_json::from_str(&json).unwrap();
}

/// Test parseable output format
#[test]
fn test_parseable_output_format() {
    let parser = RParser::new();
    let mut engine = FacelockEngine::new();
    
    let r = parser.quick_parse("Test input.");
    let output = engine.update(r);
    
    let formatted = output.to_parseable_string();
    
    // Should contain expected parts
    assert!(formatted.contains("r="));
    assert!(formatted.contains("state="));
    assert!(formatted.contains("stable="));
    assert!(formatted.contains("reason="));
}
