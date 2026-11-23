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
  id: HANDOFF-083
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-22
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-038

project:
  id: PROJ-001
  stage: STAGE-010
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
  agent: claude-opus-5-5           # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: punch-list              # verify handoffs: approved | punch-list | rejected
  basis: agent-result              # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 261175               # REAL combined count — what cost-audit reads
  estimated_usd: 1.72              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: claude/spec-038-verify
  pr: null
  completed_at: 2026-11-22         # YYYY-MM-DD
  notes: "tokens_total is null: the verifier harness reports no token count to the agent; the orchestrator fills it from the harness result"
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-083: <Task Title — same as the spec's title>

## Delegation Summary

One sentence: `<from_agent>` (acting as `<from_role>`) hands `SPEC-038`
to `<to_agent>` (acting as `<to_role>`) for the **`<cycle>`** cycle.

## Context the Receiving Agent Needs

The receiving agent MUST read these before starting work. Keep the
list tight, but don't omit anything necessary.

### Primary

- **Project brief:** `./projects/PROJ-001-<slug>/brief.md`
- **Stage:** `./projects/PROJ-001-<slug>/stages/STAGE-010-<slug>.md`
- **Spec:** `./projects/PROJ-001-<slug>/specs/SPEC-038-<slug>.md`
- **Reviewer brief (OPTIONAL, verify handoffs):** `./guidance/agents/<lens>.md` —
  pick a lens if this review should look at one thing hard. Omit for a
  general review; see `./guidance/agents/README.md`.
- **Toolchain brief:** `./guidance/toolchain-brief.md` — this repo's test
  framework, lint quirks, runtime globals and gotchas. Read it; it exists so a
  cold agent doesn't rediscover them (DEC-004 rule 5).

### Decisions that apply

- `DEC-NNN` — <one-line summary of why this matters here>
- `DEC-MMM` — <one-line summary>

### Constraints that apply

Check `./guidance/constraints.yaml` for full text. These constraints
apply to the paths touched by this task:

- `constraint-id-1` — <one-line summary>
- `constraint-id-2` — <one-line summary>

### Prior related work

- `HANDOFF-YYY` — <one-line summary, if relevant>
- `PR #NNN` — <link, if relevant>

## Expected Deliverables

*(For a `verify` handoff, replace this block with the verify contract below.)*

- Code changes implementing SPEC-038's Acceptance Criteria.
- All failing tests in SPEC-038 now passing.
- Any new tests required to cover edge cases.
- A PR against `main` from branch `feat/spec-XXX-<slug>`.
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

- ...

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

- **Branch / PR:** `claude/spec-038-verify` (from `claude/spec-038-part-b` at `aabbf69`; compared against `origin/main`); no PR.
- **Completed at:** 2026-11-22
- **All acceptance criteria met?** no: the safety criterion ("ExifTool on the OUTPUT prints no tag the preset removes") fails for one input class (H1), and the preview wording criterion is not done (M1).
- **For `verify`:** ⚠ PUNCH LIST

**Method.** An independent variant grid outside the builders' tables (scratch scripts in a `v038-` scratch directory; `exif_core::strip` called through a tiny Rust binary; ExifTool 13.55 run on every OUTPUT; a class model of my own from DEC-022's policy semantics; leak = a removed class still printed, vanish = a kept class gone). Plus my own RIFF walk (size rule, kept chunks byte-identical and in order, tail, `VP8X` bits) and a `dwebp` pixel decode of input and output, and `sips` (ImageIO) on a sample.

- FourCC discovery: 2,789 codes (every 4-character tag id in `exiftool -listx` plus a hand list of RIFF/AVI/WAV/WebP codes) times 6 payloads (text, `LIST INFO`, XMP, TIFF, `Exif\0\0`+TIFF, ID3), then 1,674 more (all 16 case spellings of every table code, NUL/tab-padded spellings, AVI `##tx`/`##dc`/`##wb`/`ix##` codes) times 7 payloads (adding JUMBF and ICC). Every code that printed a non-`Unknown` tag is in DEC-022's table; no case folding. The one code missing from the table was found by the grid, not the sweep: `RIFF` (H1).
- Grid: 261 inputs x 5 presets (every table code in five positions; kitchen sinks; RIFF size exact/-40/+40/+1/-1/0/4/12/huge/0xFFFFFFFF/odd/smaller than the first chunk; `VP8X` two, not first, 8 and 12 bytes, flags with no chunks, chunks with no flags, alpha and animation bits; XMP with GPS as `XMP ` and `XMP\0`; EXIF with GPS and Orientation, prefixed and bare; 16/17/20 `EXIF` chunks; nesting; odd sizes without pad; overlapping and 0xFFFFFFFF sizes; declared end inside a chunk; form variants; animated; 16 `JUNK` payloads; 65,535/65,536/65,537/70,000 chunks). 1,189 outputs: 0 structural problems (size rule, order, tail, flags), 0 pixel differences (1,109 decodable inputs), every output idempotent and deterministic; leaks only in H1.

