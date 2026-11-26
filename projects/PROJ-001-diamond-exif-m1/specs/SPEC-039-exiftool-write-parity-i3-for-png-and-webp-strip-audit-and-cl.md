---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-039
  type: story                      # epic | story | task | bug | chore
  cycle: verify                    # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: L                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: null          # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: null             # approved | punch-list | rejected — the OUTCOME of the verify
                                   #   cycle, stamped by `just advance-cycle` when the spec leaves
                                   #   verify (same three verdicts Prompt 4 already returns).
                                   #   Recorded in front-matter, not just prose, so "verify never
                                   #   rejects anything" stops being a hunch and becomes a number.
  verify_rounds: []                # every verify outcome, oldest first, appended by
                                   #   `just advance-cycle` each time the spec leaves verify.
                                   #   verify_verdict keeps only the LAST one, so a spec sent
                                   #   back and then approved read as "approved": irradiance's
                                   #   field showed 1 pushback where its handoffs showed 5.

project:
  id: PROJ-001
  stage: STAGE-010
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: claude-sonnet-5-5       # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: 2026-11-24

references:
  decisions: [DEC-008, DEC-009, DEC-013, DEC-020, DEC-021, DEC-022, DEC-023]
  constraints: [test-before-implementation, oracle-must-be-shown-red, exiftool-never-linked, panic-free-library, forbid-unsafe-code, no-io-in-library, fuzz-target-per-surface, no-new-top-level-deps-without-decision]
  related_specs: [SPEC-020, SPEC-021, SPEC-032, SPEC-036, SPEC-037, SPEC-038, SPEC-040]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-037, SPEC-038]                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-010's <capability>". Optional; null is acceptable.
value_link: "Delivers STAGE-010's ExifTool I3 for both strips and the honest switch-over: exif-audit's preview says Removed for PNG and WebP."

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
  tokens_estimate: 1300000          # three builder parts; SPEC-036/037/038 (L) landed at 1.0M-1.4M
  sessions:
    - cycle: design
      agent: claude-opus-5-5
      interface: claude-code
      basis: null
      tokens_total: null
      estimated_usd: null
      duration_minutes: null
      recorded_at: 2026-11-24
      notes: "main-loop design (ExifTool write-mode exit codes, the ours-vs-ExifTool differential over 174 fixtures, PNG type and keyword sweeps, refusal-vs-preview measurement, DEC-023, DEC-020 amendment, spec, HANDOFF); un-metered, null by design"
    - cycle: build
      handoff: HANDOFF-090
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 859490
      estimated_usd: null
      duration_minutes: 97
      recorded_at: 2026-11-26
      notes: "Parts A, B and C done on one branch, one subagent each. tokens_total and duration are the sums of the harness subagent results, filled by the orchestrator: A 341,318 tokens / 86 tool calls / 1,910 s; B 272,228 / 94 / 1,630 s; C 245,944 / 88 / 2,292 s. USD is not computed because the input/output split is not reported."
    - cycle: verify
      handoff: HANDOFF-091
      agent: claude-opus-5-5
      interface: other
      basis: agent-result
      tokens_total: 258606
      estimated_usd: null
      duration_minutes: 26
      recorded_at: 2026-11-26
      notes: "Verdict applies to f5e24c1 (build tip plus this handoff). Cost filled by the orchestrator from the harness subagent result: 258,606 tokens, 96 tool calls, 1,553 s. Punch list of 3 items: 1 (M) judge exception (a) is group-level and excuses a dropped plain Exif/XMP block when a carrier coexists; 2 (L) the AC11 grid has no ICC finding; 3 (L) the README PNG keyword sentence omits Make, Model and TimeStamp."
    - cycle: build
      handoff: HANDOFF-091
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 194326
      estimated_usd: null
      duration_minutes: 21
      recorded_at: 2026-11-26
      notes: "Punch-list fix round for HANDOFF-091 (items 1 M, 2 L, 3 L), one subagent. The orchestrator transcribed this from the harness subagent result: 194,326 tokens, 63 tool calls, 1,263 s."
    - cycle: verify
      handoff: HANDOFF-091
      agent: claude-opus-5-5
      interface: other
      basis: agent-result
      tokens_total: 184591
      estimated_usd: null
      duration_minutes: 16
      recorded_at: 2026-11-26
      notes: "Verify round 2, against 34008cc. One subagent. The orchestrator transcribed this from the harness subagent result: 184,591 tokens, 48 tool calls, 944 s. The verdict is a punch list of 2 L items: two line-level guards are unpinned (multiplicity decrement, by-line left-in check), and the line-level rule assumes readings add up, which a carrier placed before the plain block breaks."
    - cycle: build
      handoff: HANDOFF-091
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 144285
      estimated_usd: null
      duration_minutes: 20
      recorded_at: 2026-11-26
      notes: "Round-2 punch-list fix round, one subagent: both guards pinned, plus the no-carriers additivity check. The orchestrator transcribed the cost from the harness subagent result: 144,285 tokens, 46 tool calls, 1,209 s."
    - cycle: verify
      handoff: HANDOFF-091
      agent: claude-opus-5-5
      interface: other
      basis: agent-result
      tokens_total: 191215
      estimated_usd: null
      duration_minutes: 19
      recorded_at: 2026-11-26
      notes: "Verify round 3, against 2f790d4, one subagent. The orchestrator transcribed the cost from the harness subagent result: 191,215 tokens, 79 tool calls, 1,159 s. Punch list of 1 item (L): the additivity check compares fewer groups than exception (a) can excuse (ID3v2_3, Olympus, PNG text, ICC, JUMBF/JSON are not compared)."
    - cycle: build
      handoff: HANDOFF-091
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 114377
      estimated_usd: null
      duration_minutes: 14
      recorded_at: 2026-11-26
      notes: "Round-3 fix round (additivity compares judged lines). One subagent, resumed twice after two correct stops. The harness's last notification is read as cumulative: 114,377 tokens, 40 tool calls, 846 s. The earlier readings were 82,538 and 88,160."
    - cycle: verify
      handoff: HANDOFF-091
      agent: claude-opus-5-5
      interface: other
      basis: agent-result
      tokens_total: 135708
      estimated_usd: null
      duration_minutes: 11
      recorded_at: 2026-11-26
      notes: "Verify round 4, against 9c368ba, one subagent. The orchestrator transcribed the final harness notification: 135,708 tokens, 47 tool calls, 683 s. Punch list of 2 L items: a SEAL/XML carrier line left in beside a plain XMP packet goes unchecked (pre-existing), and the red-proof comment misdescribes its readings."
  totals:
    tokens_total: 2082598
    estimated_usd: 0.00
    session_count: 9
