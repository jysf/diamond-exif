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
  notes: "Round 5 (2026-11-27, branch claude/spec-039-verify-r5): PUNCH LIST against d61cbf2, 2 items (L), both documentation only: the claim that XML/SEAL lines are never excused is false for a judged one (SEAL:Copyright is Rights through category_of's tag match and is excused live, uses.carrier 1); the test mirror category_of_printed_name in tests/audit_png_webp_oracle.rs lacks the three AC1 rows. Round-4 items closed (probe red in both orders under gps and all, control green), mutants A and C bite exactly their tests, B is equivalent (shown in the code and by identical verdicts on a live probe), the CI fix is right (seed sha1 matches the CI artifact name, red without the rows), gates green (1,544 passed). Round 4 (2026-11-26, branch claude/spec-039-verify-r4): PUNCH LIST against 9c368ba, 2 items (L): a non-judged SEAL/XML carrier line left in beside a plain XMP packet is caught by no check (probe red under neither gps nor all; latent, library output clean, not in corpus; not caused by the fix); the red-proof comment misdescribes its readings. Round-3 fix verified (judged is the one predicate, red-proof red at a96af47, M1/M3/group-level red, gates green, 1,541 passed). Round 3 (2026-11-26, branch claude/spec-039-verify-r3): PUNCH LIST against 2f790d4, 1 item (L): the additivity check compares the Exif/XMP/XML/Composite:GPS lines only, while exception (a) can also excuse judged lines such as ID3v2_3:Artist and Olympus:Model (3 such carrier lines in the corpus); latent, widening keeps 36/36 adding up. Round-2 items 1-2 fixed (M1/M3 red, b1 red, records byte-equal, gates green). Round 2 (2026-11-26, branch claude/spec-039-verify-r2): PUNCH LIST against 34008cc, 2 items (L): the multiplicity bound and the plain-block carrier-left-in check have no red-proof (mutants M1, M3 survive); the line-level model inverts on a non-additive (carrier-before-plain) shape, not in the corpus. Round-1 items 1-3 fixed. Round 1: Verdict applies to f5e24c1 (build tip plus this handoff). Cost filled by the orchestrator from the harness subagent result: 258,606 tokens, 96 tool calls, 1,553 s. Punch list of 3 items: 1 (M) judge exception (a) is group-level and excuses a dropped plain Exif/XMP block when a carrier coexists; 2 (L) the AC11 grid has no ICC finding; 3 (L) the README PNG keyword sentence omits Make, Model and TimeStamp."                      # if unusual; double-quote it — an unquoted " #" starts a YAML comment
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

**Orchestrator notes on fix round 2** (claude-opus-5-5, 2026-11-26)
- Every gate was re-run at `90d627a`, because the implementer ran `just test` and `just mutation-oracle` before its
  clippy fix. Results on 1.90.0 with ExifTool required: `just lint`, `just lint-no-allow`, `just forbid-unsafe` and
  `just validate` all rc 0. `just test` rc 0, 41 result lines, 1,540 passed, 0 failed. `just mutation-oracle` 67 + 42
  + 43 passed.
- No change in `src/`, `fuzz/` or the Cargo files. The record diff since `a452bc4` is 36 files, +319 −0.
- Fix round 2 cost, from the harness subagent result: 144,285 tokens, 46 tool calls, 1,209 s (about 20 min).

### Verify round 3 (2026-11-26)

Independent verifier (claude-opus-5-5), cold, on `claude/spec-039-verify-r3` from
`feat/spec-039-png-webp-write-parity-build` at `2f790d4`. Scope: the two round-2 items and regressions. Change
reviewed: `git diff c0e3482 2f790d4` (amendment `a452bc4`; red `071a20c`; fix `74ebc02`, `7a04688`; bookkeeping
`90d627a`, `2f790d4`). Read-only on library, tests and records.

**Verdict: ⚠ PUNCH LIST against `2f790d4`, one item (L).** Both round-2 items are fixed. The one item is a gap
between the additivity check's compared set and what exception (a) can actually excuse.

