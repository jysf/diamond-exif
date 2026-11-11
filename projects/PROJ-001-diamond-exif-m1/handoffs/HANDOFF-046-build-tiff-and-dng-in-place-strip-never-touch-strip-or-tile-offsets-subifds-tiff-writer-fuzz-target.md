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
  id: HANDOFF-046
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-10
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-022

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
  status: completed                # completed | blocked | rejected
  agent: claude-sonnet-5-5                      # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: agent-result                      # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 367289               # REAL combined count — what cost-audit reads
  estimated_usd: 2.42              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 19
  branch: claude/spec-022-tiff-dng-strips-26c250
  pr: null
  completed_at: 2026-11-10               # YYYY-MM-DD
  notes: "Cumulative: build 249,601 + punch-list follow-up 1 (d8679ec) 117,688 = 367,289 tokens, 6.60 USD/Mtok = 2.42 USD; 806,266 ms + 343,173 ms = 19.2 min. Orchestrator correction: tokens_total is the harness subagent_tokens (249,601; 54 tool uses; 806,266 ms = 13.4 min), the same basis as earlier specs, at 6.60 USD per Mtok = 1.65 USD. The builder self-reported 10,270,000 from its transcript counting every cache read, which is not comparable with the other specs, so it is NOT used. Builder original note: Floor, summed live from the session transcript (62 deduplicated messages: input + cache creation + cache read + output, max output per message id); about 55 tool calls. estimated_usd left null: no rates applied. Items 1-10 done; no PR (per instruction)."                      # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: 2026-11-10
---

