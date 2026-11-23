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
  id: HANDOFF-079
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-22
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-037

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
  tokens_total: 281545               # REAL combined count — what cost-audit reads
  estimated_usd: 1.86              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: claude/spec-037-verify
  pr: null
  completed_at: 2026-11-22         # YYYY-MM-DD
  notes: "Tokens left null on instruction: the orchestrator records the harness figure for this delegated run. Verify of claude/spec-037-part-b at 8a5bb81."
  synced_at: 2026-11-22
---

# HANDOFF-079: <Task Title — same as the spec's title>

## Delegation Summary

One sentence: `<from_agent>` (acting as `<from_role>`) hands `SPEC-037`
to `<to_agent>` (acting as `<to_role>`) for the **`<cycle>`** cycle.

## Context the Receiving Agent Needs

The receiving agent MUST read these before starting work. Keep the
list tight, but don't omit anything necessary.

### Primary

- **Project brief:** `./projects/PROJ-001-<slug>/brief.md`
- **Stage:** `./projects/PROJ-001-<slug>/stages/STAGE-010-<slug>.md`
- **Spec:** `./projects/PROJ-001-<slug>/specs/SPEC-037-<slug>.md`
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

- Code changes implementing SPEC-037's Acceptance Criteria.
- All failing tests in SPEC-037 now passing.
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

- **Branch / PR:** `claude/spec-037-verify` (from `claude/spec-037-part-b` at `8a5bb81`; compared with `origin/main`; no PR, only this handoff committed)
- **Completed at:** 2026-11-22
- **All acceptance criteria met?** No. The tests the spec names pass, but the safety goal does not hold against ExifTool (H1). The criteria that depend on the carrier list ("opaque carriers ... removed by every preset") are met only for the spellings in the list. The list misses 28 spellings that ExifTool 13.55 decodes.
- **For `verify`:** ⚠ PUNCH LIST. H1 blocks shipping. M1 and M2 need an owner decision or a stated residual. The Low items are optional.

#### Method

This was the first check of the writer against ExifTool. The builders' tests compare the output with this library's own audit and with a harness written by the same agents. I generated a variant grid outside their fixtures with scratchpad scripts. `gen.py` makes 197 inputs. `gen_case.py` makes 384 case-spelling inputs. A separate case script makes 64 inputs for eXIf and zxIf. `gen_jumbf.py` makes one caBX input. I also dumped the 61 ExifTool-recorded `oracle_png::fixtures()`.

Each input went through `exif_core::write::strip` for all five presets, using a separate scratch crate that depends on this branch by path. ExifTool 13.55 then read each input and each output with `-a -u -G1 -s -n`, and `-H` for the evidence lines. Two checks followed:

- **Left behind:** a tag ExifTool still prints for the output although its category is in the preset.
- **Lost:** a tag in a category the preset keeps that was in the input and is missing from the output.

An independent Python check compared each output with its input:

- the inflated IDAT pixels;
- the bytes of `IHDR`, `IDAT`, `acTL`, `fcTL`, `fdAT` and `IEND`;
- no new bad CRC;
- the output chunks are a subsequence of the input chunks (an `eXIf` chunk may differ only in its bytes, at the same length);
- the output is never longer than the input.

macOS ImageIO (`sips`) decoded every output. PIL and pngcheck are not installed.

#### Variant grid (ExifTool 13.55 on every output)

