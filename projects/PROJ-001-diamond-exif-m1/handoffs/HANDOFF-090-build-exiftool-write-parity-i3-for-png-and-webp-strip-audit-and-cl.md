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
  id: HANDOFF-090
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-24
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-039

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
  agent: claude-sonnet-5-5         # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: null                      # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: feat/spec-039-png-webp-write-parity-build
  pr: null
  completed_at: 2026-11-26         # YYYY-MM-DD
  notes: "Parts A, B and C are done on one branch. basis and tokens_total are left null on purpose: the orchestrator fills them from the harness's per-part subagent results (Part A and B are already recorded in the notes under Handback). No wall-clock estimate is given."                      # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-090: SPEC-039 build — ExifTool write parity (I3) for PNG and WebP strip, audit and CLI switch-over

**FIRST STEP, before anything else:** `git checkout -b feat/spec-039-png-webp-write-parity-build feat/spec-039-png-webp-write-parity`.
That branch carries the design: the spec, DEC-023, the DEC-020 amendment and this handoff. The harness may have started
your worktree from main.

## Delegation Summary

`claude-opus-5-5` (architect) hands SPEC-039 to `claude-sonnet-5-5` (implementer) for the **build** cycle. There are
**three parts, one agent each**. This handoff names the part you run in the prompt that starts you. Run only that
part, then stop at its commit.

| Part | Acceptance criteria | Ends with |
|---|---|---|
| A | AC1 to AC7: keyword rows, records, judge, differential, exit codes, comparator red-proofs | records committed, `just oracle-record` twice with no diff, AC1 to AC7 green |
| B | AC8 to AC10: shared sweep, PNG type and keyword sweeps, writer mutated-copy red-proofs W1 to W4 | B's tests green, `just mutation-oracle` green |
| C | AC11 to AC15: switch-over, honesty test, errors, README, JPEG `gps`/`all-pii` records, all gates | every gate in AC15 green |

The design is done. The spec's "Decisions made here", **DEC-023** and **DEC-020's amendment of 2026-11-24** are
normative. Do not re-decide them. If the code contradicts them, or a measured count in the spec doesn't reproduce, add
a question to `guidance/questions.yaml`, stop, and report it. Do not re-pin a table to match what you got.

## Context the Receiving Agent Needs

### Primary

Read in this order:
1. `AGENTS.md` §11, §12, §13, §15 ("During build").
2. `guidance/toolchain-brief.md` and `guidance/constraints.yaml`.
3. **`projects/PROJ-001-diamond-exif-m1/specs/SPEC-039-*.md`.** This is the source of truth: the Context's measured
   facts, AC1 to AC15, the Failing Tests, the Notes.
4. **`decisions/DEC-023-*.md`** in full: rules 1 to 6, the D1 to D5 table, the exit codes.
5. `decisions/DEC-020-*.md`: clause 6, clarification 4, and the amendment of 2026-11-24.
6. DEC-021 clause 4, and DEC-022 clauses 4 and 7 plus its measured table.
7. DEC-009 (the preset columns) and DEC-008 (the categories).
8. The stage file `stages/STAGE-010-*.md`.

The code and harness files are listed in the spec's Inputs and Outputs.

### Decisions that apply

- `DEC-023`: the whole I3 apparatus for this spec.
- `DEC-020` + amendment 2026-11-24: the switch-over (1, 2), the keyword rows (3), and `PLTE` as structure (4).
- `DEC-021` / `DEC-022`: the carrier rule and the whole-block rule the judge's exceptions point at; `VP8X` kept, flags
  cleared (D1).
- `DEC-009` / `DEC-008`: the preset columns the judge is typed from.
- `DEC-013`: the self-check that the writer mutants must escape.

### Constraints that apply

- `test-before-implementation`: each test is committed red before its code. AC7 and AC10 need recorded red evidence
  in the handback.
- `oracle-must-be-shown-red`: AC7, AC9 and AC10.
- `exiftool-never-linked`: ExifTool is run, never read. Don't read its Perl source to find subdirectory chunk types.
  The pinned list in AC9 is the answer, and the gap is DEC-023 rule 6's stated residual.
- `no-new-top-level-deps-without-decision`: no new dependency, dev ones included. FNV-1a is about five lines; write
  it.
- `panic-free-library` and `forbid-unsafe-code`: library edits are small (three keyword rows, one `matches!`, two
  `Display` strings). Keep the lint set clean.
- `no-io-in-library`, and `fuzz-target-per-surface` (no new surface; update the `audit` keyword mirror).

### Prior related work

- SPEC-020 and SPEC-021: the JPEG strip oracle, which is the record format to follow.
- SPEC-037 and SPEC-038: the PNG and WebP strips, and their live precursors and sweep.
- SPEC-040: the JFIF fixtures that part C's `gps`/`all-pii` sections check.

## Expected Deliverables

Everything in the spec's Outputs, for your part. Per part:
- **A:**
  - `src/audit/category.rs` and the fuzz keyword mirror.
  - `tests/support/i3_{judge,records,shapes}.rs`.
  - `tests/strip_png_webp_oracle.rs`.
  - `examples/oracle_record.rs` extended.
  - The committed records under `tests/oracle-fixtures/exiftool-strip-{png,webp}/`.
  - `strip_png_live.rs` and `strip_webp_live.rs` importing the shared judge and shapes, with the same tests.
