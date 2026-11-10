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
  id: HANDOFF-042
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-10
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-021

project:
  id: PROJ-001
  stage: STAGE-004
repo:
  id: diamond-exif

# ── THE HANDBACK ────────────────────────────────────────────────────────────
# Filled in by the EXECUTING AGENT before it reports done. This is a required
# part of completing the handoff, not a courtesy.
#
# `tokens_total` is the one field the cost gate reads. Report the REAL number,
# and say in `basis` how you got it:
#   Claude Code   → your session transcript, measured per docs/cost-tracking.md
#                   "Measuring a cycle": count each message.id once, take its
#                   largest output_tokens, include subagents/ (basis: transcript).
#                   Summing every line over-counts ~2×. `/cost` in the desktop
#                   Code tab reports context size, not spend.
#   API           → the `usage` objects, summed (basis: api-usage)
#   another agent → whatever your harness reports as total tokens
# If your platform genuinely exposes NO token count, set tokens_total: null AND
# write why in `notes` — then set `cost.metering_source: none` in
# .repo-context.yaml so the gate stops asking. Do not invent a number.
handback:
  status: null                     # completed | blocked | rejected
  agent: null                      # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: null                      # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: null
  pr: null
  completed_at: null               # YYYY-MM-DD
  notes: null                      # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-042: Verify JPEG in-place Exif entry deletion with scrub (SPEC-021)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-021` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-021-*.md` (use the exact spec file, not its `-timeline.md`): rules 1 to 14, the fixtures and leak inputs, the oracle, the fuzz plan, the named tests and the 30 mutants are the contract.
