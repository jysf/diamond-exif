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
  id: HANDOFF-091
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-26
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
  agent: claude-opus-5-5           # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: punch-list              # verify handoffs: approved | punch-list | rejected
  basis: agent-result              # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 258606             # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 26
  branch: claude/spec-039-verify
  pr: null
  completed_at: 2026-11-26         # YYYY-MM-DD
  notes: "Round 2 (2026-11-26, branch claude/spec-039-verify-r2): PUNCH LIST against 34008cc, 2 items (L): the multiplicity bound and the plain-block carrier-left-in check have no red-proof (mutants M1, M3 survive); the line-level model inverts on a non-additive (carrier-before-plain) shape, not in the corpus. Round-1 items 1-3 fixed. Round 1: Verdict applies to f5e24c1 (build tip plus this handoff). Cost filled by the orchestrator from the harness subagent result: 258,606 tokens, 96 tool calls, 1,553 s. Punch list of 3 items: 1 (M) judge exception (a) is group-level and excuses a dropped plain Exif/XMP block when a carrier coexists; 2 (L) the AC11 grid has no ICC finding; 3 (L) the README PNG keyword sentence omits Make, Model and TimeStamp."                      # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: 2026-11-26
---

# HANDOFF-091: SPEC-039 verify — ExifTool write parity (I3) for PNG and WebP strip, audit and CLI switch-over

**FIRST STEP:** `git checkout -b claude/spec-039-verify feat/spec-039-png-webp-write-parity-build`. Its tip is
the build plus the commit that adds this handoff. Work read-only on the library and the tests. Commit only to this
handoff file, on `claude/spec-039-verify`.

## Delegation Summary

`claude-opus-5-5` (architect) hands SPEC-039 to an independent `claude-opus-5-5` verifier for the **verify** cycle.
Three `claude-sonnet-5-5` implementers built it from HANDOFF-090, one per part (A: AC1–AC7, B: AC8–AC10, C:
AC11–AC15). The architect amended DEC-023 twice during the build. You did not write this code or these decisions.
Review them cold.

**Read first:**
- `AGENTS.md`: §12 and §15 "During verify".
- `guidance/toolchain-brief.md` and `guidance/constraints.yaml`.
- The SPEC-039 spec: the Context's measured facts, Decisions made here, AC1–AC15, Failing Tests, Notes.
- DEC-023 in full, including its amendments of 2026-11-25 (value-level differences, class D6) and 2026-11-26
  (exception (c) covers its converse).
- DEC-020: clause 6, clarification 4, the amendment of 2026-11-24.
- DEC-021 clause 4. DEC-022 clauses 4 and 7.
- DEC-009, DEC-008, DEC-013.
- `HANDOFF-090-*.md`: the build brief, the three part entries, and the orchestrator notes after each. The notes
  list the deviations the architect accepted.

**The change:** `git diff 100b68c feat/spec-039-png-webp-write-parity-build`, 302 files. Most of them are ExifTool
records. `100b68c` is the design commit, now merged into `main`.

**Toolchain:**
- `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"`, then confirm clippy reports 0.1.90. Fuzz needs
  nightly.
- macOS has no `timeout`. Use `perl -e 'alarm 8; exec @ARGV' --`.
- ExifTool 13.55 is at `/opt/homebrew/bin/exiftool`. Run it, never read it. Never pass `-m`. Always give it a fresh
  output path.
- Restore `fuzz/Cargo.lock` after any fuzz or lint-fuzz run.
- Never commit `decisions/INDEX.md`.
- Give each historical tree its own `CARGO_TARGET_DIR`.
- Leave no background processes. Write scratch scripts in a scratch directory, not in the repo.

## Decisions that apply

- **`DEC-023` and its two amendments:** records with output digests (rule 1), recorded policies (2), the judge and
  exceptions (a)/(b)/(c) (3), the differential with key-level D1–D5 and value-level D4/D6 (4), exit codes (5), and
  the sweep residual (6).
- **`DEC-020` and its amendment of 2026-11-24:** the switch-over, the per-file honesty note, three keyword rows,
  and `PLTE` as structure.
- **`DEC-021` / `DEC-022`:** the carrier rule, the whole-block rule, and the `VP8X` flag rule. D6 rests on that
  last one.
- **`DEC-013`:** the self-check that the W1–W4 mutants must escape.
- **`DEC-009` / `DEC-008`:** the presets and categories the judge is typed from.

## Constraints that apply

- **`test-before-implementation` and `oracle-must-be-shown-red`:** AC7, AC9 and AC10 carry recorded red evidence
  in HANDOFF-090.
- **`exiftool-never-linked`, `no-io-in-library`, `panic-free-library`, `forbid-unsafe-code`.**
- **`no-new-top-level-deps-without-decision`:** FNV-1a is hand-written.

## What to establish (evidence for each, not opinion)

1. **The records are honest.**
   - Re-record into a scratch copy and diff it against the committed `exiftool-strip-png/`, `exiftool-strip-webp/`
     and `exiftool-strip/`. Expect byte equality.
   - Confirm that replay recomputes `strip` and fails on a digest mismatch: flip one byte of the writer's output in
     a scratch tree.
   - Confirm the records hold no machine paths.
   - Confirm a refused ExifTool write is a `# refused:` section.
2. **The differential is real and its pins are right.** Re-derive the counts over the 174 oracle fixtures with your
   own script, not the implementers' harness. Compare ExifTool's readings of our output and of ExifTool's own
   `-all=` / `-gps:all=` output, at both key and value level. Expected:
   - 157 identical;
   - key level: D1 72, D2 2, D3 7, D4 4, D5 1;
   - value level: D4 3, D6 31;
   - 41 outcome differences and 30 both-refuse.

   Then sample at least one fixture per class and check the class label against the mechanism DEC-023 states for
   it. For D6 especially: is every pinned `WebP_Flags` difference really ExifTool recomputing flags while we clear
   only removed kinds, or is any of them our writer leaving a flag for a chunk it removed (a DEC-022 clause 7 bug)?
3. **The judge cannot pass vacuously.**
   - Check the exception (a)/(b)/(c) conditions in `tests/support/i3_judge.rs` against DEC-023 rule 3 and both
     amendments.
   - Look for a case where an exception excuses a real leak or a real over-cut.
   - Assess the stated limit of the (c) converse: inputs with GPS in both Exif and XMP. Is it as narrow as the
     amendment claims?
   - Build one such input yourself and see what happens.