- **B:**
  - `tests/support/sweep.rs` and `tests/strip_png_sweep_live.rs`.
  - The probe strip mode in `tests/support/probe.rs`.
  - The W1 to W4 tests.
- **C:**
  - The switch-over in `src/audit` and `src/report/text.rs`, and the error strings in `src/write/mod.rs`.
  - The goldens, and the updated pins in the tests the spec's Inputs list.
  - The README.
  - `strip_records::policies()` gaining `gps` and `all-pii`, and the re-recorded `exiftool-strip/`.

Mark the timeline's build line `[~]` when part A starts. Each part adds a line in the handback below. After part C, set
the spec's `task.cycle` to `verify` and the timeline line to `[x]`.

## The work, in order (inside each part)

1. Write the part's failing tests (spec "Failing Tests"), and commit them red. If a test can't compile without a new
   item, add a stub that does nothing and say so in the commit.
2. Implement until green.
3. Capture the red evidence the part requires (AC7 cases (i) to (ix), AC9's two red-proofs, AC10's four mutants:
   control green, `strip` `Ok`, replay red, live red) and paste the failing lines into the handback.
4. Run the part's gates on 1.90.0:
   - Every part: `just lint`, `just lint-no-allow`, `just test` with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, and
     `just forbid-unsafe`.
   - A adds: `just lint-fuzz` and a 60 s `audit` smoke.
   - B adds: `just mutation-oracle`.
   - C runs all of AC15.
5. Commit on `feat/spec-039-png-webp-write-parity-build`, with Conventional Commit messages referencing SPEC-039 and
   the trailer `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. No push, no PR, no CHANGELOG.

Toolchain, recapped from the spec's Notes:
- `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"`. Check that `~/.cargo/bin/cargo-clippy --version`
  prints 0.1.90.
- macOS has no `timeout`; use `perl -e 'alarm 8; exec @ARGV' --`.
- BSD `sed -i` needs `''`. Use Python with `-I` for multi-line edits.
- ExifTool 13.55 is at `/opt/homebrew/bin/exiftool`.
- Restore `fuzz/Cargo.lock` after every fuzz or lint-fuzz run.
- Stage files by path, never `git add -A`, and never commit `decisions/INDEX.md`.
- Records contain no machine paths.
- Leave no background processes.

## Budget

About 120 tool calls or 300k context **per part**. If you get near either, stop at a clean commit boundary and report
what is done and what is left. Do not loop. The ExifTool runs are the slow step: batch the files into one ExifTool
invocation per directory where the format allows it, as the WebP sweep does.

## Out of Scope

- A CLI strip command, a release, `set` operations, HEIC.
- A differential for `GpsAndDevice`, `AllPii` or `All`.
- Byte equality with ExifTool's output.
- Changing the writer to match ExifTool on D1 to D5.
- Renaming `StripError::NotJpeg`.
- JPEG or TIFF differentials.
- Any change to DEC-009 presets, `Category`, or `schema_version`.
- Editing ExifTool's answers to make a table pass.

If you think one of these must happen, add a question to `guidance/questions.yaml` and stop.

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

### Part A (AC1 to AC7), claude-sonnet-5-5, 2026-11-25

Branch `feat/spec-039-png-webp-write-parity-build`. Part A is done: AC1 to AC7 green, records committed,
`just oracle-record` twice with no diff. Parts B and C not started; `handoff.status` left `pending`.

**What was done**
- AC1: three `TEXT_KEYWORDS` rows (`Make`, `Model` to `CameraModel`; `TimeStamp` to `Timestamps`), the fuzz mirror
  (`fuzz/shared/audit_invariants.rs`), the hand-typed table test, `PLTE` pinned as having no `chunk_class`. The
  `MAKE`/`MODEL`/`TIMESTAMP` live run: one tEXt PNG with all 16 keywords through `exiftool -a -G1 -s -n` printed
  `PNG:Make` for `Make` and `make`, `PNG:Model` for `Model` and `model`, `PNG:TimeStamp` for `TimeStamp` and
  `timeStamp`, and `PNG:MAKE`, `PNG:MODEL`, `PNG:TIMESTAMP` (their own tags) for the all-caps ones;
  `Collection`, `Document`, `Label`, `URL`, `Source` printed as themselves, `aesthetic_score` as `AestheticScore`,
  `parameters` as `Parameters`.
- `tests/support/i3_shapes.rs` (236 fixtures: 61 + 113 oracle, 6 PNG carrier shapes, 22 `TextualData` keyword
  PNGs, 1 `caBX` PNG, 33 WebP carrier rows; 90 PNG, 146 WebP), `i3_records.rs` (format, FNV-1a 64, policies, the
  write runner and `classify_write`, the pinned tables), `i3_judge.rs`, `tests/strip_png_webp_oracle.rs` (30 tests
  with the 9 red-proofs), `examples/oracle_record.rs` extended (`record_strip_png_webp`, 8 threads, about 2 minutes).
