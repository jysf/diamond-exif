---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-007
  type: story                      # epic | story | task | bug | chore
  cycle: ship  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: M                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: M             # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: approved  # approved | punch-list | rejected — the OUTCOME of the verify
                                   #   cycle, stamped by `just advance-cycle` when the spec leaves
                                   #   verify (same three verdicts Prompt 4 already returns).
                                   #   Recorded in front-matter, not just prose, so "verify never
                                   #   rejects anything" stops being a hunch and becomes a number.

project:
  id: PROJ-001
  stage: STAGE-001
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-002, DEC-003, DEC-004]
  constraints: [panic-free-library, forbid-unsafe-code, fuzz-target-per-surface, test-before-implementation, no-new-top-level-deps-without-decision]
  related_specs: [SPEC-004, SPEC-005]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-004]             # sequential: shares the fuzz target and seeds with SPEC-004

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-001's <capability>". Optional; null is acceptable.
value_link: "STAGE-001: Make, Model, Artist, dates and signed/float values become readable, so the audit CLI and the oracle comparison have values to report"

# Self-reported AI cost per cycle. Each cycle (design, build, verify,
# ship) appends one entry to sessions[]. Totals are computed at ship.
# Record a REAL tokens_total for metered cycles (build/verify) — the agent
# that runs the cycle writes it from its own interface (/cost, the API
# usage object, or its tool's report). Only un-metered main-loop cycles
# (design/ship) may be null-with-note. `just cost-audit` enforces this on
# shipped specs. See AGENTS.md §4 and docs/cost-tracking.md. interface:
# claude-code | claude-ai | api | ollama | other.
cost:
  # Optional PREDICTION of the total tokens this spec will take, set at
  # design. Never a gate — its only job is to be compared with the actual
  # below (`just calibration`), so you learn whether you systematically
  # under- or over-estimate. null = didn't predict.
  tokens_estimate: 250000
  sessions:
    - cycle: build
      agent: claude-sonnet-5-5
      interface: other
      tokens_total: 172264
      estimated_usd: 1.14
      duration_minutes: 10
      recorded_at: 2026-11-05
      notes: "tokens transcribed by the orchestrator from the sub-agent metered result"
    - cycle: verify
      agent: claude-opus-5-5
      interface: other
      tokens_total: 175106
      estimated_usd: 1.16
      duration_minutes: 8
      recorded_at: 2026-11-05
      notes: "verdict APPROVED at ffb96a5; tokens transcribed by the orchestrator from the sub-agent metered result"
  totals:
    tokens_total: 347370
    estimated_usd: 2.30
    session_count: 2
shipped_at: 2026-11-05
---

# SPEC-007: Decode ASCII, signed and float tag values

## Context

Split out of the original SPEC-004 at its design: decoders touch `types.rs` and `reader.rs`, the pointer walk touched the walker, so they verify independently. EXIF `Make`, `Model`, `Artist`, `Copyright`, `DateTime` and `DateTimeOriginal` are ASCII, and today nothing decodes ASCII, `SBYTE`, `SSHORT`, `SLONG`, `FLOAT` or `DOUBLE` (brief §0 risk 1; design §16.3). The audit CLI cannot report a single string until this lands, and SPEC-005's `parse_fields` oracle comparison needs values to compare.

The existing readers set the pattern and are not changed: `uints` (unsigned integers and `IFD`), `urationals`/`srationals` (verbatim pairs). Each reader checks the field **type first** (before touching the buffer), then the `MAX_TAG_VALUES` count cap, then reads the bounds-checked payload. New readers follow that order exactly.

## Goal

Panic-free, bounds-checked `ascii`, `sints` and `floats` readers on `Container`, with the hostile shapes of ASCII (count 0, unterminated, embedded NUL, non-UTF-8 bytes) exposed as data rather than errors, text borrowed from the input, and named `TYPE_*` constants for every TIFF field type.

## Inputs

