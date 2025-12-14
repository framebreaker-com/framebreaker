//! Integration tests for Slice 5 - HTTP API
//!
//! Tests API endpoints and WebSocket functionality

use soul0::core::create_router;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use serde_json::Value;

fn create_test_router() -> axum::Router {
    create_router("./test_snapshots".to_string())
}

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_test_router();
    
    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(json["status"], "ok");
    assert!(json["version"].is_string());
}

#[tokio::test]
async fn test_create_session() {
    let app = create_test_router();
    
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/session/new")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"observers": ["A", "B"]}"#))
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["session_id"].is_string());
    assert!(json["websocket_url"].is_string());
}

#[tokio::test]
async fn test_session_not_found() {
    let app = create_test_router();
    
    let response = app
        .oneshot(
            Request::builder()
                .uri("/session/nonexistent")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_full_session_flow() {
    // Create app with shared state
    let app = create_test_router();
    
    // Create session
    let response = app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/session/new")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"observers": ["A", "B"]}"#))
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let session_id = json["session_id"].as_str().unwrap();
    
    // Note: In a real test we'd need to share state between requests
    // This test mainly verifies the endpoint structure is correct
    assert!(!session_id.is_empty());
}

#[tokio::test]
async fn test_proof_not_found_without_lock() {
    let app = create_test_router();
    
    // Try to get proof for non-existent session
    let response = app
        .oneshot(
            Request::builder()
                .uri("/session/nonexistent/proof")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_snapshot_not_found_without_lock() {
    let app = create_test_router();
    
    let response = app
        .oneshot(
            Request::builder()
                .uri("/session/nonexistent/snapshot")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