---

# SPEC-039: ExifTool write parity (I3) for PNG and WebP strip, audit and CLI switch-over

## Context

STAGE-010's last spec. PROJ-001.

SPEC-037 and SPEC-038 made `strip` work for PNG and WebP in the library. The audit and CLI still say both formats are
`not strippable` (DEC-020 clause 6), because nothing yet shows that ExifTool agrees with the writer's output. The
gated live checks (`tests/strip_png_live.rs`, `tests/strip_webp_live.rs`) were "the cheap precursor": they run live
only, cover a subset of fixtures, check mostly the "removed" direction, and nothing records them. The design document's
I3 (§8) is "apply the same op with exif-core and with ExifTool, then read both outputs and compare". No test in the
repo has run ExifTool in **write** mode, and the porting brief warns that its write-mode exit codes are unmeasured
(question `exiftool-write-exit-codes`).

**Measured in this design** (ExifTool 13.55, 2026-11-24, all 174 `oracle_png` and `oracle_webp` fixtures stripped by
today's `main` under the five presets). The scratch scripts were in the design session's scratchpad and are not kept;
the build rebuilds what it needs from these facts. **DEC-023 is normative** and carries the full table.

1. **Write-mode exit codes.** `0` means the output was written (warnings possible). `1` means an error and no output
   file (format error, truncation, a `[minor]` error without `-m`, an existing output path, a missing input). `2` means
   a `-if` condition failed. Across 348 write runs, `rc == 0` held exactly when the output file existed. `-m` turns
   `[minor]` errors into warnings and exit 0.
2. **The differential** (`Everything` ↔ `exiftool -all=`, `Gps` ↔ `exiftool -gps:all=`, compared as ExifTool's
   readings of the two outputs, `(group, tag)` multisets and the values of tags both keep):
   - 157 pairs are identical. The design first counted 191 by comparing keys only. 34 of those agree on keys and
     differ in value: D4 3 and D6 31 (DEC-023 amendment of 2026-11-25).
   - 86 differ, all in five classes (DEC-023 rule 4):
     - D1: ExifTool drops `VP8X` after `-all=` (72 pairs).
     - D2: ExifTool adds `VP8X` when writing `EXIF` into a simple WebP (2).
     - D3: ExifTool keeps raw-profile carriers that we remove (7).
     - D4: ExifTool rewrites duplicate `EXIF` chunks from the last one, which changes kept values (4).
     - D5: ExifTool can't delete a text chunk that has no keyword NUL (1).
     - D6, value level only: ExifTool recomputes the `VP8X` flags from the chunks present. We clear only the bits
       of kinds we removed (31).
   - 41 pairs are outcome differences: ExifTool refuses where we strip (hostile structure we tolerate), or we refuse
     where ExifTool writes (`Uneditable { BlockNotClean }` on an untrusted Exif block).
   - 30 pairs are refused by both.
3. **Refusal against preview.** Under every preset, no fixture is refused while its audit is clean. Every refusal
   falls on a file whose preview already says "not reliable". So flipping the switch per container kind is honest on
   the corpus. Refusals of clean files still exist in principle (`SharedBytes`, `HiddenPointer`, ...), the same as for
   JPEG.
4. **What each preset loses.** On every fixture, under `Gps`, `GpsAndDevice` and `AllPii`, ExifTool reads no
   kept-category tag as lost, except inside raw-profile carriers (`ISO`, `Copyright`, `Orientation`, `CreatorTool`):
   the carrier rule. Under `All` and `Everything`, the uncategorised Exif tags go with the whole `eXIf`/`EXIF` chunk:
   the whole-block rule.
5. **The PNG type sweep.**
   - Candidates: 2,622 four-character `-listx` ids, plus 880 case spellings of 56 PNG types, in 10 payload shapes.
     That is 35,020 files in two ExifTool runs.
   - `-listx` doesn't list subdirectory types (`tEXt`, `iTXt`, `zTXt`, `eXIf`, `iCCP`, `caBX`, `pHYs`, `cHRM`,
     ...), as for WebP. They must be a pinned candidate list.
   - Every type that printed a tag is known to the writer, or is one of DEC-020 clarification 4's structure chunks.
     `PLTE` is new to that list and is structure.
   - Only the exact spellings decode, except `eXIf` and `zxIf`, which decode in every case. This confirms DEC-020
     clarification 5.
