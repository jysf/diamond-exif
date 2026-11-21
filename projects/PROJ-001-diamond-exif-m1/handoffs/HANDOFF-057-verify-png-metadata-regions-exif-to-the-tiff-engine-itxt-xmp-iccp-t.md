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
  id: HANDOFF-057
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-21
  status: completed                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-029

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
  verdict: approved                # verify handoffs: approved | punch-list | rejected
  basis: null                      # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 35
  branch: claude/spec-029-verify
  pr: null
  completed_at: 2026-11-21         # YYYY-MM-DD
  notes: "Verifier subagent cannot read its own transcript token counts; the orchestrator replaces tokens_total with the harness figure. Approved at build tip 01c36a2 (build commit a8c38c4). Two Medium findings are DEC-018 gaps (zTXt XMP, keyword over 79 bytes), not build defects."
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-057: Verify SPEC-029 PNG metadata regions (DEC-018)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-029` to a cold verifier. You did not write this. Review branch `claude/spec-029-png-metadata` (build commit `a8c38c4`, cost commits after it) against the spec (`projects/PROJ-001-diamond-exif-m1/specs/SPEC-029-*.md`, the file without `-timeline`), `decisions/DEC-018-png-metadata-regions-exif-hand-off-and-text-routing.md` and DEC-017. Review only; edit nothing except your own handback in this file. Do not push.

## FIRST STEP

The harness may start your worktree from main. Run `git checkout -b claude/spec-029-verify claude/spec-029-png-metadata` (local branch; do not `reset --hard`) and confirm HEAD contains `a8c38c4`.

## BUDGET (hard)

About 80 tool calls or 200k context. Record `tokens_total: null` with a note if you cannot measure it (the orchestrator replaces it with the harness figure). Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; read `guidance/toolchain-brief.md`.

## What to look hard at

1. **Panic-freedom and arithmetic** in `src/png/metadata.rs` and the new `frame` in `src/png/mod.rs`: every offset and length from the file, every slice, every loop's progress, the trailer scan bound.
2. **Did the refactor change the walker?** `frame` is shared by the walk and the trailer scan: confirm SPEC-028 behaviour is identical (all its tests and the old model pass unchanged; look for any subtle change in issue order, offsets or caps).
3. **DEC-018 conformance both directions**: every rule has code and a test that would fail if the rule were removed (keyword rule, 79-byte limit, iTXt header, zTXt, `Raw profile type ` routing, case sensitivity, `Exif\0\0` prefix, `ExtraExif`, caps, trailer scan, after_idat / after_iend, `TooManyRegions`). Judge the builder's choices where the DEC is silent: (a) any walker halt, not only a cap, suppresses all extraction issues; (b) `ImproperExifHeader` only for a parsed block; ExtraExif before ImproperExifHeader on one chunk; (c) `zTXt` with keyword `XML:com.adobe.xmp` is `Text`; (d) `TooManyRegions` stops all extraction including blocks, at the chunk that did not fit; (e) a trailer chunk is `after_idat` if any IDAT precedes it; (f) regions use `&[u8]` names. Say for each whether it is sound or needs a DEC-018 clarification. ExifTool 13.55 is installed: for (c) and any other doubtful case, build a tiny PNG and look at what `exiftool -a -G1 -s -n` does (zTXt with that keyword in particular), and report what you find.
4. **The model's independence**: `fuzz/shared/png_metadata_invariants.rs` derives regions/blocks/issues from raw bytes by its own parsing (it takes only the walker outcome from the library: judge whether that is acceptable); it must be able to fail: mutate the library on a scratch copy (rsync the tree elsewhere; do not mutate the branch) in at least six ways (keyword limit off by one, case-insensitive match, trailer cap off by one, after_iend flag, ExtraExif logic, compressed flag) and confirm tests or the model go red; list survivors and say if each is equivalent.
5. **Pins, seeds, scope**: 31 `meta-` seeds, older 33 byte-identical, `fuzz/Cargo.lock` not committed with a changed version, test-only arms in jpeg/tiff tests change no behaviour, README claims honest (PNG metadata located, not read by `exif-audit`), no CHANGELOG entry (the release pin), no new dependency.
6. Run `just lint`, `just test`, `just lint-fuzz`, `just validate`, `just forbid-unsafe`, `just cost-audit`, and a 20 s fuzz run of `png_chunks` yourself (the recipe hardcodes 60 s; use its cargo command with `-max_total_time=20`).

## Expected Deliverables

Your verdict, exactly one of APPROVED / PUNCH LIST / REJECTED, with findings ranked High, Medium, Low, each with file:line and a concrete failing input. Fill the `handback:` front matter (`verdict`) and the `## Handback` section; set statuses to `completed`; commit only the handback.

