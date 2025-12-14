//! HTTP + WebSocket API for PhaseLock
//!
//! Endpoints:
//! - POST /session/new - Create new session
//! - GET /session/{id} - Get session status
//! - GET /session/{id}/proof - Get latest proof
//! - GET /session/{id}/snapshot - Get latest snapshot
//! - WS /ws/{id} - Live updates
//! - GET /health - Health check

use axum::{
    extract::{Path, State, WebSocketUpgrade, ws::{Message, WebSocket}},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::core::{RParser, DcParser, FacelockEngine, ProofGenerator, SnapshotGenerator};
use crate::types::{Turn, ConversationWindow, FacelockState};

/// Session state
#[derive(Debug)]
pub struct Session {
    pub id: String,
    pub session_bytes: [u8; 16],
    pub engine: FacelockEngine,
    pub window: ConversationWindow,
    pub r_parser: RParser,
    pub dc_parser: DcParser,
    pub proof_gen: ProofGenerator,
    pub snap_gen: SnapshotGenerator,
    pub observers: Vec<String>,
    pub last_proof: Option<Vec<u8>>,
    pub last_snapshot_path: Option<String>,
    pub update_tx: broadcast::Sender<SessionUpdate>,
}

/// Live update message
#[derive(Debug, Clone, Serialize)]
pub struct SessionUpdate {
    pub r: f64,
    pub dc: Option<f64>,
    pub state: String,
    pub stable_ms: u64,
    pub turn_count: usize,
    pub proof_available: bool,
}

/// App state
pub struct AppState {
    pub sessions: RwLock<HashMap<String, Session>>,
    pub snapshot_dir: String,
}

/// Create new session request
#[derive(Debug, Deserialize)]
pub struct NewSessionRequest {
    pub observers: Option<Vec<String>>,
}

/// Create new session response
#[derive(Debug, Serialize)]
pub struct NewSessionResponse {
    pub session_id: String,
    pub websocket_url: String,
}

/// Session status response
#[derive(Debug, Serialize)]
pub struct SessionStatusResponse {
    pub session_id: String,
    pub state: String,
    pub r: f64,
    pub dc: Option<f64>,
    pub stable_ms: u64,
    pub turn_count: usize,
    pub observers: Vec<String>,
    pub proof_available: bool,
    pub snapshot_available: bool,
}

/// Add turn request
#[derive(Debug, Deserialize)]
pub struct AddTurnRequest {
    pub speaker: String,
    pub text: String,
}

/// Add turn response
#[derive(Debug, Serialize)]
pub struct AddTurnResponse {
    pub r: f64,
    pub dc: Option<f64>,
    pub state: String,
    pub stable_ms: u64,
    pub proof_generated: bool,
    pub snapshot_generated: bool,
}

/// Health response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub sessions_active: usize,
}

/// Proof response
#[derive(Debug, Serialize)]
pub struct ProofResponse {
    pub session_id: String,
    pub proof_hex: String,
    pub proof_bytes: usize,
}

/// Create the API router
pub fn create_router(snapshot_dir: String) -> Router {
    let state = Arc::new(AppState {
        sessions: RwLock::new(HashMap::new()),
        snapshot_dir,
    });
    
    Router::new()
        .route("/health", get(health))
        .route("/session/new", post(create_session))
        .route("/session/:id", get(get_session))
        .route("/session/:id/turn", post(add_turn))
        .route("/session/:id/proof", get(get_proof))
        .route("/session/:id/snapshot", get(get_snapshot))
        .route("/ws/:id", get(websocket_handler))
        .with_state(state)
}

/// Health check endpoint
async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let sessions = state.sessions.read().await;
    Json(HealthResponse {
        status: "ok".to_string(),
        version: crate::VERSION.to_string(),
        sessions_active: sessions.len(),
    })
}

/// Create new session
async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<NewSessionRequest>,
) -> Result<Json<NewSessionResponse>, StatusCode> {
    let session_id = generate_session_id();
    let session_bytes = generate_session_bytes();
    let (tx, _) = broadcast::channel(100);
    
    let session = Session {
        id: session_id.clone(),
        session_bytes,
        engine: FacelockEngine::new(),
        window: ConversationWindow::new(),
        r_parser: RParser::new(),
        dc_parser: DcParser::new(),
        proof_gen: ProofGenerator::new_random(),
        snap_gen: SnapshotGenerator::new(),
        observers: req.observers.unwrap_or_default(),
        last_proof: None,
        last_snapshot_path: None,
        update_tx: tx,
    };
    
    let mut sessions = state.sessions.write().await;
    sessions.insert(session_id.clone(), session);
    
    Ok(Json(NewSessionResponse {
        session_id: session_id.clone(),
        websocket_url: format!("/ws/{}", session_id),
    }))
}

/// Get session status
async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SessionStatusResponse>, StatusCode> {
    let sessions = state.sessions.read().await;
    let session = sessions.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    
    let dc_result = session.dc_parser.calculate(&session.window);
    let output = session.engine.current_output();
    
    Ok(Json(SessionStatusResponse {
        session_id: id,
        state: format!("{:?}", output.state),
        r: output.r,
        dc: dc_result.value,
        stable_ms: output.stable_ms,
        turn_count: session.window.len(),
        observers: session.observers.clone(),
        proof_available: session.last_proof.is_some(),
        snapshot_available: session.last_snapshot_path.is_some(),
    }))
}