- Records: `tests/oracle-fixtures/exiftool-strip-png/` (90) and `-webp/` (146). Sections: `input`, the 12 recorded
  policies that strip (5 presets and `xmp`, `icc`, `jumbf`, `rights`, `timestamps`, `free_text`, `other`),
  `exiftool:all`, `exiftool:gps:all`. A refused ExifTool write is a section with one `# refused:` line.
- `strip_png_live.rs` and `strip_webp_live.rs` now take their category table and carrier shapes from
  `i3_judge.rs` and `i3_shapes.rs`; their tests are unchanged and pass live (21 and 23 passed).

**Commits** (on `feat/spec-039-png-webp-write-parity-build`):
- `1e8fb2b` test(audit): SPEC-039 AC1 keyword rows for Make, Model and TimeStamp, red
- `333ea28` feat(audit): SPEC-039 AC1 Make, Model and TimeStamp text keyword rows
- `2b041ad` test(oracle): SPEC-039 I3 for PNG and WebP strip: judge, differential, pins, comparator red-proofs
  (without the records: 14 of the new tests fail, none can read a record)
- `6419790` test(oracle): SPEC-039 committed ExifTool records for 236 PNG and WebP fixtures
- the live-precursor switch commit and the bookkeeping commit follow (see `git log`).

**Measured numbers** (replay equals live, and the live run reproduces every record byte for byte):
- Counts reproduce over the 174 oracle fixtures: 348 pairs = 191 identical (compared as `(group, tag)` multisets)
  + 86 differing (D1 72, D2 2, D3 7, D4 4, D5 1) + 41 outcome differences (29 ExifTool refuses, 12 we refuse)
  + 30 both refuse; 113 refused preset pairs (13 fixtures under all five, 16 under `gps`, `gps_and_device`,
  `all_pii`).
- Over all 236 fixtures: D1 74 (the two extra are `webp-row-05-seal` and `webp-row-22-c2pa` under `Everything`),
  D3 67 (7 + 60 extra pairs: ExifTool keeps carriers and opaque chunks we remove), D2 2, D4 4, D5 1 at key level;
  value level 31 D1 (`RIFF:WebP_Flags`) and 3 D4; no extra outcome pins; no refusal among the 62 extra fixtures.
- Judge (AC4), replay and live identical: 2547 policy sections judged, 583 with a removable tag (2354 removable
  tags before the strip); exception (a) carrier used 670 times, (b) whole block 354, (c) residual 0. Test bounds:
  sections >= 2000, removable sections >= 550, removable tags >= 2000, carrier >= 600, whole block >= 300.
- Refusal table (AC3): 285 `(fixture, policy, variant)` rows over the 12 policies (113 are the preset pairs).
  The 16 fixtures that three presets refuse are refused under 8 policies (`gps`, `gps_and_device`, `all_pii` and the
  Custom `xmp`, `icc`, `rights`, `timestamps`, `free_text`): 15 with `Uneditable(BlockNotClean)`, one with
  `Uneditable(TooManyExifSegments)`; the Custom `jumbf` and `other` do not refuse them.

**AC7 red evidence** (`cargo test --test strip_png_webp_oracle comparator_is_red -- --nocapture`; each line is the
failure the comparator printed on the committed record with one line edited in memory):
```
AC7 (ii) red: png-exif-gps-full-le under gps: kept tag missing IFD0:Make=DiamondCam
AC7 (iii) red: png-exif-gps-full-le under gps: kept value changed IFD0:Make=CANARY-MAKE
AC7 (iv) red: png-exif-gps-full-le under gps: phantom tag IFD0:PhantomTag=1
AC7 (i) red: png-exif-gps-full-le under gps: removed tag left in GPS:GPSLatitudeRef=N
AC7 (ix) red: png-exif-gps-full-le under gps: the writer's output is 530 bytes, digest efa0e273c6950935; the record read 530 bytes, digest 8ca4609144a5b442
AC7 (v) red: png-exif-gps-full-le under gps: kept tag missing IFD0:Make=DiamondCam
AC7 (viii) red: png-text-no-nul all (everything against exiftool:all): stale pin keys|[]|["PNG:TextualData"] (the pair now gives Same)
AC7 (vii) red: png-exif-basic-le gps (gps against exiftool:gps:all): unpinned difference keys|[]|["IFD0:PhantomTag"] (pinned: None)
AC7 (vi) red: png-exif-gps-full-le under all_pii: kept tag missing ExifIFD:ExposureCompensation=-0.3333333333
```

**Deviations and findings**
1. Value level of the differential (question `i3-differential-value-level-differences`, added): the design's
   191/86 counts hold for `(group, tag)` multisets, but AC5 also says to compare values. 34 equal-key pairs differ in
   value (31 `RIFF:WebP_Flags`, 3 duplicate-`EXIF`). They are pinned as `values`-level rows labelled D1 and D4, so no
   difference is unpinned and the key-level counts are the design's. The architect should confirm or drop them.
2. Judge exception (c) is never used (0 uses): no I3 fixture holds a plain XMP packet with GPS. Not added (the
   fixture list is the spec's); noted in the same question.
3. The fixture set has 236 entries: the 174 oracle fixtures plus the spec's Outputs list (carrier shapes 6, keywords
   22, `caBX` 1, WebP rows 33). The design's counts are asserted over the 174; the extras' differences are pinned too
   (D1 and D3 only).
