# Soul-0

Reference implementation of the PhaseLock protocol.

**Version:** 1.0  
**Status:** Release Candidate (95/95 tests passing)

## What is this?

Soul-0 measures alignment between observers:
- **r** — ego noise (0-1)
- **ΔC** — coherence drift (0-1)

When both are low and stable for 8 seconds, a cryptographic proof is generated.

## Quick Start

```bash
# Build
cargo build --release

# Test (95 tests)
cargo test

# Solo mode (type and see r)
cargo run -- --interactive

# Duo mode (two speakers, r + ΔC)
cargo run -- --duo

# API server (http://localhost:3000)
cargo run -- --serve
```

## States

| State | Color | Meaning |
|-------|-------|---------|
| WAITING | Gray | Not enough data yet |
| APPROACHING | Orange | Moving toward alignment |
| LOCKED | Green | Full alignment — proof available |
| DRIFT | Red | Alignment lost |

## Thresholds

| Threshold | Value | Meaning |
|-----------|-------|---------|
| r_lock | 0.15 | r must be below this for LOCKED |
| r_approach | 0.25 | r below this → APPROACHING |
| r_drift | 0.30 | r above this → DRIFT |
| stability | 8 sec | Must maintain low r for 8 seconds |

## Architecture

```
src/
├── core/
│   ├── r_parser.rs      # 7 signals for ego noise
│   ├── dc_parser.rs     # 5 signals for coherence drift
│   ├── facelock.rs      # State machine
│   ├── proof.rs         # 248-byte cryptographic proof
│   ├── snapshot.rs      # 14 blind spots, horizon questions
│   └── api.rs           # HTTP + WebSocket
├── types/               # All data structures
├── lib.rs               # Exports + constants
└── main.rs              # CLI (--interactive, --duo, --serve)

tests/
├── slice1_integration.rs    # 8 tests
├── slice2_integration.rs    # 13 tests
├── slice3_integration.rs    # 9 tests
├── slice4_integration.rs    # 9 tests
└── slice5_integration.rs    # 6 tests
```

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| /health | GET | Health check |
| /session/new | POST | Create session |
| /session/:id | GET | Get session status |
| /session/:id/turn | POST | Add turn |
| /session/:id/proof | GET | Get proof (if LOCKED) |
| /session/:id/snapshot | GET | Get snapshot JSON |
| /ws/:id | WS | Live updates |

## Done Criteria

| Slice | Scope | Tests | Status |
|-------|-------|-------|--------|
| 1 | CLI → r → state | 8 | ✓ |
| 2 | +ΔC, duo mode | 13 | ✓ |
| 3 | Proof generation | 9 | ✓ |
| 4 | Snapshots | 9 | ✓ |
| 5 | HTTP + WebSocket | 6 | ✓ |
| **Total** | | **95** | **✓** |

---

🜂 *"The code is the proof."*
