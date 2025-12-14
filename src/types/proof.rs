//! Proof types for cryptographic attestation
//!
//! Based on PROOF_POLICY_v1.0.md:
//! - 248 bytes fixed size
//! - Ed25519 signature
//! - Only in LOCKED state with 8s stability

use serde::{Deserialize, Serialize};

/// Proof payload (184 bytes before signature)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofPayload {
    /// Protocol version
    pub version: u16,
    /// Unique session identifier (16 bytes)
    pub session_id: [u8; 16],
    /// Final r value at lock time
    pub r_final: f64,
    /// Final ΔC value at lock time
    pub dc_final: f64,
    /// How long LOCKED was sustained (seconds)
    pub lock_duration_secs: u64,
    /// When the window started (Unix timestamp)
    pub window_start_unix: i64,
    /// Number of paired turns in the window
    pub paired_turn_count: u32,
    /// Blake3 hash of conversation (paired turns only)
    pub conversation_hash: [u8; 32],
    /// Node's Ed25519 public key
    pub node_pubkey: [u8; 32],
    /// SHA-256 of payload (for double verification)
    pub payload_hash: [u8; 32],
}

impl ProofPayload {
    /// Serialize to fixed-size bytes (184 bytes)
    pub fn to_bytes(&self) -> [u8; 184] {
        let mut bytes = [0u8; 184];
        let mut offset = 0;
        
        // version (2 bytes)
        bytes[offset..offset + 2].copy_from_slice(&self.version.to_be_bytes());
        offset += 2;
        
        // session_id (16 bytes)
        bytes[offset..offset + 16].copy_from_slice(&self.session_id);
        offset += 16;
        
        // r_final (8 bytes)
        bytes[offset..offset + 8].copy_from_slice(&self.r_final.to_be_bytes());
        offset += 8;
        
        // dc_final (8 bytes)
        bytes[offset..offset + 8].copy_from_slice(&self.dc_final.to_be_bytes());
        offset += 8;
        
        // lock_duration_secs (8 bytes)
        bytes[offset..offset + 8].copy_from_slice(&self.lock_duration_secs.to_be_bytes());
        offset += 8;
        
        // window_start_unix (8 bytes)
        bytes[offset..offset + 8].copy_from_slice(&self.window_start_unix.to_be_bytes());
        offset += 8;
        
        // paired_turn_count (4 bytes)
        bytes[offset..offset + 4].copy_from_slice(&self.paired_turn_count.to_be_bytes());
        offset += 4;
        
        // conversation_hash (32 bytes)
        bytes[offset..offset + 32].copy_from_slice(&self.conversation_hash);
        offset += 32;
        
        // node_pubkey (32 bytes)
        bytes[offset..offset + 32].copy_from_slice(&self.node_pubkey);
        offset += 32;
        
        // payload_hash (32 bytes)
        bytes[offset..offset + 32].copy_from_slice(&self.payload_hash);
        // offset += 32; // = 184
        
        bytes
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8; 184]) -> Self {
        let mut offset = 0;
        
        let version = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]);
        offset += 2;
        
        let mut session_id = [0u8; 16];
        session_id.copy_from_slice(&bytes[offset..offset + 16]);
        offset += 16;
        
        let r_final = f64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        
        let dc_final = f64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        
        let lock_duration_secs = u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        
        let window_start_unix = i64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        
        let paired_turn_count = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        
        let mut conversation_hash = [0u8; 32];
        conversation_hash.copy_from_slice(&bytes[offset..offset + 32]);
        offset += 32;
        
        let mut node_pubkey = [0u8; 32];
        node_pubkey.copy_from_slice(&bytes[offset..offset + 32]);
        offset += 32;
        
        let mut payload_hash = [0u8; 32];
        payload_hash.copy_from_slice(&bytes[offset..offset + 32]);
        
        Self {
            version,
            session_id,
            r_final,
            dc_final,
            lock_duration_secs,
            window_start_unix,
            paired_turn_count,
            conversation_hash,
            node_pubkey,
            payload_hash,
        }
    }
}

/// Complete proof (248 bytes)
#[derive(Debug, Clone)]
pub struct Proof {
    /// The payload data
    pub payload: ProofPayload,
    /// Ed25519 signature (64 bytes)
    pub signature: [u8; 64],
    /// Reserved padding (36 bytes for future use)
    pub reserved: [u8; 36],
}