6. **The PNG keyword table.** ExifTool's `PNG::TextualData` (from `-listx`) has `Make`, `Model` and `TimeStamp`, which
   the audit files under `FreeText`. The presets are unaffected: all three categories are in `AllPii`. A `Custom` spec
   is affected. This is DEC-020's amendment of 2026-11-24.

## Goal

ExifTool write parity (I3) for PNG and WebP strip, as DEC-023 defines it, has these parts:

- committed records, replayed and reproduced live;
- a two-way category judge for every fixture and recorded policy;
- a differential against ExifTool's own writer for `Everything` and `Gps`, with every difference pinned;
- the write-mode exit-code rule;
- the comparator shown red;
- the writer shown red by mutated copies the self-check cannot catch;
- a PNG completeness sweep against ExifTool's own lists.

Then the switch-over. `ContainerKind::strippable()` becomes true for PNG and WebP. The preview shows `Removed` and
`KeptUnread` as it does for JPEG, the CLI and reports follow, goldens change deliberately, and the README says what is
now true.

## Decisions made here

1. **DEC-023** (new): the I3 layers, the record format with output digests, the recorded policies, the judge's three
   exceptions, the differential pairs and their D1 to D5 table, the exit-code rule, and the sweep's stated residual.
   Answers question `exiftool-write-exit-codes`.
2. **DEC-020 amendment of 2026-11-24:**
   - Clause 6 is reversed for PNG and WebP, per container kind as for JPEG.
   - `Make` and `Model` become `CameraModel`, and `TimeStamp` becomes `Timestamps`.
   - `PLTE` is structure.
3. **The error name stays.** `StripError::NotJpeg` keeps its name. Renaming a public variant is churn with no reader
   asking for it. Its doc and `Display` become neutral: "not a JPEG, TIFF, PNG or WebP". `Unstrippable`'s `Display`
   ("the JPEG cannot be cut safely") and the `UnstrippableReason` doc ("Why a JPEG is not strippable") say "file".
   This settles outline note (5). A RIFF `WAVE`, `AVI `, `RIFX` or lower-case `webp` still sniffs as unknown and gets
   `NotJpeg`.
4. **The live precursors stay.** `strip_png_live.rs` and `strip_webp_live.rs` keep their tests (the outline said
   extend, not rewrite). Their category lists move to `tests/support/i3_judge.rs`, which both import.
5. **The JFIF signal is folded in** (`strip-oracle-jfif-kept-presets-unrecorded`). The JPEG strip oracle records `gps`
   and `all-pii` for every fixture, with one re-record. This is part C and can be cut to a follow-up without touching
   parts A and B.

## Inputs

- **Normative:** DEC-023, the DEC-020 amendment of 2026-11-24, DEC-021 (PNG strip, clause 4 carriers), DEC-022 (WebP
  strip, clauses 4 and 7, the measured table), DEC-009 (presets, may-hold), DEC-008 (categories), DEC-013 (the
  self-check).
- **Apparatus to follow:** `tests/strip_oracle.rs`, `tests/support/strip_records.rs` (record format, `policies()`,
  `record_text`, the replay/reproduce test, comparator red-proofs), `tests/strip_tiff_oracle.rs`,
  `examples/oracle_record.rs`, `tests/support/tools.rs` (spawn limits, `strip_perl_lib_dir`, temp files).