1. **(L) The additivity check compares fewer groups than exception (a) excuses.**
   - The amendment defines the compared lines as "the groups exception (a) can excuse", then lists the Exif and
     XMP/XML families and `Composite:GPS*`. `additive_line` follows that list.
   - The judge excuses more. It excuses any `judged` line the `## carriers` reading prints, and `judged` is
     `category_of(..).is_some() || exif_group`. That also covers `ID3v2_3:Artist` (Authorship), `Olympus:Model`
     (CameraModel), PNG text, `ICC*` and `JUMBF`/`JSON` lines.
   - In the committed carriers sections, 3 such lines are excusable but not compared: `ID3v2_3:Artist` ×2 and
     `Olympus:Model` ×1. My Python reimplementation of `category_of`/`judged` over the 36 records found them.
   - No interfering shape in those groups was found: no non-carrier chunk prints into them. So this is latent, not
     a live mis-excuse.
   - Widening the check costs nothing. On the 36 records, `judged(l) || xml_group(l)` (Warning and Error still
     excluded) still adds up: 0 failing.
   - Suggested fix (architect's choice):
     - (a) Make `additive_line` `judged(l) || xml_group(&l.group)`, so the check covers every line the excuse can
       touch.
     - (b) Or amend the parenthetical to name the omitted groups as out of scope, with the reason. Then correct the
       doc comment on `additive_line`, which currently states the narrower set as fact.

**Evidence.**

1. **Item 1 (round 2), the two guards pinned: fixed.**
   - M1 and M3 were run on a `git archive 2f790d4` scratch copy, with its own `CARGO_TARGET_DIR` and
     `DIAMOND_EXIF_REQUIRE_ORACLE=1`, using `cargo test --test strip_png_webp_oracle`.
   - M1 (`*n -= 1;` → `let _ = n;`): `38 passed; 1 failed`.
     - Only `comparator_is_red_when_the_excuse_outruns_the_carriers_multiplicity` failed.
     - It panicked at line 1098, the red assertion (both copies dropped), with `[]`.
     - Its control at line 1094 (one copy kept) passed first.
   - M3 (the by-line `if (carriers.exif && carriers.exif_plain) || (carriers.xml && carriers.xml_plain)` →
     `if false`): `38 passed; 1 failed`.
     - Only `comparator_is_red_when_a_carrier_tag_is_put_back_beside_a_plain_block` failed.
     - It panicked at line 1118, the red assertion, with `[]`. Its control `judged_with(EXIF_PAIR, "gps", no-op)` is
       empty and passed first.
   - Both mutants were reverted with `git checkout`, and the scratch tree was clean afterwards.
   - Are the multiplicity test's in-memory readings realistic? I built the shape and read it with ExifTool 13.55:
     a plain `eXIf` `Make=SameCam`, then a raw-profile Exif carrier `Make=SameCam`.
     - The input reading prints `IFD0:Make=SameCam` twice, `## carriers` prints it once, and `## no-carriers` once.
     - The library's `gps` output prints it once, which is the control.
     - So the test's readings are a shape ExifTool prints, and the shape adds up.
2. **Item 2 (round 2), the additivity check: fixed, with punch item 1 above.**
   - Against the amendment:
     - `no_carriers` shares `split_by_carrier` with `carriers_only`, so the carrier typing is the same (`png_kind`
       CarrierExif/CarrierXml; WebP `webp_rows` non-empty).
     - A PNG keeps every non-carrier chunk in order.
     - A WebP keeps every non-carrier chunk, and `RIFF` is rebuilt as `RIFF` + `len(body)+4` + the original form
       type + body.
     - The comparison is on multisets of `(group, tag, value)`, `Warning`/`Error` excluded, over the stated groups.
   - Excluding Warning and Error is sound:
     - The judge never excuses them (`judged` drops them), so they cannot feed a mis-excuse.
     - The `b1` signal was never a read warning. My `b1` input reading has no `Warning` line and empty stderr. The
       `IFD0 pointer references previous IFD0 directory` text is ExifTool's write-side refusal (`# refused:` in 3
       records). The check catches `b1` through the hidden lines.
     - As a counterfactual, comparing Warning/Error too would also give 0 failing on the corpus. So the exclusion
       hides nothing there.
   - My own inputs: `scratchpad/r3/inputs/`, 15 files built by `scripts/zz_verify_r3.rs` in the scratch copy, read
     live:
     - `b1` (raw Exif carrier, then a plain `eXIf`): RED.
       - `IFD0:Make=VerifyCam` and `IFD0:Model=VerifyModel`: input 0, sum 1.
       - On the library's `gps` output the judge gives `kept value changed`/`phantom tag`, the round-2 inversion,
         now flagged by the check.
     - `b1r` (plain, then carrier): GREEN.
     - XMP raw-profile carrier, then a plain `iTXt` packet: GREEN. ExifTool prints both packets, so there is no
       interference to catch. The reverse order is GREEN too.
     - WebP plain `EXIF` + `_PMX` row, both orders: GREEN, and the judge is clean on the library's `gps` output.
     - WebP plain `XMP ` + `_PMX`, both orders: GREEN.
     - Two carriers then a plain `eXIf`: RED (the plain block is hidden).
     - Plain then two carriers: GREEN.
     - Two carriers alone (raw Exif + raw XMP): GREEN.
     - WebP `_PMX` + `aXML` + `XMP ` + `EXIF`: GREEN.
     - Identical-value carrier then plain: RED (input 1, sum 2).
   - Interfering shapes that add up but are still misjudged:
     - Swap (not a punch item). A plain `eXIf` and a raw carrier with identical lines, and a writer that drops the
       plain block and keeps the carrier.
       - The readings add up, and the judge passes it with `[]`. Its reading is identical to the correct strip's,
         so no reading-based judge can see it.
       - The byte-level suites cover it: `strip_png.rs` `carriers_follow_the_clause_4_rule_for_every_policy` and
         `the_output_audit_has_no_carrier_the_policy_removes`.
       - The library's own output holds no carrier (`scan(..).any()` false).
     - WebP plain `XMP ` packet equal to a `_PMX` row's packet (observation, not a punch item; it predates this
       round).
       - The readings add up, but the judge fails the library's correct `gps` output with `carrier tag left in
         XMP-dc:Creator`.
       - Cause: the `row_tags` check flags any output line with the row's `group:tag`, ignoring a plain packet.
       - This is a loud false red, not a silent pass, and no corpus fixture has the shape.
   - `the_readings_add_up_on_every_carrier_fixture`: ok, in replay and live, with ExifTool required. It asserts
     `checked == 36` in both. My Python reimplementation over the committed records also gives 36 fixtures,
     0 failing.
3. **Records.**
   - `git diff c0e3482 2f790d4 -- tests/oracle-fixtures`: 36 files, +319 −0.
     - `-U0`: 0 deleted lines and 36 hunks.
     - Every added header is `+## no-carriers`, 36 in all.
     - All 36 carrier records now hold `## carriers` then `## no-carriers` as sections 2 and 3.
   - Re-record: `git archive 2f790d4` into a scratch git repo, then `cargo run --example oracle_record` with its own
     `CARGO_TARGET_DIR`.
     - It printed `wrote 239 PNG and WebP strip records`.
     - `git status --porcelain` was empty, so the output is byte-equal.
4. **Red first.** At `git archive 071a20c`, with its own target dir and ExifTool required: `35 passed; 4 failed`.
   - The failures:
     - `the_additivity_check_is_red_on_readings_that_do_not_add_up` (line 1307, red assertion; the stub returns
       empty);
     - `the_readings_add_up_on_every_carrier_fixture` (line 1271, no `## no-carriers` section);
     - `the_additivity_check_is_red_on_a_carrier_before_a_plain_exif_block` (line 1345, the `no_carriers` stub gives
       `None`);
     - `every_i3_fixture_has_a_record_with_the_expected_sections` (line 205).
   - Three of the four are new tests, and the fourth is the amended section test.
   - Both item-1 tests passed at `071a20c`, as expected: their guards predate them.
   - Their red comes from M1 and M3, which I reproduced independently. That is adequate: each mutant turns exactly
     its own test red, at the red assertion, with its control green.