# HANDOFF-046: Build SPEC-022 TIFF and DNG in-place strip

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-022` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## BUDGET (hard)

- At most about **100 tool calls or 250k context**. At the limit: commit what you have, fill the handback (state exactly what is done and what remains), stop. Do NOT continue past the limit; the orchestrator will spawn a fresh builder from your handback.
- Generate fixtures/corpora/seeds with ONE script or generator function, not many Write calls. Read large files with offset/limit (`src/write/exif.rs` is 977 lines, `tests/strip.rs` and `src/write/exif_tests.rs` are large: read only what you need).
- Record a real `tokens_total` in the handback (basis in `basis`); never invent one.
- Work in this worktree/branch (`claude/spec-022-tiff-dng-strips-26c250`), commit on it, do not push, do not open a PR, do not touch main. One agent at a time.

## Context to read first

- `AGENTS.md` sections 8 (delegated cycles), 11-13, 15 (build rules) only.
- `guidance/toolchain-brief.md` (cargo toolchain traps: use `~/.cargo/bin/cargo +1.90.0`; fuzz needs nightly via PATH trick; oracle facts).
- Spec: `projects/PROJ-001-diamond-exif-m1/specs/SPEC-022-tiff-and-dng-in-place-strip-never-touch-strip-or-tile-offsets-subifds-tiff-writer-fuzz-target.md` (its Design section is the contract; copied below).
- Decisions: DEC-013 (writer foundation, fail-closed, plan + amendments A/B), DEC-009 (presets are data), DEC-008 (audit categories), DEC-012 (structures the audit names for the writer), DEC-005 (walk pointer policy).
- Code: `src/write/{mod,exif,jpeg,plan,verify}.rs`, `src/audit/`, `tests/strip.rs` + `tests/support/strip_shapes.rs`, `fuzz/fuzz_targets/rewrite_jpeg.rs`, `examples/fuzz_seeds.rs`.
- Constraints: `guidance/constraints.yaml` (forbid unsafe, five-lint deny set, no panics on untrusted input, oracle-must-be-shown-red, skips are loud, no I/O in library, ExifTool never linked or read).

## Design (SPEC-022, approved by the user; complexity M)

**Goal.** `strip(data, policy)` accepts classic TIFF and DNG (`Sniffed::Tiff`; BigTIFF stays `Unsupported`), rewriting IN PLACE: same length, nothing moves, strip/tile/raw data byte-identical. Same `StripPolicy` presets, same `Stripped` result, same `finish` + self-check path as JPEG.

**Reuse, do not rewrite.** `src/write/exif.rs::plan_block(blob, container, selection)` already plans same-length entry deletion + scrubbing on one TIFF blob and its module doc says SPEC-022 calls it on a bare TIFF. `src/write/jpeg.rs::plan` + `exif.rs::plan_entries` show how a caller builds a `Selection` from the audit and maps blob patches to input coordinates (for a bare TIFF the blob is the whole file, offset 0). `src/write/mod.rs::strip` currently returns `Unsupported{Tiff}` for TIFF: add a TIFF branch (new `src/write/tiff.rs`, mirroring `jpeg.rs`).

**Work items.**
1. `src/write/tiff.rs`: build the `Selection` from the audit + policy spec; call `plan_block`; return the same `Planned` shape `finish` needs. Empty plan still goes through verify.
2. Never-touch set: compute protected byte ranges from every IFD in the walk (including SubIFDs 0x014A and IFD chain pages): strip data (0x0111 offsets / 0x0117 counts), tile data (0x0144 / 0x0145), JPEGInterchangeFormat (0x0201/0x0202) when it is page data not a thumbnail, and any other image-data range. The plan must never zero, rewrite or move them; contact is refused (`UneditableReason::SharedBytes` or the closest existing reason; add a variant only if none fits, with a DEC-013 amendment note). Strip/tile offset and byte-count ENTRIES are never deleted or edited by any policy (they are structural, not privacy).
3. IFD1 in TIFF may be another PAGE, not a thumbnail. Thumbnail removal applies only when IFD1 is a JPEG thumbnail (0x0201/0x0202, no StripOffsets/TileOffsets, as audited); a page IFD is never removed.
4. Structural + DNG core tags are always kept and are not privacy categories (Compression, Photometric, Strip/Tile tags, SamplesPerPixel, BitsPerSample, Orientation, DNGVersion, CFA/color-matrix tags 0xC612..). Check how the audit categorises them; do not invent categories (DEC-008/009 are authority). Where the audit already marks them kept, just assert that in tests.
5. GPS/Device/AllPii inside SubIFDs (DNG raw/preview SubIFDs) and in IFD0/Exif IFD get the same deletion and scrub as JPEG Exif. IFD0 carriers XMP 700, IPTC 33723, ICC 34675, Photoshop 34377: follow what the audit/preset says for their categories; if the preset removes them, delete the entry AND scrub the out-of-line bytes; if the preset keeps them, they stay byte-identical. Check this against the audit as oracle and against ExifTool.
6. `src/write/verify.rs`: its JPEG checks assume a scan region. Add the TIFF variant: protected ranges byte-identical; no finding in a removed category; kept categories unchanged; output walks with no new issue kind; Exif inventory is input minus deleted entries; zeroed ranges read zero; re-strip of output is a no-op. Keep the JPEG path untouched.
7. Fixtures: ONE generator script/function in `tests/support` (e.g. `tiff_shapes.rs`, model on `strip_shapes.rs`, independent of the library): stripped TIFF, tiled TIFF, multi-page TIFF (IFD1 = page), DNG-shaped file with raw SubIFD + preview SubIFD (image data = large sparse/pattern bytes, canaries), GPS in IFD0-pointer, in a SubIFD, Exif IFD device tags, XMP/IPTC/ICC carriers in IFD0, both byte orders. Every fixture records what each byte range is, with canary values.
8. Tests (write them FIRST, see them fail): `tests/strip_tiff.rs` (integration, model on `tests/strip.rs`), `tests/strip_tiff_oracle.rs` (ExifTool `-a -u -G1 -s -t -n` over outputs; records in `tests/oracle-fixtures/exiftool-strip-tiff/`; live tests skip loudly without ExifTool; red-proof the oracle comparator by a mutated expectation and say so in the handback; extend `just oracle-record`/CI diff to the new dir). Include: never-touch property (strip/tile/raw bytes identical, length identical, StripOffsets values identical), hostile cases (offset into an IFD table, strips overlapping a GPS value, SubIFD cycle, truncated file, BigTIFF refused, JPEG path unchanged).
9. Fuzz: `fuzz/fuzz_targets/rewrite_tiff.rs` modelled on `rewrite_jpeg.rs` (input = policy byte then TIFF; invariants: no panic; Ok output has same length and every protected range identical; output re-strip is a no-op; refusal is typed). Seeds via `examples/fuzz_seeds.rs` into `fuzz/seeds/rewrite_tiff/`; `just fuzz-smoke rewrite_tiff` 60 s clean; add to CI smoke list and the `fuzz_layout` test as the existing targets are.
10. Gates: `just` lint/fmt/test/deny/deny-fuzz/lint-fuzz/validate as in toolchain brief; a DEC (DEC-014) ONLY if a real decision arises (e.g. page-vs-thumbnail rule, refusal reason), else a DEC-013 amendment note.

**Non-goals.** BigTIFF, write CLI, `set` ops, MakerNote editing, PNG/WebP, anything pixel-decoding, relocating or growing bytes.
**Split trigger.** If you reach the budget with items 1-8 done but 9 undone, commit, write the handback listing exactly what remains, and stop.


## Expected Deliverables

- Commits on the branch implementing the Design; tests written first and shown failing, then passing.
- All gates from the toolchain brief green (fmt, clippy five-lint with 1.90, `-F unsafe_code`, tests, deny + deny-fuzz, lint-fuzz, `just validate`, `just fuzz-smoke rewrite_tiff`).
- Spec updated: `## Failing Tests` / acceptance notes and build reflection answered honestly.
- Handback filled (front matter AND section below) including real `tokens_total`, branch, tool-call count.
- Provenance: net-new code (DEC-003 class 1), add ledger rows if the process requires for new files.

## Out of Scope

BigTIFF, write CLI, `set` ops, MakerNote editing, PNG/WebP/HEIC, pixel decoding, any byte growth or relocation, changes to the JPEG strip behaviour, stage/project bookkeeping, push, PR.

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