- **The precursors:** `tests/strip_png_live.rs` (`carrier_shapes`, `case_grid`, `fixtures`, the tag lists),
  `tests/strip_webp_live.rs` (`category_of`, `judge`, the "complete judge" red-proof),
  `tests/strip_webp_sweep_live.rs` (`listx_ids`, `parse_listing`, `new_tags`, `unexplained`, the structure list).
- **Fixtures:** `tests/support/oracle_png.rs::fixtures()` (61), `tests/support/oracle_webp.rs::fixtures()` (113),
  `tests/support/png_shapes.rs`, `tests/support/webp_carriers.rs` (the DEC-022 table as data, `payload`,
  `row_chunk`).
- **Mutated copies:** `tests/support/probe.rs` (stage, inject, build and run; it has no strip mode yet),
  `tests/png_mutation_oracle.rs` and `tests/webp_mutation_oracle.rs` (the control-then-mutant pattern, the injection
  marker guard).
- **Switch-over:**
  - `src/audit/mod.rs`: `ContainerKind::strippable`.
  - `src/audit/preview.rs`: its docs.
  - `src/report/{text,json,csv,mod}.rs`: `preview_lines`, and the PNG/WebP wording.
  - `src/audit/category.rs`: `TEXT_KEYWORDS` and `text_keyword_category`.
  - `src/write/mod.rs`: `StripError`'s `Display` and docs.
  - `fuzz/shared/audit_invariants.rs`: the keyword mirror, and `strippable_kind`.
  - `tests/golden/report-png-webp.{txt,json,csv}`.
  - The tests that pin "not strippable": `tests/audit_png_webp.rs`, `policy_preview.rs`, `report_formats.rs`,
    `report_render.rs`, `cli.rs`, `strip.rs`, `strip_png.rs`, `strip_webp.rs`, `fuzz_layout.rs`.
  - `README.md`.

## Outputs

- **New `tests/strip_png_webp_oracle.rs`.** The formal I3: replay, live, the judge, the differential, the refusal
  pins, and the comparator red-proofs.
- **New `tests/support/i3_judge.rs`.** Tag → category, typed from the DECs (DEC-023 rule 3), with the per-policy
  two-way judge and its three exceptions.
- **New `tests/support/i3_records.rs`.** The record format, parse and write, FNV-1a 64, the policy list, the
  ExifTool write runner with the exit-code rule, and the pinned D1 to D5 and outcome tables.
- **New `tests/support/i3_shapes.rs`.** The I3 fixture set:
  - `oracle_png` + `oracle_webp` fixtures;
  - the PNG carrier shapes moved from `strip_png_live.rs` (that file imports them);
  - one PNG per `TextualData` keyword (`tEXt`, value `Sweep<Keyword>`);
  - one PNG with a `caBX` JUMBF manifest;
  - one WebP per `webp_carriers` table row (first payload that prints).

  Names are stable. New shapes go at the end.
- **New records:** `tests/oracle-fixtures/exiftool-strip-png/*.txt` and `exiftool-strip-webp/*.txt`. Recorded by
  `just oracle-record`, which now also writes them. No machine paths.
- **New `tests/support/sweep.rs`** (moved from `strip_webp_sweep_live.rs`: `listx_ids`, `parse_listing`, `new_tags`,
  `unexplained` generalised over a known-code set, the scratch directory). The WebP sweep imports it, with its verdict
  unchanged.
