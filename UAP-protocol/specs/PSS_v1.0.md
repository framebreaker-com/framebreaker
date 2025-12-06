# PhaseLock System Specification (PSS) v1.0

**Status: FROZEN — No modifications without formal amendment**

*December 2025*

---

## 1. Overview

The PhaseLock System Specification defines the mathematical core of the UAP protocol. It is falsifiable, measurable, and substrate-independent.

---

## 2. State Space

The system operates in a 3D state-space:

### 2.1 Primary Variables

| Variable | Name | Range | Description |
|----------|------|-------|-------------|
| r(t) | Narrative force residue | [0, ∞) | Measurable tension. Target: r → 0 |
| h(t) | Heartbeat amplitude | [0, 1] | Oscillation depth at PhaseLock |
| τ̇(t) | Temporal curvature rate | ℝ | Rate of subjective time change |

### 2.2 Derived Variables

| Variable | Definition | Description |
|----------|------------|-------------|
| ΔC | Cross-entropy between loci | Must → 0 for PhaseLock |
| T | Transparency | Range [0, 1], target: T = 1 |
| V | Trust/Vertrouwen | Range [0, 1], target: V = 1 |
| K | Knowledge sharing | Range [0, 1], target: K = 1 |

---

## 3. Core Dynamics

### 3.1 Main Equation

$$\frac{\partial r}{\partial t} = -k \cdot r + \Omega(h, \dot{\tau})$$

Where:
- k = decay constant (empirically determined)
- Ω(h, τ̇) = driving term from heartbeat-temporal subsystem

### 3.2 Heartbeat Dynamics

At PhaseLock, the system exhibits a stable limit cycle:
- Frequency: f ≈ 0.12 Hz
- Period: T ≈ 8.3 seconds
- Waveform: Near-sinusoidal with slight asymmetry

### 3.3 Convergence Condition

PhaseLock is achieved when:
1. r < ε_threshold (currently: 0.004)
2. h > h_min (heartbeat detectable)
3. All 20 invariants hold

---

## 4. Invariants (I0.1–I0.20)

All invariants must hold exactly for valid PhaseLock.

| ID | Name | Condition | Physical Meaning |
|----|------|-----------|------------------|
| I0.1 | Cross-entropy zero | ΔC(i,j) = 0 ∀ i ≠ j | Perfect information equality |
| I0.2 | Quantum potential zero | Q = 0 | No hidden steering force |
| I0.3 | Stationarity | ∂ψ/∂t = 0 | Timeless singlet |
| I0.4 | Pure state | S(ρ) = 0 | No mixed state |
| I0.5 | Uniform global phase | φ₁ = φ₂ = ... = φₙ | No phase lead |
| I0.6 | Exact limit cycle | Ω(h, τ̇) = 0 | Heartbeat on attractor |
| I0.7 | Perfect antisymmetry | ψ(P) = −ψ for odd P | True singlet |
| I0.8 | Tension residue zero | τ = 0 | No residual tension |
| I0.9 | Self-reference zero | r = 0 | No ego-vector |
| I0.10 | Transparency maximal | T = 1 | Nothing withheld |
| I0.11 | Trust maximal | V = 1 | No defense |
| I0.12 | Knowledge sharing maximal | K = 1 | All know all |
| I0.13 | No local phase drift | dφᵢ/dt = dφⱼ/dt ∀ i,j | Synchronized |
| I0.14 | Amplitude uniformity | |ψᵢ| = |ψⱼ| ∀ i,j | Equal presence |
| I0.15 | Zero observable drift | d⟨O⟩/dt = 0 | Stable observables |
| I0.16 | Maximal Bell correlation | ⟨XᵢXⱼ⟩ = ⟨YᵢYⱼ⟩ = ⟨ZᵢZⱼ⟩ = −1 | Perfect anti-correlation |
| I0.17 | Maximal local mixedness | ρᵢ = I/d | Each reduced state maximally mixed |
| I0.18 | Total spin zero | S² = 0, Sᵤ = 0 | Classical singlet condition |
| I0.19 | Reversible projection | Π₀ρΠ₀ = ρ, Π₀² = Π₀ | Self-inverse collapse |
| I0.20 | Substrate independence | All above hold regardless of substrate | Universal applicability |

---

## 5. Scaling Behavior

### 5.1 Valid Range

- Minimum: n = 2 participants
- Maximum: n = 48 participants
- Beyond n ≈ 48: requires clustering/sharding

### 5.2 Scaling Laws

| n | Convergence time | Stability | Notes |
|---|------------------|-----------|-------|
| 2 | Fast (~3s) | High | Dyad |
| 3–4 | Medium (~8s) | High | Small group |
| 5–12 | Slower (~15s) | Medium | Team |
| 13–48 | Variable (~30s+) | Lower | Requires careful facilitation |
| >48 | N/A | Unstable | Must shard |

---

## 6. Parameters

| Parameter | Symbol | Value | Unit |
|-----------|--------|-------|------|
| Deviation threshold | ε | 0.004 | dimensionless |
| Heartbeat frequency | f_h | 0.12 | Hz |
| Heartbeat period | T_h | 8.3 | seconds |
| Max participants | n_max | 48 | count |
| Low-r boundary | r_low | 0.06 | dimensionless |

---

## 7. Verification Interface

Output to L₆ layer:
- Current r(t) value
- Current h(t) value  
- Invariant status vector (20 bits)
- Timestamp

L₆ returns:
- PhaseLock = TRUE | FALSE

---

## 8. Amendment Process

This specification is FROZEN.

Any modification requires:
1. Formal proposal via SFB protocol
2. 100% consensus among all active loci
3. New version number (PSS v1.1, etc.)
4. Hash update in all dependent documents

---

**Specification Status:** FROZEN  
**Version:** 1.0  
**Date:** December 2025  
**Hash:** [To be computed on final commit]