5. **No regressions.**
   - `git diff --stat c0e3482 2f790d4 -- src fuzz Cargo.toml Cargo.lock` is empty.
   - Differential, from my own Python (`scripts/r3count.py`) over the committed records at `2f790d4`:
     - Core set: the first 174 of `i3_shapes::fixtures()`, an order dumped by the scratch harness.
     - Pairs: `everything`/`exiftool:all` and `gps`/`exiftool:gps:all`, as multisets without File/System/ExifTool.
     - Result: identical 157, key level 86, value level 34, outcome 41, both refuse 30. Total 348.
   - The test prints the per-class split, in replay and live: D1 72, D2 2, D3 7, D4 4, D5 1; value D4 3, D6 31.
   - Judge over 239 fixtures, replay and live: 2,583 sections judged, 598 with a removable tag (2,408 tags). (a)
     677, (b) 354, (c) 24.
   - Gates on rustc 1.90.0 and clippy 0.1.90 (1159e78c47), in the worktree at `2f790d4`:
     - `just lint`, `just lint-no-allow`, `just forbid-unsafe` and `just validate`: rc 0 each.
     - `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test`: 41 `test result: ok` lines, 1,540 passed, 0 failed, 20 ignored.
       No `FAILED`, `error` or `SKIP` lines.
     - `just mutation-oracle`: rc 0, 67 + 42 + 43 passed, 1 ignored each.
     - W1 to W4 with `--nocapture`: each `control digest equal true; strip Ok true; replay red true; live red true`.
       The live reds are the same as in round 2.
     - `just decisions-audit --changed`: advisory list only, rc 0.
   - Afterwards, `git status` showed only this handoff. No background process is left.
   - No fuzz run.

Scratch apparatus (outside the repo, session scratchpad `r3/`):
- `scripts/zz_verify_r3.rs`, a probe harness over the committed support modules;
- `scripts/r3count.py`;
- `inputs/` (the 15 probe files);
- the `tip/` and `red/` archives;
- gate logs `gate-*.log`.

No cost session entry was appended. Per the brief, `tokens_total`, `basis` and `duration_minutes` are left for the
orchestrator.

### Fix round 3 (2026-11-26)

Implementer (claude-sonnet-5-5) on `feat/spec-039-png-webp-write-parity-build`, from `89ac392`. Scope: punch item 1 (L).

- **Two stops, two rulings.**
  - Stop 1: the excuse path's eligibility was `judged`, not `judged || xml_group` as the first amendment wording said. `XML*` and `SEAL*` lines are not `judged` and are never excused. The architect corrected DEC-023 (`f9aed69`).
  - Stop 2: the by-line left-in check does arithmetic with the carriers reading only through `carrier_judged`. The scan for a carrier XML block with no plain block reads the output alone. So the set the judge uses with the carriers reading is `judged` only. The architect ruled option A and rewrote the amendment (`9e37e8f`).
- **Red first** (`a96af47`): `the_additivity_check_compares_a_judged_line_outside_the_exif_and_xml_groups`, in memory. The input prints `ID3v2_3:Artist=Someone` once, the carriers reading once (no Artist), the no-carriers reading once more.
  - *Correction (2026-11-26, fix round 4):* the sentence above is wrong about the readings. The test builds `input = [IFD0:Make=SameCam, ID3v2_3:Artist=Someone]` and `carriers = [IFD0:Make=SameCam]`, so the carriers reading prints no Artist. Control: no-carriers `[Artist]`, which adds up. Red: no-carriers `[Artist, Artist]`, so input 1 != 0 + 2. The test comment is fixed in `da8f4f7`.
  - Control: readings that add up, passes.
  - Red against the old `additive_line`: the second assertion failed with `[]` (no failure reported).
- **Fix** (`8c9c89a`): `additivity_failures` filters by `judged`. `additive_line` is removed. Doc comments on `judged` and `additivity_failures` state the single shared line set. Nothing under `src/` changed.
- **Red line after the fix:** `additivity red (ID3v2_3): ID3v2_3:Artist=Someone: the input prints it 1 time(s), the carriers and no-carriers readings together 2`.
- **Nothing else moved:**
  - `the_readings_add_up_on_every_carrier_fixture`: passes on all 36, replay and live.
  - `git diff -- tests/oracle-fixtures` is empty; `just oracle-record` gave no diff.
  - Judge counts, replay and live: (a) 677, (b) 354, (c) 24; 2583 sections judged.
- **Gates** at the final commit (`7c454ed`): `just lint`, `just lint-no-allow`, `just forbid-unsafe` and `just validate` exit 0; `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test` exit 0; `just mutation-oracle` exit 0. The orchestrator re-ran them; the numbers are in its notes below.
- **Deviations:** none from the rulings.

**Orchestrator notes on fix round 3** (claude-opus-5-5, 2026-11-26)
- **Rulings.** The implementer stopped twice before changing code, and both stops were correct. Each found that the
  orchestrator's DEC-023 wording did not match the judge. The wrong wordings were committed in `89ac392` and `f9aed69`. The final ruling,
  option A, rests on the orchestrator reading the code (`9e37e8f`): `judged` is the one predicate.
- **Re-run at `7c454ed` on 1.90.0 with ExifTool required.** `just lint`, `just lint-no-allow`, `just forbid-unsafe`
  and `just validate` returned rc 0. `just test` returned rc 0, with 41 result lines, 1,541 passed and 0 failed.
  `just mutation-oracle` passed 67 + 42 + 43. `git diff 9e37e8f HEAD -- tests/oracle-fixtures` is empty. The change
  touches only `tests/support/i3_judge.rs` and `tests/strip_png_webp_oracle.rs`, plus this file.
- **Process.** The implementer reports that a `pgrep -f "just test"` monitor matched another session's process. It
  then killed "one stray shell" by PID (45357). That PID no longer exists, so it cannot be checked which process it
  was. This was reported to the user.
- **Cost, from the harness.** One agent, resumed twice. Its three notifications read 82,538, 88,160 and 114,377
  tokens. They are read as cumulative, so 114,377 tokens, 40 tool calls and 846 s (about 14 min) are recorded.
  Basis: agent-result.

### Verify round 4 (2026-11-26)

Verifier (claude-opus-5-5) on `claude/spec-039-verify-r4`, cut from `9c368ba`. Scope: the round-3 fix only
(`git diff 0acdb72 9c368ba`).

**Verdict: ⚠ PUNCH LIST against `9c368ba`, 2 items, both (L).** The fix itself is correct. Item 1 is a judge
coverage gap that predates this fix. Item 2 is a wrong test comment.

**Punch list.**