| Shape (inputs) | Runs (5 presets) | Result |
|---|---|---|
| eXIf with full Exif before IDAT, after IDAT, after IEND, with `Exif\0\0` header, bad CRC (6) | 30 | OK. Under Gps, GpsAndDevice and AllPii the entries are deleted in place: GPS is gone and its bytes are scrubbed (the 51/1 rational is no longer in the file), the CRC is valid and there is no ExifTool error. All and Everything remove the chunk whole, so Orientation goes too (documented). |
| eXIf with GPS and Orientation under Gps | 5 | OK: `[IFD0] 0x0112 Orientation : Rotate 90 CW` kept, no GPS, valid CRC, no Error |
| several eXIf (GPS in the second; GPS in the last, after IEND), 16 and 17 eXIf | 20 | OK. With 17, the partial presets refuse with `TooManyExifSegments` (typed). |
| eXIf with a failed or empty block first, then GPS | 10 | Partial presets refuse with `Uneditable BlockNotClean` (typed). All and Everything remove both chunks. No leak. |
| text: 36 keywords (every spelling of Author, Copyright, Creation Time, Software, Artist, create-date, modify-date; Make, Model, parameters, unknown, ...) x tEXt, zTXt, iTXt, compressed iTXt x before IDAT, after IDAT, after IEND (12 files, 432 chunks), plus empty values and a language tag | 70 | No leak and no wrongful loss. Free text and the unmatched spellings are kept by Gps and GpsAndDevice and removed by AllPii and up. Software is kept by AllPii. |
| XMP in iTXt (before IDAT, after IDAT, after IEND), compressed iTXt, tEXt, zTXt, with a language tag, lower-case keyword (8) | 40 | OK. Kept by Gps through AllPii (KeptUnread), removed by All and Everything. |
| iCCP named, unnamed, empty, name only, invalid, after IEND (6) | 30 | OK. Removed only by Everything. |
| tIME of length 0 to 12, before IDAT and after IEND (26) | 130 | OK. Removed by AllPii and up, after IEND included. |
| raw profiles (exif, `raw profile type exif`, APP1, xmp, iptc, icc, icm, 8bim, unknown) and not-raw spellings (`RAW PROFILE TYPE EXIF`, `Raw Profile Type exif`, a double space, no space) x tEXt, zTXt, iTXt, compressed iTXt, plus a zTXt after IEND (53) | 265 | OK. Every audit-raw spelling is removed by every preset. The not-raw spellings are kept as free text by Gps and GpsAndDevice, and ExifTool prints them as plain text (L1 has the over-removal note). |
| zxIf, zxif, exif x 6 encodings (zlib, zlib with header, bare TIFF, length+zlib, flag+length+zlib, 00 00+length+zlib) (18), plus after IEND and with a bad CRC | 100 | OK. Removed by every preset. |
| **mixed-case `eXIf` and `zxIf` spellings (28 x 2 encodings)** | 280 | **LEAK (H1)** |
| tXMP, meTa, seAl with XMP, before IDAT and after IEND; `txmp`, `META`, `mETa` | 60 | OK. ExifTool ignores the off-case spellings. |
| case permutations of iCCP, tEXt, zTXt, iTXt, tIME, tXMP, meTa, seAl, dSIG, cpIp, fRAc, gIFx, before IDAT and after IEND (384) | 1920 | OK. ExifTool decodes only the canonical spellings, and Everything removes all of them. |
| dSIG, cpIp, gIFx, gIFg, gIFt, fRAc, orNT, iDOT, caBX, caNv, ceXt, gdAT, vpAg, prVW, mkBF, mkTS, mkBS, mkBT, cICP, jumb, ... (27) | 135 | OK. The dSIG class goes only with Everything. iDOT, gdAT, vpAg and cICP are structure, kept (documented). |
| caBX holding C2PA `stds.exif` JSON with GPS and a creator (1) | 5 | **GPS kept by Gps through All (M2)** |
| trailer garbage; tEXt after IEND with a bad CRC; text then garbage; eXIf then text after IEND (4) | 20 | OK. The bad-CRC trailer text is removed by AllPii and up. |
| length lie, chunk past EOF, length 0x80000001, 70k chunks, 70k chunks then GPS | 25 | Typed refusals (`ChunkPastEnd`, `BadSegmentLength`, `TooManySegments`). No bytes. |
| missing IEND, kept bad CRC, removed bad CRC, APNG with eXIf (acTL, fcTL, fdAT) | 20 | OK. fdAT and fcTL are byte-identical, and a kept bad CRC is still bad. |
| kitchen sink (11 kinds, both sides of IDAT and IEND) | 5 | OK |
| repo `oracle_png::fixtures()` (61) | 305 | 277 OK and 28 typed refusals, no leak |

Structural check: 1545 outputs, 0 problems. Pixels, structure chunks, CRCs, order and length all held, every output is idempotent, and ExifTool printed no `Error` for any output.

#### Findings

**High**