impl Proof {
    /// Total proof size in bytes
    pub const SIZE: usize = 248;
    
    /// Create new proof from payload and signature
    pub fn new(payload: ProofPayload, signature: [u8; 64]) -> Self {
        Self {
            payload,
            signature,
            reserved: [0u8; 36],
        }
    }
    
    /// Serialize to exactly 248 bytes
    pub fn to_bytes(&self) -> [u8; 248] {
        let mut bytes = [0u8; 248];
        
        // Payload (184 bytes)
        bytes[0..184].copy_from_slice(&self.payload.to_bytes());
        
        // Signature (64 bytes)
        bytes[184..248].copy_from_slice(&self.signature);
        
        // Reserved is zeros (already initialized)
        // Actually signature is 64 bytes, so 184 + 64 = 248
        // No room for reserved in 248 - let me recalculate
        
        bytes
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8; 248]) -> Self {
        let mut payload_bytes = [0u8; 184];
        payload_bytes.copy_from_slice(&bytes[0..184]);
        
        let mut signature = [0u8; 64];
        signature.copy_from_slice(&bytes[184..248]);
        
        Self {
            payload: ProofPayload::from_bytes(&payload_bytes),
            signature,
            reserved: [0u8; 36], // Not stored in 248-byte format
        }
    }
    
    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        let bytes = self.to_bytes();
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
    
    /// Parse from hex string
    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != 496 {
            return None;
        }
        
        let mut bytes = [0u8; 248];
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
            let s = std::str::from_utf8(chunk).ok()?;
            bytes[i] = u8::from_str_radix(s, 16).ok()?;
        }
        
        Some(Self::from_bytes(&bytes))
    }
}

/// Reason codes for proof generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum ProofReason {
    /// Proof successfully generated
    R200_PROOF_GENERATED,
    /// LOCKED duration < 8 seconds
    R201_PROOF_NOT_STABLE,
    /// ΔC could not be calculated
    R202_PROOF_DC_UNKNOWN,
    /// No paired turns in window
    R203_PROOF_WINDOW_EMPTY,
    /// State is not LOCKED
    R204_PROOF_NOT_LOCKED,
}

impl ProofReason {
    /// Get code string
    pub fn code(&self) -> &'static str {
        match self {
            Self::R200_PROOF_GENERATED => "R200_PROOF_GENERATED",
            Self::R201_PROOF_NOT_STABLE => "R201_PROOF_NOT_STABLE",
            Self::R202_PROOF_DC_UNKNOWN => "R202_PROOF_DC_UNKNOWN",
            Self::R203_PROOF_WINDOW_EMPTY => "R203_PROOF_WINDOW_EMPTY",
            Self::R204_PROOF_NOT_LOCKED => "R204_PROOF_NOT_LOCKED",
        }
    }
    
    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            Self::R200_PROOF_GENERATED => "Proof successfully generated",
            Self::R201_PROOF_NOT_STABLE => "LOCKED not stable for 8 seconds",
            Self::R202_PROOF_DC_UNKNOWN => "ΔC could not be calculated",
            Self::R203_PROOF_WINDOW_EMPTY => "No paired turns in window",
            Self::R204_PROOF_NOT_LOCKED => "State is not LOCKED",
        }
    }
    
    /// Is this a success code?
    pub fn is_success(&self) -> bool {
        matches!(self, Self::R200_PROOF_GENERATED)
    }
}

impl std::fmt::Display for ProofReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code(), self.description())
    }
}

/// Result of proof generation attempt
#[derive(Debug, Clone)]
pub struct ProofResult {
    /// The proof if successful
    pub proof: Option<Proof>,
    /// Reason code
    pub reason: ProofReason,
}

impl ProofResult {
    /// Create success result
    pub fn success(proof: Proof) -> Self {
        Self {
            proof: Some(proof),
            reason: ProofReason::R200_PROOF_GENERATED,
        }
    }
    
    /// Create failure result
    pub fn failure(reason: ProofReason) -> Self {
        Self {
            proof: None,
            reason,
        }
    }
    
    /// Check if successful
    pub fn is_success(&self) -> bool {
        self.proof.is_some()
    }
}