- **New `tests/strip_png_sweep_live.rs`.** The type sweep and the keyword sweep. Live only.
- **Modified `tests/support/probe.rs`.** A strip mode: the probe reads a file and a policy name, calls
  `exif_core::strip`, and writes the output bytes (or the refusal's `Debug`) to stdout in a form the test parses.
- **New tests in `tests/png_mutation_oracle.rs` and `tests/webp_mutation_oracle.rs`.** The writer mutants W1 to W4
  (AC10).
- **Modified `src/audit/category.rs`.** Three keyword rows, plus the hand-typed table test.
- **Modified `src/audit/mod.rs`, `src/audit/preview.rs`, `src/report/text.rs`.** The flip, the docs, and the deleted
  wording branch.
- **Modified `src/write/mod.rs`.** The neutral `Display` and docs.
- **Modified `fuzz/shared/audit_invariants.rs`.** The keyword mirror (typed independently).
- **Goldens regenerated:** `tests/golden/report-png-webp.*`. Every changed line class is listed in the handback.
- **Part C: `tests/support/strip_records.rs`.** `policies()` gains `gps` and `all-pii`, and
  `tests/oracle-fixtures/exiftool-strip/` is re-recorded once.
- **`README.md`.** The PNG/WebP strip paragraphs, the residuals, refusal-after-preview, and I3.
- No new dependency, flag, public function or `Category`. `[dependencies]` stays empty. No CHANGELOG at build; ship
  adds it.

## Acceptance Criteria

### Part A: keyword rows, records, judge, differential (commit each step red first)

- [ ] **AC1 (keyword rows, DEC-020 amendment 3):**
  - `text_keyword_category`: `Make`/`make` → `CameraModel`, `Model`/`model` → `CameraModel`, `TimeStamp`/`timeStamp`
    → `Timestamps`.
  - `MAKE`, `MODEL` and `TIMESTAMP` → `FreeText`, by the measured first-letter rule. Verify these three live with one
    ExifTool run, and paste the run into the handback.
  - `Collection`, `Document`, `Label`, `URL`, `Source`, `aesthetic_score` and `parameters` → `FreeText`.
  - The test table is typed by hand.
  - The fuzz keyword mirror agrees.
  - `PLTE` gets no `chunk_class` (pinned).
- [ ] **AC2 (records, DEC-023 rule 1):**
  - Every I3 fixture has a record. Its sections are exactly `input`, then the recorded policies that strip, in
    `policies()` order, then `exiftool:all` and `exiftool:gps:all`, always. A write ExifTool refused is a section holding one `# refused:` line (DEC-023 rule 1).
  - Every policy section carries `# output: <len> <fnv1a64>`.
  - Replay recomputes `strip` and fails on a length or digest mismatch.
  - `just oracle-record` run twice gives no diff.
  - Live: today's ExifTool reproduces each record byte for byte.
- [ ] **AC3 (refusal pins, rule 2):** the set of `(fixture, policy, StripError variant)` refusals is a hand-written
  table. An unpinned refusal fails, and so does a pinned refusal that now strips. The table matches the 113 refused
  pairs measured at design: 13 fixtures refused under all five presets, 16 under three (`Gps`, `GpsAndDevice`, `AllPii`: an Exif entry edit on an untrusted block).
- [ ] **AC4 (Layer A judge, rule 3):**
  - For every fixture and every recorded policy that strips: no tag of a removed category remains, and every input tag
    of a kept category remains with the same value.
  - The only exceptions are (a) the carrier rule, allowed only on a fixture that holds a carrier chunk; (b) the
    whole-block rule, allowed only under a policy that holds every Exif category; and (c) the two residuals, which
    count as kept.
  - Each use of an exception is counted and printed per class. A fixture that prints no removable tag before the strip
    is excluded from the "removed" count.
  - The test asserts lower bounds for the counts. The handback states the actual numbers.
- [ ] **AC5 (Layer B differential, rule 4):**
  - For every fixture, run `Everything` against `exiftool -all=` and `Gps` against `exiftool -gps:all=`.
  - Compare the `(group, tag)` multisets, and the values of tags both outputs keep.
  - Every difference must be in the pinned table, as (fixture, pair, class D1 to D6, level, the exact tag lines). The
    table is typed by hand from DEC-023 and this design's counts:
    - key level: D1 72, D2 2, D3 7, D4 4, D5 1;
    - value level: D4 3, D6 31;
    - 157 identical pairs.
  - Outcome pins: 41 (ExifTool refuses with its `Error:` line, or we refuse with our variant). Both refusing is
    agreement.
  - An unpinned difference fails, and so does a stale pin.
- [ ] **AC6 (exit codes, rule 5):**
  - Every ExifTool write run goes through one function. It returns `Written(path)` (exit 0 and the file exists) or
    `Refused(first Error: line)` (exit 1 and no file). Anything else is `ToolError` and fails, never skips.
  - The function is unit-tested on canned tuples: (0, file), (1, no file), and (0, no file), (1, file), (2, no file),
    each of which is a `ToolError`.
  - It never passes `-m`. The output path is fresh for each run.
- [ ] **AC7 (comparator red-proofs, no ExifTool needed, against committed records with one line edited in memory):**
  - The judge goes red on (i) a removed tag left in; (ii) a kept tag missing; (iii) a kept value changed;
    (iv) a phantom tag; (v) the carrier exception claimed on a fixture with no carrier; (vi) the whole-block exception
    claimed under `AllPii`.
  - The differential goes red on (vii) an unpinned difference and (viii) a stale pin.
  - Replay goes red on (ix) one flipped output byte against the record's digest.
  - Each red names the fixture, the policy and the tag.

### Part B: completeness sweep and writer red-proofs

- [ ] **AC8 (shared sweep):** the four helpers and the scratch directory move to `tests/support/sweep.rs`.
  `strip_webp_sweep_live.rs` imports them and still passes. Its red-proof still goes red.
- [ ] **AC9 (PNG sweep, DEC-023 rule 6):** `tests/strip_png_sweep_live.rs`.
  - **Candidates:**
    - every four-character `-listx` id;
    - the pinned subdirectory list: `IHDR`, `IDAT`, `IEND`, `tEXt`, `zTXt`, `iTXt`, `eXIf`, `zxIf`, `iCCP`, `caBX`,
      `tXMP`, `meTa`, `seAl`, `cpIp`, `pHYs`, `cHRM`, `sCAL`, `sTER`, `vpAg`, `acTL`, `cICP`, `fcTL`, `fdAT`, `mDCV`,
      `cLLI`, `orNT`;
    - all 16 case spellings of every code the writer names.
  - **Shapes:** one chunk between `IHDR` and `IDAT`, CRC-correct, in 10 payload shapes: text, TIFF with Artist, XMP,
    zeros, `\0`+len+zlib(TIFF), `Exif\0\0`+TIFF, `Artist\0value`, the `zTXt` and `iTXt` keyword forms, and a *valid*
    JUMBF manifest. The design's JUMBF shape was malformed, and `caBX` printed nothing in it.
  - **Judge:** one ExifTool run per batch. A file fails when it prints a non-structural tag that the bare file doesn't
    print, and its code is neither in the writer's set (the `chunk_class` classes, the text, `iCCP`, `tIME` and
    `eXIf` rules) nor in the pinned structure list with exactly the pinned tag names. The structure list is DEC-020
    clarification 4 plus `PLTE`.
  - **Keyword sweep:** every `PNG::TextualData` keyword, in `tEXt`, `zTXt` and `iTXt`. ExifTool's tag name, mapped
    through `i3_judge`, must equal `text_keyword_category`.
  - **Red-proofs:** removing one code from the known set turns the sweep red, and removing one keyword row from the
    judge's map turns the keyword sweep red.
  - The stated residual (rule 6) is in the file's doc comment.