4. **The writer mutants and sweeps.**
   - Reproduce W1–W4: control green, `strip` returns `Ok`, replay red, live red. Confirm each mutant lives only in
     the test-side probe and never touches `src/`.
   - Reproduce AC9's two red-proofs.
   - Check that the sweep's 29,130 files match AC9's candidate definition (the orchestrator notes explain why this
     is not the Context's 35,020).
5. **The switch-over is honest (AC11–AC13).**
   - Diff `src/` (37 insertions, 34 deletions). Every change must follow from DEC-020's amendment items 1 and 2 or
     from decision 3's neutral `Display` strings.
   - Check every golden and pin change in `tests/golden/report-png-webp.*`, `audit_png_webp.rs`, `report_render.rs`,
     `cli.rs`, `strip_png.rs`, `strip_webp.rs`, `policy_preview.rs` and `fuzz_layout.rs`. Each must be explained by
     the flip.
   - The `fuzz_layout.rs` plants were re-aimed. Confirm each still goes red on its own.
   - Build PNG and WebP inputs yourself and look for a file whose preview says `reliable` while `strip` refuses it
     (AC12).
   - Read the README's new claims against the evidence.
6. **The JPEG `gps`/`all-pii` records (AC14), and the accepted deviation `entry_edit_groups`** in
   `tests/support/strip_records.rs`.
   - Confirm `git diff 100b68c -- tests/oracle-fixtures/exiftool-strip` contains only additions.
   - Judge whether `entry_edit_groups` is sound. Could it hide a writer that wrongly empties a group? Is it typed
     independently of the library?
   - Check the JFIF thumbnail kept under `gps` and gone under `all-pii`.
7. **Panic-freedom and constraints.** Check the library diff. Run `just decisions-audit --changed`.
8. **Gates on 1.90.0:**
   - `just lint`, `just lint-fuzz`, `just lint-no-allow`, `just forbid-unsafe`;
   - `just deny`, `just deny-fuzz`;
   - `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test`;
   - `just mutation-oracle`;
   - `just validate`;
   - 60 s smokes of `audit`, `audit_render`, `rewrite_png`, `rewrite_webp`.

Output exactly ONE verdict:
- ✅ APPROVED (at commit SHA);
- ⚠ PUNCH LIST: numbered items, each with severity H/M/L, evidence and a suggested fix;
- ❌ REJECTED.

Write it into the `## Handback` section and into `handback.verdict` (approved | punch-list | rejected).

## Budget

About 120 tool calls or 300k context. If you get near either, report what you established and what you did not,
then stop.

## Out of Scope

- Fixing anything. Report it instead.
- Byte equality with ExifTool's output.
- Changing the writer to match ExifTool on D1–D6.
- A CLI strip command, `GpsAndDevice`/`AllPii`/`All` differentials, and HEIC.
- CHANGELOG, push, or PR.

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

- **Branch / PR:** `claude/spec-039-verify` (from `feat/spec-039-png-webp-write-parity-build` at `f5e24c1`); no PR.
- **Completed at:** 2026-11-26
- **All acceptance criteria met?** Yes for AC1 to AC15 as tested. Item 1 below is a hole in the AC4 judge's exception
  (a), not a failing test.
- **For `verify`:** ⚠ PUNCH LIST, against `f5e24c1`.

**Punch list**

1. **(M) Judge exception (a) is group-level. It excuses a dropped plain Exif (or XMP) block when a carrier of the same
   family coexists.**
   - Where: `tests/support/i3_judge.rs`, `judge`, the `carrier_line` test:
     `(carriers.exif && in_exif) || (carriers.xml && xml_group(&k.0))`.
   - The problem: any missing tag in an Exif (or XML) group is counted as a carrier use. That holds even when the
     file also has a plain `eXIf`/`EXIF` (or plain XMP) whose kept tags must survive. The positive "carrier tag left
     in" check is skipped when `exif_plain`/`xml_plain` is set, so nothing compensates. DEC-023 rule 3 (a) excuses
     only "every tag a carrier chunk prints".
   - Evidence: I built `plain-and-carrier.png` with my own builder: a plain `eXIf` (Make `VerifyCam`, Model,
     Orientation, GPS) plus a `Raw profile type exif` zTXt (Make `CarrierCam`).
     - Real strip under `gps`: the judge gives 0 failures and 3 carrier uses. That is correct.
     - Simulated over-cut: I deleted every `IFD0` line from the `gps` output, as if the writer had dropped the plain
       `eXIf` too. The judge still gives **0 failures** (carrier uses 6).
     - Control: the same deletion on `both-gps-same.png`, which has no carrier, is red (`kept tag missing
       IFD0:Make=VerifyCam`, Model, Orientation).
   - Corpus exposure: `png-exif-and-raw-exif` has this shape. Under `gps` the differential would still catch it (an
     unpinned key difference). Under `gps_and_device`, `all_pii` and the Custom rows only the judge covers it, so a
     writer change plus a re-record would pass.
   - The `derived` Composite GPS clause has the same breadth: any `carriers.exif`/`xml` excuses a missing
     `Composite:GPS*`.
   - Suggested fix: when `exif_plain` (or `xml_plain`) is also set, excuse at most the multiset of lines the carrier
     itself prints. One way is to read each carrier chunk alone, wrapped in a bare PNG or WebP, at record time and
     commit that reading. Another is to excuse only the multiset difference between the input reading and a reading
     of the input without the carrier. Add a red-proof: a plain block dropped beside a carrier must fail. If the
     architect prefers to keep the group-level rule, record that limit in DEC-023 like the (c) converse limit.
2. **(L) The AC11 dispositions grid has no ICC finding.**
   - AC11 asks for a grid "over a file with Exif GPS, XMP, ICC, a `Copyright` text and a `caBX`/`C2PA`".
     `grid_png`/`grid_webp` in `tests/audit_png_webp.rs` carry Gps, Xmp, Rights and Jumbf, but no `iCCP`/`ICCP`.
   - So the `Icc` column (kept by four presets, removed by `Everything`) is not pinned for PNG or WebP by this test.
     The golden WebP file does show an ICC `kept` line under `all_pii` only.
   - Suggested fix: add `iCCP`/`ICCP` to both grid files and an `Icc` column to the grid: `kept` for gps,
     gps_and_device, all_pii and all; `removed` for everything.
3. **(L) The README's PNG keyword sentence is stale after AC1.**
   - The PNG audit paragraph still lists `Artist`/`Author`, `Copyright`, `Creation Time`/`create-date`/`modify-date`
     and `Software`, then says "the rest free text".
   - Since DEC-020 amendment 3, `Make`/`Model` are camera model and `TimeStamp` is a timestamp (`grep -c CameraModel
     README.md` gives 0 hits for the PNG list).
   - Suggested fix: add the three rows to that sentence.

**Evidence for the eight items**

1. **The records are honest.**
   - Re-record: I extracted `git archive HEAD` into a scratch git repo and ran `cargo run --release --example
     oracle_record` (own `CARGO_TARGET_DIR`): 27 strip, 12 entry, 16 strip-tiff and 238 PNG/WebP records written.
     `git status` was empty, so all four record dirs are byte-equal, `exiftool-strip/` included.
   - Digests, my own script: my own dumper crate (`exif_core::strip`, my own FNV-1a 64) gave 1,286 `# output:`
     lines equal to the records, with 0 mismatches, 0 missing sections and 0 sections for a refused strip.
   - Byte flip: in the scratch tree I wrapped `strip` to flip one byte of PNG output that has `eXIf`. `records_replay`
     went red: `png-exif-basic-le under gps: the writer's output is 416 bytes, digest d17b081a2d744199; the record
     read 416 bytes, digest a389c48e9b0f803a`.
   - Machine paths: `grep -rlE '/Users|/private|/tmp|/var/folders|/opt'` over the three record dirs found 0 files.
   - Refused writes: 59 `exiftool:*` sections are a single `# refused:` line with no other line. Every record starts
     with `input` and ends with `exiftool:all`, `exiftool:gps:all`, with 0 exceptions.
2. **The differential is real and its pins are right.**
   - My own Python, independent of `i3_records`/`i3_judge`: `exiftool -all=`/`-gps:all= -o <fresh>`, exit code
     plus file existence, then `-a -u -G1 -s -t -n` minus File/System/ExifTool, compared as multisets.
   - Over the 174: identical 157; key level 86 = D1 72, D2 2, D3 7, D4 4, D5 1; value level 34 = D4 3, D6 31;
     ExifTool refuses 29, we refuse 12 (41 outcome differences); both refuse 30; ToolError 0.
   - Over all 238: 183 differences. Each matches a `DIFFERENCES` pin by (fixture, pair, level) and key lines, with 0
     mismatches. My 41 outcome keys equal `OUTCOMES`.
   - Refused preset pairs: 113 (13 fixtures under five presets, 16 under three).
   - Class mechanisms:
     - D1: our extra keys are exactly the `VP8X`-derived five and ExifTool has none.
     - D2: the reverse, on `webp-exif-no-vp8x` and `webp-vp8x-len-0`.
     - D3: ExifTool-only carrier tags on the 7 raw-profile fixtures.
     - D4: the 4 key pairs and 3 value pairs are duplicate-`EXIF` rewrites (`Block00..` against `Block15` x16).
     - D5: `PNG:TextualData` on `png-text-no-nul`.
   - D6, all 31 checked with my own RIFF walker: in every pair our `VP8X` flags are byte-identical to the input's, and
     no metadata kind was fully removed. So no pair is our writer leaving a flag for a chunk it removed (DEC-022
     clause 7). Each is ExifTool recomputing flags the input had wrong: lower-case or ANMF-nested chunks, an empty
     packet, flags unset or extra.
3. **The judge cannot pass vacuously.**
   - Checked against rule 3 and both amendments. (b) is gated by `holds_every_exif_category` and is red-proofed under
     `all_pii`. (c) kept direction is limited to `Composite` with XMP GPS in the output. (c) converse is limited to
     `Composite:GPS*`, XMP GPS in the input, and the policy removing Xmp. (a): see punch item 1.
   - Both-GPS inputs, built by me (`both-gps.png`, `both-gps-same.png`, `both-gps.webp`; Exif GPS 10/20, XMP GPS 30/40
     or equal): the judge is clean under all five presets and Custom `xmp`.
     - Under `xmp` the converse is used 2 times. `Composite:GPSLatitudeRef`/`LongitudeRef` go, because ExifTool
       derives them only from XMP; `GPS:` stays.
     - Simulated Exif GPS loss under `xmp`: red, with 4 `kept tag missing GPS:*` lines.
   - So the limit is as narrow as the amendment says: only `Composite:GPS*` lines can be excused, and the source
     `GPS:` lines are always checked.
4. **The writer mutants and sweeps.**
   - W1 to W4: `cargo test --test png_mutation_oracle --test webp_mutation_oracle writer_mutant` printed `control
     digest equal true; strip Ok true; replay red true; live red true` for each. The live reds name
     `XMP-dc:Creator=Zed`, `XML-dc:Creator`, `PNG:Copyright=SweepCopyright` and `JUMBF:JUMDLabel=c2pa`.
   - The mutants are staged by `probe::stage_probe_crate` + `inject_replace` in temp dirs, and `git status` stayed
     clean. `git diff 100b68c -- src` touches only `audit/category.rs`, `audit/mod.rs`, `audit/preview.rs`,
     `report/text.rs` and `write/mod.rs`, with no needle text.
   - AC9 red-proofs reproduced: `without tXMP in the known set ... neither the writer's nor pinned as structure` and
     `without the Make row in the judge ... text_keyword_category says Some(CameraModel)`.
   - The sweep ran 2,913 codes x 10 = 29,130 files, 0 unexplained. My own count from `exiftool -listx` (4-char ids
     union the 26 pinned types union the 16 spellings of the 17 writer codes) is 2,913 = 29,130 files. That matches
     AC9's definition. The 17 codes equal `png/metadata.rs` `chunk_class` plus text, `iCCP`, `tIME` and `eXIf`/`zxIf`.
5. **The switch-over is honest.**
   - The `src/` diff for Part C is the `strippable()` `matches!`, docs, the deleted two PNG/WebP wording arms, and
     the neutral `NotJpeg`/`Unstrippable` `Display` and docs. Nothing else. `category.rs` holds the three AC1 rows plus
     a hand-typed test. The fuzz mirror adds the same rows, typed separately.
   - Goldens: only these line classes changed:
     - `not_strippable` became `removed`/`kept_unread`/`kept` (CSV, 9 lines);
     - finding lines `kept (not strippable)` became the DEC-009 dispositions;
     - the two "not checked against ExifTool" preview lines became counts;
     - the summary changed to reliable 2/0 with residual 4;
     - JSON changed one line.
     Every disposition follows DEC-009 rule 2 for `all_pii`.
   - Test pins in `audit_png_webp`, `cli`, `report_render`, `policy_preview`, `strip_png`, `strip_webp`: each changed
     assertion inverts a "not strippable" pin or adds the grid.
   - `fuzz_layout`: `84 plants, each rejected by its own guard alone`, `21 preview plants ...`; all of
     `cargo test --test fuzz_layout` is green.
   - AC12, built by me: PNG and WebP with an `eXIf`/`EXIF` whose Make string shares bytes with `GPSLatitude`
     (`SharedBytes`), and two GPS pointers in IFD0 (`HiddenPointer`). `strip` refuses under `gps`/`gps_and_device`
     (and `all_pii`/`xmp` for `HiddenPointer`) while `preview(...).reliable()` is **true**.
   - That is the residual DEC-020 amendment item 2 accepts ("the same as a JPEG's"), and the README says it in one
     sentence. AC12's corpus claim holds (the test is green over 238 fixtures). Not a punch item.
   - The README's other new claims match the evidence:
     - `-gps:all=` leaves XMP-derived `Composite:GPS*`: measured on `both-gps.png`, ExifTool's output keeps
       `GPSPosition 30 40`.
     - C2PA is kept except under `Everything`.
   - The keyword sentence is stale (punch item 3).
6. **JPEG records and `entry_edit_groups`.**
   - `git diff --numstat 100b68c -- tests/oracle-fixtures/exiftool-strip` shows 25 files, +1074 -0.
   - `entry_edit_groups` is typed from the builders. I checked it against `full_tiff`:
     - ExifIFD = DateTimeOriginal, 0xA431, UserComment, MakerNote;
     - GPS all Gps;
     - IFD1 = thumbnail offset and length.
     `no_gps_tiff` and `exif_camera` also match. It imports no library table.
   - It is sound. It drops a group from `want` only when the policy holds every listed category, and the comparison is
     set equality. So it asserts the group's absence and cannot hide a group emptied under a policy that lacks one of
     its categories, which stays wanted and fails as missing.
   - Minor: its doc says "a group with an entry of no category is never listed", yet the `mpo-*` IFD0 holds the
     ExifIFD pointer. The pointer is structural, so this is harmless.
   - JFIF: in `jfif-thumbnail`, `-short` and `-two-segments`, `JFIF:ThumbnailTIFF` is in the `gps` section and absent
     from `all-pii`. `jfif-no-thumbnail` keeps `JFIF` (4 lines) in both.
     `jfif_fixtures_keep_the_thumbnail_under_gps_and_lose_it_under_all_pii` passes with the oracle required.
7. **Panic-freedom and constraints.**
   - The library diff adds no indexing, arithmetic, `unwrap` or `expect`: a `matches!`, a `write!` string, three data
     rows and a `#[cfg(test)]` test.
   - `just lint-no-allow` and `just forbid-unsafe` pass. `[dependencies]` is unchanged, and FNV-1a is hand-written in
     `i3_records.rs`.
   - `just decisions-audit --changed` returned rc 0 (it fell back to `origin/main...HEAD`, which contains `100b68c`).
     It flags DEC-001 to DEC-004, DEC-006 to DEC-013 and DEC-017 to DEC-023, advisory only. No drift found beyond item
     1 (DEC-023 rule 3 (a)).
8. **Gates on 1.90.0 (clippy 0.1.90):**
   - `just lint` rc 0; `just lint-fuzz` rc 0; `just lint-no-allow` rc 0; `just forbid-unsafe` rc 0.
   - `just deny` rc 0; `just deny-fuzz` rc 0; `just validate` rc 0.
   - `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test` rc 0: 41 `test result: ok` lines, 1,532 passed, 0 failed, 20 ignored.
   - `just mutation-oracle` rc 0: 67 + 42 + 43 passed, 1 ignored each.
   - 60 s smokes, rc 0, no crash: `audit` 157,936 runs; `audit_render` 32,902; `rewrite_png` 550,295;
     `rewrite_webp` 1,008,322.
   - `fuzz/Cargo.lock` was restored, and `git diff origin/main -- fuzz/Cargo.lock` is empty.

Scratch apparatus (outside the repo, session scratchpad): `dumper/` (fixture bytes, strip outputs, digests, preview
reliability), `scripts/diff.py` (differential), `scripts/riff.py` (VP8X/kind walker), `scripts/build.py` (my
inputs), `judgerun/` (calls the committed `i3_judge::judge` on arbitrary readings, with a stub `Line`).

### Fix round (2026-11-26)

Implementer of the punch list above, on `feat/spec-039-png-webp-write-parity-build`. No change under `src/`, no new
dependency, no question added, no deviation from the brief.

**Commits** (in order):

- `c105798` test(spec-039): red-proofs for line-level judge exception (a), red (HANDOFF-091)
- `0b82da9` test(spec-039): judge exception (a) is line-level, carriers section recorded (HANDOFF-091)
- `720b899` test(spec-039): add ICC to the AC11 dispositions grid (HANDOFF-091)
- `3437f11` docs(spec-039): README PNG keyword sentence names Make, Model, TimeStamp (HANDOFF-091)

**Item 1 (M), exception (a) is line-level: done.**

- Red first. `c105798` adds the fixture `png-xmp-and-raw-xmp` (a plain XMP `iTXt` with `CreatorTool=DiamondTool` plus a
  raw-profile XMP `tEXt` with `RawTool`), its record, and its D3 pin (`gps`, theirs only `XMP-xmp:CreatorTool`). It adds
  two red-proof tests. Against the group-level judge both failed: the judge returned `[]` (no failures) for the planted
  over-cut. The controls passed.
- The fix, `0b82da9`:
  - `carriers_only(ext, bytes)` in `tests/support/i3_records.rs` builds the carriers file. PNG: signature, the 14
    skeleton chunk types (IHDR, PLTE, IDAT, IEND, tRNS, gAMA, cHRM, sRGB, sBIT, bKGD, pHYs, acTL, fcTL, fdAT) and the
    carrier chunks, byte for byte, in order. WebP: the RIFF header with a recomputed size, the skeleton chunks (VP8,
    VP8L, VP8X, ANIM, ANMF, ALPH) and the carrier rows. Every other chunk is dropped.
  - Carrier identification is typed there (DEC-021 clause 4, DEC-022 clause 4). `scan` was moved into the same file and
    shares the classification. Nothing is read from `src/`.
  - `record_text` writes `## carriers` right after `## input`, with the same read flags and dropped groups.
  - `judge` takes the carriers lines. A missing line is excused only up to its `(group, tag, value)` count there, and
    only if the file holds a carrier. `Composite:GPS*` is excused only when that section prints it. "Carrier tag left
    in" compares against the carriers lines by count when a plain block of the same family exists (output count above
    input count minus carrier count).
  - A missing `## carriers` section on a carrier fixture is a judge failure.