- **H1. A mixed-case `eXIf` or `zxIf` chunk survives every preset with full Exif, and the audit calls the file clean.** ExifTool 13.55 matches these two chunk types without regard to case. All 32 spellings of `exif` and of `zxif` decode, both as a bare TIFF and as `\0`+length+zlib, each with a `[minor] XXXX chunk should be eXIf/zxIf` warning. The library knows only `eXIf`, `zxIf`, `zxif` and `exif` (`src/png/metadata.rs` `chunk_class`). It leaves the other 28 untouched, Everything included: the output is byte-identical to the input (744 bytes to 744).
  - Example input: `scratchpad/v037-grid/case_in/EXIF-45584946-tiff.png`, a chunk of type `EXIF`. ExifTool on `strip(.., Everything)`:
    - `[ExifTool] - Warning : [minor] EXIF chunk should be eXIf`
    - `[ExifIFD] 0xa430 OwnerName : Owner Secret`
    - `[ExifIFD] 0xa431 SerialNumber : SERIAL123456`
    - `[GPS] 0x0002 GPSLatitude : 51 deg 30' 0.00"`
    - `[Composite] - GPSPosition : 51 deg 30' 0.00" N, 0 deg 7' 0.00" W`
  - The same holds for `zXIf-7a584966-flz.png`, with `[minor] zXIf chunk should be zxIf`.
  - `exif-audit` on these files prints `status: ok`, `0 findings`, exit 0. The audit gap predates SPEC-037, but the strip makes it a removal claim.
  - Cause: DEC-020's SPEC-037 clarification 5 and DEC-021's row "`txmp` ... type bytes are case-sensitive" generalise one measurement (tXMP) to every type. The `carrier-case-sensitive` seed pins the wrong rule.
  - Fix: case-fold `exif` and `zxif` in `chunk_class` (non-canonical `eXIf` spellings become ExifCarrier); correct DEC-020 and DEC-021; update the harness model, the seeds and strip_png's carrier table; add a test that iterates all 32 spellings.

**Medium**

- **M1. The fuzz harness and the test model are independent in code but not in facts.**
  - `fuzz/shared/rewrite_png_invariants.rs` and `tests/strip_png.rs` copy the same carrier list from DEC-021.
  - Their "no category left" and "no carrier left" guards read `exif_core::audit`.
  - The self-check reuses the planner's own helpers (`chunk_class`, `trailer_chunks`, `removes_carriers`, `plan` for idempotence).
  - So H1 passes every guard. `cargo fuzz run rewrite_png` on `fz/everything-EXIF` and `fz/gps-zXIf` runs clean, and the 60 s smoke (637,734 runs) did not find it.
  - No guard rests on a fact this crate did not supply. Until SPEC-039's ExifTool check exists, keep a case-permutation seed and a recorded ExifTool table.
- **M2. Judgement call 3 (`caBX` is `Jumbf`, kept by AllPii and All) keeps GPS.**
  - A caBX chunk with a C2PA `stds.exif` JSON assertion keeps its GPS and creator under Gps, GpsAndDevice, AllPii and All.
  - ExifTool on the output prints `[JSON] Exif_GPSLatitude : 51,30.0N` and `[JSON] Dc_creator : Jane Jumbf Secret`.
  - This matches the JPEG JUMBF precedent. It contradicts clause 4's own principle: a chunk ExifTool decodes into Exif or XMP tags that the library does not read.
  - Owner decision: either treat a JUMBF box as a carrier (which breaks C2PA provenance), or state the residual in README and the preview.

**Low**

- **L1. Over-removal, the safe direction.**
  - `Raw profile type  exif` (double space) and `Raw profile type unknownthing` are removed by Gps, although ExifTool prints them only as text (`PNG:RawProfileTypeExif`, `PNG:RawProfileTypeUnknownthing`).
  - `Raw profile type icc` and `icm` hold no GPS but are removed by Gps, which drops ImageMagick's ICC.
- **L2. AC1 says "every PNG fixture", but the ExifTool-recorded `oracle_png::fixtures()` never go through `strip` in the tests.**
  - I ran all 61 against the five presets: no leak and no structural problem.
  - GpsAndDevice and AllPii seeds exist for only 3 shapes.
- **L3. The PNG preview reason is now false.**
  - The CLI preview still says "not strippable: the writer refuses this container" for PNG. `strippable()` is correctly false until SPEC-039, but the writer no longer refuses.
  - The audit now marks zxIf, tXMP, meTa and seAl files `unreadable` and adds the region names `carrier` and `other_chunk`. This is intended by the DEC-020 clarification, but the README audit paragraph still names only raw profiles.