- **Read first:** `src/tiff/types.rs` (the pattern: `uints`, `rational_chunks`, `type_size`), `src/reader.rs` (`ByteOrder::u16/u32/i32`), `src/tiff/mod.rs` (`payload`, the test module and its `Build` byte-builder), `src/tiff/tags.rs`, `src/error.rs`, `fuzz/fuzz_targets/tiff_ifd.rs`, `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs`.
- **Decisions:** DEC-002 (borrow from the input; no new dependencies), DEC-003 (net-new code, no ledger row; add a note to the `types.rs` row), DEC-004 (fuzz layout).
- **Host traps:** `guidance/toolchain-brief.md` (`~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`).
- No irradiance source: nothing here is ported.

## Outputs

### Public API (all in `exif_core::tiff`)

```rust
// types.rs: complete the named field-type set (TIFF 6.0 §2 "Types"; types 6-12)
pub const TYPE_ASCII: u16 = 2;
pub const TYPE_SBYTE: u16 = 6;
pub const TYPE_SSHORT: u16 = 8;
pub const TYPE_SLONG: u16 = 9;
pub const TYPE_FLOAT: u16 = 11;
pub const TYPE_DOUBLE: u16 = 12;

/// An ASCII field's payload, split at its first NUL. Borrows the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ascii<'a> { /* private: text: &'a [u8], terminated: bool, rest: &'a [u8] */ }
impl<'a> Ascii<'a> {
    /// Bytes before the first NUL (all of the payload if there is none).
    pub fn as_bytes(&self) -> &'a [u8];
    /// Whether a NUL was found (TIFF says ASCII ends with one; real files often do not).
    pub fn is_terminated(&self) -> bool;
    /// Bytes after the first NUL (further strings, padding or garbage); empty if unterminated.
    pub fn rest(&self) -> &'a [u8];
    /// `Ok` only if `as_bytes` is valid UTF-8 (camera strings are often Latin-1).
    pub fn to_str(&self) -> Result<&'a str, core::str::Utf8Error>;
    /// Invalid sequences replaced with U+FFFD; borrows when already valid.
    pub fn to_string_lossy(&self) -> std::borrow::Cow<'a, str>;
}

impl<'a> Container<'a> {
    /// ASCII (type 2) only.
    pub fn ascii(&self, entry: &Entry) -> Result<Ascii<'a>, Error>;
    /// SBYTE (6), SSHORT (8) and SLONG (9), widened to i32. Unsigned types are refused.
    pub fn sints(&self, entry: &Entry) -> Result<Vec<i32>, Error>;
    /// FLOAT (11) and DOUBLE (12), widened to f64; NaN and infinities pass through verbatim.
    pub fn floats(&self, entry: &Entry) -> Result<Vec<f64>, Error>;
}
```

Semantics, all normative:
- **Type gate first**, before the buffer: any other type is `Error::UnexpectedFieldType { tag, field_type }`. In particular `ascii` refuses `BYTE`/`UNDEFINED`, `uints` keeps refusing the signed and float types, and `sints`/`floats` refuse each other's and the unsigned types. (`UNDEFINED` blobs are read with `payload()`.)
- **Count cap second**: a count above `MAX_TAG_VALUES` is `TagTooLarge` (for ASCII the count is a byte count), before any payload read.
- **Then** the payload via `Container::payload` (so `Truncated`/`ValueOverflow` come from the single existing place).
- **`ascii`**: `as_bytes` is the payload up to the first NUL; `rest` is everything after it; `terminated` says whether a NUL was found. Invariant: `as_bytes().len() + usize::from(terminated) + rest().len() == payload.len()`, and `as_bytes()` contains no NUL. Count 0 gives empty, unterminated, empty rest. No trimming of spaces, no decoding, no allocation (except `to_string_lossy` on invalid input).
- **`sints`**: `SBYTE` sign-extends, `SSHORT` sign-extends, `SLONG` as is; byte order respected. Result length equals the declared count.
- **`floats`**: `FLOAT` widens exactly to `f64`; `DOUBLE` is read as is; byte order respected. No arithmetic is done on the values and NaN/inf/subnormals are returned as the file wrote them (callers must check `is_finite`). Result length equals the declared count.
- **`ByteOrder`** (in `reader.rs`, `pub(crate)`): add `i16`, `f32`, `f64` (and whatever integer width `f64` needs) next to the existing `u16/u32/i32`, using `from_be_bytes`/`from_le_bytes` over `as_chunks` arrays so widths are type-level facts. No indexing, no `try_into().unwrap()`.

