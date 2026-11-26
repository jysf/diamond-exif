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
  notes: "Verdict applies to f5e24c1 (build tip plus this handoff). Cost filled by the orchestrator from the harness subagent result: 258,606 tokens, 96 tool calls, 1,553 s. Punch list of 3 items: 1 (M) judge exception (a) is group-level and excuses a dropped plain Exif/XMP block when a carrier coexists; 2 (L) the AC11 grid has no ICC finding; 3 (L) the README PNG keyword sentence omits Make, Model and TimeStamp."                      # if unusual; double-quote it — an unquoted " #" starts a YAML comment
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