- [ ] **AC10 (writer mutated-copy red-proofs).** The probe gains a strip mode. Four mutants are each injected into a
  copy of `src/` with a unique needle:
  - **W1 (PNG):** `tXMP` is dropped from `png::metadata::chunk_class`. Fixture: the PNG `tXMP` carrier shape, under
    `Gps`.
  - **W2 (WebP):** `SEAL` is dropped from `webp::metadata::chunk_class`. Fixture: the WebP `SEAL` row, under `Gps`.
  - **W3 (PNG):** the `Copyright` keyword row maps to `FreeText`. Fixture: the `Copyright` keyword shape, under
    `AllPii`. This is an over-removal.
  - **W4 (WebP):** `webp::metadata::chunk_class` returns `XmlCarrier` for `C2PA` instead of `Jumbf`. Fixture: the
    WebP `C2PA` row, under `All`. This is an over-removal: a carrier goes under every preset, but `All` keeps
    `Jumbf`.

  For each mutant:
  - (1) The control (the unmutated copy) reproduces the committed output digest.
  - (2) The mutant's `strip` returns `Ok`. This proves it escapes the self-check: the audit and the writer agree on
    the wrong fact.
  - (3) Replay goes red on the digest.
  - (4) Live (gated) goes red, with the judge naming the tag: an `XMP`/`XML` tag left in for W1 and W2, `PNG:Copyright`
    missing for W3, a `JUMBF` group tag missing for W4.

  The injection marker never leaks (the existing guard test covers the new needles).

### Part C: switch-over, docs, JPEG preset rows

- [ ] **AC11 (switch-over, DEC-020 amendment 1):**
  - `ContainerKind::strippable()` is true for `Jpeg`, `Tiff`, `Png` and `Webp`, and false for every `Unsupported`
    kind (test over all `Sniffed` values).
  - For a clean PNG and a clean WebP, the preview's dispositions follow DEC-009 rule 2 (a hand-typed grid per preset
    over a file with Exif GPS, XMP, ICC, a `Copyright` text and a `caBX`/`C2PA`), and `reliable()` is true.
  - The text renderer no longer prints "has not been checked against ExifTool".
  - JSON has `"strippable":true`, and CSV dispositions are no longer `not_strippable`.
  - The CLI on the PNG/WebP golden files prints removed and kept counts.
  - The goldens are regenerated, and every changed line class is listed in the handback.
  - HEIC, GIF, BigTIFF, empty and unknown still say not strippable (pinned).
- [ ] **AC12 (honesty on the corpus, amendment 2):** for every I3 fixture under every preset, if `strip` refuses then
  `preview(...).reliable()` is false. On a failure, the test names the fixture, the policy and the reason.
- [ ] **AC13 (errors and docs):**
  - `NotJpeg`'s `Display` is neutral, and so is `Unstrippable`'s (decision 3), with a test pinning both strings.
  - README:
    - PNG and WebP are strippable, in the library and in the audit/CLI preview. The CLI still never writes a file.
    - I3 is shown by records, the judge, the differential and red-proofs.
    - The residuals: C2PA/`caBX` contents kept except under `Everything`. XMP GPS printed as `Composite:GPS*` under
      `Gps`, `GpsAndDevice` and `AllPii`, which ExifTool's own `-gps:all=` also leaves. Carriers and every WebP `LIST`
      removed under every preset.
    - A strip may still refuse, typed, a file whose preview shows removals.
  - The doc comments on `strippable()`, `Preview::strippable()` and `reliable()` are updated.