`tags.rs` adds spec-citing constants (TIFF 6.0 §8 / Exif 2.32): `IMAGE_DESCRIPTION = 270`, `MAKE = 271`, `MODEL = 272`, `SOFTWARE = 305`, `DATE_TIME = 306`, `ARTIST = 315`, `COPYRIGHT = 33432`, `DATE_TIME_ORIGINAL = 0x9003`.

### Files modified

- `src/tiff/types.rs`, `src/reader.rs`, `src/tiff/tags.rs`, `src/tiff/mod.rs` (re-export `Ascii` and the new consts; tests per below).
- `fuzz/fuzz_targets/tiff_ifd.rs`, `tests/fuzz_layout.rs` (same `exercise`), `examples/fuzz_seeds.rs`: new calls, invariants and three seeds (below). The existing 26 seeds stay byte-identical.
- `docs/provenance-ledger.md` (note on the `types.rs` row: decoders net-new, SPEC-007), `CHANGELOG.md` (`Added`), `README.md` status line.

### Fuzz changes (same change)

For every entry on `Ok(container)`, call `ascii`, `sints`, `floats` (plus the existing readers) and assert, each in **both directions** where it is an iff:
- **Type gate**: for each reader, `Err(UnexpectedFieldType)` **iff** the field type is outside its accepted set. (Check the type set against a table in the harness; an `Ok` for a rejected type or an `UnexpectedFieldType` for an accepted one is a finding.)
- **Length**: when `uints`, `sints` or `floats` returns `Ok`, `len() == entry.count()`.
- **ASCII partition**: on `Ok`, `as_bytes().len() + usize::from(is_terminated()) + rest().len() == payload.len()`, `as_bytes()` has no NUL, `is_terminated()` is true **iff** the payload contains a NUL, and when terminated `payload[as_bytes().len()] == 0` (use `get`, the fuzz crate may index but be defensive).
- **Zero-copy**: `as_bytes()` and `rest()` lie inside the input slice (pointer-range check as in `payload_borrows_from_the_input_not_the_container`).
- `to_str` is `Ok` iff `core::str::from_utf8(as_bytes())` is; `to_string_lossy` never panics.

Three new seeds: `ascii-shapes` (ASCII entries with count 0, unterminated, embedded NUL, inline terminated, non-UTF-8), `signed-and-float` (SBYTE/SSHORT/SLONG extremes; FLOAT/DOUBLE with NaN, infinity, subnormal and -0.0, both inline and external), `ascii-count-past-eof`.

## Acceptance Criteria

- [ ] `ascii`, `sints` and `floats` behave exactly as the semantics above, in both byte orders, inline and external payloads.
- [ ] Every hostile ASCII shape (count 0, unterminated, embedded NUL, non-UTF-8, count past EOF, count over the cap) is a typed outcome or data, never a panic; text is borrowed from the input.
- [ ] The type gate is checked before the buffer for every reader (a wrong-type entry whose payload is also out of bounds reports `UnexpectedFieldType`, not `Truncated`).
- [ ] All eight TIFF/Exif tag constants and six type constants have the documented values (tested).
- [ ] **Shown red** (record test names): (a) f64/f32 read with the wrong byte order for big-endian; (b) `SSHORT` zero-extended instead of sign-extended; (c) the NUL included in `as_bytes`; (d) `ascii` accepting `UNDEFINED`; (e) `rest` off by one; (f) `uints` accepting `SLONG`.
- [ ] `tiff_ifd` fuzz target extended; at least two of the new invariants shown red with planted bugs (one against the real target, e.g. (e) or a `sints` length bug); `just fuzz-smoke` 60 s clean, executions and coverage recorded.
- [ ] Library gates green and independently re-run by the orchestrator: `cargo fmt --check`, `just lint`, `just lint-fuzz`, `rustfmt --check` on the fuzz crate, `cargo test` on stable and 1.90.0, `cargo rustc --lib -- -F unsafe_code`, `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit`, `just cost-audit`.
- [ ] `[dependencies]` still empty; five lints clean in non-test code; irradiance and `scripts/` untouched; the 26 existing seeds byte-identical.