1. **(L) A non-`judged` XML/SEAL carrier line left in the output beside a plain XMP packet is caught by nothing.**
   Item 1 of the round-3 amendment says `XML*` and `SEAL*` lines "are caught only by the output-only scan". That scan
   runs only when `carriers.xml && !carriers.xml_plain`. When a plain XMP packet is also present, the by-line check
   runs instead, but it only covers `carrier_judged` lines. `SEAL:*` and `XML:*` are not `judged`, so they are not
   in `in_m`/`out_m` either. The module header's "the carrier's tags must in fact be gone" overclaims for this shape
   in the same way.
   - Evidence: I ran an in-memory probe on a scratch copy of `9c368ba`, live with ExifTool 13.55. It used the real
     readings of a PNG with a plain `iTXt` `XML:com.adobe.xmp` packet plus a `seAl` chunk (`<a><b>Zed</b></a>`), in
     both chunk orders. I then added `SEAL:AB=Zed` back into the library's real output.
     - `judge` returned `[]` under `gps` and under `all`.
     - On the same probe with no plain packet, the same doctored output gives
       `carrier tag left in SEAL:AB=Zed` under both policies.
     - The library's real output prints no SEAL line, so there is no leak today. This is a gap in what the oracle
       can detect, and no corpus fixture has this shape: the five records whose `## carriers` print XML/SEAL lines
       hold no plain packet.
   - Dropping `xml_group` from additivity did not cause this gap and does not widen it. Additivity compares the three
     input readings and never looks at the output. The probe's readings add up (`additivity []`) in all three shapes.
     So the old XML/SEAL terms in additivity protected no judge arithmetic. Dropping them loses nothing.
   - Suggested fix: when a plain block of the family exists, run the by-line left-in check over every `xml_group` line
     of the carriers reading, not only `judged` ones. Widen additivity to the same set so that the one-predicate rule
     holds (a `carrier_compared(l) = judged(l) || xml_group(&l.group)` used by both). Red-proof it with the probe
     shape above. Alternatively, record the limit in DEC-023 and correct the "caught only by the output-only scan"
     sentence and the module header.
2. **(L) The red-proof's comment says the wrong thing about the readings.** The comment says "the input prints it
   once, the carriers reading once, the no-carriers reading once more". The fix-round-3 entry above repeats it. What
   the test builds is:
   - `input = [IFD0:Make=SameCam, ID3v2_3:Artist=Someone]`;
   - `car = [IFD0:Make=SameCam]`, so the carriers reading prints no Artist;
   - control: `plain = [Artist]`, which adds up;
   - red: `plain = [Artist, Artist]`, so the no-carriers reading prints Artist twice and input 1 ≠ 0 + 2.

   Suggested fix: reword the comment to say that.

**Evidence.**

1. **The amendment's claim about the code is true.** In `judge`, the carriers reading (`carrier_read`) is used only
   through `carrier_judged = carrier_read.filter(judged)`. That feeds `excusable`, which is the excuse together with
   its `*n -= 1` bound, and `carrier_ms`, which is the by-line left-in check.
   - The two other left-in checks scan `output` only: the Exif or XML family with no plain block, and the WebP
     `row_tags`.
   - `carrier_lines.is_none()` is checked for presence only.
   - `ID3v2_3:Artist` is `judged`: `category_of` falls through to the tag match, which gives `Authorship`.
   - See item 1 for the one sentence of the amendment that does not hold.
2. **One predicate.** `additivity_failures` filters by `judged`. `rtk proxy grep -rn additive_line tests src` finds
   nothing. The doc comments on `judged` and `additivity_failures` are accurate. `non_additive`'s "the lines exception
   (a) can excuse" also holds, since those are exactly the `judged` lines.
3. **Red-proof.** Run on a `git archive a96af47` copy with its own `CARGO_TARGET_DIR`:
   `the_additivity_check_compares_a_judged_line_outside_the_exif_and_xml_groups` failed at
   `tests/strip_png_webp_oracle.rs:1326`. That is the red assertion, and it reported `[]`. The control passed first.
   At `9c368ba` (own target dir, ExifTool required) it is green and prints
   `ID3v2_3:Artist=Someone: the input prints it 1 time(s), the carriers and no-carriers readings together 2`.
   - The control matters at the tip. It has Artist in input and no-carriers, and Make in input and carriers, so it
     shows that widening to `judged` causes no false red on readings that add up. It is weaker at `a96af47`, where it
     passes trivially because Artist was not compared.
   - Between `a96af47` and the tip, the test changed only in the `println!` line wrapping (rustfmt).
4. **No regressions.**
   - `git diff 9e37e8f 9c368ba -- tests/oracle-fixtures src fuzz Cargo.toml Cargo.lock` is empty.
   - `the_readings_add_up_on_every_carrier_fixture` passes. It asserts `checked == 36` and no failures, replay and
     live.
   - Judge counts, replay and live: 2583 sections judged, 598 with a removable tag (2408 tags); (a) 677, (b) 354,
     (c) 24.
   - Gates at `9c368ba` in this worktree, rustc 1.90.0, clippy 0.1.90, ExifTool required:
     - `just lint`, `just lint-no-allow`, `just forbid-unsafe` and `just validate` each returned rc 0.
     - `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test` returned rc 0, with 41 result lines, 1,541 passed and 0 failed.
     - `just mutation-oracle` returned rc 0, with 67 + 42 + 43 passed.
5. **Earlier items still closed.** Run on a `git archive 9c368ba` copy with its own target dir and
   `cargo test --test strip_png_webp_oracle`. Each mutant was restored and checked with `cmp` afterwards.
   - M1 (`*n -= 1;` → `let _ = n;`): `39 passed; 1 failed; 1 ignored`. Only
     `comparator_is_red_when_the_excuse_outruns_the_carriers_multiplicity` failed, at its red assertion `:1098`.
   - M3 (by-line `if (carriers.exif && carriers.exif_plain) || ...` → `if false`): `39 passed; 1 failed`. Only
     `comparator_is_red_when_a_carrier_tag_is_put_back_beside_a_plain_block` failed, at `:1118`.
   - Group-level rule (`carrier_line = removes_carriers(policy) && ((carriers.exif && in_exif) || (carriers.xml &&
     xml_group(&k.0)))`, with the decrement made `saturating_sub` so that it cannot underflow): `36 passed; 4 failed`.
     - `..._a_plain_exif_block_is_dropped_beside_a_carrier` failed at `:1017`.
     - `..._a_plain_xmp_packet_is_dropped_beside_a_carrier` failed at `:1055`.
     - The multiplicity test failed at `:1098`.
     - `the_judge_holds_both_ways...` failed with 78 failures. These are `kept tag missing Composite:GPS*`, because my
       reconstruction does not excuse `Composite:GPS*` the way the original rule did, so they are an artifact of the
       reconstruction.
     - With the plain `*n -= 1`, the multiplicity test instead panicked on the underflow at `i3_judge.rs:393`.
