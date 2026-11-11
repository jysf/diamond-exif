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
  id: HANDOFF-047
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-10
  status: completed                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-022

project:
  id: PROJ-001
  stage: STAGE-004
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
  status: completed                     # completed | blocked | rejected
  agent: claude-opus-5-5                      # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: punch-list                    # verify handoffs: approved | punch-list | rejected
  basis: agent-result
  tokens_total: 197870
  estimated_usd: 1.31
  duration_minutes: 8
  branch: claude/spec-022-tiff-dng-strips-26c250
  pr: null
  completed_at: 2026-11-10               # YYYY-MM-DD
  notes: "tokens_total, estimated_usd and duration_minutes left null on purpose: the verifier (a subagent) cannot see its own metered token count; the orchestrator transcribes the harness subagent_tokens and duration. Verdict PUNCH LIST at a9ead7a (code fd5a2c3): no High; three Medium (ExifTool oracle blind to XMP/IPTC/ICC; Custom(Jumbf) is a silent no-op on a TIFF holding a 0xCD41 JUMBF entry; DNG identity tags outside DEC-008 survive Everything) plus a Medium usability follow-up (multi-page TIFF refused by the three presets that include Thumbnail); Lows on DEC-013 D.3/D.4 wording and verify_tiff using the output walk only. Core property held: 264,579 Ok strips from an independent input-walk differential, 0 violations; fuzz 315,498 runs clean. No code changed, nothing pushed, no cycle advanced."
  synced_at: 2026-11-10
---

# HANDOFF-047: TIFF and DNG in-place strip: never touch strip or tile offsets, SubIFDs, TIFF writer fuzz target

## Delegation Summary

`claude-opus-5-5` (acting as `architect`) hands `SPEC-022` to
`claude-opus-5-5` (acting as `verifier`) for the **`verify`** cycle (round 1,
cold, adversarial; HANDOFF-046 was the build).

## Context the Receiving Agent Needs

- Spec: `projects/PROJ-001-diamond-exif-m1/specs/SPEC-022-tiff-and-dng-in-place-strip-never-touch-strip-or-tile-offsets-subifds-tiff-writer-fuzz-target.md` (Design section)
- `AGENTS.md` sections 8, 12, 13, 15; `guidance/toolchain-brief.md`
- DEC-013 (incl. amendment D), DEC-008, DEC-009
- Diff under review: `git diff 023f5cc..HEAD -- src tests fuzz examples`

## Out of Scope

Fixing code, pushing, opening a PR, advancing the cycle.

---

## Handback

### Execution notes

- **Branch / PR:** `claude/spec-022-tiff-dng-strips-26c250`, no PR (per instruction)
- **Completed at:** 2026-11-10
- **All acceptance criteria met?** Mostly. Items 1-4, 6, 7, 9 and 10 are met. Item 5 ("check this against the audit as oracle and against ExifTool" for the XMP/IPTC/ICC carriers) and item 8 (the ExifTool oracle) are only partly met: see M1.
- **Verdict:** ⚠ PUNCH LIST at `a9ead7a` (code at `fd5a2c3`)

#### Gates run by the verifier (host: Apple silicon, `~/.cargo/bin` first in PATH)

| Gate | Result |
|---|---|
| `cargo +1.90.0 fmt --all -- --check` + `rustfmt --check fuzz/shared/*.rs` | clean |
| `cargo +1.90.0 clippy --all-targets -- -D warnings` (clippy 0.1.90) | clean |
| `cargo +1.90.0 rustc --lib -- -F unsafe_code` | clean |
| `just lint-no-allow` (the five lints `-F`) | clean |
| `cargo +1.90.0 test` (ExifTool 13.55 on PATH, live oracles ran) | 907 passed, 0 failed, 9 ignored, 22 result lines |
| `just validate` | 22 artifacts valid, 95 parse |
| `just fuzz-smoke rewrite_tiff` | 315,498 runs in 61 s, exit 0, no crash |
| `just mutation-oracle` | 58 passed, 1 ignored |
| `just lint-red-proof` | control clean, injection rejected, all five lints fired (exit 0) |
| `strip_tiff_oracle` with ExifTool off PATH | loud `SKIP`; with `DIAMOND_EXIF_REQUIRE_ORACLE=1` it fails, as designed |
| `cargo run --example fuzz_seeds` | rewrites 98 `rewrite_tiff` seeds (and the others) byte-identically (git clean) |
| `just decisions-audit --changed` | advisory: DEC-013 governs the touched files; amendment D is consistent with B |

#### Independent differential (verifier's own, scratch crate, not committed)