## Failing Tests

In `src/tiff/mod.rs` `mod tests` with the `Build` byte-builder (extend it with small helpers for ASCII/float payloads as needed). "Both orders" means `for big in [false, true]` as the existing rational tests do.

**ASCII**
- `ascii_reads_a_terminated_external_string` — `MAKE` = `b"Canon\0"` (count 6, external): `as_bytes()==b"Canon"`, `is_terminated()`, `rest()` empty, `to_str()==Ok("Canon")`, identical in both orders (ASCII has no byte order).
- `ascii_reads_inline_values` — count 1 `"\0"` (empty, terminated), count 3 `"ab\0"`, count 4 `"abc\0"`: values and flags as stated; the value comes from the entry, not the file.
- `ascii_count_zero_is_empty_and_unterminated` — count 0: empty, `!is_terminated()`, empty rest, no panic.
- `ascii_without_a_terminator_is_data_not_an_error` — `b"Canon"` count 5 external: bytes `"Canon"`, `!is_terminated()`, rest empty.
- `ascii_splits_at_the_first_nul` — `b"ab\0cd\0"` count 6: bytes `"ab"`, terminated, rest `b"cd\0"`.
- `ascii_non_utf8_is_exposed_not_rejected` — bytes `[0x43, 0xE9, 0x00]` (Latin-1 `Cé`): `as_bytes()` exact, `to_str()` is `Err`, `to_string_lossy() == "C\u{FFFD}"`; valid input gives a borrowed `Cow`.
- `ascii_refuses_other_types_before_touching_the_buffer` — a `BYTE` and an `UNDEFINED` entry, each with a payload offset past EOF: `UnexpectedFieldType{tag, field_type}` (not `Truncated`).
- `ascii_count_past_eof_is_truncated` and `ascii_count_over_the_cap_is_too_large` — external count beyond the buffer: `Truncated`; count `MAX_TAG_VALUES + 1`: `TagTooLarge` (cap checked before the payload).
- `ascii_borrows_from_the_input_not_the_container` — like `payload_borrows_from_the_input_not_the_container`: slices outlive the `Container` and point into `data`.