- Red-proof lines after the fix (`--nocapture`):
  - `png-exif-and-raw-exif under gps: kept tag missing IFD0:Make=AlphaCam` (plain `IFD0` lines deleted from the output
    reading; `IFD0:Model=OnlyInA` is missing too)
  - `png-xmp-and-raw-xmp under gps: kept tag missing XMP-xmp:CreatorTool=DiamondTool` (plain XMP lines deleted)
  - Controls pass: Exif under `gps` and `gps_and_device`; XMP under `gps`, `gps_and_device` and `all_pii`.
  - The old test `comparator_is_red_when_the_carrier_exception_is_claimed_without_a_carrier` is reworked to give the
    carriers reading explicitly: excused with a carrier file, red on a file with no carrier.
- New test `the_carriers_reading_prints_nothing_the_input_does_not`: over all 36 carrier fixtures, the carriers reading
  is a sub-multiset of the input reading. It holds.
- Re-record: `just oracle-record` twice. Run 1 added 36 `## carriers` sections (14 PNG, 22 WebP): 36 files, +453
  lines, 0 deleted. A script confirmed that every record minus its `## carriers` section equals the committed text.
  Run 2 gave no diff. `png-xmp-and-raw-xmp.txt` itself was added in `c105798`. No machine path in any record.