## Out of Scope

Fixing anything, DEC edits, audit wiring, ExifTool parity tests (SPEC-031).

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

- **Branch / PR:** `claude/spec-029-verify` (local, not pushed; branched from `claude/spec-029-png-metadata` at `01c36a2`, which contains build commit `a8c38c4`). No PR (verify).
- **Completed at:** 2026-11-21
- **All acceptance criteria met?** yes. Every SPEC-029 criterion has code and a test; gates green (below).
- **For `verify`:** APPROVED (at `01c36a2`). No change to the build branch is required. The two Medium findings are gaps in DEC-018 (the builder followed its text exactly) and should be settled by a DEC-018 clarification before SPEC-031/SPEC-032.

#### Findings

**High:** none.

**Medium (DEC-018 gaps, not build defects)**

- **M1. A `zTXt` with keyword `XML:com.adobe.xmp` is routed to `Text`, but ExifTool reads it as XMP.** `src/png/metadata.rs:472` (`&& kind != ChunkKind::Ztxt`). Failing input: signature, IHDR, IDAT, `zTXt` with data `XML:com.adobe.xmp\0` + `\0` + zlib(an XMP packet with `dc:format="secret/zz"`), IEND. ExifTool 13.55 `-a -G1 -s -n` prints `[XMP-dc] Format : secret/zz`: it inflates the stream and parses it as XMP. With a stream that is not zlib it warns `Error inflating XML:com.adobe.xmp` and prints `[PNG] XMP : (Binary data 5 bytes ...)`, so it still names the chunk XMP. The library reports `Text { keyword: "XML:com.adobe.xmp", compressed: true }`. DEC-018 Routing only names `iTXt` and `tEXt`, so the code matches the DEC, but the measurement did not test `zTXt`. Effect: SPEC-032 would label this XMP as free text. It is still marked unreadable, because `compressed` is true. Suggested clarification: `zTXt` with that keyword gives `Xmp { compressed: true }`.
- **M2. A text keyword over 79 bytes (and an `iTXt` flag other than 0 or 1) gives `BadTextChunk` with no region, but ExifTool reads the chunk.** `src/png/metadata.rs:360-364` and `:385-389`. Failing input: IHDR, IDAT, `tEXt` with data `"A"*80 + "\0hello"`, IEND. ExifTool prints a PNG tag named by the 80-byte keyword with value `hello`. For `iTXt` with data `Comment\0\x02\0\0\0hello`, ExifTool warns `Error inflating Comment` and prints `[PNG] Comment : (Binary data 3 bytes)`. DEC-018 and the spec's acceptance list require this exact behaviour, so the code is right by contract. For a privacy audit, though, a text value that ExifTool shows becomes only an issue with no located region. A clarification should decide this before SPEC-031 measures it: either locate the chunk anyway (`Text` plus the issue) or record it as a known divergence.

**Low**

