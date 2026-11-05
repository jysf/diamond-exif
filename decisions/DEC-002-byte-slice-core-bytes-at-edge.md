---
insight:
  id: DEC-002
  type: decision
  confidence: 0.85
  audience:
    - developer
    - agent

agent:
  id: claude-sonnet-5-5
  session_id: null

project:
  id: PROJ-001
repo:
  id: diamond-exif

created_at: 2026-11-04
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope: ["src/**"]

tags:
  - api
  - zero-copy
  - dependencies
---

# DEC-002: `&[u8]` core, `bytes::Bytes` only at the API edge

## Decision

The parsing core borrows `&'a [u8]` and returns payload slices of type `&'a [u8]` tied to the input; `bytes::Bytes` appears only in the future public API layer, which holds the `Bytes` and lends `&[u8]` inward.

## Context

Design §3 proposed `Bytes` throughout. The ported code does not agree: `Container<'a>` borrows `&'a [u8]`, `payload` returns a slice tied to `&self`, and the decoders allocate a `Vec` (porting brief §0 risk 2, §1.4). Adopting `Bytes` in the core would change every borrowing signature and every ported test. irradiance's PROJ-004 Stage A locked the same rule (slice accessors tied to the input, not `&self`).

## Alternatives Considered

- **Option A: `Bytes` everywhere.**
  - Why rejected: rewrites every ported signature and test; adds a dependency to the innermost layer for no parsing benefit.
- **Option B: owned `Vec<u8>` payloads.**
  - Why rejected: copies on every read; contradicts the zero-copy product claim.
- **Option C (chosen): `&[u8]` core, `Bytes` at the edge.**
  - Why selected: lifetimes and tests port unchanged; `payload` becomes `&'a [u8]` (a one-token change); WASM and Workers stay trivial.

## Consequences

- **Positive:** no dependency in the core; `[dependencies]` stays empty through M1a (the ported `Error` has a hand-written `Display`, so `thiserror` is not needed yet); borrowed slices can outlive the `Container`.
- **Negative:** the zero-copy claim is narrower than "nothing allocates": value decoders that return `Vec`s allocate. The claim must be worded as "payload access never copies file bytes" until lazy decoders exist, and backed by a test (payload pointer lies inside the input buffer).
- **Neutral:** `bytes` and `thiserror` are each a new top-level dependency and need their own DEC (and a `cargo deny` check) when the API layer arrives.

## Validation

Revisit if the writer's EditPlan needs owned, shareable buffers inside the core, or if an async server integration shows the borrow lifetimes are unworkable.

## References

- Related specs: SPEC-001, SPEC-002
- External: docs/porting-brief-diamond-exif.md §0 risk 2, §1.4; docs/exif-core-design.md §3, §16.4