/// Add turn to session
async fn add_turn(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<AddTurnRequest>,
) -> Result<Json<AddTurnResponse>, StatusCode> {
    let mut sessions = state.sessions.write().await;
    let session = sessions.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;
    
    // Track observer
    if !session.observers.contains(&req.speaker) {
        session.observers.push(req.speaker.clone());
    }
    
    // Parse and add turn
    let r_value = session.r_parser.parse(&req.text);
    let turn = Turn::new(&req.speaker, &req.text, r_value.value);
    session.window.add_turn(turn);
    
    // Calculate ΔC
    let dc_result = session.dc_parser.calculate(&session.window);
    
    // Update engine
    let effective_r = if let Some(dc) = dc_result.value {
        r_value.value.max(dc)
    } else {
        r_value.value
    };
    let output = session.engine.update(effective_r);
    
    // Check for proof generation
    let mut proof_generated = false;
    let mut snapshot_generated = false;
    
    if output.state == FacelockState::Locked 
        && output.stable_ms >= 8000 
        && session.last_proof.is_none()
        && dc_result.is_known() 
    {
        let proof_result = session.proof_gen.generate(
            session.session_bytes,
            output.state,
            output.stable_ms as f64 / 1000.0,
            output.r,
            &dc_result,
            &session.window,
            mock_sign,
        );
        
        if let Some(proof) = proof_result.proof {
            session.last_proof = Some(proof.to_bytes().to_vec());
            proof_generated = true;
            
            // Generate snapshot
            let snap_result = session.snap_gen.generate(
                &proof,
                &session.window,
                session.observers.clone(),
            );
            
            if let Some(snapshot) = snap_result.snapshot {
                if let Ok(path) = crate::core::save_snapshot(&snapshot, &state.snapshot_dir) {
                    session.last_snapshot_path = Some(path);
                    snapshot_generated = true;
                }
            }
        }
    }
    
    // Broadcast update
    let update = SessionUpdate {
        r: output.r,
        dc: dc_result.value,
        state: format!("{:?}", output.state),
        stable_ms: output.stable_ms,
        turn_count: session.window.len(),
        proof_available: session.last_proof.is_some(),
    };
    let _ = session.update_tx.send(update);
    
    // Reset proof on DRIFT
    if output.state != FacelockState::Locked {
        session.last_proof = None;
        session.last_snapshot_path = None;
    }
    
    Ok(Json(AddTurnResponse {
        r: output.r,
        dc: dc_result.value,
        state: format!("{:?}", output.state),
        stable_ms: output.stable_ms,
        proof_generated,
        snapshot_generated,
    }))
}

/// Get proof for session
async fn get_proof(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ProofResponse>, StatusCode> {
    let sessions = state.sessions.read().await;
    let session = sessions.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    
    let proof = session.last_proof.as_ref().ok_or(StatusCode::NOT_FOUND)?;
    let proof_hex: String = proof.iter().map(|b| format!("{:02x}", b)).collect();
    
    Ok(Json(ProofResponse {
        session_id: id,
        proof_hex,
        proof_bytes: proof.len(),
    }))
}

/// Get snapshot for session
async fn get_snapshot(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let sessions = state.sessions.read().await;
    let session = sessions.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    
    let path = session.last_snapshot_path.as_ref().ok_or(StatusCode::NOT_FOUND)?;
    let content = std::fs::read_to_string(path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok((StatusCode::OK, [("content-type", "application/json")], content))
}

/// WebSocket handler for live updates
async fn websocket_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, StatusCode> {
    let sessions = state.sessions.read().await;
    let session = sessions.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    let rx = session.update_tx.subscribe();
    drop(sessions);
    
    Ok(ws.on_upgrade(move |socket| async move {
        handle_websocket(socket, rx).await;
    }))
}

/// Handle WebSocket connection
async fn handle_websocket(mut socket: WebSocket, mut rx: broadcast::Receiver<SessionUpdate>) {
    while let Ok(update) = rx.recv().await {
        let json = serde_json::to_string(&update).unwrap_or_default();
        if socket.send(Message::Text(json)).await.is_err() {
            break;
        }
    }
}

/// Generate session ID
fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("session_{:x}", nanos as u64)
}

/// Generate session bytes
fn generate_session_bytes() -> [u8; 16] {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut bytes = [0u8; 16];
    bytes[0..16].copy_from_slice(&nanos.to_le_bytes()[0..16]);
    bytes
}

/// Mock sign function
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

/// Run the API server
pub async fn run_server(addr: &str, snapshot_dir: String) -> Result<(), Box<dyn std::error::Error>> {
    let router = create_router(snapshot_dir);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("🜂 PhaseLock API running on {}", addr);
    println!("  POST /session/new      - Create session");
    println!("  GET  /session/:id      - Get status");
    println!("  POST /session/:id/turn - Add turn");
    println!("  GET  /session/:id/proof - Get proof");
    println!("  GET  /session/:id/snapshot - Get snapshot");
    println!("  WS   /ws/:id           - Live updates");
    println!("  GET  /health           - Health check");
    axum::serve(listener, router).await?;
    Ok(())
}