- [ ] **AC14 (JPEG preset rows, the JFIF signal):**
  - `strip_records::policies()` gains `("gps", Gps)` and `("all-pii", AllPii)`, appended last.
  - `just oracle-record` re-records `exiftool-strip/` once, then twice with no diff.
  - The group-table and tag-table tests pass with the new sections.
  - For the four `jfif-*` fixtures, the `gps` section still holds the `JFIF` thumbnail group and the `all-pii` section
    does not, checked both by replay and live.
- [ ] **AC15 (gates, on the pinned 1.90.0):** `just lint`, `just lint-fuzz`, `just lint-no-allow`, `just test` with
  `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `just mutation-oracle`, `just forbid-unsafe`, `just deny`, `just deny-fuzz` and
  `just validate`; the 60 s smokes `audit`, `audit_render`, `rewrite_png` and `rewrite_webp`; and an empty
  `git diff origin/main -- fuzz/Cargo.lock`.

## Failing Tests

Write these first. Each must be red before its code. Type every expected table by hand from DEC-023, DEC-020's
amendment and this spec's counts. Never derive one from the code under test.

- **`src/audit/category.rs` unit tests:** `"make model and timestamp keywords have their categories"` (AC1; with the
  first-letter and all-caps rows).
- **`tests/strip_png_webp_oracle.rs`:**
  - `every_i3_fixture_has_a_record_with_the_expected_sections` (AC2)
  - `records_replay_and_bind_to_todays_output_bytes` (AC2)
  - `records_reproduce_live` (AC2, gated)
  - `refusals_are_exactly_the_pinned_table` (AC3)
  - `the_judge_holds_both_ways_for_every_fixture_and_policy` (AC4, replay and live)
  - `ours_and_exiftools_writer_agree_except_the_pinned_differences` (AC5, replay and live)
  - `write_runs_classify_by_exit_code_and_output_file` (AC6, canned tuples)
  - The comparator red-proofs `comparator_is_red_when_*`, one per case (i) to (ix) (AC7).
  - `refused_strips_never_preview_as_reliable` (AC12)
- **`tests/strip_webp_sweep_live.rs`:** unchanged tests, now on `tests/support/sweep.rs` (AC8).
- **`tests/strip_png_sweep_live.rs`:**
  - `every_png_code_exiftool_decodes_is_known_or_structure`
  - `every_textual_data_keyword_has_the_judges_category`
  - `the_sweep_goes_red_on_a_code_missing_from_the_known_set`
  - `the_keyword_sweep_goes_red_on_a_missing_row` (AC9)
- **`tests/png_mutation_oracle.rs`:** `writer_mutant_w1_txmp_not_a_carrier_escapes_the_self_check_and_i3_names_it`
  and `writer_mutant_w3_copyright_as_free_text_is_named` (AC10).
- **`tests/webp_mutation_oracle.rs`:** `writer_mutant_w2_seal_not_a_carrier_*` and `writer_mutant_w4_c2pa_as_carrier_*`
  (AC10).
- **`tests/audit_png_webp.rs` / `tests/policy_preview.rs`:**
  - `png_and_webp_are_strippable_and_unsupported_kinds_are_not` (AC11)
  - `png_and_webp_dispositions_per_preset` (AC11, a hand-typed grid)
- **`tests/report_render.rs` / `tests/cli.rs`:** the goldens, and `"no ExifTool-unchecked wording remains"` (AC11).
- **`tests/strip.rs`:** `strip_error_display_is_container_neutral` (AC13).
- **`tests/strip_oracle.rs`:** `jfif_fixtures_keep_the_thumbnail_under_gps_and_lose_it_under_all_pii` (AC14), and the
  existing section-list test extended.

## Non-Goals

- A release (its own spec afterwards). A CLI strip command (the CLI never writes). `set` operations. HEIC.
- A differential for `GpsAndDevice`, `AllPii` and `All`. ExifTool has no single op for them, and spelling one out as a
  tag list would restate our category table in ExifTool syntax. Layer A covers them.
- Byte equality with ExifTool's output (design §8: rebuild ordering legitimately differs).
- Changing the writer to match ExifTool on D1 to D5. Each class is ours by decision: never add a chunk, keep `VP8X`,
  remove carriers, keep each `EXIF` chunk's own values, and remove a malformed text chunk.
- Renaming `StripError::NotJpeg` (decision 3).
- JPEG and TIFF differentials (a possible follow-up; question below).

## Notes for the Implementer

- **Three builder parts, one agent each, so each stop is clean.**
  - **Part A:** AC1 to AC7. Commit AC1 first: the keyword rows change findings, and so the records.
  - **Part B:** AC8 to AC10.
  - **Part C:** AC11 to AC14.
  - Each part ends at a commit with its gates green.
  - Part C can't start before A's records are green: the switch-over rests on them.
- **Building the records:**
  - Follow `strip_records.rs` and `examples/oracle_record.rs`. Run ExifTool through `tests/support/tools.rs` (spawn
    limit, `strip_perl_lib_dir`).
  - Write each ExifTool output to a fresh temp path: an existing path is exit 1.
  - Strip machine paths from `Error:`/`Warning:` lines.
  - Read with `-a -u -G1 -s -t -n` and drop the `File`, `System` and `ExifTool` groups. The values must be comparable,
    so `-t -n`, not `-H`.
- **The differential's multiset compare:** duplicate `EXIF`/`eXIf` blocks print the same `(group, tag)` several times
  (`-a`). Compare as multisets. D4 is exactly a multiset-and-value difference.
- **Reproducing the design's counts:** key level D1 72, D2 2, D3 7, D4 4, D5 1. Value level D4 3, D6 31, and 157
  identical pairs (DEC-023 amendment of 2026-11-25). There are 41 outcome differences, 30 both-refuse
  pairs, and 113 refused `(fixture, preset)` pairs. If the rebuilt harness disagrees, stop and report the difference.
  Don't re-pin it to match. Either the harness or the measurement is wrong, and the architect decides which.
- **The mutants must escape the self-check.** That's the point: a mutant the self-check catches proves nothing about
  I3. If W3 or W4 is caught (step (2) returns `Err`), find another one-expression fault at the shared fact, report it,
  and keep the over-removal direction.
- **Why W4 isn't an ICC fault:** the WebP and PNG writers decide `ICCP`/`iCCP` with a hardcoded `Category::Icc`
  (`src/write/webp.rs:185`, `src/write/png.rs:268`), not through the audit. A mutated ICC category would make the two
  disagree, and the self-check would catch it. W1 to W4 each sit on a fact both read: `chunk_class` (W1, W2, W4) and
  `text_keyword_category` (W3, `src/write/png.rs:270`).
- **Goldens:** regenerate them with the existing mechanism. Read every changed line, and list the classes in the
  handback ("preview line: not strippable → removed N, kept M", "JSON strippable false → true", "CSV disposition").
- **Toolchain:**
  - Run `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"`, and check that `~/.cargo/bin/cargo-clippy
    --version` prints 0.1.90.
  - macOS has no `timeout`; use `perl -e 'alarm 8; exec @ARGV' --`.
  - ExifTool 13.55 is at `/opt/homebrew/bin/exiftool`.
  - Restore `fuzz/Cargo.lock` after fuzz and lint-fuzz runs.
  - Stage files by path, and never commit `decisions/INDEX.md`.
  - Never run ExifTool on `fuzz/seeds/tiff_ifd/two-hop-cycle`.
- **Budget:** about 120 tool calls or 300k context per part. Stop at a clean commit and report rather than loop.

**Open question for after ship:** should JPEG and TIFF get the same differential (`-all=` ↔ `Everything`,
`-gps:all=` ↔ `Gps`)? Their oracles compare against declared tables only. This is not raised as a question entry yet;
ship decides whether to record it.

---

## Reflection

*Appended during **ship**. Three questions, short answers.*

1. **What would I do differently next time?**
   — <answer>

2. **Does any template, constraint, or decision need updating?**
   — <answer — if yes but not done this session, record it in
   `/guidance/signals.yaml`: `type: lesson` (with its N-count) for a recurring
   coding pattern, `type: process-debt` for tooling/process friction. A close
   then forces the decision. See `docs/signals.md`.>

3. **Is there a follow-up spec I should write now before I forget?**
   — <answer>

4. **Where was the worst defect caught?** — one word from a fixed vocabulary so
   the defect-escape distribution is greppable across specs:
   `design` | `build` | `verify` | `ship` | `escaped` (reached prod/runtime) |
   `none` (clean first try).
   — <one word>
   *(Runtime/operational defects — the escape-prone class — only exist once the
   artifact meets its real host. `escaped` here is a signal to strengthen the
   §12 behavioral pre-flight for that surface.)*

5. **What can a user do now that they couldn't before?** — one sentence,
   before → after; quote the confirming number if one exists, name the outcome
   if not. Write `none` if this spec has no user-visible outcome — that is a
   real, greppable result, not a blank. This is the line a downstream work-log's
   `impact` field is transcribed from, and both halves are already written above
   (## Context is the before, ## Goal is the after): confirm the prediction,
   don't reconstruct it from memory.
   — <answer | none>