- The differential counts over the 174 oracle fixtures are unchanged. The tests that hold them pass.
- Judge exception uses over 239 fixtures x all recorded policies (2,583 sections, 598 with a removable tag, 2,408 tags):
  - **(a) carrier 677, (b) whole block 354, (c) residual 24.**
  - Before the round (HANDOFF-090): 670 / 354 / 24 over 238 fixtures. The 7 extra (a) uses are the new fixture, which
    has 7 carrier-removing policies that keep `Xmp`'s `CreatorTool=RawTool` line missing. So line-level excuses exactly
    as many lines as the group-level rule did on the 238 fixtures.
  - No real fixture failed the judge, so no writer finding and no question.
- Lower bounds in `assert_judge_holds` were raised to honest values: sections 2,000 to 2,500; removable sections 550 to
  590; removable tags 2,000 to 2,400; carrier 600 to 670; whole block 300 to 350; residual 20 to 24.
- Fixture-set test: 239 fixtures (92 PNG, 147 WebP), the new one last.

**Item 2 (L), ICC in the AC11 grid: done** (`720b899`). `grid_png` has an `iCCP`, `grid_webp` an `ICCP` (with
`FLAG_ICC`). `png_and_webp_dispositions_per_preset` now has Icc in the findings, and asserts `kept` under `gps`,
`gps_and_device`, `all_pii` and `all`, `removed` under `everything`. Hand-typed from DEC-009: `Icc` is removed only by
`Everything` (table row, rule 3), and `may_hold(Icc)` is empty (clarification 1), so it is never `kept_unread`.