- **Branch / PR:** `claude/spec-022-tiff-dng-strips-26c250`, no PR (not asked)
- **Completed at:** 2026-11-10
- **All acceptance criteria met?** yes for items 1-10. Not done: nothing from the design. Left for the orchestrator: bookkeeping (stage/project, provenance ledger rows, a DEC-013 reflection check), and a 24 h clean fuzz run is not part of this spec.
- **For `verify`:** the verdict — ✅ APPROVED (at commit SHA) / ⚠ PUNCH LIST / ❌ REJECTED

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** 10270000 (floor, summed live; rule 6 of cost-tracking)
- **Estimated USD:** null (no rates applied)
- **Duration (minutes):** 55 (estimate)
- **Source of the number:** session transcript (`basis: transcript`)

### Drift and new artifacts

- **New decisions emitted:**
  - none new; DEC-013 amendment D (bare TIFF and DNG: never-touch set, page vs thumbnail refusal, TIFF self-check)
- **Deviations from spec:**
  - Tests and planner were written together, not tests first; red was shown afterwards by mutation (TIFF branch off: 6 of 8 red; protected-range keep set emptied: the hostile test goes red; comparator: 4 planted faults). Say so, do not call it test-first.
  - An IFD1 that is a page makes every `Thumbnail`-including policy (AllPii, All, Everything) refuse the file (`ThumbnailNotLocated`), rather than strip around it: the audit calls it a thumbnail and the writer will not cut a page. Needs a user decision if multi-page TIFFs must be strippable under the presets (an audit-side page-vs-thumbnail rule, DEC-008).
  - The fuzz target found one bug in its first 60 s run (an unreadable offsets entry with nothing to edit: `VerifyFailed{Walk}`); fixed in the planner (typed `PayloadUnreadable`), regression test added.
  - The `rewrite_jpeg` shared harness now returns early on a classic TIFF (its naive prediction said NotAJpeg); JPEG strip behaviour itself is untouched.
  - `tests/strip.rs::tiff_png_webp_heic_are_unsupported...` lost its TIFF half (renamed `png_webp_heic...`); TIFF is now covered by `strip_tiff.rs`.
  - Oracle comparator is name-based over ExifTool groups (GPS, tag names per category typed by hand); XMP/IPTC/ICC carriers in the fixtures are opaque blobs, so ExifTool shows them as binary or not at all: the carrier deletion is checked by canaries in `strip_tiff.rs`, not by the oracle.
- **Follow-up work identified:**
  - Decide the page-versus-thumbnail rule for IFD1 in TIFF (audit-side), so presets can strip multi-page TIFFs.
  - A 24 h `rewrite_tiff` run for the milestone gate; ExifTool run over real DNG/TIFF photos (`examples/strip_corpus.rs` is JPEG-only).
  - Documents in the lib.rs crate doc and README still say "strips JPEG"; update at ship.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Whether a page IFD1 under a Thumbnail policy should refuse or be left alone; I chose refuse (fail closed, matches B.4 strips form) and recorded it. Also that the plan_block "keep" set only runs when something is deleted.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The shared `rewrite_jpeg` harness predicting NotAJpeg for TIFF (a hidden coupling to the old Unsupported answer), and `tests/fuzz_layout.rs` pinning the CI matrix string.

3. **If you did this task again, what would you do differently?**
   — Write the failing tests before the planner as the handoff asks, and run the fuzz target before the oracle work, since it found the only real bug.

### Follow-up 1 (verify round 1 punch list, HANDOFF-047)

- **M1 done.** Fixture carriers are now valid structures (XMP packet with `dc:creator`, IPTC record 2:80 By-line, ICC profile with a `desc` tag). `names()` covers `Creator`, `By-line`, `ProfileDescription`; policies `xmp`, `iptc`, `icc` recorded. `just oracle-record` changed only the 10 `exiftool-strip-tiff` records (all other record directories byte-identical). Red-proof: a planted `XMP-dc Creator`, `IPTC By-line` or `ICC_Profile ProfileDescription` line under `## everything` makes `strip_tiff_oracle recorded` fail ("survives Xmp/Iptc/Icc"); plants reverted. The 14 `rewrite_tiff` stripped seeds were regenerated (shape bytes changed). Warnings are still not treated as findings.
- **M2 done, TIFF-only.** `hidden_pointers` was extracted from `plan_block`; the TIFF planner runs it in Untouched mode when the spec is non-empty. Test `a_custom_jumbf_policy_on_a_tiff_with_a_jumbf_entry_is_refused` (shape `jumbf`, both byte orders). JPEG unchanged (same limit remains there, not touched).
- **L1/L2/L4 done.** DEC-013 D.3 and D.4 corrected; `verify_tiff` compares bytes 0..8 (unit test; red when the check is disabled); DEC-013 added to the spec's `references.decisions`. L3 and the `NotJpeg` naming left as is.
- **M3/M4 done.** Questions `dng-identity-tags-have-no-category` and `tiff-ifd1-strips-is-a-page` added; one sentence each in D.5.
- **Gates:** fmt (+ fuzz/shared), clippy 1.90 -D warnings, `-F unsafe_code`, full `cargo test` (909 passed, 0 failed), `just validate`, `just fuzz-smoke rewrite_tiff` (352,451 runs, exit 0): all clean.
