---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-001                     # stable, zero-padded, continuous across the repo
  status: shipped                   # proposed | active | shipped | cancelled | on_hold
  priority: high                    # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-04
shipped_at: 2026-11-05

# What part of the project's value thesis this stage advances.
# If you can't articulate value_contribution, the stage may be
# infrastructure-only — acceptable but flag it.
value_contribution:
  advances: "The reader foundation of the thesis: a tolerant, bounds-checked, ExifTool-verified IFD engine that every later stage (audit CLI, writer) builds on."
  delivers:
    - "Buildable, gated single crate with irradiance's five-lint deny set and hardened gate harness"
    - "Tolerant multi-IFD EXIF reader (IFD0, Exif, GPS, Interop) reporting per-IFD ParseIssues"
    - "ASCII, signed and float tag decoding"
    - "ExifTool-oracle parity tests with red-proofs and a tiff_ifd fuzz target"
  explicitly_does_not:
    - "Parse JPEG/PNG/WebP containers (next stage)"
    - "Write or strip anything (writer is net-new, later stage)"
    - "Ship the exif-audit CLI"

# Orchestration cost — the spend that has no spec to attach to (roadmap:
# orchestration + framing cost attribution). Framing a stage, deciding the spec
# breakdown, and cross-spec steering all happen BEFORE/BETWEEN specs, so today
# they are invisible and recorded cost is systematically under-counted.
#
# THE ORCHESTRATOR FILLS THIS — not the human. At stage close, read your own
# session total (`/cost` in Claude Code; the `usage` object via API) and append
# one entry. Stage grain ONLY: do not try to split orchestration across specs —
# that is a division you cannot observe, so any per-spec number is invented.
# Warn-only, never a gate. A null here is honest; a guess is not. (DEC-013 §5)
orchestration_cost:
  sessions:
    - tokens_total: 1028071
      estimated_usd: 6.79
      recorded_at: 2026-11-05
      notes: "the orchestrator conversation, from init through stage close (framing, designs, dispatches, independent gate re-runs, ships and PRs for SPEC-001 through SPEC-008); recovered from transcript usage (input + output + cache_creation, de-duplicated by message id, cache_read excluded) at 6.60 USD/Mtok. The transcript file 6e1a0b8d is an earlier slice of the same conversation (same first message and start time), so it is not counted separately. Read at stage close; the closing turns after the read are not included"
---

# STAGE-001: M1a foundation — TIFF/IFD engine

## What This Stage Is

Stand up the single `exif-core` crate with irradiance's hardened gate harness, record the three founding decisions, port the strict DNG-stripped TIFF/IFD skeleton with its 17 hostile-input tests, and then extend it: tolerant-walk policy, Exif/GPS/Interop pointer walk, ASCII/signed/float decoders, and the ExifTool oracle with red-proofs and a `tiff_ifd` fuzz target. At ship the crate parses bare TIFF/EXIF IFD data, reports per-IFD issues instead of failing whole, and every parity claim has been shown red.

## Why Now

First stage; everything else (containers, audit CLI, writer) depends on the IFD engine. DEC-001 gates the test port: deciding the tolerant-walk policy first avoids churning ~25% of ported tests twice, so the port is deliberately two-phase (strict skeleton green, then policy change as its own diff).

## Success Criteria

- `cargo fmt`, `clippy` (five-lint deny) and `cargo test` green at every spec boundary; `-F unsafe_code` re-check passes.
- DEC-001, DEC-002, DEC-003 merged before any ported test lands.
- 17 hostile-input tests ported; a corrupt GPS IFD leaves IFD0 readable and yields a `ParseIssue`.
- Every ported file has a `docs/provenance-ledger.md` row; ledger header states ExifTool is run, never read.
- Oracle parity (`-T -n -s3`) red-proofed; `tiff_ifd` fuzz target exists and has run 60 s.

## Scope

### In scope
- Crate scaffold, design docs, provenance ledger
- DEC-001/002/003 and the strict skeleton port (`error.rs`, `reader.rs`, `tiff/{mod,types,tags}.rs`)
- Tolerant walk, multi-IFD pointer walk, wider types
- ExifTool-only `tools.rs` subset, `mutate.rs`, shared `probe.rs`, `tiff_ifd` fuzz

### Explicitly out of scope
- JPEG/PNG/WebP parsing, writer, strip policies, `exif-audit`, XMP/IPTC/ICC, workspace promotion

## Spec Backlog