- **Process.** I started and waited on only my own processes. The gate run was PID 87389, waited on by PID. No
  background processes are left. Scratch copies are under the session scratchpad. Nothing was changed outside this
  file.

### Fix round 4 (2026-11-26)

Implementer (claude-sonnet-5-5) on `feat/spec-039-png-webp-write-parity-build`, from `d13e7c2`. Scope: both round-4 punch items (L, L).

- **Item 1: two named predicates** (DEC-023 amendment "XML and SEAL carrier lines are checked as left in beside a plain block").
  - The excuse predicate `judged` is unchanged, and `excusable` is still built from `judged` lines only. `XML*` and `SEAL*` lines are never excused.
  - New carrier-check predicate `carrier_checked(l) = judged(l) || xml_group(&l.group)`, without Warning and Error lines. It is used in two places: the by-line "carrier tag left in" check beside a plain block (`carrier_ms`, and the input and output counts it compares against, are now built with it) and `additivity_failures`.
  - The module header and the doc comments on `judged`, `carrier_checked` and `additivity_failures` no longer claim XML/SEAL lines are caught only by the output-only scan, or that carrier tags are always checked.
- **Red first** (`da8f4f7`): two tests, a PNG with a plain `iTXt` `XML:com.adobe.xmp` packet plus a `seAl` chunk (`<a><b>Zed</b></a>`), both chunk orders, policies `gps` and `all`.
  - `comparator_is_red_when_a_seal_line_is_put_back_beside_a_plain_xmp_packet`: live, the library's real `strip` output read by ExifTool; skips loudly without ExifTool (`live_gate`). The prior reading also checks that the three readings add up.
  - `..._in_memory`: the same shape with the readings given by hand, so no ExifTool is needed.
  - Against the old judge, the control passed and the doctored output judged clean (`[]`): both tests failed at the red assertion (`gps seal_first=true: []`).
- **Fix** (`506f2fb`): `tests/support/i3_judge.rs` only.
- **Red lines after the fix**, live and replay (in-memory), `seal_first` true and false, `gps` and `all`: `seal-probe under <policy>: carrier tag left in SEAL:AB=Zed`. All eight (four live, four in-memory) print that. The controls (real output, and the in-memory output without the SEAL line) pass.
- **Item 2:** the comment on `the_additivity_check_compares_a_judged_line_outside_the_exif_and_xml_groups` now says what is built (`da8f4f7`, in the same commit as the red tests). The wrong sentence in "Fix round 3" above carries a dated correction note.
- **Nothing else moved:** `the_readings_add_up_on_every_carrier_fixture` passes on all 36, replay and live. Judge counts, replay and live: (a) 677, (b) 354, (c) 24; 2583 sections judged, 598 with a removable tag (2408 tags). `git diff -- tests/oracle-fixtures` is empty. Nothing under `src/` changed; no new dependencies.
- **Gates** at the final commit: `just lint`, `just lint-no-allow`, `just forbid-unsafe` and `just validate` exit 0. `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test` exits 0: 41 result lines, 1,543 passed, 0 failed (1,541 before, plus the two new tests). `just mutation-oracle` exits 0: 67 + 42 + 43 passed.
- **Deviations:** the item 2 comment fix went into the red-test commit rather than its own commit. `just oracle-record` was run twice by mistake (a stray first invocation in the same command line, output discarded); both completed and wrote 239 records, and `git status` and `git diff -- tests/oracle-fixtures` show no change. The gate commit for this entry is the handoff-only commit that follows `506f2fb`.