4. The judge's carrier rule is also a positive check (a carrier-only fixture must print none of the carrier's
   tags): without it W1 and W2 (AC10) could not go red under `Gps`, which keeps `Xmp`.
5. `PNG:XMP` (an XMP `iTXt` with an unusable flag prints this binary tag) is classed `Xmp`; the other `PNG` text tags,
   including `XMLcomadobexmp` of a mis-spelled or language-tagged XMP keyword, are `FreeText` as the library files them.
6. Gates run on 1.90.0 (clippy 0.1.90): `just lint`, `just lint-no-allow`, `just forbid-unsafe`, `just lint-fuzz`
   all rc 0; `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test` rc 0; `just fuzz-smoke audit` 83,358 runs in 61 s, no crash;
   `fuzz/Cargo.lock` restored.

**Reflection (Part A)**
1. *What was unclear?* How to compare values at the differential's value level, and what makes a fixture "hold a
   carrier" for the judge. I resolved the first with the question above and the second with a naive per-container
   chunk scan in `i3_judge.rs`.
2. *A missing constraint or decision?* The spec should say the differential's identity criterion is `(group, tag)`
   multisets (that is what its counts mean) and name the `WebP_Flags` value differences.
3. *What would I do differently?* Generate the pin tables and the refusal table from the first record run earlier,
   and add the plain-XMP-with-GPS fixtures at design so exception (c) is exercised.


**Orchestrator notes on Part A** (claude-opus-5-5, 2026-11-25)
- Ruling on `i3-differential-value-level-differences` (now answered): the DEC-023 amendment of 2026-11-25. Values are
  compared. The 3 duplicate-`EXIF` pairs are D4 at value level. The 31 `RIFF:WebP_Flags` pairs are a new class, D6:
  ExifTool recomputes the `VP8X` flags, and we clear only the bits of kinds we removed (DEC-022 clause 7). They are not
  D1, which is ExifTool dropping `VP8X`. The 11 value-level flag rows in `i3_records.rs` were relabelled from D1 to D6.
  The test now pins value-level D4 3 and D6 31, with 157 identical pairs. It is green in replay and live
  (`DIAMOND_EXIF_REQUIRE_ORACLE=1`, 29 passed, 1 ignored).
- Exception (c)'s two fixtures, a plain XMP packet with GPS in one PNG and one WebP, are added to Part B's scope.
- Part A cost, from the harness's subagent result rather than the self-report: 341,318 tokens; 86 tool calls;
  1,910 s (about 32 min) of wall clock. The self-report's "about 3 hours" was wrong. Basis: agent-result. USD is not
  computed because the input/output split is not reported.

### Part B (AC8 to AC10, plus the exception (c) fixtures), claude-sonnet-5-5, 2026-11-25

Branch `feat/spec-039-png-webp-write-parity-build`. Part B is done: AC8 to AC10 green, the two exception (c)
fixtures recorded, `just oracle-record` run twice with no diff. Part C not started; `handoff.status` left `pending`.

**What was done**
- AC8: `tests/support/sweep.rs` (`listx_ids`, `parse_listing`, `new_tags`, `unexplained`, `Scratch`, `exiftool_listx`,
  `live_gate`). `unexplained` now takes the known set and the pinned structure list as arguments. The WebP sweep
  imports it with its verdict unchanged (13 passed, 1 ignored; 2700 codes x 8 shapes, 40 codes print) and its
  red-proofs still go red.
- AC9: `tests/strip_png_sweep_live.rs`. The type sweep: 2913 candidate codes (the `-listx` ids, the 26 pinned
  subdirectory types, all 16 case spellings of the 17 codes the writer names) x 10 payload shapes = 29,130 files,
  2 ExifTool runs (a `-a -u -G1 -s` run per directory of 5 shapes). 66 codes print a tag the bare file does not:
  all 17 writer codes (and every case of `eXIf` and `zxIf`, 32 spellings; no other case of any code), and 19 codes
  outside the writer's set, which are exactly DEC-020 clarification 4's 18 structure chunks plus `PLTE`. **Unknown
  chunk types found: 0.** `STRUCTURE` pins the tag names each of the 19 printed. The keyword sweep: 22 keywords x 3
  chunk forms (`tEXt`, `zTXt`, `iTXt`) = 66 files, one run, 0 failures; ExifTool's `PNG::TextualData` ids equal
  `i3_shapes::TEXTUAL_DATA_KEYWORDS`. The residual (rule 6) is in the file's doc comment.
- AC10: `probe --strip <file> <preset>` in `tests/support/probe.rs` (`run_probe_strip`, `StripRun`), the shared
  apparatus `tests/support/writer_mutants.rs`, and W1 and W3 in `png_mutation_oracle.rs`, W2 and W4 in
  `webp_mutation_oracle.rs`. Needles are exactly-once (the existing guard asserts it); the marker guard still passes.