**Item 3 (L), README: done** (`3437f11`). The PNG keyword sentence now reads `Creation Time`, `create-date`,
`modify-date` and `TimeStamp` timestamps, `Software` software, `Make` and `Model` camera model, the rest free text.

**Gates, Rust 1.90.0, `DIAMOND_EXIF_REQUIRE_ORACLE=1`, ExifTool 13.55:**

- `just lint` rc 0; `just lint-no-allow` rc 0; `just forbid-unsafe` rc 0; `just validate` rc 0.
- `just test` rc 0: 41 `test result: ok` lines, 1,535 passed, 0 failed, 20 ignored (1,532 before, plus the 3 new tests).
- `just mutation-oracle` rc 0: 67 + 42 + 43 passed, 1 ignored each.
- W1 to W4 (`writer_mutant`, replay and live): each `control digest equal true; strip Ok true; replay red true; live
  red true`.
- `fuzz/Cargo.lock` untouched (no fuzz run). Working tree clean after the commits. No background process left.

**Orchestrator notes on the fix round** (claude-opus-5-5, 2026-11-26)
- Re-run on 1.90.0 with ExifTool required: `just test` rc 0, 41 `test result: ok` lines, 1,535 passed, 0 failed.
  `just mutation-oracle` passed 67 + 42 + 43.
- No `src/` changes. `git diff 9900b2b HEAD -- tests/oracle-fixtures` touches 36 files, +634 −0, all additions: the
  `## carriers` sections plus the new `png-xmp-and-raw-xmp` record.
- Fix-round cost, from the harness subagent result: 194,326 tokens, 63 tool calls, 1,263 s (about 21 min). Recorded in
  the spec's `cost.sessions` as a build session against HANDOFF-091.

### Verify round 2 (2026-11-26)

Independent verifier (claude-opus-5-5), cold, on `claude/spec-039-verify-r2` from
`feat/spec-039-png-webp-write-parity-build` at `34008cc`. Scope: the three round-1 items and regressions. Change
reviewed: `git diff fef653a 34008cc` (amendment `9900b2b`; fixes `c105798`, `0b82da9`, `720b899`, `3437f11`,
`47ada16`; bookkeeping `34008cc`). Read-only on library, tests and records.

**Verdict: ⚠ PUNCH LIST, against `34008cc`.** Items 1 to 3 of round 1 are fixed as specified. The line-level code
matches the DEC-023 amendment and my own attacks are now red. Two low-severity items remain: two guards the amendment
names are not pinned by any test, and the line-level model breaks when ExifTool's reading is not additive.

**Punch list**