- **DEC-013 with its 2026-11-10 amendment (normative; part A ratifies SPEC-020's refusal order, part B is the entry-editing contract), DEC-012, DEC-009, DEC-008, DEC-005, DEC-001:** `decisions/`.
- **Build handoff and Handback:** `HANDOFF-041-build-...md` (claims to verify, not to trust; read the eleven deviations). Open questions the builder raised are in `guidance/questions.yaml`: `allpii-refuses-ifd1-with-a-following-ifd`, `writer-refusals-beyond-the-spec` (`HiddenPointer`, `NestedIfd`, `PayloadUnreadable`, always-on table-overlap check), `partial-policy-refuses-unreadable-exif-for-xmp-icc-iptc`. A fourth (joined stray bytes) was answered by a follow-up (below).
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe; Python reading ExifTool output needs `errors="replace"`; macOS `sed -i ''`; do not leave the shell in a subdirectory).
- **Code under review:** `git diff 6518356..HEAD` (the build `defcd75` and its WIP commits, the orchestrator's cost record `8ecbb29`, and the follow-up `56bd4a6`, `9315047`). Main: `src/write/{exif,mod,plan,jpeg,verify,verify_tests}.rs`, the tiff accessors in `src/tiff/mod.rs`, `tests/strip.rs`, `tests/strip_oracle.rs`, `tests/fuzz_layout.rs`, `tests/support/{exif_layout,exif_records,strip_shapes,strip_records}.rs`, `tests/oracle-fixtures/exiftool-strip/`, `fuzz/fuzz_targets/rewrite_jpeg.rs`, `fuzz/shared/rewrite_invariants.rs`, `fuzz/seeds/rewrite_jpeg/`, `examples/strip_corpus.rs`.
- **Follow-up already made (judge it):** my independent 60 s `rewrite_jpeg` smoke crashed with guard `VerifyAllowed` (`VerifyFailed("mpf-fields")`, harness predicted Ok). Root cause per the builder: removing an XMP segment makes fill bytes meet stray bytes and read as a marker of length 27,760, so the output walks without the MPF index; the library correctly refuses; reproduces on the SPEC-020 library. Fix was harness-only (`walks_as_other_segments`), with a deterministic test and two seeds. Decide whether the excuse is exact or could hide a real library bug.
- **Local photos:** `/Users/jyashinsky/PSeven/experiments/crustimg_redo_plus/_incoming0`. **Privacy rule: read in place, copy nothing, write no derived file (no temp files, no saved ExifTool output; pipes only), record only counts and 8-hex short hashes; never tag values, file contents or paths; run the binary without `--values`.** Public samples may be in the orchestrator scratchpad `.../scratchpad/public/{A,B,C}`.
- Orchestrator's own gate run after the follow-up (all steps exit 0; 2547 tests passed, 0 failed over three runs; five 60 s smokes clean) is evidence, not a substitute: re-run.

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- **Gates:** fmt both graphs and `rustfmt --check --edition 2021 fuzz/shared/*.rs`; `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`; `cargo test --no-fail-fast` on stable and 1.90.0 with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`; `-F unsafe_code`; `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`; `oracle-record` leaves an empty `git status --porcelain`; the five fuzz smokes; release build of `exif-audit`. Confirm `decisions/` and `scripts/` unchanged, no dependency added, no `allow` outside test modules (one `#[allow(dead_code)]` in the fuzz harness is declared), no `unsafe`, no `unwrap`/`expect`/indexing/panic/arithmetic in `src/write/` and the new tiff accessors.
- **Read `src/write/exif.rs` line by line.** The refusal order (TooManyExifSegments, InventoryTruncated, AfterScan and BlockNotClean for every block, then per-block planning); the keep set K and `SharedBytes` (a scrub range touching any kept byte, including kept IFD tables, kept values, kept thumbnails); GPS child removal (pointer deleted from the parent, table and values scrubbed, nested or doubly-pointed IFDs); thumbnail child removal (IFD0 next word to 0; IFD1 table, values and thumbnail bytes scrubbed; `ChainAfterThumbnail`); entry deletion (count minus k, survivors copied in order, next-IFD word, 12k zero bytes; byte order handling; counts at the 16-bit limit; empty-IFD behaviour); MakerNote scrub; the three extra reasons. Every `Ok` path: can an entry the audit reports in a removed category survive, can a kept entry change, can a byte outside the recorded patches change? Every `Err` path: no bytes escape.
- **Try the inputs the builder did not:** entries sorted and unsorted; duplicate tags; a deleted entry whose value is inline vs out of line vs overlapping another entry's value; value offsets pointing into the IFD table of another IFD; a GPS pointer and an Exif pointer to the same offset; IFD1 reached by two pointers; thumbnail bytes inside the MakerNote; count fields near 0xFFFF; IFD offsets at `u32::MAX`; a TIFF header with a bad magic; Exif block of length 6 or 8; big-endian blocks with the same shapes; Exif plus MPF files where the Exif segment is removed or patched; an Exif segment after the scan; 17 Exif segments; Exif-resident Xmp/Iptc/Icc tags under partial and whole policies; `Orientation` under `Gps`, `AllPii`, `All`.
- **Fail-closed and independence.** Look for a path where `Entries` or `Scrubbed` is skipped, short-circuited, or compares the output with itself; check 7(b) exemption applies only when the whole Exif segment was removed; check (g) derives the deleted set from the input, not the plan. Confirm the fuzz harness's naive TIFF reader/editor shares no code with the library (list every library call the harness makes and judge each) and that `walks_as_other_segments` is exact.
- **Oracle discipline.** ExifTool run, never linked, `-a -u -G1 -s -t -n`. Re-measure on at least three public samples and the fixtures; check the declared tag tables state what ExifTool printed, not what the library did; the six red-proofs still go red; re-record the emptied-IFD measurement (rule 9) and judge allow vs refuse. Confirm the three re-recorded SPEC-020 records changed only for the reason stated.
- **Test the testers.** Re-run the 30 mutants for at least 1, 4, 7, 11, 13, 14, 17, 19, 20, 22, 23, 25, 26, 29 and 30. Write mutants the builder did not try, for example: scrub applied before the shift; deletion shifts survivors but forgets the next-IFD word in big-endian only; `SharedBytes` compares against removed instead of kept; `Entries` compares tags but not values; `Scrubbed` checks only the first range; GPS child removal skipped when GPS has no entries; the audit-derived target set replaced by a hard-coded tag list; the 16-segment limit off by one; `BlockNotClean` testing one counter only; `walks_as_other_segments` always true. Report which the suite catches (name the test or guard) and which survive.
- **Fuzz.** Redo one plant against the real libFuzzer `rewrite_jpeg` target from the committed seeds only; run `rewrite_jpeg` for 5 minutes and report any crash with its input hash; check seeds cover every new policy-byte family.
- **Local real-file run (counts only).** Run `examples/strip_corpus.rs` over `_incoming0` under `Gps`, `GpsAndDevice`, `AllPii`, `All`, `Everything` and reproduce the builder's numbers (21 files, 13 JPEGs, 4 with Multi-Picture; `Gps`/`GpsAndDevice` 13 Ok, 0 bytes removed, length equal, scan identical, audit clean; `AllPii` 12 Ok and one `Uneditable(ChainAfterThumbnail)` on hash `a3b66f84`). Pipe a few outputs to `exiftool -` (in memory only) and compare groups, the `Orientation` count and `MPImage` groups with the input's; confirm 0 new warnings.
- **Judge the questions.** For each open question the builder raised, say whether the pinned behaviour fails closed and whether it needs a decision before SPEC-022.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles. A Low finding goes to a builder follow-up via the orchestrator, so give each a concrete reproducer.

## Out of Scope

- Implementing fixes; TIFF/DNG (SPEC-022); keeping Orientation under `All`/`Everything` (`strip-keeps-orientation`); XMP or IPTC editing; changing DEC-013 (report doubts as questions).

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

- **Branch / PR:** [link]
- **Completed at:** YYYY-MM-DD
- **All acceptance criteria met?** yes/no (if no, explain)
- **For `verify`:** the verdict — ✅ APPROVED (at commit SHA) / ⚠ PUNCH LIST / ❌ REJECTED

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** <real number, or null + why>
- **Estimated USD:** <number, or null>
- **Duration (minutes):** <estimate>
- **Source of the number:** `/cost` | API `usage` | harness report | none available

### Drift and new artifacts

- **New decisions emitted:**
  - `DEC-NNN` — <title> (if any)
- **Deviations from spec:**
  - [list]
- **Follow-up work identified:**
  - [any new specs that should be added to the stage's backlog]

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — <answer>

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — <answer>

3. **If you did this task again, what would you do differently?**
   — <answer>