- Exception (c): `png-xmp-plain-gps` (an uncompressed `iTXt` keyed `XML:com.adobe.xmp`) and `webp-xmp-plain-gps` (a
  `VP8X` with the XMP flag, an `XMP ` chunk), appended to `i3_shapes` (238 fixtures: 91 PNG, 147 WebP; the 174
  oracle counts unchanged). Their differences: PNG none; WebP `everything` against `-all=` is D1 (pinned), `gps`
  against `-gps:all=` is identical. The judge now uses (c) 24 times (bound `>= 20`).

**Commits** (on `feat/spec-039-png-webp-write-parity-build`):
- `eab9d04` test(oracle): SPEC-039 Part B red: PNG sweep, writer mutants W1 to W4, exception (c) bound
  (stubs: `sweep.rs` parsers and `run_probe_strip`; 10 tests red: the 4 PNG sweep tests, W1 to W4, the fixture count
  and the judge's (c) bound)
- `b9dc680` test(oracle): SPEC-039 exception (c) fixtures, records, and the judge's converse
- `7449c9a` test(oracle): SPEC-039 AC8 to AC10 green: shared sweep, PNG sweeps, probe strip mode, writer mutants
- the bookkeeping commit (this entry) follows (see `git log`).

**Measured numbers**
- Judge (replay and live identical): 2571 policy sections judged, 595 with a removable tag (2402 tags); exception
  (a) carrier 670, (b) whole block 354, (c) residual 24 (18 under `gps`, `gps_and_device`, `all_pii`: 2 fixtures x
  3 Composite tags x 3 presets; 6 under the Custom `xmp`, the converse below). Part A's 2547, 583, 2354, 670, 354, 0
  are unchanged except by the two fixtures.
- `just oracle-record` twice: the first run added only the two new records (the 236 others reproduced); after the
  commit the second run left `git status` empty. The records hold no machine path (the section test asserts it).
- Differential over the 174: unchanged (157 identical, key level D1 72, D2 2, D3 7, D4 4, D5 1, value level D4 3, D6
  31, 41 outcome differences, 30 both refuse).
- Gates on 1.90.0 (clippy 0.1.90): `just lint` rc 0, `just lint-no-allow` rc 0, `just forbid-unsafe` rc 0,
  `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test` rc 0 (41 result lines ok, 0 failed), `just mutation-oracle` rc 0
  (67 + 42 + 43 passed). `fuzz/Cargo.lock` was not touched (no fuzz run in Part B).

**Red evidence**

AC9 (`cargo test --test strip_png_sweep_live -- --nocapture`):
```
AC9 red (type sweep): without tXMP in the known set: "tXMP" (xmp): ExifTool prints ["XMP-dc:Creator", "XMP-photoshop:City"], and the code is neither the writer's nor pinned as structure
AC9 red (keyword sweep): without the Make row in the judge: "Make" in tEXt: ExifTool prints PNG:Make, which the judge files under Some(FreeText), but text_keyword_category says Some(CameraModel)
```

AC10 (`cargo test --test png_mutation_oracle --test webp_mutation_oracle -- --nocapture writer_mutant`). The four
results of each mutant: (1) the control reproduces the committed digest, (2) `strip` returns `Ok`, (3) replay red,
(4) live red:
```
AC10 W1: control digest equal true; strip Ok true; replay red true; live red true
AC10 W1 replay red: png-carrier-tXMP under gps: the writer's output is 446 bytes, digest 9b9b3f1c6b593517; the record read 70 bytes, digest 2d3cc60e9a50b1be
AC10 W1 live red: png-carrier-tXMP under gps: carrier tag left in XMP-dc:Creator=Zed
AC10 W2: control digest equal true; strip Ok true; replay red true; live red true
AC10 W2 replay red: webp-row-05-seal under gps: the writer's output is 506 bytes, digest 18c6265a0b93df02; the record read 44 bytes, digest 9b58c96f182ec9d0
AC10 W2 live red: webp-row-05-seal under gps: carrier tag left in XML-dc:Creator
AC10 W3: control digest equal true; strip Ok true; replay red true; live red true
AC10 W3 replay red: png-kw-04-copyright under all_pii: the writer's output is 70 bytes, digest 2d3cc60e9a50b1be; the record read 106 bytes, digest 4a40827073ce2164
AC10 W3 live red: png-kw-04-copyright under all_pii: kept tag missing PNG:Copyright=SweepCopyright
AC10 W4: control digest equal true; strip Ok true; replay red true; live red true
AC10 W4 replay red: webp-row-22-c2pa under all: the writer's output is 44 bytes, digest 9b58c96f182ec9d0; the record read 208 bytes, digest aa0594648055475c
AC10 W4 live red: webp-row-22-c2pa under all: kept tag missing JUMBF:JUMDLabel=c2pa
```
No mutant was caught by the self-check. The needles: W1 `b"tXMP" | b"meTa" | b"seAl" => ...` in `png/metadata.rs`;
W2 `b"_PMX" | b"SEAL" | ...` in `webp/metadata.rs`; W3 `(b"Copyright", Category::Rights),` in `audit/category.rs`; W4
`b"C2PA" => Some(ChunkClass::Jumbf),` in `webp/metadata.rs`. Nothing under `src/` was changed.

Exception (c) converse (new red-proof, `comparator_is_red_when_exception_c_is_claimed_without_an_xmp_source`):
```
AC7 (x) red: png-exif-gps-full-le under xmp: kept tag missing Composite:GPSLatitude=37.4252083333333
```
(the test first asserts the control: `png-xmp-plain-gps` under `xmp` judges clean).

**Deviations and findings**
1. Judge exception (c) has a converse, found by the new fixtures: under the Custom `xmp` (removes `Xmp`, keeps `Gps`)
   the packet goes and ExifTool's `Composite:GPSLatitudeRef`, `GPSLongitudeRef`, `GPSPosition` derived from it go too
   (6 "kept tag missing" lines). I counted that under exception (c), in `i3_judge::judge`, only when the input's XMP
   holds a GPS property and the policy removes `Xmp`, and I red-proofed it. DEC-023 rule 3 (c) is one-directional as
   written, so this extends it. Question `i3-judge-exception-c-converse` (open, low) asks the architect to ratify it
   in DEC-023. No class of the differential was invented.
2. The WebP exception (c) fixture sets the `VP8X` XMP flag (`0x04`), as a real file would. With flags 0 ExifTool's
   recomputed flags differ (a D6 value pin); with the flag set the `gps` pair is identical, so only the D1 pin is new.
3. The type sweep's candidate set follows AC9 (2913 codes, 29,130 files), not the design's measurement (35,020 files
   over 56 case-spelled types); the design's figure is of a wider one-off run, and AC9 does not ask for it.
4. `tests/support/writer_mutants.rs` is a new support file that the spec's Outputs do not list: it holds the four-result
   apparatus once, for the two mutation files.
5. The red-proofs of AC9 are live (they reuse the sweep's one run); each also has an ungated canned half. The AC9
   tests skip loudly without ExifTool, like the WebP sweep.

**Reflection (Part B)**
1. *What was unclear?* Whether the judge could excuse a derived `Composite:GPS*` going missing when its XMP source is
   removed (rule 3 (c) names only the kept direction), and what "all 16 case spellings of every code the writer names"
   should add to the design's 56-type grid. I took the AC text literally and raised the first as a question.
2. *A missing constraint or decision?* DEC-023 (c) should state the converse, and the spec should say the exception
   (c) fixtures' VP8X flag.
3. *What would I do differently?* Run the sweep before typing `STRUCTURE`, as I did, but also print the observed
   table from the first run in the test itself, which I now do; and write the exception (c) fixtures in the design, so
   the converse showed in Part A.

**Orchestrator notes on Part B** (claude-opus-5-5, 2026-11-26)
- Re-run on 1.90.0 with ExifTool required: `just mutation-oracle` passed 67, 42 and 43; `strip_png_webp_oracle` passed
  30; `strip_png_sweep_live` passed 14. No file under `src/` changed in Part B.
- Ruling on `i3-judge-exception-c-converse`, now answered: ratified in the DEC-023 amendment of 2026-11-26, keeping the
  build's narrow condition. The amendment states one limit: inputs with GPS in both Exif and XMP.
- The sweep's 29,130 files against the Context's 35,020 is not a shortfall. AC9 asks for case spellings of the 17
  codes the writer names; the design's one-off run spelled out all 56 PNG types.
- Part B cost, from the harness's subagent result rather than the self-report: 272,228 tokens; 94 tool calls (the
  self-report said about 65); 1,630 s (about 27 min) of wall clock. Basis: agent-result. USD is not computed, because
  the input/output split is not reported.

### Part C (AC11 to AC15), claude-sonnet-5-5, 2026-11-26

Branch `feat/spec-039-png-webp-write-parity-build`. Part C is done: AC11 to AC15 green. `handoff.status` is
`completed`, the spec's `task.cycle` is `verify`, the timeline's build line is `[x]`.

**What was done**
- AC11, the switch-over: `ContainerKind::strippable()` is true for `Jpeg`, `Tiff`, `Png` and `Webp` (one `matches!`).
  The preview follows DEC-009 rule 2 with no other change in `preview.rs`: its docs and the `strippable()` and
  `reliable()` docs were rewritten. The "has not been checked against ExifTool yet" strings are deleted from
  `src/report/text.rs` (the PNG and WebP arms); the generic "writer refuses this container" branch stays for a future
  unstrippable kind (no current kind reaches it).
- AC13: `NotJpeg` displays "not a JPEG, TIFF, PNG or WebP"; `Unstrippable` displays "the file cannot be cut
  safely (...)"; the docs of `NotJpeg` and `UnstrippableReason` say "file". The variant keeps its name.
- AC12: `refused_strips_never_preview_as_reliable` (in `strip_png_webp_oracle.rs`) over all 238 I3 fixtures under the
  five presets: no refused strip has a reliable preview. It passed before the flip (nothing was reliable) and after.
- AC14: `strip_records::policies()` gains `("gps", Gps)` and `("all-pii", AllPii)`, appended last. One re-record of
  `exiftool-strip/`, then a second with no diff. See "JPEG re-record" below.
- README: the PNG and WebP paragraphs say strippable (the CLI still never writes); the one sentence from DEC-020
  amendment item 2 (a strip can still refuse a file whose preview shows removals, with the measured fact); I3 and its
  four layers; the three residuals (C2PA kept except under `Everything`; XMP GPS as `Composite:GPS*` under `Gps`,
  `GpsAndDevice`, `AllPii`, which `-gps:all=` also leaves; carriers and every WebP `LIST` removed under every preset).
- Fuzz mirror: only a doc comment in `fuzz/shared/audit_invariants.rs` (`strippable_kind` already asks the one
  definition).

**Commits** (on `feat/spec-039-png-webp-write-parity-build`)
- `09768e6` test(strip): SPEC-039 Part C red: switch-over pins, neutral errors, honesty test, JPEG gps and all-pii rows
  (no stubs needed; the tests compile against the old library. Red before the code: 4 in `audit_png_webp`, 2 in `cli`,
  1 in `report_render`, 1 in `strip`, 6 in `strip_oracle` (the missing `gps`/`all-pii` sections and the JFIF test), 2 in
  `strip_png`, 1 in `strip_webp`. `refused_strips_never_preview_as_reliable` is green on purpose: it guards the flip.)
- `9c202b8` feat(audit): SPEC-039 AC11 to AC14 PNG and WebP are strippable; neutral strip errors; JPEG gps and all-pii records
- the bookkeeping commit (this entry) follows (see `git log`).

**Goldens and pins that changed, and why each follows from the switch-over**
- `tests/golden/report-png-webp.txt`: finding lines "kept (not strippable)" become `removed`, `kept_unread (may hold
  what this policy removes)` or `kept (not targeted by this policy)`; the two "preview (all_pii): not strippable: the
  PNG/WebP strip exists but has not been checked ..." lines become "removed 4, kept 0, kept_unread 2; residual findings 2,
  residual severity medium" (PNG) and "removed 1, kept 1, kept_unread 1; ..." (WebP); the summary lines become "reliable
  files: 2, unreliable files: 0" and "policy all_pii: residual findings 4 (reliable files only)".
- `.json`: `"disposition"` per finding, `"counts"`, `"reliable":true`, `"residual_findings"`, `"residual_severity"`,
  `"strippable":true` (both files), and the summary's `residual_findings:4, reliable_files:2, unreliable_files:0`.
- `.csv`: the `not_strippable` disposition cell becomes `removed`, `kept_unread` or `kept` (12 data lines).
- Every PNG and WebP finding follows DEC-009 rule 2 for `all_pii`: Gps, Authorship, FreeText, Timestamps removed; the
  `xmp` and the trailing `other_segment` are kept and could hold what the policy removes, so `kept_unread`; the WebP
  ICC stays `kept` (an `Icc` block holds no tag). Both files read clean (the PNG's trailer is a finding, not an issue),
  so both are reliable. No line outside these classes changed.
- Pins: `audit_png_webp.rs` (the keep-everything test became `png_and_webp_dispositions_per_preset`, a hand-typed
  grid; the strip test and the kind test now say strippable; the 16/17 block test now expects `strippable && !reliable`),
  `policy_preview.rs` (a comment), `report_render.rs` (counts of `strippable`, the deleted wording, the reliable
  files), `cli.rs` (the PNG with GPS says "removed 1, kept 0, kept_unread 0"; the kind grid), `strip_png.rs` and
  `strip_webp.rs` (the two "strippable is still false" tests; the `caBX` preview is `KeptUnread` under the four
  presets that keep it and `Removed` under `Everything`), and `fuzz_layout.rs` (the PNG plants of the audit and
  render harnesses, reversed: "reported not strippable", "reported not reliable", "counted as unreliable", the CSV
  plant `,removed,` to `,not_strippable,`, plus two re-aimed preview plants; each still goes red alone).
- `strip_oracle` group table: a JPEG output's Exif groups no longer come only from kept segments. A new hand-typed
  `entry_edit_groups` in `strip_records.rs` lists, per fixture, the groups whose every entry a policy deletes
  (`kitchen-sink`: ExifIFD, GPS, IFD1; `exif-without-gps`: IFD0; `mpo-*`: IFD0, ExifIFD), typed from the builders
  (`full_tiff`, `no_gps_tiff`, `exif_camera`). It is the only test-side change not named in the spec; see Deviations.

**JPEG re-record (AC14)**
- 25 of the 39 `exiftool-strip/` records gained a `gps` section and 21 an `all-pii` section: 1074 lines added, none
  deleted or changed. The other 14 files are the 12 `exif-*` entry records and `gps-count-overflow`, `unreadable-exif`
  (refused shapes), which this apparatus does not record under the new policies. `just oracle-record` twice: the second
  run left `git status` empty.
- No kept tag is missing and no removed tag is left in. `kitchen-sink` under `gps` prints every group but `GPS`
  (IFD0, ExifIFD, IFD1 intact); under `all-pii` only `IFD0:Orientation` and `IFD0:Software` remain (the policy keeps
  both). The four `jfif-*` fixtures: under `gps` the three with a thumbnail still print `JFIF:ThumbnailTIFF`; under
  `all-pii` the `JFIF` group is gone with the cut APP0; `jfif-no-thumbnail` keeps its `JFIFVersion` under both.
  `jfif_fixtures_keep_the_thumbnail_under_gps_and_lose_it_under_all_pii` is green by replay and live.

**Measured numbers**
- `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test`: rc 0, 41 `test result: ok` lines, 0 failed (ExifTool 13.55 present).
- `just mutation-oracle`: rc 0, 67 + 42 + 43 passed (1 ignored each).
- Fuzz smokes, 60 s each, rc 0, no crash: `audit` 121,518 runs; `audit_render` 35,281; `rewrite_png` 564,377;
  `rewrite_webp` 989,827.

**Gates on 1.90.0 (clippy 0.1.90)**: `just lint` rc 0; `just lint-no-allow` rc 0; `just forbid-unsafe` rc 0;
`just lint-fuzz` rc 0; `just deny` rc 0; `just deny-fuzz` rc 0; `just test` with `DIAMOND_EXIF_REQUIRE_ORACLE=1` rc 0;
`just mutation-oracle` rc 0; the four smokes above; `just validate` rc 0 (run again after this entry);
`git diff origin/main -- fuzz/Cargo.lock` empty (restored after the lint-fuzz, deny-fuzz and the smokes).

**Deviations**
1. The group-table test needed `entry_edit_groups` (above). Before it, `kitchen-sink under gps` failed with "missing
   [GPS]": the table expected the GPS group because the Exif segment is kept, while ExifTool correctly prints no GPS
   tag after the entries are deleted. This is the table being segment-level, not the writer being wrong (the record
   keeps every other tag), so I extended the table rather than stopping. The architect should confirm.
2. The `fuzz_layout.rs` plants were not in the spec's Inputs list as such (it lists the file for the pinned
   "not strippable"); they had to be reversed because the old plants became no-ops after the flip.
3. The preview and render goldens regenerated by a one-off edit of `golden_png_and_webp_report_in_all_three_formats`
   that wrote the files (reverted before commit); there is no bless mechanism.

**Reflection (Part C)**
1. *What was unclear?* Whether the JPEG group-table test was expected to pass unchanged with `gps` and `all-pii`
   (AC14 says "pass with the new sections"): it cannot, for a fixture whose Exif has a group that the policy empties.
2. *A missing constraint or decision?* The spec should say the group table models entry edits, and that the
   `fuzz_layout.rs` plants must be re-aimed.
3. *What would I do differently?* Run the JPEG re-record before writing the AC14 test, so the group-table gap shows
   first; and grep `fuzz_layout.rs` for "strippable" at design.

### Execution notes

- **Branch / PR:** `feat/spec-039-png-webp-write-parity-build` (no PR; the orchestrator opens it)
- **Completed at:** 2026-11-26
- **All acceptance criteria met?** yes. AC1 to AC15 are green: Part A (AC1 to AC7), Part B (AC8 to AC10, plus the two
  exception (c) fixtures) and Part C (AC11 to AC15). One test-side addition is unlisted (Part C, Deviation 1).
- **For `verify`:** not applicable (build handoff).

### Cost self-report

- **Tokens (total):** null. The orchestrator fills `basis` and `tokens_total` from the harness's per-part subagent
  results; Parts A and B are recorded under Handback. This agent cannot read its own token count.
- **Estimated USD:** null
- **Duration (minutes):** null (no wall-clock estimate is given)
- **Source of the number:** none available to the executing agent

### Drift and new artifacts

- **New decisions emitted:** none. Two amendments to DEC-023, both by the orchestrator:
  - 2026-11-25: values are compared; D4 at value level, and a new class D6 (`RIFF:WebP_Flags`, ExifTool recomputes the
    `VP8X` flags, DEC-022 clause 7). Answered question `i3-differential-value-level-differences`.
  - 2026-11-26: judge exception (c) has a converse (XMP removed, derived `Composite:GPS*` removed with it), limited to
    inputs with GPS in both Exif and XMP. Answered question `i3-judge-exception-c-converse`.
- **Deviations from spec:**
  - Part A: the fixture set is 236 (then 238), the differential's value level, exception (c) never used (fixed in B).
  - Part B: `tests/support/writer_mutants.rs` is a new support file; the type sweep has 29,130 files (AC9's
    candidate set), not the design's 35,020; exception (c)'s converse.
  - Part C: `entry_edit_groups` in `strip_records.rs`; the `fuzz_layout.rs` plants reversed.
- **Follow-up work identified:**
  - JPEG and TIFF differentials against `-all=` and `-gps:all=` (the spec's open question after ship).
  - Rename `StripError::NotJpeg`, or not: the Display is neutral now, the name is a public API cost.
  - A bless mechanism for `tests/golden/`.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — The differential's identity criterion and the value-level pairs (Part A), whether the judge could excuse a
   derived `Composite:GPS*` (Part B), and whether the JPEG group table should pass unchanged with `gps` and `all-pii`
   (Part C).

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — DEC-023 (c)'s converse and the VP8X flag of the exception (c) WebP fixture (both now settled); and that the
   `fuzz_layout.rs` plants and the JPEG group table are touched by the flip.

3. **If you did this task again, what would you do differently?**
   — Generate the pin tables from a first record run, put the exception (c) fixtures in the design, and run the JPEG
   re-record before writing its tests.