**Findings.**

- **H1 (High) A top-level chunk coded `RIFF` is a container ExifTool walks; the strip keeps it under every preset, including `Everything`, and the audit reports the file clean.** ExifTool 13.55 descends into a top-level `RIFF` chunk of any form type (`WEBP`, `WAVE`, `AVI `, even `zzzz`), at any depth (`RIFF` in `RIFF`), before or after the image, and into a second `RIFF....WEBP` appended after the first (the walker reads it as a chunk beyond the declared size). Everything inside prints: `EXIF` with GPS, `XMP `, `ICCP`, `C2PA`, `LIST`, `IDIT`, `JUNK`, `ID3 `. Not walked: `RF64`, `RIFX`, `riff`, a `RIFF` chunk with no form type. Evidence: input `riff([VP8L, chunk("RIFF", "WEBP" + EXIF(GPS, Artist, Make) + XMP + ICCP + C2PA + IDIT + SLLT + ID3 + JUNK)])`, `strip(.., Everything)` is `Ok`, and `exiftool -a -u -G1 -H -s` on the OUTPUT prints `[GPS] 0x0002 GPSLatitude : 51 deg 30' 0.00"`, `[IFD0] 0x013b Artist : NRexifART`, `[IFD0] 0x010f Make : NRexifMK`, `[XMP-dc] - Creator : NRxmp`, `[ICC_Profile] - ProfileCopyright : NRicc`, `[JSON] - Dc_creator : NRc2pa`, `[ID3v2_3] - Artist : NRid3`, `[RIFF] - DateTimeOriginal : NRidit`, `[RIFF] - TextJunk : NRjunk`. A minimal case, `riff([VP8L, chunk("RIFF", "WEBP" + LIST INFO(IART))])`, prints `[RIFF] - Artist : JDrrLIST` after `Everything`, and `riff([VP8L]) + riff([EXIF(GPS)])` prints `[GPS] 0x0002 GPSLatitude` after `Gps`. `exif-audit` on the first file: `status: ok`, `0 findings`. The self-check passes because the audit does not see inside. Fix direction (the builders' call): treat a top-level `RIFF` chunk as a carrier (DEC-022 table row, reader `chunk_class`, audit region, harness and live rows), or refuse; add the appended-RIFF case; the live grid should include it.
- **M1 (Medium) The WebP preview wording was not updated.** The criterion says "CLI and reports unchanged apart from the preview wording for WebP", but `src/report/text.rs` `preview_lines` still prints for a WebP `not strippable: the writer refuses this container, so nothing would be removed`, which is now false (the same defect SPEC-037 verify L3 fixed for PNG). No test pins the WebP wording.
- **M2 (Medium) The live check is narrower than the criterion.** `tests/strip_webp_live.rs` `removed_by` pins only GPS, four serial-number tags and five authorship tags. Nothing checks that `AllPii` removes `Make`/`Model` (CameraModel), timestamps or free text, that `All` removes XMP, or that a kept class did not vanish. My grid checked those (Make, Artist, Orientation, XMP, ICC, C2PA, other chunks, in both directions) and found no failure, but the committed test would not catch one. Completeness of the carrier table is only as good as the builders' candidate list: nothing outside the table is probed, which is how H1 got through.
- **L1 (Low) Malformed `VP8X`: ExifTool's flags contradict the output.** With two `VP8X` chunks, ExifTool reports the last one's flags; the writer patches only the first (DEC-022 clarification 7), so after `All` the output still prints `[RIFF] 0x0000 WebP_Flags : XMP, EXIF, ICC Profile` with no `EXIF`. A `VP8X` of 8 bytes is never patched, so it still prints `WebP_Flags : XMP` after the XMP is gone. This follows the DEC, but the DEC's reason ("the one the reader reports") is not the one ExifTool reports. ImageIO refuses these files before and after.
- **L2 (Low, by design, confirmed live) Residuals.** Under `Gps`, `GpsAndDevice` and `AllPii`, an `XMP ` chunk with GPS still prints `XMP-exif:GPSLatitude` and `Composite:GPSPosition`. Under every preset but `Everything`, a `C2PA` chunk prints `JSON:Exif_GPSLatitude`. Both are stated in the README and DEC-022 and match JPEG and PNG, so they are defensible; a user reading "Gps" will still be surprised. The `JUNK` text rule matched ExifTool on all 16 payload variants (tab, newline, CRLF, UTF-8, 0xFF, DEL, embedded NUL and leading NUL print nothing; spaces, a single byte, text plus NULs, 4 KB text print `TextJunk`). Removing every `LIST` and the `SGLT`/`gsen` split are defensible (over-removal only).
- **L3 (Low) Independence of facts.** The harness table is compared row by row with `tests/support/webp_carriers.rs`, which is a second typed copy; only the gated live test ties that copy to ExifTool. That holds the rows, not completeness (see M2).
- **L4 (Low, notes)** A `LIST` whose size covers the `VP8L` gives an output with no image chunk; this agrees with ExifTool's view of the input, which also sees no image. RIFF `WAVE`/`AVI `/`RIFX` and a lower-case `webp` form are refused as `StripError::NotJpeg` (old naming, no bytes). An odd unknown chunk without its pad byte, followed by more chunks, is refused (`ChunkPastEnd`); ExifTool also loses sync there.

**Variant grid (presets gps/gad/pii/all/every; "ok" = no leak and no vanish, idempotent, deterministic).**

| Group | Inputs | Result |
|---|---|---|
| Every DEC-022 row (33 codes, `EXIF` bare and prefixed, `XMP\0`) x 5 positions | 175 | ok (5 `_PMX` vanish flags are my classifier treating `_PMX` XMP-GPS as `XMP `; checked by hand) |
| Kitchen sinks, RIFF size variants (13) | 15 | ok; declared size = declared - bytes removed inside the window in all 75 outputs (for example, exact 1036 -> 952, +40 1076 -> 992, -40 996 -> 930, 0/4/12/8 unchanged, 0xFFFFFFFF -> 4294967211) |
| `VP8X` variants | 7 | ok; L1 for two-`VP8X` and 8-byte `VP8X` |
| XMP location, EXIF GPS+Orientation | 4 | ok; Orientation kept for gps/gad/pii; ImageIO reads the outputs |
| 16/17/20 `EXIF` | 3 | 16 ok; 17 and 20: gps/gad/pii `Uneditable TooManyExifSegments`, all/every ok |
| Nesting: `LIST` in unknown, `LIST` in `LIST`, 10 `LIST` types, metadata in `ANMF`/`ALPH`/`VP8L` | 6 | ok (ExifTool does not read inside `ANMF`/`ALPH`/`VP8L`) |
| Nested `RIFF` chunk, appended second RIFF | 2 (+13 in a focused probe) | **LEAK under every preset (H1)** |
| Lengths (odd without pad, overlap, 0xFFFFFFFF, end inside a chunk, declared end inside a chunk) | 10 | ok or typed refusal (`ChunkPastEnd`) |
| Forms (truncated, header only, garbage tail, WAVE, AVI, RIFX, `webp`) | 9 | refused or ok as expected; a garbage tail of 20 bytes is refused |
| Animated with `ICCP`/`EXIF`/`XMP ` | 1 | ok; animation bit kept |
| `JUNK` payloads | 16 | ok (rule matches ExifTool) |
| 65,535 / 65,536 / 65,537 / 70,000 chunks | 4 | ok, ok, `TooManySegments`, `TooManySegments` |

**Gates (pinned 1.90.0).** `just lint`, `just lint-fuzz`, `just forbid-unsafe` clean. `just test` with `DIAMOND_EXIF_REQUIRE_ORACLE=1`: 38 suites, 1,427 passed, 0 failed, no SKIP. `just mutation-oracle` green. `just validate` and `just cost-audit` green. `just oracle-record` run twice: git status clean. `just fuzz-smoke rewrite_webp` (722,532 runs) and `webp_riff` (1,972,920 runs), 60 s each, no crash. `fuzz/Cargo.lock` restored. Nothing from `decisions/INDEX.md` committed, and no machine path in any added line.

**Mutations (scratch copy, reverted).** Skipping the RIFF size patch: 1 unit test red. Counting bytes beyond the declared end: 4 red. Clearing `0x10` in place of the EXIF bit: 5 red. Stopping the plan at a null chunk: 2 red.

**Other checks.** `strippable()` is still false for WebP, and a test pins strip working with `strippable()` false. HEIC and GIF still refuse. Reader and audit: `chunk_class` matches the table (case-sensitive; `JUNK` by the text rule), and a non-empty XML carrier counts as a failed block.

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** 261175 (harness subagent_tokens)
- **Estimated USD:** 1.72
- **Duration (minutes):** null
- **Source of the number:** harness report (filled by the orchestrator)

### Drift and new artifacts

- **New decisions emitted:**
  - none (H1 needs a DEC-022 amendment: a top-level `RIFF` chunk row)
- **Deviations from spec:**
  - The WebP preview wording was not changed (M1).
- **Follow-up work identified:**
  - Fix round: H1 (writer, reader, audit, harness and live rows, appended-RIFF case), M1 (wording and a pin), M2 (widen the live judge to every preset's categories and the kept direction).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Nothing blocking. The handoff body is the unfilled template; the orchestrator's message carried the real brief.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — DEC-022's table covers the chunk types ExifTool decodes into tags, but not chunk types ExifTool treats as a container to walk. The design should ask both questions for every container.

3. **If you did this task again, what would you do differently?**
   — I would start the grid with nesting and concatenation. The FourCC sweep, the most expensive part, found nothing new; the cheap nesting probe found the leak.