- **L1. Choice (a): the halt rule suppresses more in PNG than in JPEG.** `src/png/metadata.rs:409`. A JPEG halt is caps only (`src/jpeg/mod.rs:398`). A PNG halt also comes from `Truncated` and `BadChunkLength`, and a truncated download is common. Failing input: IHDR, `tEXt "bad"` (no NUL), `eXIf "junk"`, `eXIf "junk"`, then a chunk whose length runs past the end of the input. `png_issues()` is just `[Truncated]`: `BadTextChunk` and `ExtraExif` are dropped. The regions and blocks are kept. `walker_halt_suppresses_extraction_issues_but_keeps_found_regions` pins this behaviour. It is sound and keeps DEC-017's "the halt is the last issue", but DEC-018 should say so, because the spec's wording ("add no extraction issue after the halt") also allows the narrower reading.
- **L2. DEC-018 says "Extraction issues are at most one per chunk", but one `eXIf` can yield two.** `src/png/metadata.rs:439-447`. Failing input: two `eXIf` chunks, the second starting `Exif\0\0MM\0*`. Issues: `[ExtraExif{at}, ImproperExifHeader{at}]` at the same offset. The module doc admits it ("at most two per chunk") and the total is still bounded by constants. The DEC text needs one word changed.
- **L3. There is no public bound on `png_issues().len()`.** JPEG has `MAX_JPEG_METADATA_ISSUES`; PNG extraction issues are not capped by `MAX_PNG_ISSUES` (1024). Failing input: 2000 `eXIf` chunks give 1999 `ExtraExif` issues, more than the walker's 1024. The total is bounded by the cap arithmetic: 2 x (65,536 + 4,096) + 1 + walker issues. That meets the DEC, but nothing asserts it and it differs from JPEG.
- **L4. The fuzz model alone misses 7 of my 24 mutants because no seed has the shape.** Unit tests kill all 7. They are: keyword limit 78, case-insensitive `Raw profile type `, trailer cap +1 and -1, allowing `zTXt` XMP, region cap `>` and no-stop, and a 17th Exif block. No seed has a `zTXt` with the XMP keyword, a lower-case `raw profile type `, or a run of more than 4096 trailer chunks or 16 `eXIf` chunks. Adding `meta-ztxt-xmp` and `meta-raw-lowercase` seeds would let the fuzz model catch the two routing mutants. The caps are correctly left to unit tests.
- **Observation (not a finding):** in the refactor, `frame`'s unreachable `None` arms now give `Frame::Short`, which halts with `Truncated{len: FRAME}`. The old code returned silently. They cannot be reached, because `remaining >= FRAME` and `total <= remaining` are checked first (`src/png/mod.rs:215-251`). The behaviour is identical: `tests/png_walk.rs` and `fuzz/shared/png_chunk_invariants.rs` are byte-identical to `fb54eae` and pass.

#### Builder choices (a) to (f)

- **(a)** Any walker halt suppresses all extraction issues. Sound, and it keeps "halt is last". Needs a DEC-018 sentence (L1).
- **(b)** `ImproperExifHeader` only for a parsed block, and `ExtraExif` before it on the same chunk. Sound: a chunk past the 16-block cap is not looked into, and the order matches "is it extra, then what is inside". The DEC's "one per chunk" wording needs fixing (L2).
- **(c)** `zTXt` with the XMP keyword is `Text`. Literal to the DEC, but ExifTool reads it as XMP. Needs a DEC-018 clarification (M1).
- **(d)** `TooManyRegions` stops all extraction, blocks included, at the chunk that did not fit. Sound: it matches "extraction stops", and `region_cap_halts_once` pins the offset and the missing block. It could be stated in the DEC, but it is not ambiguous.
- **(e)** A trailer chunk is `after_idat` if any `IDAT` comes before it, the trailer included. Sound; it is the natural reading of "after the first IDAT".
- **(f)** Names are `&[u8]`. Sound: PNG keywords are Latin-1 by the specification, and DEC-002 calls for borrowed, allocation-free results.

#### What else I checked

