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
  id: HANDOFF-062
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-21
  status: completed                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-031

project:
  id: PROJ-001
  stage: STAGE-008
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
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 45
  branch: claude/spec-031-verify2
  pr: null
  completed_at: 2026-11-21         # YYYY-MM-DD
  notes: "tokens_total is null: a sub-agent cannot read its own transcript token counts; the orchestrator replaces it with the harness figure. About 45 tool calls."  # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-062: Verify SPEC-031 PNG read parity, round 2 (fixes for 3 Mediums)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-031` to a cold verifier for a SECOND, focused round. Round 1 (HANDOFF-061, read its handback first) returned PUNCH LIST with 3 Medium and 6 Low findings; the builder fixed them in commit `29df025` (bookkeeping commits after it). You did not write this and you are not the round-1 verifier. Review branch `claude/spec-031-png-parity` against the spec (`projects/PROJ-001-diamond-exif-m1/specs/SPEC-031-*.md`, the file without `-timeline`). Review only; edit nothing except your own handback in this file. Do not push.

## FIRST STEP

The harness may start your worktree from main. Run `git checkout -b claude/spec-031-verify2 claude/spec-031-png-parity` (local branch; do not `reset --hard`) and confirm HEAD contains `29df025`.

## BUDGET (hard)

About 70 tool calls or 180k context. Record `tokens_total: null` with a note if you cannot measure it (the orchestrator replaces it with the harness figure). Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; read `guidance/toolchain-brief.md`. ExifTool 13.55 is installed (run it, never read its source); use `perl -e 'alarm 8; exec @ARGV' -- exiftool ...` for ad hoc runs.

## What to look hard at

1. **Were the three Mediums really fixed?** (M1) `iCCP` with names of 0, 1, 79, 80, 81, 200 bytes: re-measure with ExifTool and confirm `png::read` locates exactly what ExifTool reads; also no-NUL and NUL-without-method-byte and method byte 1. (M2) `iTXt` XMP keyword with language tag and translated keyword variants, both flags: re-measure at least eight variants (including an empty language tag with a non-empty translated keyword, a language tag of one byte, uppercase, and a language tag followed immediately by the end of the chunk) and confirm the routing agrees. (M3) the new test that located region bytes equal the builders' bytes: run your OWN mutants on a scratch copy (rsync the tree elsewhere; do not mutate the branch): at least six: XMP data shifted by one, ICC data the whole chunk, translated keyword NUL not skipped, raw-profile data offset wrong, `Text` data off by one, region offset field wrong. Every one must turn some PNG test red; report survivors and whether each is equivalent or a gap.
2. **Unmeasured shapes again.** Round 1 found its Mediums among shapes outside the fixture list; the builder added nine. Probe at least ten MORE shapes not in the list and report agreement with `png::read` on located metadata (disagreement on located metadata is Medium, warnings only Low). Suggestions: `iTXt` with compression flag 1 and method 1; `iTXt` keyword exactly 79 bytes with a language tag; `tEXt` XMP keyword followed by an empty packet; `zTXt` XMP with method byte 1; `eXIf` with `MM` header and zero IFD entries; `eXIf` whose IFD0 offset points inside the header; raw-profile text with the wrong declared length; raw profile of type `icc`, `iptc` and `APP1`; two `iCCP` chunks; `iCCP` after IDAT; a `Raw profile type exif` keyword in upper case.
3. **The fuzz model after the rule change.** `fuzz/shared/png_metadata_invariants.rs` was rewritten for the new iCCP and iTXt rules: it must still derive from raw bytes only, assert both directions, and fail on a planted fault (try three). Run the `png_chunks` fuzz target for 20 s (the recipe hardcodes 60 s; use its cargo command with `-max_total_time=20`). The two seeds whose parse result changed (`meta-iccp-empty-name`, `meta-xmp-itxt-compressed`) keep their bytes: confirm, and confirm every other seed is byte-identical (`cargo run --example fuzz_seeds` leaves the tree clean).
4. **DEC-018 and the divergence text.** Read the appended "Clarifications from the SPEC-031 verify" section: does it state exactly what the code does? Read the `png-exif-and-raw-exif` entry and the `png-raw-profile-not-decoded` question: is the privacy consequence (an audit or strip trusting the `eXIf` can miss values a reader shows, including raw-only tags such as GPS) stated plainly?
5. **Collateral.** Records: `just oracle-record` twice leaves no diff; only `png-warnings.txt` changed among existing records and 9 were added. No change to JPEG, TIFF or WebP behaviour; only `src/png/metadata.rs` changed under `src/` since `937b15f`.
6. Run `just lint`, `just test`, `just mutation-oracle`, `just validate`, `just cost-audit`, `just forbid-unsafe` yourself.

## Expected Deliverables

