---
# Maps to ContextCore handoff.* semantic conventions.
#
# ONE handoff per delegated CYCLE. With build and verify running on different
# agents you get TWO handoffs per spec (HANDOFF-N build, HANDOFF-M verify) —
# `handoff.cycle` is what distinguishes them.
#
# The `handback:` block below is the RETURN path and it is not optional: it is
# how cost gets into the spec without the orchestrator hand-counting anything.
# `just handback-sync SPEC-NNN` reads it and appends the cost session for you.
# Rationale + the full contract: docs/decisions/DEC-013-delegated-cost-handback.md

handoff:
  id: HANDOFF-003
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-05
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-002

project:
  id: PROJ-001
  stage: STAGE-001
repo:
  id: diamond-exif

# ── THE HANDBACK ────────────────────────────────────────────────────────────
# Filled in by the EXECUTING AGENT before it reports done. This is a required
# part of completing the handoff, not a courtesy.
#
# `tokens_total` is the one field the cost gate reads. Report the REAL number
# from your own interface:
#   Claude Code   → run `/cost`
#   API           → the `usage` object (input + output, summed)
#   another agent → whatever your harness reports as total tokens
# If your platform genuinely exposes NO token count, set tokens_total: null AND
# write why in `notes` — then set `cost.metering_source: none` in
# .repo-context.yaml so the gate stops asking. Do not invent a number.
handback:
  status: completed                # completed | blocked | rejected
  tokens_total: 193631               # REAL combined count — what cost-audit reads
  estimated_usd: 1.28              # tokens_total x 6.60 USD/Mtok
  duration_minutes: 8
  branch: worktree-agent-a790d2df452429634
  pr: null
  completed_at: 2026-11-05         # YYYY-MM-DD
  notes: "tokens_total left for the orchestrator (metered from the task result); worktree base was stale and was fast-forwarded to claude/rust-exif-metadata-6351d5"
  synced_at: 2026-11-05
---

# HANDOFF-003: Port the strict TIFF skeleton

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-002` to `claude-sonnet-5-5` (implementer) for the **build** cycle: port irradiance's DNG-stripped, strict TIFF/IFD walker and its hostile-input tests into `exif-core`, with provenance-ledger rows. Policy stays strict (SPEC-003 changes it).

## Context the Receiving Agent Needs

The receiving agent MUST read these before starting work. Keep the
list tight, but don't omit anything necessary.

### Primary

- **Project brief:** `./projects/PROJ-001-diamond-exif-m1/brief.md`
- **Stage:** `./projects/PROJ-001-diamond-exif-m1/stages/STAGE-001-m1a-foundation-tiff-ifd-engine.md`
- **Spec:** `./projects/PROJ-001-diamond-exif-m1/specs/SPEC-002-strict-tiff-skeleton.md`
- **Reviewer brief (OPTIONAL, verify handoffs):** `./guidance/agents/<lens>.md` —
  pick a lens if this review should look at one thing hard. Omit for a
  general review; see `./guidance/agents/README.md`.
- **Toolchain brief:** `./guidance/toolchain-brief.md` — this repo's test
  framework, lint quirks, runtime globals and gotchas. Read it; it exists so a
  cold agent doesn't rediscover them (DEC-004 rule 5).

### Decisions that apply

- `DEC-001` — context only: do NOT implement the tolerant walk here; keep the strict fatal-on-any-bad-IFD behaviour.
- `DEC-002` — `&[u8]` core; `payload` returns `&'a [u8]` tied to the input; no new dependencies.
- `DEC-003` — every ported file gets a `docs/provenance-ledger.md` row (class 3 derived from class 1) naming the irradiance commit; ExifTool is RUN never READ.

### Constraints that apply

Full text in `./guidance/constraints.yaml`: `forbid-unsafe-code`, `panic-free-library` (no `#[allow]` below the crate root except the scoped test module), `no-io-in-library`, `no-new-top-level-deps-without-decision`.

### Prior related work