1. **(L) Two line-level guards have no red-proof. Mutants of each survive the whole `strip_png_webp_oracle` test.**
   - Where: `tests/support/i3_judge.rs`, `judge`.
   - M1 deletes the `*n -= 1` decrement of `excusable`, so the multiplicity bound of amendment item 2 goes. Result:
     `34 passed; 0 failed`.
   - M3 replaces the new "carrier tag left in" by-line block (`if (carriers.exif && carriers.exif_plain) || ...`) with
     `if false`, so amendment item 3 goes. Result: `34 passed; 0 failed`.
   - What M1 lets through, with my own inputs (below) run through the mutated judge: a plain block and a carrier that
     print the identical `(group, tag, value)` (`a3`, `a3c`, `a4c`). Dropping the plain block gives 0 failures under
     M1. With the committed judge it gives 1 failure (`kept tag missing IFD0:Make=SameCam`, and the same for
     `XMP-xmp:CreatorTool=SameTool`).
   - What M3 lets through: a carrier left in beside a plain block. On `a1`, "output = input" names
     `carrier tag left in IFD0:Make=CarrierCam` only through that block. On `png-exif-and-raw-exif` (no GPS) nothing
     else would catch `IFD0:Make=RawCam` left in under `gps`.
   - For contrast, M5 (the group-level rule restored) is red: both new red-proofs fail. M4 (the "no carriers section"
     failure removed) survives too. That is harmless, because `expected_sections` already requires the section on
     every carrier fixture.
   - Suggested fix: two in-memory red-proofs with controls.
     - (i) A plain block and a carrier that print the same line: the `a3` shape (`eXIf` Make `SameCam`, then a
       raw-profile Exif with Make `SameCam`; ExifTool prints the line twice), or an in-memory plant on
       `png-exif-and-raw-exif`. Both lines dropped must be red "kept tag missing", and one dropped must pass.
     - (ii) `png-exif-and-raw-exif` under `gps` with `IFD0:Make=RawCam` put back into the output must be red
       "carrier tag left in".