Your verdict, exactly one of APPROVED / PUNCH LIST / REJECTED, with findings ranked High, Medium, Low, each with file:line and a concrete failing input. Fill the `handback:` front matter (`verdict`) and the `## Handback` section; set statuses to `completed`; commit only the handback.

## Out of Scope

Fixing anything, further DEC edits, WebP parity (SPEC-034), audit wiring (SPEC-032).

---

## Handback` section below.** An unfilled handback is an
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

- **Branch / PR:** `claude/spec-031-verify2`, cut from `claude/spec-031-png-parity` at `32b9125` (contains `29df025`). No PR, since this is a verify.
- **Completed at:** 2026-11-21
- **All acceptance criteria met?** The three round-1 Mediums are fixed and measured, and every gate is green. One more shape that is not in the fixture list disagrees with ExifTool on located metadata: a text chunk with an empty keyword. That disagreement is in the direction where we see less. Six Lows.
- **For `verify`:** ⚠ PUNCH LIST (1 Medium, 6 Low; no High).

#### Findings

**Medium**

- **M1. A text chunk with an empty keyword: ExifTool shows its value, and we locate nothing.** `src/png/metadata.rs:375-377` (`if end == 0 { return None; }`). The rule comes from DEC-018:56 ("an empty keyword ... BadTextChunk with no region (nothing to locate)"). `tests/png_metadata.rs:299` and the fuzz model (`fuzz/shared/png_metadata_invariants.rs:139`, `.filter(|&k| k > 0)`) pin it. Nobody measured this rule against ExifTool. Failing inputs, measured with ExifTool 13.55 `-a -G1 -s`:
  - `tEXt` with data `\0John Doe` gives `[PNG] Tag : John Doe`.
  - `zTXt` with data `\0\0` plus zlib(`secret value`) gives `[PNG] Tag : secret value` (inflated).
  - `iTXt` with data `\0\0\0\0\0secret value` gives `[PNG] Tag : secret value`.

  In all three cases `png::read` records `BadTextChunk` and no region. This is readable text that a reader displays, so an audit misses it and a strip driven by regions leaves it. It is the same class as round 1's M1: a malformed but locatable chunk. Fix it the way the long keyword was fixed: locate it as `Text` with an empty keyword and the issue, and update DEC-018:56, the model and `png_metadata.rs:299`. Or pin it as a divergence with a question. Add fixture `png-text-empty-keyword` and the `zTXt`/`iTXt` variants.

**Low**

- **L1. Chunks we cannot parse are still exposed by ExifTool as raw binary.** This covers text with no NUL, `zTXt` with no method byte, an `iTXt` header truncated after the flag or at the language tag or translated keyword, and `iCCP` with no method byte or only a NUL. In each case ExifTool prints the whole chunk as `PNG:TextualData`, `CompressedText`, `InternationalText` or `ICC_Profile` (Binary data N bytes), and we locate nothing. Two inputs show it:
  - `iTXt` `Author\0\0\0en\0John Doe secret` (no NUL after the translated keyword) gives `PNG:InternationalText (Binary data 27 bytes)`.
  - `iCCP` `diamond\0` (no method byte) gives `PNG:ProfileName = diamond` plus `ICC_Profile` of 8 bytes. Ours has no region, so the comparator's own `PNG:ProfileName` selector would disagree. This shape is not a fixture or a divergence.

  The value is opaque (it needs `-b`), so this is Low. A strip that relies on regions still keeps these bytes. Round 1 wrote "language tag and no NUL (both nothing)", which holds for the selector set only.
- **L2. One DEC-018 clarification sentence is not exact.** `decisions/DEC-018-...md:101` says ExifTool "prints a name for those" (no NUL, or no method byte). With a true no-NUL `iCCP` (`diamondABCDEFG`), ExifTool prints NO name, only `PNG:ICC_Profile (Binary data 14 bytes)`. The name appears only in the no-method case. The code is right; the sentence is wrong.
- **L3. No record pins that ExifTool reads the ICC when the name is empty.** `png-iccp-name-empty` records `-` for `PNG:ProfileName`, and no selector records the ICC header. I measured `ICC-header:ProfileFileSignature = acsp`. If a later ExifTool stopped reading it, nothing would turn red. Round 1's optional `ICC-header:ProfileFileSignature` selector would close this.
- **L4. The fuzz seeds do not exercise two of the new rules.** These planted library faults leave `tests/fuzz_layout.rs` green:
  - f3: restore the 80-byte search limit for the `iCCP` name, `src/png/metadata.rs:526`.
  - f4: key `tagged` on the translated keyword instead of the language tag, `:413`.

  `png_oracle` and `png_metadata` catch both. The model's own rules are correct (f1 and f2 below turn it red). Seeds with an 80-byte `iCCP` name and with a language-tagged `iTXt` XMP would close the gap.