All 14 `-none` seeds and the 10 shapes, 20,000 random mutations each (byte set, bit flip, increment, zero, 10 % truncation), every input under 19 policies (5 presets, `Custom(none)`, one per category). 264,579 `Ok`, 155,421 typed refusals, **0 violations** of: same length; every strip/tile/free/JPEG range of the **input's** walk byte-identical (IFD1's 0x0201 range skipped when Thumbnail is removed); no finding of a removed category in the output audit. No `VerifyFailed` and no untyped error. Refusals: BlockNotClean 89,187; PayloadUnreadable 31,080; SharedBytes 18,119; ThumbnailNotLocated 10,790; NestedIfd 852; HiddenPointer 17; NotJpeg 4,872 and BigTIFF 504 (mutated magic). Not covered: real-world DNG/TIFF files, BigTIFF, MakerNote-internal absolute offsets.

#### Findings

**High:** none.

**M1 (Medium) — the ExifTool oracle cannot see the XMP, IPTC or ICC carriers.**
`tests/support/tiff_records.rs:131-142`: `names()` returns `&[]` for `Xmp`, `Iptc` and `Icc`; `policies()` (lines 29-40) records no policy that removes only one of them. The fixture carriers are canary text, not valid structures, so ExifTool parses none of them. It prints only warnings (`[minor] Empty XMP`, `Bad IPTC data tag`, `Bad length ICC_Profile`), and `check()` ignores warnings. Reproducer: add `XMP-dc	Creator	CANARY-LEAK`, `IPTC	By-line	CANARY-LEAK` and `ICC_Profile	ProfileDescription	CANARY-LEAK` under `## everything` in `tests/oracle-fixtures/exiftool-strip-tiff/stripped-le.txt`, then run `cargo test --test strip_tiff_oracle recorded`. Result: `ok. 1 passed`. A planted SubIFD `Artist` line, by contrast, is caught (`SubIFD:Artist survives Authorship`). The removal itself is correct: the canary byte-search in `tests/strip_tiff.rs:106-113` proves it. What is missing is the ExifTool evidence that spec item 5 asks for. Fix: give the fixtures a minimal valid XMP packet, IPTC record and ICC header; check them by group (`XMP-*`, `IPTC`, `Photoshop`, `ICC_*`) and treat their warnings as findings; record `Custom(Xmp)`, `Custom(Iptc)` and `Custom(Icc)`.

**M2 (Medium) — `Custom(Jumbf)` reports success on a TIFF that holds a JUMBF entry but does not remove it.**
`src/write/tiff.rs:59-61` returns `Planned::empty()` when `mode(spec) == Untouched` (`src/write/exif.rs:796-803`: Jumbf is not in `can_occur`). That skips the `EXIFTOOL_BLOB_CARRIER_TAGS` guard in `plan_block` (`exif.rs:359-365`), which exists to refuse a policy when "an entry of a removed category could sit behind it". Reproducer: IFD0 with strips plus `0xCD41` UNDEFINED `"....jumbPROBE-C2PA-MANIFEST..."`. The audit has 0 findings. `strip(Custom{Jumbf})` returns `Ok` with the output byte-identical to the input, so the C2PA/JUMBF blob is kept. `Everything` refuses this file (`HiddenPointer`, offset 34). The same path exists for JPEG since SPEC-021 (an Exif 0xCD41 under `Custom(Jumbf)`), but in a bare TIFF that tag is the only place JUMBF can be. Fix: run the hidden-pointer and blob-carrier guard, or refuse, whenever the spec names `Jumbf` and the walk holds 0xCD41 (or name it in DEC-013 D.5 as a stated limit).