2. **(L) The line-level model assumes ExifTool's reading is additive. When a carrier comes before the plain block, it
   is not, and the judge is inverted.**
   - The amendment reads the carriers-only file as "what the carriers contribute". With a raw-profile Exif carrier
     *before* a plain `eXIf`, ExifTool reads only the carrier's IFD0 in context. It prints
     `Warning IFD0 pointer references previous IFD0 directory`, and the plain block's tags are absent from the input
     reading.
   - `b1` (carrier `CarrierCam` first, plain `VerifyCam`/`VerifyModel` + GPS second):
     - the correct strip under `gps` is **red** (`kept value changed IFD0:Make=VerifyCam`,
       `phantom tag IFD0:Model=VerifyModel`);
     - an over-cut (all `IFD0` lines dropped) is **green** (0 failures).
   - `a3b` (the same shape, identical values) behaves the same way: genuine red (`carrier tag left in`), over-cut green.
   - Exposure today is none:
     - the two corpus fixtures with a carrier and a plain block of the same family (`png-exif-and-raw-exif`,
       `png-xmp-and-raw-xmp`) put the plain block first, and their readings are additive (checked with ExifTool on
       the dumped bytes and their carriers-only files);
     - no corpus WebP has a plain `EXIF`/`XMP ` beside a carrier row;
     - a correct writer is red on this shape, so such a fixture cannot be added silently.
   - `the_carriers_reading_prints_nothing_the_input_does_not` checks only that the carriers reading is a
     sub-multiset of the input, so it does not see this.
   - Suggested fix (architect's choice):
     - add a fixture-set invariant: on judged lines, the input reading equals the carriers reading plus the reading
       of the input with the carrier chunks removed. A non-additive fixture then fails loudly instead of inverting the
       judge;
     - or record the limit in DEC-023 next to the (c) converse limit.

**Evidence per item**

1. **Item 1 (M), exception (a) line-level: fixed, matches the amendment.**
   - Code against the amendment:
     - `excusable` is the multiset of judged `## carriers` lines. A missing line is excused only while its
       `(group, tag, value)` count is above 0, and only if the fixture `scan`s as holding a carrier (`carrier_read` is
       empty otherwise).
     - The old `derived` `Composite:GPS*` clause is gone. A `Composite:GPS*` line is excused only through
       `excusable`, so only when the carriers section prints it.
     - "Carrier tag left in" with a plain block of the same family fails when `n_out > n_in - n_car`.
     - A carrier fixture judged with no section fails: on `a1`, `no carriers section` gives
       `the fixture holds a carrier and the record has no carriers section`.
   - Carrier typing is independent of `src/`:
     - `PngKind`/`png_kind`, `PNG_SKELETON` (14), `WEBP_SKELETON` (6), `png_chunks`/`webp_chunks` and `webp_rows` live
       in `tests/support/i3_records.rs`.
     - `webp_carriers.rs` and `webp_shapes.rs` import nothing from `exif_core`. `i3_records` imports only `Category`
       and `StripPolicy`/`StripSpec`, for `policies()`.
   - `carriers_only` keeps exactly the skeleton and the carriers. My own Python chunk walk of the dumped files:
     `png-exif-and-raw-exif` `[IHDR, eXIf, tEXt(Raw profile type exif), IDAT, IEND]` becomes
     `[IHDR, tEXt(Raw profile type exif), IDAT, IEND]`, and `png-xmp-and-raw-xmp` likewise drops only the plain
     `iTXt(XML:com.adobe.xmp)`.
   - My attack, rebuilt fresh: Python builder and Rust harness in the session scratchpad, calling the committed
     `scan`, `carriers_only` and `judge` on my own inputs, stripped by `exif_core::strip`, read by ExifTool 13.55.
     - `a1`: plain `eXIf` (`VerifyCam`, `VerifyModel`, GPS N 10) plus a raw-profile Exif zTXt (`CarrierCam`). The
       genuine strip under `gps` and `gps_and_device` gives 0 failures. With `IFD0` dropped it is red:
       `kept tag missing IFD0:Make=VerifyCam`, `IFD0:Model=VerifyModel`.
     - `a2`: plain XMP `iTXt` plus a raw-profile XMP `tEXt`. With `XMP*` dropped it is red:
       `kept tag missing XMP-xmp:CreatorTool=PlainTool`, `XMP-xmp:Label=PlainLabel`.
   - Further attacks:
     - identical lines, where the multiplicity decides (`a3` Exif, `a3c` XMP, `a4c` WebP `XMP ` + `_PMX`): genuine 0
       failures, plain dropped red;
     - WebP `EXIF` + `IDIT` (`a4`) and WebP `XMP ` + `_PMX` (`a4b`): genuine 0 failures, plain dropped red;
     - a malformed carrier (`a5`, raw-profile length field corrupted): ExifTool still prints its line in both
       readings, and the judge is right;
     - an `EXIF` nested in `ANMF` beside an `IDIT` (`a6`): ExifTool reads neither nested block, so there is no
       difference;
     - a carrier before the plain block: punch item 2.
   - Records are honest:
     - `git diff fef653a 34008cc -- tests/oracle-fixtures` gives 36 files, +634 −0, 36 hunks: 35 files gain one
       `## carriers` hunk, and `png-xmp-and-raw-xmp.txt` is new (192 lines);
     - 36 records hold `## carriers`, always as the second section;
     - re-record into a `git archive 34008cc` scratch repo (`cargo run --release --example oracle_record`, own
       `CARGO_TARGET_DIR`) printed `wrote 239 PNG and WebP strip records`, rc 0, and `git status --porcelain` was
       empty, so the records are byte-equal.
   - Red first: `git archive c105798` with its own `CARGO_TARGET_DIR`. Both
     `comparator_is_red_when_a_plain_{exif_block,xmp_packet}_is_dropped_beside_a_carrier` FAILED at the red
     assertions (lines 917 and 955) with `[]`, after their controls passed.
   - Bounds: `assert_judge_holds` has lower bounds only (sections ≥2,500, removable sections ≥590, removable tags
     ≥2,400, carrier ≥670, whole block ≥350, residual ≥24), no upper bounds.
     - They guard against vacuity, and they are honest against the stated 677/354/24.
     - They cannot see a loosening, since group-level would raise the counts. That is covered by the two red-proofs
       (M5 red), not by the bounds.
     - Upper bounds are not needed given those red-proofs. Punch item 1 covers the two guards nothing pins.
2. **Item 2 (L), ICC in the AC11 grid: fixed.**
   - `grid_png` gains `iCCP` and `grid_webp` gains `ICCP` with `FLAG_ICC`. `Icc` is in `want_present`.
   - The grid has `Icc` in `removed` only for `Everything`, never in `kept_unread`. The explicit assertion is `kept`
     for the four other presets and `removed` for `everything`.
   - That matches DEC-009: the table row `Icc | | | | | removed`, rule 3 (`All` keeps `Icc`), and clarification 1
     (`may_hold(Icc)` is empty).
3. **Item 3 (L), README: fixed.** The sentence now lists `TimeStamp` among the timestamps and `Make` and `Model` as
   camera model. That matches DEC-020 amendment (2026-11-24) item 3 and `TEXT_KEYWORDS` in `src/audit/category.rs`.
4. **No regressions.**
   - `git diff --stat fef653a 34008cc -- src fuzz Cargo.toml Cargo.lock` is empty.
   - The differential counts come from my own Python over the committed records: the 174 = all records minus
     `png-carrier-*`, `png-kw-*`, `webp-row-*`, `png-cabx-jumbf` and the three added shapes. Pairs are
     `everything`/`exiftool:all` and `gps`/`exiftool:gps:all`, compared as multisets without File/System/ExifTool.
   - At `34008cc` and at `fef653a` alike: identical 157, key level 86, value level 34, outcome differences 41, both
     refuse 30.
   - The per-class split (D1 72, D2 2, D3 7, D4 4, D5 1; D4 3, D6 31) is held by the pinned `DIFFERENCES`. The only
     change to them is one D3 entry for `png-xmp-and-raw-xmp`, which is not one of the 174. The differential tests
     pass at the gate.
   - Gates on rustc 1.90.0, clippy 0.1.90 (1159e78c47), in the worktree at `34008cc`:
     - `just lint`, `just lint-no-allow`, `just forbid-unsafe` and `just validate`: rc 0 each.
     - `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test`: rc 0. 41 `test result: ok` lines, 1,535 passed, 0 failed, 20
       ignored, no SKIP.
     - `just mutation-oracle`: rc 0, 67 + 42 + 43 passed, 1 ignored each.
     - `writer_mutant` W1 to W4: each `control digest equal true; strip Ok true; replay red true; live red true`. The
       live reds:
       - W1 `png-carrier-tXMP under gps: carrier tag left in XMP-dc:Creator=Zed`;
       - W2 `webp-row-05-seal under gps: carrier tag left in XML-dc:Creator`;
       - W3 `png-kw-04-copyright under all_pii: kept tag missing PNG:Copyright=SweepCopyright`;
       - W4 `webp-row-22-c2pa under all: kept tag missing JUMBF:JUMDLabel=c2pa`.
     - No fuzz run, so `fuzz/Cargo.lock` was untouched. `git status` afterwards showed only this handoff. No
       background process is left.

Scratch apparatus (outside the repo, session scratchpad `r2/`): `scripts/build.py` (my inputs `a1`–`a6`, `b1`–`b3`),
`attack/tests/zz_verify_attack.rs` (a harness over the committed judge, plus the M1/M3/M4/M5 mutants applied to a
scratch copy only), `scripts/diffcount.py`, and the `tip/` and `red/` archives.

### Fix round 2 (2026-11-26)

Implementer (claude-sonnet-5-5), on `feat/spec-039-png-webp-write-parity-build` from `a452bc4`. Scope: the two round-2
punch items. No `src/` change, no new dependency, verdict fields untouched.

**Commits** (work order: red tests first, then the implementation)
- `071a20c` test(spec-039): HANDOFF-091 round 2 red tests: multiplicity, carrier left in, additivity
- `74ebc02` test(spec-039): HANDOFF-091 round 2: no-carriers record section and additivity check
- `7a04688` test(spec-039): HANDOFF-091 round 2: clippy from_ref in the multiplicity test

**Item 1 (L), the two guards pinned: done.** Both tests are in `tests/strip_png_webp_oracle.rs`, with controls.
- `comparator_is_red_when_the_excuse_outruns_the_carriers_multiplicity`: the input prints `IFD0:Make=SameCam` twice,
  `## carriers` once, on the bytes of `png-exif-and-raw-exif` (plain `eXIf` first, then the carrier). The readings are
  given in memory, not read from ExifTool. Control: one copy dropped (the carrier's) is green. Both dropped is red
  with `same-line under gps: kept tag missing IFD0:Make=SameCam`.
- `comparator_is_red_when_a_carrier_tag_is_put_back_beside_a_plain_block`: `png-exif-and-raw-exif` under `gps`.
  Control: the unaltered output is green. With `IFD0:Make=RawCam` pushed into the output it is red with
  `png-exif-and-raw-exif under gps: carrier tag left in IFD0:Make=RawCam`.
- Mutants, applied to the working copy of `tests/support/i3_judge.rs` and reverted with the backup copy each time
  (never committed; `git status` clean afterwards). Both ran against `74ebc02`, with ExifTool required.
  - M1 (the `*n -= 1` decrement replaced by `let _ = n;`): `38 passed; 1 failed`. Failed:
    `comparator_is_red_when_the_excuse_outruns_the_carriers_multiplicity`, at its red assertion with `[]`. The other
    38 tests passed.
  - M3 (the by-line block's `if (carriers.exif && carriers.exif_plain) || (carriers.xml && carriers.xml_plain)`
    replaced by `if false`): `38 passed; 1 failed`. Failed:
    `comparator_is_red_when_a_carrier_tag_is_put_back_beside_a_plain_block`, at its red assertion with `[]`. The
    other 38 tests passed.
  - The red assertion prints the failure list, which is empty under a mutant (that is the red).

**Item 2 (L), the additivity check: done.**
- `no_carriers(ext, bytes)` in `tests/support/i3_records.rs`. It shares one splitter with `carriers_only`, so the
  carrier typing is the same. A PNG keeps every chunk that is not a carrier. A WebP keeps every chunk that is not a
  carrier row, and the `RIFF` size is recomputed. The record gains `## no-carriers` right after `## carriers`, for
  the 36 carrier fixtures.
- `additivity_failures(input, carriers, no_carriers)` in `tests/support/i3_judge.rs`. It compares multisets of
  `(group, tag, value)` on the Exif groups, the XMP/XML groups and `Composite:GPS*`, leaving out `Warning` and
  `Error`. It returns one message per line whose counts differ.
- `the_readings_add_up_on_every_carrier_fixture` runs it over the committed records and, with ExifTool, over a fresh
  live recording. It names the failing fixture. Result: 36 carrier fixtures checked, 0 failing, in replay and live.
  No question was added.
- Red-proofs:
  - `the_additivity_check_is_red_on_readings_that_do_not_add_up`: in memory, with a control that adds up.
  - `the_additivity_check_is_red_on_a_carrier_before_a_plain_exif_block` (live; skipped without ExifTool): the `b1`
    PNG is built in the test, a raw-profile Exif `CarrierCam` before a plain `eXIf` of `VerifyCam`/`VerifyModel`,
    and read with ExifTool 13.55. Red line: `IFD0:Make=VerifyCam: the input prints it 0 time(s), the carriers and
    no-carriers readings together 1`. Controls: `png-exif-and-raw-exif` and `png-xmp-and-raw-xmp` pass.
- `expected_sections` now requires `no-carriers` after `carriers`.

**Records.** `just oracle-record` run twice. After the first run, `git diff -- tests/oracle-fixtures` was 36 files,
319 lines added, 0 deleted, and in each file one hunk adding one `## no-carriers` section. A script
(`git diff -U0` per file) checked that there are no deleted lines, that each file adds exactly one `## ` header and
that it is `## no-carriers`. The second run left the diff byte-identical (same md5 of the diff). The other 203 records
are unchanged.

**Numbers (unchanged).** Judge over the 239 fixtures: 2,583 sections judged, 598 with a removable tag (2,408 tags);
exception (a) used 677 times, (b) 354, (c) 24, in replay and live, as before. The differential tests over the 174
oracle fixtures pass with the pinned counts untouched (157 identical, 41 outcome differences, 30 both refuse). I did
not recompute those counts with a separate script this round; the differential code and the sections it reads are
unchanged.

**Gates, Rust 1.90.0, ExifTool 13.55, `DIAMOND_EXIF_REQUIRE_ORACLE=1`:**
- `just lint`: the first run failed on `clippy::cloned_ref_to_slice_refs` in my new test; fixed in `7a04688`; rc 0
  after. `just lint-no-allow` rc 0. `just forbid-unsafe` rc 0. `just validate` rc 0.
- `just test` rc 0: 41 `test result: ok` lines, 1,540 passed, 0 failed, 20 ignored (1,535 before, plus 5 new tests).
  This run was made before the one-line clippy change in `7a04688`; after it I re-ran
  `cargo test --test strip_png_webp_oracle` (39 passed, 1 ignored).
- `just mutation-oracle` rc 0: 67 + 42 + 43 passed, 1 ignored each. Also run before `7a04688`, which touches only
  the oracle test file. W1 to W4 re-run afterwards with `--nocapture`: each `control digest equal true; strip Ok
  true; replay red true; live red true`.
- No background process left. `fuzz/` untouched.

**Deviations.**
- Item 1 test 1 uses real fixture bytes with in-memory readings, not an ExifTool reading, so it also runs in replay.
- `just test` and `just mutation-oracle` were not re-run in full after the clippy fix (see above). The affected
  test file was re-run alone.
- The DEC-023 amendment names "the verifier's `b1` shape"; I rebuilt it from the description, not from the
  verifier's scratch file.

### Cost self-report

- **Tokens (total):** 258,606, filled by the orchestrator from the harness subagent result (96 tool calls).
- **Estimated USD:** null, because the input/output split is not reported.
- **Duration (minutes):** 26 (1,553 s of subagent wall clock, from the harness).
- **Source of the number:** harness report (agent-result).

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** none by the verifier. In the build: AC11's grid omits ICC (punch item 2). The accepted
  deviations (`entry_edit_groups`, re-aimed `fuzz_layout` plants, `writer_mutants.rs`, 29,130 sweep files) check out.
- **Follow-up work identified:**
  - Punch item 1: make exception (a) carrier-exact, or record its limit in DEC-023.
  - The JPEG/TIFF differential question from the spec (unchanged).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Whether "flip one byte of the writer's output" meant the record or the writer. I mutated the writer in a scratch
   tree, which tests the digest binding end to end.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — DEC-023 rule 3 (a) does not say how the judge decides which tags "a carrier chunk prints". The implementation
   chose group-level, and that choice is where the hole is. The rule should name the granularity.

3. **If you did this task again, what would you do differently?**
   — Build the stub-`Line` judge runner first. Feeding the committed judge doctored readings was the fastest way to
   test each exception, faster than reasoning about the code.
