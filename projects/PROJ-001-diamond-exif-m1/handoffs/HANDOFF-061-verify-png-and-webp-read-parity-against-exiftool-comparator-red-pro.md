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
  id: HANDOFF-061
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
  tokens_total: 184662              # harness subagent_tokens
  estimated_usd: 1.22              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 40
  branch: claude/spec-031-verify
  pr: null
  completed_at: 2026-11-21         # YYYY-MM-DD
  notes: "tokens_total is null: a sub-agent cannot read its own transcript token counts; the orchestrator replaces it with the harness figure. About 45 tool calls."  # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-061: Verify SPEC-031 PNG read parity against ExifTool

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-031` to a cold verifier. You did not write this. Review branch `claude/spec-031-png-parity` (build commits `79712b5` checkpoint 1 and `358322b` checkpoint 2, cost commit after) against the spec (`projects/PROJ-001-diamond-exif-m1/specs/SPEC-031-*.md`, the file without `-timeline`). This is an evidence spec: your job is to decide whether the evidence is real and the red-proofs bite. Review only; edit nothing except your own handback in this file. Do not push.

## FIRST STEP

The harness may start your worktree from main. Run `git checkout -b claude/spec-031-verify claude/spec-031-png-parity` (local branch; do not `reset --hard`) and confirm HEAD contains `358322b`.

## BUDGET (hard)

About 80 tool calls or 200k context. Record `tokens_total: null` with a note if you cannot measure it (the orchestrator replaces it with the harness figure). Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; read `guidance/toolchain-brief.md`. ExifTool 13.55 is installed (run it, never read its source). Use a timeout-safe approach for any ad hoc ExifTool run (`perl -e 'alarm 8; exec @ARGV' -- exiftool ...`).

## What to look hard at

1. **Is the evidence real?** Re-run ExifTool yourself on at least 12 committed fixtures (write the fixture bytes out through the harness or a scratch example, run `exiftool -T -n -s3` with the recorded selectors) and compare to the committed `png-*.txt` records byte for byte. Check that the record headers carry the real command, version and stderr. Check `just oracle-record` is idempotent (twice, no diff).
2. **Fixture independence.** `tests/support/oracle_png.rs` must share no encoding code with `src/`, `examples/fuzz_seeds.rs`, `fuzz/shared/*` or `tests/support/png_shapes.rs`. Check imports and look for copied logic. Its CRC-32, Adler-32 and stored-deflate vectors must be checked against published values by a test that can fail.
3. **Comparator honesty.** The builder defined the XMP selector as readable XMP and the ICC selector as `PNG:ProfileName`. Judge whether that hides anything: could a located-but-wrong region pass? The several-`eXIf` finding (ExifTool reads only the first block that is a TIFF, unlike JPEG) is claimed measured: re-measure it on at least five variants including garbage-then-good, good-then-garbage, `Exif\0\0`-prefixed first, and two good blocks with different IFD0 content, and confirm the harness model and `PngMetadata::exif()` agree on every one.
4. **Divergences.** The raw-profile and compressed-XMP entries must each have the measured stdout, the right-side sentence and a `questions.yaml` id; re-measure two of them. One matters for the audit: with `eXIf` plus `Raw profile type exif` both present, ExifTool shows the RAW profile's value (`IFD0:Make = RawCam`) while we read the `eXIf` (`AlphaCam`). Say whether the divergence entry and question text make that privacy-relevant consequence clear (an audit that trusts the `eXIf` can miss what a reader sees). Check that the test requiring each divergence to be PRESENT really fails when a divergence disappears.
5. **Red-proofs.** Run `just mutation-oracle`. For the seven mutants and two byte mutations: confirm each control is green first, each mutation is marked `RED-PROOF INJECTION`, located by a needle that must occur exactly once, and the leak test works. Then add your OWN mutants on a scratch copy (rsync the tree elsewhere; do not mutate the branch): at least five library faults the builder did not use (for example: `iCCP` name length limit, text keyword NUL search, `after_idat` flag, `ExtraExif` logic, region cap, `BadTextChunk` routing), and report which turn the PNG comparison red and which survive (and whether the survivor is equivalent or a gap in fixtures).
6. **Shape coverage (lesson from SPEC-029/030: unmeasured variants cost fix rounds).** Probe ExifTool on PNG variants the fixture list does NOT contain and report agreement with `png::read` on located metadata: for example `eXIf` of exactly 8 bytes, `eXIf` with a big-endian TIFF whose IFD offset is odd, `iTXt` XMP with a non-empty language tag and translated keyword, `iCCP` with name length 79 and 80, a `tEXt` with an empty value, `zTXt` with method byte 1, two IHDR chunks, an `IDAT`-less file with `eXIf`. Disagreements on located metadata are Medium; on warnings only, Low.
7. **Collateral changes.** The builder added a `.filter` skipping `png-*` records in `tests/jpeg_oracle.rs`, `tests/budget_oracle.rs`, `tests/inventory_oracle.rs`: confirm these weaken nothing (they must still cover every non-PNG record). `app.just` change to `just mutation-oracle` is sound. No change under `src/` or `fuzz/` (diff against `a04413d`). README and counts.
8. Run `just lint`, `just test`, `just mutation-oracle`, `just validate`, `just cost-audit`, `just forbid-unsafe` yourself.

## Expected Deliverables

Your verdict, exactly one of APPROVED / PUNCH LIST / REJECTED, with findings ranked High, Medium, Low, each with file:line and a concrete failing input. Fill the `handback:` front matter (`verdict`) and the `## Handback` section; set statuses to `completed`; commit only the handback.

## Out of Scope

Fixing anything, DEC edits, WebP parity (SPEC-034), audit wiring (SPEC-032).

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

### Execution notes

- **Branch / PR:** `claude/spec-031-verify` (from `claude/spec-031-png-parity` at `937b15f`, which contains `79712b5` and `358322b`); no PR (verify).
- **Completed at:** 2026-11-21
- **All acceptance criteria met?** The listed criteria are met as written and every gate is green. The evidence is real and the red-proofs bite. The punch list is about coverage: three shapes the fixture list leaves out disagree with ExifTool on located metadata, and one comparator blind spot lets a located-but-wrong XMP region pass the whole suite.
- **For `verify`:** ⚠ PUNCH LIST (3 Medium, 6 Low; no High).

#### Findings

**Medium**

- **M1. `iCCP` profile-name rule: ExifTool reads ICC that we do not locate.** `src/png/metadata.rs:364` (`name_end`), used at `:516`. ExifTool 13.55 reads the profile, the full `ICC-header` and `ICC_Profile:ProfileDescription`, when the name is 80 bytes or empty. `png::read` records `BadIccpChunk` and locates no `Icc` region. Failing inputs (built from `oracle_png::iccp`): a name of 80 `N` bytes gives ExifTool `PNG:ProfileName = NNN…(80)` plus the ICC header, and ours gives no region. An empty name gives ExifTool every ICC-header tag with `ProfileName` absent, and ours gives no region. With a 79-byte name the two agree. None of these is a fixture. For the strip and audit this is located metadata missed in the "we see less" direction (DEC-018 says malformed but locatable is located with an issue, as for an 80-byte text keyword). Fix: locate the chunk with `bad` set, as `text_parts` does, or pin it as a divergence with a question. Add fixtures `png-iccp-name-79`, `-80` and `-empty`.
- **M2. `iTXt` XMP with a language tag: we locate XMP that ExifTool does not treat as XMP.** `src/png/metadata.rs:402` and `:487`. The failing input is `iTXt`, keyword `XML:com.adobe.xmp`, flag 0, method 0, language `en-US`, translated keyword `XMP Daten`, then the `oracle_png::xmp_packet()` body. ExifTool prints `XMP-xmp:CreatorTool` as `-` and shows the packet only as the plain PNG text tag `XMLcomadobexmp`. Ours locates `PngRegion::Xmp` (readable), so the presence selector mismatches. That is not a fixture either: every `iTXt` fixture has empty language and translated keyword. The direction is safe (we report more), but it is an unmeasured disagreement on located metadata. Either route a language-tagged XMP keyword to `Text`, or record a divergence with a question.
- **M3. Comparator honesty: XMP (and ICC) presence lets a located-but-wrong region pass.** `tests/support/oracle.rs:858` (`PNG_REGION_SELECTORS`) and `read_png_regions`. My mutant v7 changes the `Xmp` arm's `data: value,` (`src/png/metadata.rs:491`) to `data: value.get(7..).unwrap_or(value),`. **The entire `cargo test` stays green**, and so do `png_oracle`, `png_metadata` and `png_walk` on their own. Nothing checks the bytes of a located XMP region against the packet the harness wrote. My mutants v8 (`Icc` `data` becomes the whole chunk) and v9 (`iTXt` translated-keyword NUL not skipped) also pass the PNG comparison, and only `tests/png_metadata.rs` catches them. On the ICC side, the `PNG:ProfileName` selector cannot see an ICC profile whose name is empty (M1's empty-name input: ExifTool reads the ICC, its ProfileName is `-`, ours has no region, and the comparison would say "agree"). Fix: in `png_oracle`, assert that every hand-built fixture's located `Xmp`/`Icc` region `data` is byte-equal to the packet or zlib stream the builder put there. That uses no ExifTool and is independent of `src/`. Optionally, use `ICC-header:ProfileFileSignature` (or `-ICC_Profile:all` presence) instead of `PNG:ProfileName` for ICC presence.

**Low**

- **L1. Fixture gap: `iCCP` name of 79 bytes.** My mutant v1 (`name_end` limit `MAX_KEYWORD.saturating_add(1)` becomes `MAX_KEYWORD`, `src/png/metadata.rs:365`) rejects a 79-byte name and survives the full `cargo test`. Today's behaviour agrees with ExifTool (measured), but nothing pins it. The M1 fixtures close this.
- **L2. Zero-length `tEXt` XMP reads as readable XMP present.** The input is `tEXt` `XML:com.adobe.xmp\0` with an empty value. ExifTool gives `Warning: Invalid XMP` and binary `XMP` of 0 bytes, with no CreatorTool. Ours gives an `Xmp` region of 0 bytes, `compressed = false`, so `read_png_regions` says present, with no issue. This is the empty-packet case of M3.
- **L3. The XMP selector is "XMP carrying CreatorTool", not "readable XMP".** `tests/support/oracle.rs:850-857`, doc comment. The failing input is an uncompressed `iTXt` XMP with only `dc:format`: ExifTool's CreatorTool is `-` and ours is present, a false red. It hides nothing (it goes red), but the definition should say that every XMP fixture must carry `xmp:CreatorTool`.
- **L4. Privacy consequence of `png-exif-and-raw-exif` is not stated.** `tests/support/oracle.rs:513` and `guidance/questions.yaml:1420`. The measured fact is right. I re-measured it: `-T` gives `RawCam OnlyInA`, and `-a -G1` lists `IFD0:Make` twice, AlphaCam then RawCam, so ExifTool merges and the later value wins. The entry says "exif-core for the specification", and the question's "cost of leaving it" covers only raw-profile-only PNGs. Add one sentence: when a PNG has both, a reader shows the raw profile's values (and any tags that exist only there, such as GPS), so an audit or strip that trusts the `eXIf` alone can miss metadata a reader displays. That needs a SPEC-032 or strip follow-up (at least flag a RawProfile region beside an `eXIf`).
- **L5. Library faults the PNG comparison misses, caught only by unit tests.** v2 (an `iTXt`/`tEXt` keyword is bad at 79, `:378`), v5 (region cap 1, `:428`: no fixture has a second region that matters) and v6 (`BadTextChunk` not recorded on the `None` path, `:481`) all leave `png_oracle` green. `tests/png_metadata.rs` catches each. Issue-only faults are expected to miss a value comparator. v5 is a fixture gap: one fixture with `eXIf` + XMP + `iCCP` together would close it.
- **L6. `measured_stdout` in the PNG `KNOWN_DIVERGENCES` entries is a paraphrase, not the literal stdout.** `tests/support/oracle.rs:458-537`. The records hold the literal line (verified byte for byte), so this is presentation only.

#### Focus-area results

1. **Evidence is real.** I wrote every fixture out with a scratch example that `#[path]`-includes `oracle_png.rs` and `oracle_tiff.rs`, and ran `perl -e 'alarm 8; exec @ARGV' -- exiftool -T -n -s3 <the 22 recorded selectors>` on all **49** hand-built fixtures. Stdout equals the committed record body byte for byte on 49 of 49. The `Warning:`/`Error:` stderr lines, with the path replaced by `<file>`, equal the `# stderr:` lines on 49 of 49. Headers carry the command, `exiftool-version: 13.55` and the source. `just oracle-record` run twice leaves `git status` empty both times (117 PNG records plus the warnings record).
2. **Fixture independence: holds.** `tests/support/oracle_png.rs` uses only `crate::oracle_tiff`. It has no `exif_core` import, nothing from `src/`, `examples/fuzz_seeds.rs`, `fuzz/shared` or `png_shapes.rs`. CRC-32 (bitwise, reflected) and Adler-32 are written from scratch. `tests/png_oracle.rs:264-265` checks them against the published vectors (`crc32("123456789") = 0xCBF43926`, `adler32("Wikipedia") = 0x11E60398`), and that test can fail.
3. **Comparator.** The several-`eXIf` model is confirmed. `read_ours_png` equals `PngMetadata::exif()` (the same `find_map` over parsed containers, `src/png/metadata.rs:283`), and ExifTool agrees with it on **13** variants outside the fixture list: garbage then good, good then garbage, `Exif\0\0`-prefixed first then plain, plain then prefixed, two good blocks with different IFD0 content in both orders, empty then good, bare `Exif\0\0` then good, an 8-byte header then good, an IFD past the end then good, a bad-CRC good block then good, good before IDAT with another after, and after-IDAT then after-IEND. Honesty: see M3, L2 and L3.
4. **Divergences.** All 9 PNG entries name a question, and both questions exist in `guidance/questions.yaml`. I re-measured `png-exif-and-raw-exif` (`- - RawCam OnlyInA …`) and `png-xmp-ztxt` (CreatorTool `DiamondTool`) plus `png-raw-exif-ztxt`, and all match their records. `a_divergence_that_stops_being_one_fails` flips ExifTool to agree and asserts `stale` is non-empty. Mutant p7 shows a vanished divergence going red through `stale`. The privacy wording gap is L4.
5. **Red-proofs.** `just mutation-oracle`: 58 + 39 passed, the 1 ignored test is the shared `tools::env_limit_child`. The seven mutants p1 to p7 each run `assert_control_clean` first and are injected with `marked()` (`RED-PROOF INJECTION`) via `probe::inject_replace`, which asserts the needle occurs exactly once. The leak test also pins all seven needles at count 1 and proves the scan catches a planted marker. Both byte mutations check that the unmutated control is clean first. My own mutants, on an rsync copy with a scratch `src/png/metadata.rs`, never on the branch:

   | Mutant | `png_oracle` | full suite |
   |---|---|---|
   | v1 `iCCP` name limit 78 | green | **green** (L1) |
   | v2 keyword bad at 79 | green | red (`png_metadata`) |
   | v3 `after_idat` false | **red** | red |
   | v4 ExtraExif from the third block | **red** (warning table, several-eXIf) | red |
   | v5 region cap 1 | green | red (`png_metadata`) |
   | v6 BadTextChunk not recorded | green | red (`png_metadata`) |
   | v7 XMP data shifted 7 | green | **green** (M3) |
   | v8 ICC data is the whole chunk | green | red (`png_metadata`) |
   | v9 translated NUL not skipped | green | red (`png_metadata`) |
   | v10 trailer `after_idat` false | **red** | red |

   v1 and v7 are gaps, not equivalent mutants (each changes observable output: a located ICC region, and the XMP bytes).
6. **Shape coverage.** I probed ExifTool on these shapes, none of which is a fixture. Agree: `eXIf` of exactly 8 bytes (both read nothing, ExifTool warns `Bad IFD0 directory` and we raise no issue: Low, warnings only), a big-endian TIFF with IFD0 at odd offset 9, `iCCP` name 79, `zTXt` method 1 (both locate; ExifTool warns `Unknown compression method 1`), `iTXt` flag 0 method 1, `iTXt` with a language tag and no NUL (both nothing), two IHDR chunks with `eXIf`, and an IDAT-less file with `eXIf` (both read all 14 values). Disagree: `iCCP` name 80 and empty (M1), `iTXt` language tag plus translated keyword (M2), and an empty `tEXt` XMP (L2).
7. **Collateral.** No change under `src/` or `fuzz/` since `a04413d`. The three `.filter(|n| !n.starts_with("png-"))` additions only remove `png-*` names. The non-PNG lists they compare against are unchanged. `tests/png_oracle.rs:382` (`the_png_record_set_is_exactly_the_fixtures_and_the_seeds`) pins the PNG set exactly, so nothing is orphaned. The `app.just` change adds `--test png_mutation_oracle` to `just mutation-oracle`, which is sound. The README has no PNG record counts, so no change is needed.
8. **Gates** (`RUSTUP_TOOLCHAIN=1.90.0`, `~/.cargo/bin` first, `DIAMOND_EXIF_REQUIRE_ORACLE=1`): `just lint` 0, `just test` 0 (png_oracle 37 passed, 1 ignored; png_mutation_oracle 39 passed, 1 ignored; no failures), `just mutation-oracle` 0, `just validate` 0, `just cost-audit` 0, `just forbid-unsafe` 0. `just oracle-record` twice: no diff.

### Cost self-report

- **Tokens (total):** null. A sub-agent cannot read its own transcript token counts; the orchestrator replaces this with the harness figure.
- **Estimated USD:** null
- **Duration (minutes):** about 40
- **Source of the number:** harness report (to be filled in by the orchestrator)

### Drift and new artifacts

- **New decisions emitted:** none
- **Deviations from spec:** none. Review only. The scratch probes and mutants lived in the scratchpad copy, never on the branch.
- **Follow-up work identified:**
  - Fixtures `png-iccp-name-79`, `-80` and `-empty`, `png-xmp-itxt-lang`, `png-xmp-text-empty`, and one with `eXIf` + XMP + `iCCP` together; decide M1 and M2 (fix or divergence plus question).
  - A region-bytes check in `png_oracle` (M3).
  - SPEC-032: flag a RawProfile beside an `eXIf` (L4).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   Nothing material. The handoff's list of probe shapes was exactly where the problems were.
2. **Was there a constraint or decision that should have been listed but wasn't?**
   DEC-018 says what to do with a malformed text keyword (locate with an issue) but not with a malformed `iCCP` name. The reader rejects it, ExifTool reads it, and the spec never asked.
3. **If you did this task again, what would you do differently?**
   Write the scratch fixture dumper first. It made the 49-record byte comparison, the 13 several-`eXIf` variants and the shape probes one loop each.