- SPEC-001 (merged, PR #3): the crate, lint block and ledger header you build on.
- Source to port FROM, READ ONLY, never modify: `~/PSeven/experiments/crustimg_redo_plus/irradiance/src/ifd.rs` (HEAD `2b6b07a`) and the `Error` enum in its `src/lib.rs`. The port map is `docs/porting-brief-diamond-exif.md` §1.1-§1.4 with line numbers.
- Host toolchain traps: see `guidance/toolchain-brief.md` (Homebrew cargo rejects `+toolchain`; Homebrew clippy shadows rustup's; `~/.cargo/bin/cargo +1.90.0` for the 1.90 check).

## Expected Deliverables

*(For a `verify` handoff, replace this block with the verify contract below.)*

- Code changes implementing SPEC-002's Acceptance Criteria.
- All failing tests in SPEC-002 now passing.
- Any new tests required to cover edge cases.
- Commits on your branch (no push, no PR); the architect opens the PR after verify.
- PR description referencing: this handoff ID, the spec ID, the stage
  ID, the project ID, all referenced `DEC-*`, and any new `DEC-*`
  created during implementation.

### If `cycle: verify` — the verify contract

You are **not** the agent that implemented this. Review it cold.

- Acceptance criteria met? Tests actually pass? Build reflection answered
  honestly (*"nothing was unclear"* is suspicious)?
- Decision drift — run `just decisions-audit --changed`.
- Constraint violations; non-trivial choices missing a `DEC-*`.
- For any criterion claiming **runtime behavior** (a component registers, a hook
  fires, a binary resolves on PATH, a config takes effect), confirm the
  *behavioral* surface was exercised — not just the shape validated. That is the
  defect class that escapes (AGENTS.md §12).
- Output exactly ONE of: ✅ APPROVED / ⚠ PUNCH LIST / ❌ REJECTED.

## Out of Scope

Explicit list of what this handoff does NOT include. If the receiving agent
thinks any of these need to happen, they should create a new spec in
the stage's backlog, not expand this handoff.

- Tolerant-walk policy, `ParseIssue`, pointer-tag walking, ASCII/signed/float decoders, oracle or fuzz work (SPEC-003..006).
- Any writer-facing API (encode twins, `Entry` constructors), `cost_the_field`, a new `is_structural` set.
- Editing irradiance, adding dependencies, publishing.

## Return Criteria — how to hand back

**Completing this handoff means filling in the `handback:` front-matter block
above AND the `## Handback` section below.** An unfilled handback is an
incomplete cycle, and `just handback-sync` will tell the orchestrator so.

On success:
1. Fill the `handback:` front-matter — **including a real `tokens_total`**.
2. Fill the `## Handback` section (the reflection is part of it, not optional).
3. Set `handoff.status` → `completed` and `handback.status` → `completed`.
4. Open a PR (build) or return your verdict (verify).

If you cannot complete the task:
1. Fill the `## Handback` section with what was done and what blocked you.
2. Set `handoff.status` → `rejected`, `handback.status` → `blocked`.
3. **Still report your token usage** — blocked work costs money too.
4. Set the spec's `task.blocked: true` and add a question to
   `/guidance/questions.yaml`.

---

## Handback

*Filled in by the receiving agent. The orchestrator does not reconstruct any of
this — it transcribes it. The reflection questions are part of completion.*

### Execution notes

- **Branch / PR:** `worktree-agent-a790d2df452429634` (no push, no PR). Build commit `fa44c05`.
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** yes. Gates run and green: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (23 unit + 4 crate_policy), `cargo rustc --lib -- -F unsafe_code`, `cargo deny check licenses`, `just validate`, `just decisions-audit`; on 1.90.0: clippy 0.1.90 (`~/.cargo/bin/cargo-clippy +1.90.0`, confirmed by `--version`) and `cargo +1.90.0 test` green. The DNG-symbol grep returns only three provenance comments (types.rs, tags.rs, mod.rs headers saying "DNG removed"). irradiance `status --porcelain` is empty.
- Test-first: the test module was written first and `cargo test` failed to compile (114 errors, `Container` undeclared); then the code was added and all passed.
- Environment: the worktree was cut from `50aa761`, which lacks the SPEC-002 design and HANDOFF-003 (they live on `claude/rust-exif-metadata-6351d5`); I fast-forwarded the worktree branch to that branch (`--ff-only` merge) before starting.

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** null here; left for the orchestrator to transcribe from this task's metered result (instructed).
- **Estimated USD:** null (same reason)
- **Duration (minutes):** not measured
- **Source of the number:** harness report (orchestrator)

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:**
  - `Error` variants dropped: `MalformedRationalValue` and `RationalValueOutOfRange`, plus all RAW/develop variants. Proof: `grep -rn "MalformedRationalValue\|RationalValueOutOfRange" src/` returns nothing; in irradiance `ifd.rs` their only producers are the integer-division arm of `uints` (`:1100`) and the `i32::try_from` narrowing in `rationals` (`:1179-1186`), both removed. 11 variants remain, as the spec expected.
  - `Entry` gained a private `value_at: usize` (input offset of its 4-byte value field), and `Entry::parse` takes it. Reason: the spec's `payload` returns `&'a [u8]`, but irradiance's inline branch returned `entry.value.get(..len)`, a borrow of the (Copy) entry, which cannot outlive it. Inline payloads are now a slice of the input at `value_at`; the bytes are identical. The brief's "one-token change" claim (§1.4) does not hold for the inline branch. `Entry` stays `Copy`.
  - `rationals` is two methods, `urationals` -> `Vec<URational>` and `srationals` -> `Vec<SRational>` (sharing a private `rational_chunks`), rather than one method returning both types. Each accepts only its own type (RATIONAL / SRATIONAL).
  - Added `tags::X_RESOLUTION` (282) so the unsigned-RATIONAL tests use a baseline TIFF tag; the SRATIONAL tests use a test-local `EXPOSURE_BIAS_VALUE` (0x9204) const since Exif-IFD tags are SPEC-004's.
  - `Build`: unused helpers (`bytes_word`, `two_shorts_word`, `longs_bytes`) were not ported to avoid dead test code; the rest is verbatim.
  - The error-Display test additionally has an exhaustive, wildcard-free `match` plus an index check so the variant list provably equals the enum.
  - irradiance HEAD is `b3725d4`, not the handoff's `2b6b07a`; `src/ifd.rs` (last changed `cc67ad6`) and `src/lib.rs`'s `Error` (last changed `bb4aab1`) predate `2b6b07a`, so the ported text is the same. Ledger rows record `b3725d44e7dd` plus the file's last-change commit.
- **Test classification:** every ported test behaved as the brief classified it. The only edits were the ones the spec lists: tests that went through `sensor()` (`header_reads_both_byte_orders`, `an_ifd0_offset_of_zero_yields_no_ifds`) now read baseline tags via `required_scalar`/`ifd0()` (the `NoSensorIfd` assertion is gone); `a_readable_type_this_module_does_not_widen_is_typed` swapped `TAG_BLACK_LEVEL` for `IMAGE_WIDTH`. `an_inline_value_is_read_from_the_entry_not_the_file` keeps its name but, after the `value_at` change, reads the value field from the input; the assertion is unchanged.
- **Follow-up work identified:** `Entry::value_at` and the `payload` signature should be noted for SPEC-003/004 (the writer will need an `Entry` constructor that supplies it). The spec's stale "HEAD `2b6b07a`" wording could be refreshed. The toolchain brief could record that `~/.cargo/bin/cargo-clippy +1.90.0` gives clippy 0.1.90 even when `cargo +1.90.0 clippy` picks up Homebrew's 0.1.99.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — The worktree base lacked the SPEC-002 design and HANDOFF-003 (stale branch point). Also the `&'a [u8]` requirement versus the inline-payload borrow: the brief called it a one-token change, but it needs the value field's input offset.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — DEC-002 should say that an `Entry` has to know where its value field sits in the input for inline payloads to be `&'a`. Otherwise no.

3. **If you did this task again, what would you do differently?**
   — Check that the worktree contains the handoff before starting, and decide the `value_at` question up front by reading `payload` against the `&'a` requirement, instead of finding it while editing.