- **Panic-freedom:** `metadata.rs` uses only `get`, `split_first`, `strip_prefix`, `checked_add` and `saturating_add`, with no indexing. The trailer loop is bounded by `MAX_PNG_TRAILER_CHUNKS` and always advances (`end >= at + 12`). The 20 s fuzz run and the prefix and flip test are clean.
- **DEC-018 both directions:** I ran 24 mutants on an rsync'd scratch copy (the branch was never mutated). All 24 were killed with 0 survivors, after a control pass: keyword limit 80 and 78, case-insensitive XMP, case-insensitive raw prefix, trailer cap +1 and -1, `after_iend` false, trailer `after_idat` false, `ExtraExif` at `>1` and never, `zTXt` uncompressed, `iTXt` flag inverted, flag 2 accepted, `zTXt` XMP allowed, halt not suppressing, region cap `>`, region cap without stop, 17 blocks, `tiff_at` without the prefix skip, `ImproperExifHeader` dropped, raw prefix without its space, `iCCP` method skipped, `zTXt` accepted with no method byte, and walked `IDAT` not tracked into the trailer.
- **Model independence:** acceptable. It frames, parses keywords and headers, routes and sets position flags from the raw bytes, and works out the halt from the walk's issues. From the library it takes only the chunk count, and it asserts its own framing agrees on every offset and type. The walk itself is modelled separately by `png_chunk_invariants`. Limit: it mirrors the DEC as the builder read it, so a shared misreading such as M1 cannot show up in it.
- **Pins and scope:** 31 `meta-` seeds plus 33 older ones; all 33 older seeds are unchanged against `fb54eae`. `fuzz/Cargo.lock` is not changed in any commit (`just lint-fuzz` rewrites it locally to `0.1.0-alpha.1`; I restored it). No changes to `Cargo.toml`, `Cargo.lock` or `CHANGELOG.md`, and no new dependency. The jpeg and tiff test arms only extend the test-only `index` exhaustiveness tables. The README is accurate: PNG metadata is located, `exif-audit` still reports PNG as unsupported, and compressed or raw-profile metadata is listed as a known limit.
- **ExifTool 13.55 extra probes:** ExifTool reads an `iTXt` `Raw profile type exif` too (it prints `[IFD0] Make : ABC`), so routing raw profiles in `iTXt` is right. A lower-case `xml:com.adobe.xmp` is a plain text tag, not XMP, so case-sensitive matching is right. With two `eXIf` chunks where the second is cut short, ExifTool reads the first; the library keeps the first block and halts on the second.

#### Commands run (all from the worktree, `RUSTUP_TOOLCHAIN=1.90.0`, `~/.cargo/bin` first)

- `just lint`: exit 0. `just test`: exit 0. `just lint-fuzz`: exit 0. `just validate`: exit 0. `just forbid-unsafe`: exit 0. `just cost-audit`: exit 0.
- `PATH="$HOME/.cargo/bin:$PATH" ~/.cargo/bin/cargo +nightly fuzz run png_chunks fuzz/corpus/png_chunks fuzz/seeds/png_chunks -- -max_total_time=20`: exit 0, 524,032 runs in 21 s, cov 1139, no crash.
- Mutation script (scratchpad copy, `cargo test --test png_metadata --test fuzz_layout --test png_walk --lib` per mutant): 24 of 24 killed.
- `exiftool -a -G1 -s -n` on 7 hand-built PNGs (results under M1, M2 and the extra probes above).

### Cost self-report

- **Tokens (total):** null. A verifier subagent cannot see its own transcript's token counts; the orchestrator should use the harness figure for this agent.
- **Estimated USD:** null
- **Duration (minutes):** about 35
- **Source of the number:** none available (harness report to be transcribed by the orchestrator)

### Drift and new artifacts

- **New decisions emitted:** none. I recommend a DEC-018 clarification covering M1, M2, L1 and L2.
- **Deviations from spec:** none found.
- **Follow-up work identified:**
  - DEC-018 clarification before SPEC-031: `zTXt` XMP routing, the long-keyword and bad-flag policy, the halt rule's scope, and "at most two per chunk".
  - Add `meta-ztxt-xmp` and `meta-raw-lowercase` seeds when the routing changes (L4).
  - Optionally add a public `MAX_PNG_METADATA_ISSUES` with a test, to match JPEG (L3).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Running `cargo fuzz` needs both `~/.cargo/bin` first in PATH and `+nightly`. `RUSTUP_TOOLCHAIN=nightly` alone still fails because Homebrew's rustc is on PATH. The toolchain brief says this, but the handoff's PATH line points at 1.90.0 only.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — DEC-018's ExifTool probe set left out `zTXt` with the XMP keyword and keywords over 79 bytes. Both show ExifTool reading what the DEC calls Text or invalid.

3. **If you did this task again, what would you do differently?**
   — I would probe ExifTool with every cell of the routing table (each chunk type crossed with each keyword class), not only the doubtful case named in the handoff.