- **L5. Warnings only: we raise no issue in four cases where ExifTool warns.**
  - `iCCP` method 1: "Unknown compression method 1 for iCCP".
  - XMP in `iTXt` with flag 1 and method 1, or in `zTXt` with method 1: "Unknown compression method 1 for XML:com.adobe.xmp".
  - An empty XMP packet (`<?xpacket begin...?><?xpacket end="w"?>` in `tEXt`): "[minor] Empty XMP". Here we also report a readable `Xmp` region while ExifTool has no CreatorTool (round 1's L3 rule).
  - An `eXIf` whose IFD0 offset is 4: ExifTool shows `PNG:eXIf` as binary.

  All of these locate the same thing on both sides.
- **L6. Mutant v6 survives every PNG test.** v6 sets the `Xmp` region `offset` to `offset + 1` (`src/png/metadata.rs:493`). `png_oracle`, `png_metadata` and `png_walk` stay green; only the fuzz model in `tests/fuzz_layout.rs` catches it (3 failures, all within `just test`). It is not a gap in the suite. `located_region_bytes_equal_what_the_builders_wrote` compares kind, compression, name and data, but not the offset.

Note, not a finding: when `Raw profile type exif` sits in an `iTXt` with a language tag, ExifTool does NOT decode it (`PNG:RawProfileTypeExif` binary). We route it to `RawProfile` either way. That is harmless today because nothing is decoded, but any decoding in SPEC-032 should apply the language-tag rule.

#### Focus-area results

1. **The three Mediums are fixed.** I measured these with ExifTool 13.55 on bytes from the `oracle_png` builders, through a scratch example on an rsync copy.
   - **M1 (`iCCP` name).** Names of 0, 1, 79, 80, 81 and 200 bytes all give ExifTool the ICC header (`acsp`, `mntr`). Every one prints `ProfileName` except the empty name. Ours locates `Icc` for all six, with `BadIccpChunk` for 0, 80, 81 and 200. That agrees.
     - NUL without a method byte: ExifTool shows the name plus 8 bytes of binary, and we locate nothing (L1).
     - True no NUL: ExifTool shows 14 bytes of binary and no name (L2).
     - Method byte 1: both locate it; ExifTool warns and we do not (L5).
   - **M2 (`iTXt` language tag).** I measured 13 variants, and all agree on routing. `Xmp` with CreatorTool:
     - flag 0 or 1 with an empty tag and translated keyword `XMP Daten`
     - flag 0 with both empty (control)

     `Text` with ExifTool's `PNG:XMLcomadobexmp`:
     - tags `x` (flag 0 and 1), `EN`, `" "` (a single space)
     - `en-US` with translated keyword `XMP` (flag 0 and 1)
     - flag 2 with `en`
     - `en\0\0` and an empty body

     The tag runs to the end of the chunk with no NUL (`en`, and `en\0`): ExifTool shows only `InternationalText` binary, and we record `BadTextChunk` with no region (L1).
   - **M3 (region bytes).** My mutants were applied by line on the scratch copy.

     | Mutant | `png_oracle` | `png_metadata` | `png_walk` |
     |---|---|---|---|
     | v1 XMP data shifted by 1 (`:495`) | **red** (located_region_bytes) | red | green |
     | v2 ICC data = whole chunk (`:544`) | **red** | red | green |
     | v3 translated-keyword NUL not skipped (`:407`) | **red** | red | green |
     | v4 raw-profile data shifted by 1 (`:504`) | **red** | red | green |
     | v5 `Text` data shifted by 1 (`:513`) | **red** | red | green |
     | v6 `Xmp` offset + 1 (`:493`) | green | green | green; full suite **red** (`fuzz_layout`, L6) |
     | v7 `tagged` needs 2 bytes of tag (`:413`) | green | red | green |
     | v8 `iCCP` name of 79 is bad (`:536`) | green | red | green |
     | v9 empty `iCCP` name raises no issue (`:536`) | **red** (warning table) | red | green |
     | v10 `Text` offset + 1 (`:510`) | green | red | green |

     Every mutant turns some test red. v1 to v5 are caught by the new test. v6 survives the PNG tests only (L6). v7, v8 and v10 are caught by unit tests, which is expected for a 1-byte tag (no `x` fixture), an issue-only fault and an offset.
2. **Unmeasured shapes: I probed 27 shapes beyond the fixture list.** These agree on located metadata:
   - `iTXt` XMP with flag 1 and method 1 (both compressed or binary XMP).
   - `iTXt` keywords of 79 and 80 bytes with a language tag (both `Text`, with `BadTextChunk` at 80).
   - `tEXt` XMP with an empty packet (both locate it; L5).
   - `zTXt` XMP with method 1.
   - `eXIf` `MM` with zero entries (no values on either side).
   - `eXIf` with IFD0 at offset 4: alone, then with a good block before and after. Values agree in all three (`GoodCam` from the good block).
   - Raw-profile exif with a declared length too large and too small (ExifTool decodes with "wrong size"; we locate `RawProfile`).
   - Raw profiles of type `icc`, `iptc` and `APP1`. ExifTool decodes the ICC, `IPTC:ObjectName` and `IFD0:Make`; we locate `RawProfile` and do not decode, which is the `png-raw-profile-not-decoded` class.
   - `Raw profile type XMP` and `EXIF` in upper case (ExifTool does not decode them; we locate them).
   - Two `iCCP` chunks.
   - `iCCP` after IDAT.
   - An empty-named `iCCP` followed by a named one.
   - Two XMP chunks.
   - A tagged XMP chunk followed by a plain one.
   - `eXIf` plus a raw profile with tags found only in the raw profile.

   These disagree: an empty keyword in `tEXt`, `zTXt` and `iTXt` (M1), and the parse-failure chunks (L1).
3. **The fuzz model** still derives everything from raw bytes: its own framing, keyword, `iTXt` header and name parsing. It takes only the walk outcome from the library and compares field by field in both directions. I planted library faults:
   - f1, the `tagged` check removed: **red** (`png_harness_holds_on_every_seed_prefix_and_flip`, `png_metadata_seeds_are_pinned...`).
   - f2, the empty-name `iCCP` issue removed: **red**.
   - f3, the old 80-byte name limit: green (L4).
   - f4, `tagged` keyed on the translated keyword: green (L4).

   `cargo +nightly fuzz run png_chunks ... -max_total_time=20` ran 695,770 runs in 21 s with no crash (cov 1160, exit 0). `cargo run --example fuzz_seeds` leaves `git status` empty. `git diff 937b15f..HEAD -- fuzz/seeds` is empty, so every seed is byte-identical, including `meta-iccp-empty-name` and `meta-xmp-itxt-compressed`.
4. **DEC-018 and the divergence text.** The `iTXt` clarification states exactly what the code does. The `iCCP` clarification matches the code, but one sentence about ExifTool is inexact (L2). The `png-exif-and-raw-exif` right side (`tests/support/oracle.rs:518`) and `png-raw-profile-not-decoded` (`guidance/questions.yaml:1422-1429`) now state the privacy consequence plainly. I confirmed the "tags only in the raw profile" claim: with `eXIf` Make `AlphaCam` and a raw profile holding Make `RawCam`, Software and Artist `Jane Doe`, ExifTool lists `IFD0:Make` twice plus the raw-only Software and Artist.
5. **Collateral.** Since `937b15f`, the only file changed under `src/` is `src/png/metadata.rs`. Among the existing records only `png-warnings.txt` changed, and 9 records were added: `png-exif-xmp-iccp`, `png-iccp-name-79/80/empty`, `png-text-no-nul`, `png-xmp-itxt-lang`, `-lang-compressed`, `-lang-translated` and `-translated`. `just oracle-record` run twice leaves `git status` empty.
6. **Gates.** I ran them with `RUSTUP_TOOLCHAIN=1.90.0`, `~/.cargo/bin` first and `DIAMOND_EXIF_REQUIRE_ORACLE=1`. All exited 0: `just lint`, `just test` (no failures), `just mutation-oracle` (39 passed, 1 ignored), `just validate` (34 artifacts), `just cost-audit` and `just forbid-unsafe`.

### Cost self-report

- **Tokens (total):** null. A sub-agent cannot read its own transcript token counts; the orchestrator replaces this with the harness figure.
- **Estimated USD:** null
- **Duration (minutes):** about 45
- **Source of the number:** harness report (to be filled in by the orchestrator)

### Drift and new artifacts

- **New decisions emitted:** none
- **Deviations from spec:** none. This was review only. The probes and mutants ran on an rsync copy in the scratchpad, never on the branch, and `src/png/metadata.rs` on the branch is unchanged.
- **Follow-up work identified:**
  - Decide M1 (locate an empty-keyword text chunk, or record a divergence) and add `png-text-empty-keyword` fixtures.
  - Decide whether chunks we cannot parse (L1) should be located as opaque regions so the strip removes them (SPEC-032 or strip).
  - Fuzz seeds for an 80-byte `iCCP` name and a language-tagged `iTXt` XMP (L4).
  - An ICC-header presence selector (L3).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   Nothing material. The probe list was useful. Each class of "nothing to locate" rule in DEC-018 still needed its own ExifTool check.
2. **Was there a constraint or decision that should have been listed but wasn't?**
   DEC-018's "nothing to locate" cases (empty keyword, no NUL, truncated header) were never measured. ExifTool exposes every one of them, as a readable tag or as binary.
3. **If you did this task again, what would you do differently?**
   I would probe every "None" branch of the parser against ExifTool first, because that is where both rounds found their Mediums.