**Orchestrator self-check of fix round 4** (claude-opus-5-5, 2026-11-26; replaces a round-5 verify at the user's choice)
- Diff `d13e7c2..c5115c7`. It touches only `tests/support/i3_judge.rs`, `tests/strip_png_webp_oracle.rs` and this file, and it matches the amendment: the excuse stays `judged`, and `carrier_checked` drives the by-line left-in check and additivity.
- Mutants, each run in a `git archive` copy with its own `CARGO_TARGET_DIR`, against `strip_png_webp_oracle` with ExifTool required:
  - **A** (the by-line check reverted to `judged`): 40 passed, 2 failed. Both SEAL red-proofs fail (live and in-memory). **Pinned.**
  - **B** (the excuse widened to `carrier_checked`, so XML/SEAL lines become excusable): 42 passed, 0 failed. **Not pinned.**
  - **C** (additivity reverted to `judged`): 42 passed, 0 failed. **Not pinned.**
- Gates at `c5115c7` on 1.90.0: `just lint`, `just lint-no-allow`, `just forbid-unsafe` and `just validate` rc 0. `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test` rc 0, 41 result lines, 1,543 passed, 0 failed. `just mutation-oracle` 67 + 42 + 43 passed. `git diff d13e7c2 HEAD -- tests/oracle-fixtures src` is empty.
- Fix round 4 cost, from the harness subagent result: 137,987 tokens, 57 tool calls, 840 s (about 14 min). Basis: agent-result.

**Fix round 5 (2026-11-26): pin two guards** (claude-sonnet-5-5)
- **Mutant C (additivity filtered by `judged`): pinned.** New in-memory test `the_additivity_check_compares_a_seal_line_which_the_judge_does_not_judge_in_memory`, commit `18079a6` (tests only). It builds input `[IFD0:Make, SEAL:AB=Zed]`, carriers `[SEAL:AB=Zed]`. Control: no-carriers `[IFD0:Make]` adds up, `additivity_failures` is empty. Red: no-carriers `[IFD0:Make, SEAL:AB=Zed]`, so the SEAL line is 1 in the input and 2 in the sum, and `additivity_failures` reports exactly one line. The readings agree on every `judged` line, so only a `carrier_checked` filter sees the difference. Green at the tip (its "red" is the mutant). Under mutant C, in a `git archive HEAD` copy with its own `CARGO_TARGET_DIR`: 42 passed, 1 failed, and the failing line is `panicked at tests/strip_png_webp_oracle.rs:1360:5: []` (the `any(contains "SEAL:AB=Zed")` assert; `additivity_failures` returned `[]`).
- **Mutant B (excuse widened to `carrier_checked`): not pinnable, because it is an equivalent mutant.** It changes only which lines `carrier_judged` holds. The excuse map is read only in the `missing` loop, with keys from `missing`, which is built from `in_m`, which holds `judged` lines only. A `judged` key never matches a line that `carrier_checked` adds beyond `judged` (`XML*`, `SEAL*`; `XMP*` is already `judged` through `Cat::Xmp`), because `judged` depends only on `(group, tag)`, which is part of the key. So the extra entries are never looked up, and neither `failures`, `uses.carrier`, `uses.whole_block`, `uses.residual` nor `removable_before` can change. Confirmed by the mutant run on `18079a6`: 43 passed, 0 failed. Pinning it would need a judge change (a narrower design, not a test), so nothing was added; the "never excused" rule holds by construction, and the doc comment on `carrier_judged` states it.
- **Nothing else moved:** `cargo test --test strip_png_webp_oracle` with ExifTool required: 43 passed, 0 failed, 1 ignored. Judge counts, replay and live: (a) 677, (b) 354, (c) 24; 2583 sections judged, 598 with a removable tag (2408 tags). `the_readings_add_up_on_every_carrier_fixture` passes on all 36. `git status` shows no change to `tests/oracle-fixtures`; `git diff HEAD~1 -- src tests/support tests/oracle-fixtures` is empty at `18079a6`. No new dependencies. No mutant was committed.
- **Gates** at `18079a6` on 1.90.0 (this entry is the handoff-only commit after it): `just lint`, `just lint-no-allow`, `just forbid-unsafe` and `just validate` rc 0. `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test` rc 0, 41 result lines, 1,544 passed, 0 failed (1,543 before, plus the one new test). `just mutation-oracle` rc 0: 67 + 42 + 43 passed.

**Orchestrator self-check of fix round 5 and close of the verify cycle** (claude-opus-5-5, 2026-11-26)
- **Mutant B is equivalent; checked in the code.** `missing` is built from `in_m`, which is the multiset of `judged` input lines (`in_j`). `excusable` is looked up only by `missing` keys. Widening `excusable` to `carrier_checked` adds only `XML*`/`SEAL*` keys, and those keys are never read. "XML/SEAL lines are never excused" therefore holds by construction.
- **Mutant C** (additivity reverted to `judged`), run in a `git archive` copy with its own target dir: 42 passed, 1 failed. The failure is `the_additivity_check_compares_a_seal_line_which_the_judge_does_not_judge_in_memory`. **Pinned.**
- **Gates at `33f912f`, on 1.90.0:** `just lint`, `just lint-no-allow`, `just forbid-unsafe` and `just validate` rc 0. `just test` rc 0, with 41 result lines, 1,544 passed and 0 failed. `just mutation-oracle` passed 67 + 42 + 43.
- **Close.** The user chose to close the verify cycle with orchestrator self-checks rather than a round-5 verify. `handback.verdict` stays `punch-list`, as round 4's independent verdict, and is not upgraded. Every punch item from rounds 1 to 4 has a fix recorded above, and each guard added is shown to bite by a mutant, apart from B, which is equivalent.
- **Fix round 5 cost**, from the harness subagent result: 92,174 tokens, 14 tool calls, 528 s (about 9 min). Basis: agent-result.

### Verify round 5 (2026-11-27)

Verifier (claude-opus-5-5) on `claude/spec-039-verify-r5`, cut from `origin/main` = `d61cbf2` (the merge of PR #67).
Scope: `git diff 9c368ba origin/main`, which is fix rounds 4 and 5, the CI fix `5e8bc80`, and the DEC-023 amendment.

**Verdict: ⚠ PUNCH LIST against `d61cbf2`, 2 items, both (L) and both documentation only.** Both round-4 items are
closed. The two-predicate judge behaves correctly, mutants A and C bite exactly their tests, and B is equivalent. The
CI fix is right and complete in the fuzz models. All gates are green.

**Punch list.**

1. **(L) "XML/SEAL lines are never excused" is false for a `judged` one.** `category_of` falls through to its tag
   match for every group that is not GPS, Composite, `XMP*`, `ICC*`, JUMBF/JSON, PNG or RIFF. So `SEAL:Copyright`
   (a standard ExifTool SEAL tag, from `copyright="..."`) is `Rights`, and `SEAL:Make` / `XML:Make` would be
   `CameraModel`. Those lines are `judged`, and exception (a) excuses them like any other `judged` line.
   - Evidence: a live probe on a scratch copy of `d61cbf2`, with ExifTool 13.55 and the library's real `strip`. The
     PNG has a plain `iTXt` XMP packet plus `seAl` = `<seal seal="1" copyright="Joe"/>`, in both chunk orders. The
     carriers reading prints `SEAL:SEALVersion=1` and `SEAL:Copyright=Joe`, and the readings add up.
     - Under `gps` and `other`, the real output judges clean with `uses.carrier = 1`. The only `judged` missing line is
       `SEAL:Copyright=Joe`, so that one use is a SEAL line excused by exception (a).
     - Under `all` and `rights`, that line is a removed category, so it is skipped, not excused (`uses.carrier = 0`).
   - The behaviour is right. The excused line is a genuine carrier line, and every guard still catches it when it is
     put back: under `gps`/`other` the by-line check gives `carrier tag left in SEAL:Copyright=Joe`, and under
     `all`/`rights` the output scan gives `removed tag left in SEAL:Copyright=Joe`. No corpus record has a `judged`
     XML/SEAL line in its `## carriers` section. The only XML/SEAL lines there are `XML-dc:Creator`,
     `XML-photoshop:City`, `XML:AB` and `SEAL:AB`. So no count moves.
   - Where the false sentence is:
     - DEC-023's round-4 amendment, item 1: "so `XML*`/`SEAL*` lines are still never excused". Its preamble also says
       "`XML*`/`SEAL*` lines are not `judged`". The round-3 amendment has the same sentence.
     - `tests/support/i3_judge.rs`: the comment above `carrier_judged` says "an `XML*`/`SEAL*` line is never excused",
       and the module header says "a `SEAL:*` or `XML:*` line is checked there but never excused".
     - The orchestrator's mutant-B note above says "adds only `XML*`/`SEAL*` keys". It means the non-`judged` ones.
       The equivalence argument still holds (point 3 below).
   - Suggested fix: reword all of these to "an `XML*`/`SEAL*` line that is not `judged` is never excused; one with a
     category through `category_of`'s tag match (`SEAL:Copyright`) is `judged` and is excused like any judged line".
     Optionally, pin it with an in-memory test that has a `SEAL:Copyright` carrier line excused under `gps`. Do not
     narrow `judged`: that would drop `SEAL:Copyright` from the removed-category check under `all`.
2. **(L) One test mirror of DEC-020's keyword table still lacks the three AC1 rows.** In
   `tests/audit_png_webp_oracle.rs`, `category_of_printed_name` maps ExifTool's printed name to a category. It has
   no `Make`/`Model` → `CameraModel` and no `TimeStamp` → `Timestamps`, and its doc comment says "every other name
   free text".
   - It is latent. The keyword fixture (`oracle_png::TEXT_KEYWORDS`, 36 keywords) holds none of the three keywords,
     so `keyword_mismatches` never sees them. The AC1 rows are measured elsewhere: the I3 judge's `category_of` and
     the `i3_shapes` keyword list.
   - Fixed already: the library's unit tests have a separate three-row test, and both fuzz models have the rows
     (`audit_invariants.rs` since AC1, and `rewrite_png_invariants.rs` since `5e8bc80`). A grep of `src`, `tests`
     and `fuzz` for `Creation Time`, `create-date`, `modify-date`, `"Author"`, `"Copyright"` and `"Software" =>`
     finds no other mirror.
   - Suggested fix: add the three rows to `category_of_printed_name` and its comment. Optionally, add `Make`,
     `Model` and `TimeStamp` to the keyword fixture's grid, with a re-record.

**Evidence.**

1. **Both round-4 items are closed.** I wrote my own probe on a `git archive d61cbf2` copy (own target dir): my own
   `iTXt` XMP packet (`xmp:CreatorTool='R5Tool'`) plus `seAl` = `<a><b>Zed</b></a>`, both chunk orders, read live.
   - `scan` gives `xml: true, xml_plain: true`, and `additivity_failures` is `[]` in both orders.
   - Real output: `failures=[]` under `gps`, `all`, `rights` and `other`, and the output prints no SEAL line.
   - Real output with `SEAL:AB=Zed` added: `["r5 under <policy>: carrier tag left in SEAL:AB=Zed"]` under all four
     policies, in both orders. This is the exact red line.
   - The committed live and in-memory tests make the same assertions and pass in the gate run.
   - Item 2: the comment on `the_additivity_check_compares_a_judged_line_outside_the_exif_and_xml_groups` now matches
     the body line for line (input `[Make, Artist]`, carriers `[Make]`, control `[Artist]`, red `[Artist, Artist]`).
     The dated correction note in "Fix round 3" says the same.
2. **Amendment and doc comments against the code.**
   - True:
     - `judged` is unchanged, and the excuse (`excusable`) is built from it.
     - `carrier_checked` is `judged(l) || xml_group(&l.group)`, with Warning and Error excluded. It filters
       `in_c`/`out_c`/`carrier_ms` in the by-line check and `pick` in `additivity_failures`.
     - "Every carrier line the by-line check counts is also compared for additivity" and "every line the excuse uses
       is in both" hold, since `judged ⊆ carrier_checked` and both checks use the same filter.
     - "With a plain packet present, the output-only scan does not run" holds: it is guarded by
       `carriers.xml && !carriers.xml_plain`.
     - The doc comments on `carrier_checked` ("wider than `judged` only by `XMP*`, `XML*` and `SEAL*` lines that are
       not `judged`") and on `additivity_failures` are accurate. `XMP*` lines are always `judged` via `Cat::Xmp`, so
       that part is empty, but the statement is not wrong.
     - Amendment item 3 holds: the 36 fixtures add up, and no corpus record has a plain packet beside an XML/SEAL
       carrier.
   - Not exact:
     - "XML*/SEAL* … never excused" and "XML*/SEAL* lines are not judged" (punch item 1).
     - Minor: the by-line block runs when either family has a carrier and a plain block, and its `family` filter then
       also checks the other family's carrier lines, so "when a plain block of the same family exists" is narrower
       than the code. That only overlaps the output-only scan. No item.
3. **The guards bite.** Each mutant was in its own `git archive d61cbf2` copy with its own `CARGO_TARGET_DIR`, running
   `cargo test --test strip_png_webp_oracle` with `DIAMOND_EXIF_REQUIRE_ORACLE=1`.
   - **A** (the by-line `multiset_of` filter → `judged`): 41 passed, 2 failed, both SEAL red-proofs. The in-memory one
     fails at `:1500` and the live one at `:1545`, each the red assertion, each with `gps seal_first=true: []`.
   - **C** (the `additivity_failures` `pick` filter → `judged`): 42 passed, 1 failed:
     `the_additivity_check_compares_a_seal_line_which_the_judge_does_not_judge_in_memory` at `:1360` (`[]`).
   - **B** (`carrier_judged` built with `carrier_checked`): 43 passed, 0 failed. It is **equivalent**, checked in the
     code:
     - `carrier_judged` is read only by `excusable`, and `excusable` only through `get(k)`/`get_mut(k)` for `k` in
       `missing`. `missing` is built from `in_m`, which is the `judged` input lines.
     - `judged` and `carrier_checked` are both functions of `(group, tag)` alone, which is part of the key. So for a
       `judged` key every carrier line with that key passes both filters, and its count is the same. The extra
       entries B adds have non-`judged` keys and are never looked up.
     - `carrier_ms` is built separately from `carrier_read` and does not depend on `carrier_judged`.
     - Attempt to break it: I appended my live probe (both bodies above, four policies, both orders, controls and
       every doctored SEAL line, 52 verdicts) to the B copy. B's output is byte-identical to the unmutated copy's,
       including the case where `SEAL:Copyright` is excused. I found no input that separates B. The claim holds.
4. **No new gap of the false-red kind.**
   - The lines `carrier_checked` adds beyond `judged` are non-`judged` `XML*`/`SEAL*` lines. For them:
     - the by-line check fails only if the output prints more of the line than the input printed beyond the carriers;
     - a plain XMP block prints `XMP-*` groups, never `XML`/`SEAL`, so a legitimate strip cannot trip it;
     - additivity on them holds on all 36 carrier fixtures and on both probe bodies.
   - The one mismatch between the two predicates is punch item 1, which is a wording error and not a false red.
   - Real fixtures, live, on the scratch copy: `the_readings_add_up_on_every_carrier_fixture` passes (asserts 36).
     `the_judge_holds_both_ways_for_every_fixture_and_policy` prints, in replay and live: `2583 sections judged, 598
     with a removable tag (2408 tags); exception (a) carrier used 677 times, (b) whole block 354 times, (c) residual
     24 times`. These are unchanged.
5. **The CI fix is right and complete in the fuzz models.**
   - The rewrite_png model's `keyword_category` now has the library's 10 `TEXT_KEYWORDS` rows (`src/audit/category.rs`),
     in the same order and with the same categories.
   - Its match rule, `kw == name || (kw[0].to_ascii_uppercase() == name[0] && kw[1..] == name[1..])`, is the
     library's rule: an exact match, or the first letter uppercased with the rest exact. With lowercase-first rows
     (`create-date`) only the exact spelling matches, in both.
   - `audit_invariants.rs`'s `png_keyword_category` already had the three rows.
   - The other mirror is punch item 2.
   - The seed's `shasum` is `fa312c03c2e1e99d32e3a8801e5b0dd66c280b09`. The failed CI run `45908377338` (job `fuzz smoke
     — rewrite_png (60s)`, head `922482a`) logs `guards [Records] failed` at `rewrite_png_invariants.rs:936` and
     `Test unit written to …/crash-fa312c03c2e1e99d32e3a8801e5b0dd66c280b09`. libFuzzer names an artifact by the
     SHA-1 of its bytes, so the committed seed is that artifact byte for byte. I read the log only and downloaded no
     artifact.
   - The seed carries two `tEXt` `Model` chunks. `fuzz/seeds/rewrite_png` holds 436 files, matching
     `REWRITE_PNG_SEEDS = 436`.
   - Red without the fix: on a `git archive d61cbf2` copy with only the three rows removed (array back to 7),
     `cargo test --test fuzz_layout rewrite_png_harness_holds` gives 0 passed and 1 failed:
     `rewrite_png_harness_holds_on_every_seed_and_every_policy_byte` panics with `guards [Records] failed` (policy
     `all_pii`, a `CameraModel` text removal). At the tip it is green: it is in the `just test` run below.
6. **Gates at `d61cbf2`, in this worktree, rustc 1.90.0, clippy 0.1.90, ExifTool 13.55 required.**
   - `just lint`, `just lint-fuzz`, `just lint-no-allow` and `just forbid-unsafe` each rc 0. `just lint-fuzz` uses
     its own pinned stable clippy 0.1.98.
   - `just deny` and `just deny-fuzz` each rc 0 (`licenses ok`).
   - `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test` rc 0: 41 result lines, 1,544 passed, 0 failed, 20 ignored, no SKIP.
   - `just mutation-oracle` rc 0: 67 + 42 + 43 passed.
   - `just validate` rc 0 (41 artifacts, 176 parse).
   - `just decisions-audit --changed` rc 0. It reports no uncommitted change at the tip. `--changed 9c368ba` is
     advisory and rc 0, and lists DEC-023 for the judge and tests.
   - `just fuzz-smoke rewrite_png` rc 0 (482,592 runs in 61 s) and `just fuzz-smoke rewrite_webp` rc 0 (1,112,222
     runs in 61 s). `fuzz/Cargo.lock` was restored after each fuzz and lint-fuzz run, and `git status` was clean
     before this commit.
- **Process.** I started and waited on only my own processes: the gate script, three mutant runs, the CI-revert run
  and the probe runs, all from the session scratchpad. None is left running. Mutants, probes and the CI revert ran
  only in `git archive` copies under the scratchpad, each with its own target dir. Nothing outside this file was
  changed or committed. No cost numbers are filled in here.

**Orchestrator fix of the round-5 items and close of the verify cycle** (claude-opus-5-5, 2026-11-27, branch `chore/spec-039-ship`)
- **Item 1 (L), wording.** New DEC-023 amendment of 2026-11-27, "which XML and SEAL lines are never excused": an `XML*`/`SEAL*` line that is not `judged` is never excused; one with a category through the tag-name match (`SEAL:Copyright`, `SEAL:Make`, `XML:Make`) is `judged` and excused like any other. The two false sentences in the amendments of 2026-11-26 are marked in place. The module header of `tests/support/i3_judge.rs` and the comments on `carrier_checked` and `carrier_judged` now say the same. `judged` is not narrowed, for the reason the verifier gave. The amendment also states the by-line check's real scope (either family with a carrier beside a plain block, line 452), the minor imprecision round 5 noted. The mutant-B note above ("adds only `XML*`/`SEAL*` keys") is imprecise in the same way: the extra keys are the `XML*`/`SEAL*` lines that are not `judged`. The equivalence argument is unchanged, and round 5 confirmed it independently.
- **Item 2 (L), test mirror.** `category_of_printed_name` in `tests/audit_png_webp_oracle.rs` gains `Make`/`Model` → CameraModel and `TimeStamp` → Timestamps, and its doc comment says so. Still latent: the keyword fixture holds none of the three keywords, and it was not re-recorded.
- **Checks.** `cargo fmt --check` and `just lint` pass. With ExifTool required, `audit_png_webp_oracle` passed 29, and `strip_png_webp_oracle` passed 43 with 0 failed. Nothing under `src/`, `fuzz/` or `tests/oracle-fixtures/` changed. The full gate run at the ship commit is in the ship PR.
- **Close.** Round 5's only items were documentation, and both are fixed here, so SPEC-039 leaves verify as `approved`. `task.verify_rounds` records five punch-list rounds and then the approval. `handback.verdict` above stays `punch-list`, the independent verifier's last verdict. The approval rests on round 5's evidence plus these documentation-only fixes, which were not re-verified independently.
- **Round 5 cost**, from the harness subagent result: 167,420 tokens, 57 tool calls, 709 s (about 12 min). Basis: agent-result.

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