- [x] SPEC-001 (shipped on 2026-11-05) — [S] Scaffold the `exif-core` crate: five-lint deny, deny.toml, licenses, provenance-ledger header
- [x] SPEC-002 (shipped on 2026-11-05) — [L] Port the strict TIFF skeleton and the 17 hostile-input tests, green, with ledger rows
- [x] SPEC-006 (shipped on 2026-11-05) — [M] `tiff_ifd` fuzz target on the strict parser (constraint `fuzz-target-per-surface`: lands before SPEC-003)
- [x] SPEC-003 (shipped on 2026-11-05) — [M] Apply the tolerant-walk policy (`ParseIssue` channel), flipping the affected tests
- [x] SPEC-004 (shipped on 2026-11-05) — [M] Multi-IFD pointer walk (Exif/GPS/Interop/SubIFDs), `IfdKind`, bounded per-tag work (DEC-005)
- [x] SPEC-007 (shipped on 2026-11-05) — [M] ASCII, signed and float tag-value decoders (split out of SPEC-004 at its design)
- [x] SPEC-005 (shipped on 2026-11-05) — [L] Oracle discipline: ExifTool `tools.rs` subset, `parse_fields`, normalisation table, committed records, replay-perturb red-proof, live parity
- [x] SPEC-008 (shipped on 2026-11-05) — [L] `mutate.rs` byte-mutation helpers and the shared mutated-copy `probe.rs` red-proof (split out of SPEC-005 at its design)

**Count:** 8 shipped / 0 active / 0 pending

## Design Notes

- DEC-001 (tolerant-walk), DEC-002 (`&[u8]` core, `Bytes` at the edge) and DEC-003 (provenance class 3) are decision files authored in the design session by the architect, not delegated build specs. They land before the port spec is built.
- Open questions to close inside this stage: ifd.rs test classification was by name/range, not execution (verify by running); CI job-to-script wiring is untraced; ExifTool write-mode exit codes are needed before M1c, not here.
- Gate harness is repo setup, NOT a spec or stage (process, not product). It is a precondition for SPEC-001's green bar and is tracked outside the backlog (template-port session). Reconciliation rule: diff each script against irradiance's before overwriting; irradiance is ahead on the hardened audit/red-proof set (SPEC-011/016/023, PATCH-002), the template (0.6.39) may be ahead elsewhere. Check `gh release list -R jysf/spec-driven-template` (needs sandbox bypass).

## Dependencies

### Depends on
- External: ExifTool installed locally and in CI
- Previous projects: irradiance (read-only source)

### Enables
- The JPEG/container stage, the `exif-audit` CLI stage, the writer stages

## Stage-Level Reflection

- **Did we deliver the outcome in "What This Stage Is"?**
  Yes. The single `exif-core` crate parses bare TIFF/EXIF under a tolerant walk (IFD0 and header fatal, every other bad IFD an omitted-and-reported `ParseIssue`), follows SubIFDs, Exif, GPS and Interop with every IFD labelled by how it was reached, decodes ASCII, signed, float and rational values panic-free and zero-copy, and carries a `tiff_ifd` fuzz target (29 seeds, invariants checked in both directions, 60 s smoke clean on every spec). The ExifTool oracle is in: 47 values over 9 hand-built fixtures plus a seed sweep agree with ExifTool 13.55, the comparator is red-proofed by replay-and-perturb, and a repeatable mutated-copy apparatus shows six deliberately broken copies of the parser get caught. All five Success Criteria hold. Not in this stage by design: the 24 h clean fuzz run (milestone gate), CI wiring, JPEG/PNG/WebP, the writer.

- **How many specs did it actually take?**
  Eight against the six framed: SPEC-001, 002, 006 (fuzz, pulled forward), 003, 004, 005 and 007 and 008 (decoders, and mutated-copy red-proofs, each split out of a larger spec at design). Verify verdicts: approved first time 5 (SPEC-002, 003, 006, 007, 008), punch list 3 (SPEC-001 vacuous policy tests; SPEC-004 issues recorded after the budget halt; SPEC-005 a false "ExifTool exits 0 on every seed" claim). Metered build and verify cost about 2.71M tokens (about $17.85) across 8 specs, plus about 1.03M tokens (about $6.79) of orchestration; roughly 3.7M tokens (about $24.6) in total.

- **What changed between starting and shipping?**
  (1) The fuzz target moved from the oracle spec to its own early spec so it landed with the first parser; (2) SPEC-004 and SPEC-005 were each split at design, which kept every spec at M or L and independently verifiable; (3) cost metering switched from sessions (which cannot report tokens) to metered sub-agents for build and verify; (4) each verify improved the next spec's instructions (iff-invariants in both directions, report every renamed or missing test, test the testers); (5) the oracle found a real ExifTool behaviour (it does not return on a two-hop IFD cycle) and two spec errors of mine (signed GPS needs the Composite selector, and an exit-status claim measured through a pipe), both caught by build or verify before they spread.

- **Carried forward (not blockers):** probe build/run has no wall-clock limit; marker-leak test misses `tests/support/oracle.rs`; no SBYTE oracle selector; `decisions-audit.sh` reads only block-style lists (worked around, upstream fix needed); the `rust-ci-and-fixtures` question (CI jobs, ExifTool pin, `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `lint-red-proof` scripts) and the JPEG-placement question decide what the next stage assumes.
