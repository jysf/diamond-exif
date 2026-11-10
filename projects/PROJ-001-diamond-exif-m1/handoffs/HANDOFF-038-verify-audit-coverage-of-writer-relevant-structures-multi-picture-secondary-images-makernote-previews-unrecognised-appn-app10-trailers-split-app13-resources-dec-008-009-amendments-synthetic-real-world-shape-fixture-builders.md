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
  id: HANDOFF-038
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-09
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-019

project:
  id: PROJ-001
  stage: STAGE-004
repo:
  id: diamond-exif

# ── THE HANDBACK ────────────────────────────────────────────────────────────
# Filled in by the EXECUTING AGENT before it reports done. This is a required
# part of completing the handoff, not a courtesy.
#
# `tokens_total` is the one field the cost gate reads. Report the REAL number
# from your own interface:
#   Claude Code   → run `/cost`
#   API           → the `usage` object (input + output, summed)
#   another agent → whatever your harness reports as total tokens
# If your platform genuinely exposes NO token count, set tokens_total: null AND
# write why in `notes` — then set `cost.metering_source: none` in
# .repo-context.yaml so the gate stops asking. Do not invent a number.
handback:
  status: null                     # completed | blocked | rejected
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # tokens_total × your rate, or your harness's number
  duration_minutes: null
  branch: null
  pr: null
  completed_at: null               # YYYY-MM-DD
  notes: null                      # one line if unusual (rework, no meter, etc.)
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-038: Verify audit coverage of writer-relevant structures

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-019` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-019-*.md` (normative rules 1 to 9, the fixtures, the ~52 named tests and the 28 mutants are the contract)
- **DEC-012 (normative, with its 2026-11-09 amendment that rules on the build's four doubts), DEC-006, DEC-008, DEC-009, DEC-010 with their amendments:** `decisions/`
- **Build handoff and Handback:** `HANDOFF-037-build-...md` (claims to verify, not to trust; read the seven deviations)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe; python reading ExifTool output needs `errors="replace"`)
- Code under review: `git diff 8d755b9..HEAD` (build `ccbae57..05fb303`, then `75d8fdc` = cost record, rustfmt of two fuzz helper files, DEC-012 amendment). Main: `src/jpeg/metadata.rs`, `src/audit/{mod,category}.rs`, `src/policy.rs`, `src/report/{json,text,mod}.rs`, `tests/support/real_shapes.rs`, `tests/real_shapes.rs`, `fuzz/shared/{jpeg_metadata,audit,render}_invariants.rs`, `examples/{fuzz_seeds,oracle_record}.rs`, `tests/oracle-fixtures/exiftool-inventory/real-shapes*.txt`, goldens, seeds `shape-*`.
- Real-file evidence: SPIKE-001 (archived under `spikes/done/`). Public samples may still be in the scratchpad of the orchestrator (`.../scratchpad/public/{A,B,C}`); the user's local photos are in `/Users/jyashinsky/PSeven/experiments/crustimg_redo_plus/_incoming0`. **Privacy rule for local photos: read in place, copy nothing, record only counts, short hashes, category and region names; never tag values, file contents or paths. Run the binary without `--values`.**

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs and `rustfmt --check --edition 2021 fuzz/shared/*.rs`, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test --no-fail-fast` on stable and 1.90.0 with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`, `-F unsafe_code` on lib and bin, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with an empty `git status --porcelain`), `just fuzz-smoke jpeg_segments`, `audit` and `audit_render` 60 s each, `cargo build --release --locked --bin exif-audit`. Confirm `decisions/` is unchanged by the build commits (only `75d8fdc` amends DEC-012, by the architect), no dependency added, `scripts/` untouched.
- **Read the JPEG layer line by line** (`src/jpeg/metadata.rs`): the MPF locator (header `MPF\0`, TIFF header BE and LE, tag 0xB002, 16-byte entries, offsets relative to the byte after `MPF\0`, entry 0 primary, `FF D8` check, range arithmetic with overflow, the 16-image bound, the second MPF segment, the interaction with the tolerant TIFF engine's entry budget), COM, OtherApp and its benign list (APP0 `JFIF\0`, APP14 `Adobe\0` only; `JFXX`, `FPXR`, `AROT` are OtherApp), the trailer computation (trailer minus embedded ranges: overlapping or nested ranges, a range reaching past the trailer start, a range inside the scan data, adjacent ranges, zero-length remainder), and the Photoshop rule (every `Photoshop 3.0\0` APP13 segment is an Iptc region; the data is the payload after the header). Try to break it with hostile inputs the builder did not: overlapping MPF entries, an entry pointing at the MPF segment itself, an entry whose range covers the primary image, offsets near `u32::MAX`, a 0xB002 count that lies, MPF inside a thumbnail, an MPF whose image is itself a JPEG with an MPF (no recursion allowed), 100 MPF segments, an APP2 `MPF\0` of 5 bytes.
- **Audit and policy against DEC-012 as amended:** the 15 categories in `Category::ALL` order; `is_block`; severities (Medium); the preset grid cell by cell (`EmbeddedImage` and `OtherSegment` removed only by `Everything`, `KeptUnread` under the other four); `may_hold` (every category except itself; `MakerNote` may hold `EmbeddedImage`); `holds` is informational, one level, never a disposition input, never added to the outer inventory; region-to-category mapping (`Comment` to `FreeText`); the `StripSpec` 16-bit set still round-trips with 15 categories.
- **Report contract:** `schema_version` 2; `embedded` key after `thumbnail` with the pinned shape; 15 `by_category` keys; region and category enumerations; CSV unchanged at 19 columns; text prints `holds`; goldens regenerated deliberately (the builder's account of the golden diff: JSON schema 1 to 2, two zero categories, `"embedded":null` per finding, iptc region `len` 6 to 18). Check that diff yourself with `git diff 8d755b9 -- tests/golden`.
- **Oracle discipline.** ExifTool is run, never linked. The builder reports ExifTool counts the MPF primary as `MPImage1` so groups = regions + 1 (constant `MPIMAGE_PRIMARY_OFFSET = 1`), with damaged-file pairs pinned (3 groups/1 region, 17/15, 0/0). Re-measure with ExifTool on at least three public samples and the synthetic MPOs. Confirm the class S exclusion is exact (it removes only the new structures), the two inputs moved by the Photoshop rule are pinned as class R with a `KNOWN_DIVERGENCES` entry, and the oracle-must-be-shown-red proofs still go red (`lint-red-proof`). Judge the builder's deviation 1 (the 178-results claim) and deviations 2 to 7.
- **Local real-file re-run (counts only).** Build the release binary and run it over `_incoming0` and the public samples with `--format json --policy all-pii`; reproduce the builder's numbers (4 JPEGs with MPF and 2 images each, 8 `embedded_image` findings, `holds` of the Nikon-style extras empty and of the iPhone-style extras `[xmp]` and `[xmp, icc]`, two `AROT` files, one Leica trailer, split APP13 sample giving 2 iptc regions) and compare each with ExifTool. Report any file where we name a structure ExifTool does not or the reverse.
- **Test the testers.** Re-run the builder's 28 mutants (the Handback maps each to a test) for at least 1, 4, 7, 9, 12, 16, 19, 22, 25, 27 and 28, including the two the builder had to rewrite (22 and 27). Write mutants the builder did not try, for example: entry offsets taken relative to the file start instead of the MPF header; the `FF D8` check dropped; the benign list including `JFXX`; `Adobe` matched by prefix only; trailer computed without subtracting overlapping ranges twice-safely; `holds` computed with embedded location left on (recursion); `Comment` mapped to `OtherSegment`; the 16-image bound off by one; the Photoshop region data starting at the 8BIM instead of after the header; `KeptUnread` for `EmbeddedImage` under `Everything`; the `by_category` order wrong in text but right in JSON. Report which the suite catches (name the test or guard) and which survive.
- **Fuzz.** Redo one plant against the real libFuzzer `jpeg_segments` target and one against `audit`, from the committed `shape-*` seeds only. Check that every new class of input (BE and LE MPF, truncated, out-of-range, non-JPEG, inner, 17-image, split APP13, AROT, FPXR, COM, trailer, Canon-style absolute note, 64 KB Exif) has a seed in each of the three corpora, and that the old three corpora and their recorder counts (249 inputs, 180 distinct) are untouched.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes; locating MakerNote previews; any writer; TIFF-side changes; changing DEC-012 (report doubts as questions)

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

- **Branch / PR:** [link]
- **Completed at:** YYYY-MM-DD
- **All acceptance criteria met?** yes/no (if no, explain)
- **For `verify`:** the verdict — ✅ APPROVED (at commit SHA) / ⚠ PUNCH LIST / ❌ REJECTED

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** <real number, or null + why>
- **Estimated USD:** <number, or null>
- **Duration (minutes):** <estimate>
- **Source of the number:** `/cost` | API `usage` | harness report | none available

### Drift and new artifacts

- **New decisions emitted:**
  - `DEC-NNN` — <title> (if any)
- **Deviations from spec:**
  - [list]
- **Follow-up work identified:**
  - [any new specs that should be added to the stage's backlog]

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — <answer>

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — <answer>

3. **If you did this task again, what would you do differently?**
   — <answer>