**Signed**
- `sints_sign_extend_every_width_in_both_orders` — SBYTE `[-1, 127, -128]`, SSHORT `[-2, 32767, -32768]`, SLONG `[i32::MIN, -1, i32::MAX]`; inline and external cases; exact `Vec<i32>`.
- `sints_refuse_unsigned_and_unsigned_readers_refuse_signed` — `sints` on SHORT/LONG/RATIONAL/FLOAT gives `UnexpectedFieldType`; `uints` on SBYTE/SSHORT/SLONG/FLOAT/DOUBLE gives `UnexpectedFieldType` (pins the signedness-is-the-type's rule).
- `sints_count_cap_and_truncation` — count over the cap `TagTooLarge`; external payload past EOF `Truncated`; `u32::MAX` count does not allocate.

**Float**
- `floats_read_verbatim_in_both_orders` — FLOAT `[1.5, -0.0, f32::INFINITY, NaN with a nonzero payload, f32 subnormal]` and DOUBLE `[1e300, -0.0, f64::NEG_INFINITY, NaN, f64 subnormal]`: compare with `to_bits()` (NaN-safe); the FLOAT results equal `f64::from(x)` bit-for-bit for finite values.
- `floats_refuse_other_types` — RATIONAL, SLONG, LONG: `UnexpectedFieldType`; type check precedes bounds.
- `floats_count_cap_and_truncation` — as for `sints`.

**Constants and plumbing**
- `field_type_constants_match_the_tiff_spec_and_type_size` — the six new `TYPE_*` have values 2/6/8/9/11/12 and `type_size` returns 1/1/2/4/4/8 for them.
- `tag_constants_match_the_spec` — the eight tag constants have their documented values.

**Fuzz layout** (`tests/fuzz_layout.rs`): `SEEDS` gains the three new names; `exercise` mirrors the extended invariants.

## Non-Goals

- Interpreting a tag's meaning: tag tables, `UserComment` charset prefixes, GPS coordinate arithmetic, `DateTime` parsing, string trimming or normalisation (SPEC-005's `parse_fields` and later stages)
- Rational changes (done), `UNDEFINED` typed readers, 64-bit BigTIFF types (13+ beyond `IFD`)
- Walk changes, oracle comparison (SPEC-005), writer encoders, CI wiring, the 24 h fuzz run

## Notes for the Implementer

- Keep the five lints satisfied in non-test code: `get`, `as_chunks`, `checked_*`, no indexing, no `unwrap`. Use the same shape as `rational_chunks`; consider one shared "type gate, count cap, payload" prelude helper so the three readers cannot drift apart (do not change the behaviour of the existing readers).
- `Ascii` borrows the input (`&'a [u8]`), the same lifetime as `payload`, so values outlive the `Container` (DEC-002). Split the payload with `iter().position(|b| *b == 0)` and `get`/`split_at_checked`-style slicing, not indexing.
- Do not add any dependency (no `memchr`, no `byteorder`).
- Update the fuzz target and seeds in the same change; run the 60 s smoke with the PATH-first `~/.cargo/bin` and `+nightly` invocation in the target's doc comment. Write the fuzz iff-invariants both ways; a one-directional check once let a constant-`true` plant through.
- Do not push or open a PR; do not advance cycles; do not modify irradiance or `scripts/`.

---

## Reflection

1. **What would I do differently next time?**
   — Do not ask a test for something the language does not promise: the spec required `to_bits()` equality for a FLOAT NaN widened to f64, but Rust does not guarantee a NaN payload survives f32 to f64 widening, so the builder (rightly) checked `is_nan()` only for FLOAT. DOUBLE NaN is pinned bit-exact. Next time check an invariant is achievable before writing it into a spec.

2. **Does any template, constraint, or decision need updating?**
   — No. The explicit instructions added after SPEC-004 (iff-invariants in both directions; report every renamed, folded or missing test) worked: the builder reported all deviations and the verifier found no medium defect. Verify findings carried forward, none blocking: `sints` and `floats` fall through to the last width in their decode `match` rather than an explicit error arm like `uints` (drift risk if a gate is ever widened); the fuzz target does not check cap-before-payload order or an "Ok iff type, cap and payload" rule for `sints`/`floats`.

3. **Is there a follow-up spec I should write now before I forget?**
   — No new spec. Candidate small tightening for the next change that touches `types.rs`: make the decode arms total with an explicit error arm. The orchestrator also completed `tests/fuzz_layout.rs` `SEEDS` to name all 29 committed seeds (it had drifted to 21) and added a ledger note for the new `reader.rs` reads, both low-severity verify nits.

4. **Where was the worst defect caught?** — verify
   *(low only: the unpinned FLOAT NaN payload, which turned out to be a spec error, not a code defect; the build itself was clean first try.)*

5. **What can a user do now that they couldn't before?** — Read a photo's Make, Model, Artist, Copyright and dates as borrowed text, even when the camera left the string unterminated, padded or in Latin-1, and read signed and floating-point tag values, with no input able to make the reader panic; before, only unsigned integers and rationals could be decoded.