- **L4. HANDOFF-077's Handback section is template text apart from the token line.**

#### Review items

1. **Acceptance criteria:**
   - Each criterion has a test, and the tests pass.
   - The opaque-carrier criterion is only as complete as the measured list (H1).
   - The fixture coverage gap is L2.
   - Idempotence, I1, refusals, kept bad CRC and the eXIf CRC were confirmed independently above.
2. **DEC-021 and the code agree on all four judgement calls:**
   - Call 1, Custom OtherSegment removes carriers: defensible, and needed for the self-check.
   - Call 2, trailer whole under OtherSegment, Overlap for a kept located chunk: defensible. It follows the JPEG precedent, and Everything never meets the Overlap.
   - Call 3, caBX as Jumbf: see M2.
   - Call 4, carriers go under Gps while plain XMP stays: defensible and fail-safe, because the library cannot read them. It surprises in the over-removal direction (L1), and README states it.
3. **Fuzz harness:**
   - The code does not read the writer.
   - The plants mutate the facts. Each of the 11 guards has at least 2 plants and goes red (test passes).
   - The seeds come from `examples/fuzz_seeds.rs`, and the byte-for-byte pins pass.
   - Independence of facts: M1.
4. **Preview and CLI:**
   - `ContainerKind::strippable()` is still `Jpeg | Tiff`.
   - `strip_works_while_strippable_is_still_false_for_png` pins it.
   - The CLI says not strippable (wording: L3).
5. **Reader and audit changes:**
   - `RegionKind::Carrier` and `OtherChunk` map to `OtherSegment`.
   - A non-empty carrier counts as a failed block.
   - `tIME` moved from the audit's walk to the reader, so a tIME after IEND is now found. ExifTool prints `Trailer:ModifyDate`, so this is correct.
   - The SPEC-036 inventory and category parity tests pass unchanged.
6. **Mutation, on the scratch working tree, each reverted with `git checkout`:**
   - Skip the CRC patch: 8 tests fail.
   - Drop the trailer chunks: 8 tests fail.
   - Lower-case the keyword before categorising: 3 tests fail (self-check, VerifyFailed).
   - Removal range one byte short: 14 tests fail.
   - Bonus, `exif` removed from `chunk_class`: 3 tests fail.
   - All killed.
7. **Gates on the pinned 1.90.0:**
   - `just lint`, `just test` (every suite ok), `just forbid-unsafe`, `just validate`, `just cost-audit` and `just mutation-oracle` all exit 0.
   - `just oracle-record` was run twice, and `git status` was clean both times.
   - `just fuzz-smoke rewrite_png`: 637,734 runs in 61 s. `just fuzz-smoke png_chunks`: 1,614,583 runs in 61 s. No crash.
   - `fuzz/Cargo.lock` was restored afterwards.
   - `decisions/INDEX.md` is not in the diff.
   - The CI matrix, `app.just`, README and `fuzz/Cargo.toml` all list `rewrite_png` consistently.
   - `just decisions-audit --changed` is advisory and lists DEC-013, DEC-020 and DEC-021.

### Cost self-report

- **Tokens (total):** 281545 (harness subagent_tokens)
- **Estimated USD:** 1.86
- **Duration (minutes):** null
- **Source of the number:** harness report (to be transcribed by the orchestrator)

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** none by the verifier. The build deviates from the goal through H1.
- **Follow-up work identified:**
  - The H1 fix round: case-folded eXIf and zxIf, DEC-020 and DEC-021 corrections, model, seeds and tests.
  - The M2 owner decision on JUMBF and C2PA under the non-Everything presets.
  - SPEC-039 should keep the case-permutation grid and the C2PA JSON shape in its ExifTool strip-output corpus.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   - This handoff was still template text, so the brief came from the orchestrator's message. Separately, macOS's case-insensitive filesystem merged my first case-variant fixtures; I re-ran them with hex-named files.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   - A rule that a "case-sensitive" claim about ExifTool must be measured for every type it covers. One measurement (tXMP) became a rule for all types, and that is H1.

3. **If you did this task again, what would you do differently?**
   - Start with the brute-force case-permutation grid on every chunk type in the carrier table. It found the only High in under a minute of ExifTool time.