**M3 (Medium, follow-up, not this spec's code) — DNG identity tags outside DEC-008 survive every preset.**
DNG IFD0 with `UniqueCameraModel` 0xC614, `LocalizedCameraModel` 0xC615, `OriginalRawFileName` 0xC68B and `DNGPrivateData` 0xC634 (Adobe `MakN`, a copy of the original MakerNote, serials included). Under `GpsAndDevice`, `AllPii` and `Everything`, all four are kept in both byte orders. ExifTool on the `Everything` output prints `IFD0:UniqueCameraModel`, `IFD0:LocalizedCameraModel` and `IFD0:OriginalRawFileName`. The writer follows the audit, as the spec requires ("do not invent categories"), but now that DNG support exists, `Everything` and the MakerNote and CameraModel categories promise more than they deliver on real DNGs, which always carry 0xC614. Record a `guidance/questions.yaml` entry or a DEC-008 note, and add one sentence to DEC-013 D.5.

**M4 (Medium, usability; the decision itself is defensible) — multi-page TIFFs are refused by `AllPii`, `All` and `Everything`.**
Refusing is fail-closed and is documented in DEC-013 D.3, so it is not a defect for this spec. It does make the main privacy presets unusable on a common class of file (scans, faxes). The cause is upstream: the audit's IFD1-with-strips rule calls page 2's `StripOffsets` a `Thumbnail`, while ExifTool names it `IFD1:StripOffsets` (record `multipage-le.txt:13`), so even a planner that kept the page would fail self-check (a). Follow-up: in a bare TIFF, IFD1 with strips is a page, not a thumbnail (a DEC-008 note), after which page-keeping becomes possible.

**L1 (Low) — D.3 does not describe the refusals exactly.**
- A **tiled** IFD1 page (TileOffsets and TileByteCounts, no 0x0111, no 0x0201) gets **0** `Thumbnail` findings from the audit, but `AllPii` still refuses it with `ThumbnailNotLocated` at offset 50. `plan_block` refuses any IFD1 that is not a single JPEG pair (`exif.rs:415-431`). This over-refuses in a fail-closed direction and is not documented.
- An IFD1 page stored as old-style JPEG (Compression 6, 640x480, 0x0201/0x0202, no strips) is **zeroed** under `AllPii` as a thumbnail. That matches spec item 3 and ExifTool's naming, but "the writer never cuts a page" in D.3 is then not strictly true. Rewrite D.3 to say "never cuts a page held in strips or tiles".

**L2 (Low) — D.4 claims a header check that `verify_tiff` does not make.**
D.4 says (c) is "same length, same header, and ...", but `verify_tiff` (`src/write/verify.rs:149-198`) compares the length and the protected ranges, not bytes 0..8. A changed header is caught only indirectly, by Walk or Entries. The fuzz target does assert the header. Either add the 8-byte compare or reword D.4.

**L3 (Low) — `verify_tiff` and the fuzz oracle take the protected ranges from the output's walk only.**
See `verify.rs:171` and `rewrite_tiff.rs::protected`. Together with (g) Entries this is sound, because offset entries are never deletable and only GPS IFDs and the thumbnail IFD1 can leave the walk; my input-walk differential agrees. Comparing the input walk as well would make the check independent of what the plan did to the tables.

**L4 (Low) — naming and bookkeeping.**
- `strip` still returns `StripError::NotJpeg` for empty or unknown input (`src/write/mod.rs:358`), now that TIFF is accepted.
- The spec's `references.decisions` lists DEC-008, DEC-009 and DEC-010 but not DEC-013, which this spec amends (D).
- `hostile_inputs_*` (`tests/strip_tiff.rs:211-236`) runs only under `Gps`. The differential above covers the other policies.

#### Focus answers

1. **Image data, length:** no strip, tile, raw or JPEG data change, move or length change found, by code reading (`offset_targets` in `keep`, kept tables in `must_stay`, table rewrites checked against `keep`, any walk issue refused) or by test (the differential, 315k fuzz runs, hostile seeds: offset into an IFD table and strip over a GPS value give `SharedBytes`; SubIFD cycle and truncation give `BlockNotClean`). An offsets array that cannot be read is refused even with no edit.
2. **Leaks:** the audited categories are removed in IFD0, the Exif IFD, SubIFDs and chain pages. Out-of-line values are zeroed; a shared value is refused. Outside the audit's view: M2 and M3.
3. **Fail-closed and panic-free:** five-lint deny set clean, `-F unsafe_code` clean, no indexing or arithmetic in `tiff.rs` or the `verify.rs` additions; fuzz and differential found no panic.
4. **`verify_tiff`:** no bad output passed it in the differential; L2 and L3 are wording and independence points, not holes found.
5. **Page versus thumbnail:** defensible and documented (D.3); see M4 and L1.
6. **Oracle and wiring:** the comparator red-proof is genuine for the categories it names, and blind for XMP, IPTC and ICC (M1). CI fuzz matrix, `app.just`, `fuzz_layout`, `oracle-record` and the CI record diff are consistent. On the JPEG path, `jpeg.rs` only loses one `#[cfg(test)]`, `entries()` moved its GPS-pointer code into `gps_pointers` with the same logic, and the JPEG strip and oracle tests pass. `rewrite_jpeg` now skips TIFF input, which `rewrite_tiff` covers.

### Cost self-report

- **Tokens (total):** null. A subagent cannot see its own metered count; the orchestrator transcribes the harness `subagent_tokens`.
- **Estimated USD:** null
- **Duration (minutes):** null (harness figure to be transcribed)
- **Source of the number:** harness report (orchestrator)

### Drift and new artifacts

- **New decisions emitted:** none
- **Deviations from spec:** item 5's "against ExifTool" for the XMP/IPTC/ICC carriers is not shown (M1).
- **Follow-up work identified:**
  - DEC-008 note: IFD1 with strips in a bare TIFF is a page (M4).
  - DEC-008 gap: DNG identity tags 0xC614, 0xC615, 0xC68B and 0xC634 (M3).
  - The JUMBF Untouched-mode guard, if not fixed here (M2).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Item 5 says "follow what the audit/preset says"; it is not clear whether ExifTool parity for the IFD0 carriers was required. I read it as required.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — DEC-013 (amended by D) is missing from the spec's `references.decisions`.

3. **If you did this task again, what would you do differently?**
   — Start with the planted-record red-proof for every category, which found M1 in one step, before doing the structural fuzzing.

### Round 2

**Verdict: APPROVED.** Narrow re-verify of fix commit `d8679ec` (Follow-up 1 in HANDOFF-046). Toolchain 1.90.0, ExifTool 13.55. Every mutation below was restored, and `git status` was clean before this section was added.

**(a) M1 closed.** The XMP, IPTC and ICC carriers are now real structures (`tests/support/tiff_shapes.rs` `xmp_packet`/`iptc_record`/`icc_profile`), and the `## input` sections of `stripped-{le,be}.txt` show ExifTool 13.55 reading `XMP-dc Creator`, `IPTC By-line` and `ICC_Profile ProfileDescription` with no warning on those carriers. The only IRB warning comes from the unrelated fake 0x8649 blob. I planted each of the three canary lines one at a time under `## everything` of `stripped-le.txt`. Each made `recorded_exiftool_agrees_with_every_stripped_output` fail (for example "stripped-le under everything: XMP-dc:Creator survives Xmp"), and the file was restored. The live oracle test passes with `DIAMOND_EXIF_REQUIRE_ORACLE=1`. `git diff --stat d054731..d8679ec` touches only `exiftool-strip-tiff/` under `tests/oracle-fixtures/`; the other record directories and the JPEG seeds are unchanged.

**(b) M2 closed.** `a_custom_jumbf_policy_on_a_tiff_with_a_jumbf_entry_is_refused` passes: `Custom{Jumbf}` and `Everything` are both refused with `HiddenPointer`, and `Custom(none)` returns the input unchanged, in both byte orders. With the new `hidden_pointers` call at `src/write/tiff.rs:63` disabled, the test fails (`be=false: Ok(Stripped{..})`), so it would have caught the pre-fix behaviour. `hidden_pointers` (`src/write/exif.rs:348-375`) moves the old guard out unchanged, and `plan_block` still calls it first. The JPEG `Untouched` path is not touched. All JPEG tests pass and `fuzz-smoke rewrite_jpeg` is clean. The round 1 probe `bin/jumbf` now prints `HiddenPointer` at offset 34 for both policies.

**(c) L1/L2/L4.** I disabled the header check at `src/write/verify.rs:168`. `verify_tiff_accepts_the_identity_and_rejects_a_changed_header` then fails at header byte 0 with `Err(Walk)` where it expects `Err(Scan)`, so the test needs that check. DEC-013 D.3 matches `exif.rs:424-440`: a tiled IFD1 has no `thumbnail_range`, so the policy is refused with `ThumbnailNotLocated`, and the round 1 probe confirms this (`AllPii`, offset 50). D.4 states the header compare. D.5 states the TIFF-only guard and the M3/M4 limits. `guidance/questions.yaml` records `dng-identity-tags-have-no-category` (M3) and `tiff-ifd1-strips-is-a-page` (M4), both open.

**(d) Regression sweep.** fmt passes, clippy 1.90 all-targets `-D warnings` passes, and `-F unsafe_code` check all-targets passes. `cargo test` with `DIAMOND_EXIF_REQUIRE_ORACLE=1`: 909 passed, 0 failed, 10 ignored, no SKIP. `just validate` passes. `just fuzz-smoke rewrite_tiff` ran 366,800 runs and `rewrite_jpeg` ran 164,269 runs in 61 s each, with no crash. The round 1 differential probe ran `probe 5000`: mutations ok=66,694, refusals=38,306, bad=0. The DNG identity tags still survive every preset; this is M3, recorded and documented and not a blocker.

**Findings (Low only, none blocking):**
- L-r2-1 `tests/support/tiff_shapes.rs` `jumbf()`: the box header says length 24, but `"jumb" + "PROBE-C2PA-MANIFEST"` makes the box 27 bytes. The guard checks the tag, not the box, so the test still holds. If a later audit parses JUMBF boxes, this fixture would give a malformed-box result instead of a JUMBF finding.
- L-r2-2 `src/write/tiff.rs:62-68`: the new guard runs for every non-empty `Untouched` spec, not only `Jumbf`. For example, `Custom{EmbeddedImage}` on a TIFF with a second GPS pointer is now refused where it used to return Ok. This fails closed and matches D.5 ("a policy that removes anything"), but no test pins it, and no test pins the JPEG-unchanged claim (JPEG `Custom{Jumbf}` over an Exif 0xCD41 still returns Ok).
