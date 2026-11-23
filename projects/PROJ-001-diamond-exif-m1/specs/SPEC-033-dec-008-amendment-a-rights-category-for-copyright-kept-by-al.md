---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-033
  type: story                      # epic | story | task | bug | chore
  cycle: verify                    # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: M                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
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
  stage: STAGE-009
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-008, DEC-009, DEC-010, DEC-012, DEC-013, DEC-020, DEC-021, DEC-022]
  constraints: []                  # [constraint-id-1, constraint-id-2]
  related_specs: [SPEC-014, SPEC-021, SPEC-023, SPEC-027, SPEC-032, SPEC-037]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-038]             # WebP strip (PR #55) adds more EXIF9 copies; build after it ships

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-009's <capability>". Optional; null is acceptable.
value_link: "AllPii, the default privacy preset, stops removing the copyright notice an owner attached, while still removing every name that identifies a person."

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
  tokens_estimate: 500000
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-033: DEC-008 amendment: a `Rights` category for Copyright, kept by `AllPii`

## Context

STAGE-009, first and only spec. DEC-008 files the Exif Copyright tag (0x8298) under `Authorship`, beside Artist,
and DEC-009's `AllPii` removes `Authorship`. So the default privacy strip (and the default `--policy` of
`exif-audit`) removes a copyright notice. The owner decided on 2026-11-20 that it must not: a copyright notice is a
statement the owner attached on purpose, not something the camera leaked, and removing it by default works against
the person stripping their own photos (17 U.S.C. §1202 is about knowingly removing such notices).

A policy is a set of categories (DEC-009), so Copyright can only be kept apart from Artist in a category of its
own. The decisions are already written as amendments dated 2026-11-21 in **DEC-008** (the `Rights` category and
the moved row), **DEC-009** (the preset row, the departure from design section 5, the may-hold rows) and
**DEC-013** (EXIF9 becomes EXIF10). Read those three amendments first. They are normative; this spec turns them
into code and tests.

**Revision (2026-11-22): PNG and WebP.** This spec was designed on 368c9f1. Since then STAGE-008 and STAGE-010 added
PNG and WebP audit (SPEC-032, DEC-020), the PNG strip (SPEC-037, DEC-021, merged at fd94d4f) and the WebP strip
(SPEC-038, DEC-022, PR #55, merged at 9133727). They change this spec in three ways:

- **A second Copyright source.** A PNG text chunk keyed `Copyright` (or `copyright`, by the measured first-letter
  rule) is `Authorship` through the `TEXT_KEYWORDS` table in `src/audit/category.rs`, a row the SPEC-032 build
  left for this spec to re-home. It moves to `Rights` too (DEC-020 and DEC-021 amendments, 2026-11-22). WebP has
  no keyword table: its `LIST/INFO` Copyright is inside an opaque carrier, removed whole by every preset.
- **More copies of the Exif set.** `src/write/png.rs` calls `exif9()` (carrier rule and whole-removal category),
  `src/write/webp.rs` does the same (and reuses `png::removes_carriers`), `verify.rs` has a WebP `exif_set()` user, and hand-typed
  `EXIF9` copies in `fuzz/shared/rewrite_png_invariants.rs`, `fuzz/shared/rewrite_webp_invariants.rs`,
  `tests/strip_png.rs` and `tests/strip_webp.rs`. Every one becomes the ten-category set.
- **Ordering.** `depends_on: [SPEC-038]`, now shipped: the build branches from current `main`, which holds every
  WebP site. The builder greps for `exif9`, `EXIF9`, `exif_set`, `all9` and `Authorship` over
  `src`, `fuzz/shared` and `tests` first and treats every hit as in scope; the lists below are the known sites,
  not a cap.

Governing code, verified on `origin/main` at 368c9f1 and re-checked at fd94d4f (line numbers unchanged):

- `src/audit/category.rs`: `Category` (15 variants, `#[non_exhaustive]`), `Category::ALL: [Category; 15]`,
  `severity`, `is_block`, `name`, `categorize(kind, tag)` (line 158) and its private `by_id(tag)` (line 169; line
  189 holds `0x8298` in the `Authorship` arm), and the hand-typed
  test tables (rows at about lines 238, 376, 419, 455, 502).
- `src/policy.rs`: `bit` (a literal per category, `u16`, `0x4000` is `OtherSegment`), `StripPolicy::spec`
  (`All` lists its categories by hand), `may_hold` (`TAGS_AND_SOFTWARE`, the `Iptc` arm, the tag-category arm),
  and the hand-typed `REMOVED` grid and may-hold grid in its tests.
- `src/write/exif.rs`: `exif9()` (line 781) and `mode` (line 799). `src/write/jpeg.rs` line 191 uses
  `exif9()`. `src/write/verify.rs` line 50 holds its own hand-typed nine-category list for `exif_gone`.
- `src/audit/preview.rs` tests (line 272).
- PNG and WebP (verified at 9133727): `src/audit/category.rs` `TEXT_KEYWORDS` (the `Copyright` row) and
  its unit test `text_keywords_match_the_measured_rule_row_by_row` (`Copyright`, `copyright` rows);
  `src/write/png.rs` (`removes_carriers` line 59, the `Mode::Whole` arm line 328); `src/write/webp.rs` (`exif9()` line 225, `all9` line 239;
  carriers through `png::removes_carriers`); `src/write/verify.rs` (`exif_set()` line 381 and its PNG/WebP callers). Models and tests:
  `fuzz/shared/audit_invariants.rs` (keyword model line 513), `fuzz/shared/rewrite_png_invariants.rs` (`EXIF9`
  line 58, keyword table line 494), `fuzz/shared/rewrite_webp_invariants.rs` (`EXIF9`), `tests/strip_png.rs`
  (`EXIF9` line 61, the `copyright` text part line 222), `tests/strip_webp.rs` (`EXIF9`),
  `tests/strip_png_live.rs` (`AUTHORSHIP_TAGS` line 55, which pins `Copyright` as removed by `AllPii`),
  `tests/strip_webp_live.rs` (its live judge maps ExifTool's `Copyright` to `Authorship`, line 253, and its doc
  comment line 27 says so), `tests/audit_png_webp.rs` (line 157),
  `tests/audit_png_webp_oracle.rs` (`by_hand` line 554 and its assertion line 698).
- Fuzz models, each an independent hand-typed copy: `fuzz/shared/audit_invariants.rs` (tag table line 232,
  severity line 277, category list line 443, may-hold rows from line 967), `fuzz/shared/render_invariants.rs`
  (name table line 57), `fuzz/shared/rewrite_invariants.rs` (`EXIF9` line 251, categorize line 1038).
- Tests that pin categories: `tests/policy_preview.rs` (grid line 24), `tests/report_formats.rs` (name lists
  lines 186 and 1134), `tests/fuzz_layout.rs` (lines 2827, 2874), `tests/support/strip_shapes.rs` (`EXIF9` line
  1266 and the expectation model), `tests/support/strip_records.rs` (`policies()`), `tests/strip.rs` (the
  `only(&["authorship"])` tests from line 1990 and the table from line 2314).
- ExifTool strip records: `tests/oracle-fixtures/exiftool-strip/*.txt`. The `exif-full-le/be`, `kitchen-sink`,
  `exif-makernote-canon-style` and `exif-at-the-ceiling` fixtures carry `IFD0 Copyright CANARY-COPYRIGHT-…`.
  Today the `## all_pii` and `## authorship` sections have no Copyright line. After this spec they must have one.
  Regenerated with `just oracle-record` (`cargo run --example oracle_record`, needs ExifTool 13.55).

## Goal

`AllPii` (and any set without `Rights`) keeps the Exif Copyright entry byte-identical, and ExifTool agrees.
`All` and `Everything` still remove it, with `All` still removing every Exif segment whole. The same holds for
a JPEG, a TIFF, a PNG (`eXIf` entry and `Copyright` text chunk) and a WebP (`EXIF` entry). Every report names
the Copyright finding `rights`, and no audit's severity changes.

## Decisions made here

Everything normative is in the three DEC amendments. The implementation choices are:

1. `Category::Rights`, doc comment "A copyright or rights notice the owner attached." It is appended after
   `OtherSegment`, so `ALL: [Category; 16]` and `bit(Rights) = 0x8000`. Its `name` is `"rights"`, its severity
   is `Medium`, and `is_block` is false. `by_id`: `0x8298 => Some(Category::Rights)`, and 0x8298 is removed
   from the `Authorship` arm. Fix the `Authorship` doc comment to "Names a person."
2. `StripPolicy::All` lists `Category::Rights`. The `AllPii` doc comment adds "and the copyright notice" to what
   it keeps. `may_hold`: rename `TAGS_AND_SOFTWARE` to `TAG_CATEGORIES` (8 entries, `Rights` added), add
   `Rights` to the `Iptc` arm, and add `Category::Rights` to the tag-category arm (holds nothing).
   `EmbeddedImage` and `OtherSegment` pick it up through `StripSpec::all()`.
3. Rename `exif9` to `exif10` and add `Rights`. Do the same in `jpeg.rs` (rename the local `all9` too),
   `verify.rs` (add `Category::Rights` to its own list and keep that list hand-typed, because it is the
   independent self-check) and the doc comments that say "nine" or "EXIF9". Rename `EXIF9` to `EXIF10` in
   `rewrite_invariants.rs` and `strip_shapes.rs` the same way.
4. `StripSpec` stays `u16`; it is now full. Add one comment at `bit` saying that a 17th category needs a wider
   integer. Do not widen it in this spec.
5. `strip_records::policies()` gains `("rights", Custom([Rights]))`. The existing `all_pii` and `authorship`
   sections are re-recorded, not hand-edited.
6. `TEXT_KEYWORDS`: the `Copyright` row becomes `(b"Copyright", Category::Rights)`; the match rule is unchanged.
   Update the SPEC-032 comment above the table (it says SPEC-033 may re-home the row) to say it did.
7. `png.rs` and `webp.rs` call `exif10()`; rename their local `all9`. `removes_carriers` therefore also fires on a
   `Custom` set holding only `Rights`: a carrier may hold a Copyright (DEC-021 amendment). No preset changes,
   because every preset already holds a tag category.
8. Under `Mode::Whole` (only `All`, `Everything` and supersets), an Exif block whose first Exif-set finding is
   Copyright is now recorded as removed under `Rights`, not `Authorship`, on every container. Models that compute
   that recorded category take the ten-category set.

## Inputs

- The three DEC amendments (bottom of `decisions/DEC-008-…`, `DEC-009-…`, `DEC-013-…`).
- The code and records listed in Context.
- `guidance/toolchain-brief.md` (pinned toolchain, `RUSTUP_TOOLCHAIN=1.90.0` for `just lint` when
  `~/.cargo/bin` is first on PATH; the nightly invocation for fuzz smokes).

## Outputs

- **Modified (src):** `src/audit/category.rs`, `src/policy.rs`, `src/write/exif.rs`, `src/write/jpeg.rs`,
  `src/write/png.rs`, `src/write/webp.rs`, `src/write/verify.rs`, `src/audit/preview.rs` (tests only).
- **Modified (fuzz):** every `fuzz/shared/*_invariants.rs` model listed in Context (audit, render, rewrite,
  rewrite_png, rewrite_webp).
- **Modified (tests):** the files listed in Context, plus the new tests below.
- **Seeds:** one audit seed `audit-copyright-only-le` (a TIFF with IFD0 holding only 0x8298 and the structural
  minimum), one `rewrite_jpeg` seed with Artist and Copyright in IFD0, and one `rewrite_png` seed with an `eXIf`
  holding Artist and Copyright plus a `tEXt` keyed `Copyright`, all through `examples/fuzz_seeds.rs`.
  Update every seed-count pin the new seeds move, and name the moved pins in the handback.
- **Records:** re-recorded `tests/oracle-fixtures/exiftool-strip/*.txt`. The diff must touch only the
  `## all_pii` and `## authorship` sections (a Copyright line appears) and add the new `## rights` sections.
  Any other change in the diff is a finding to report, not something to commit.
- **Docs:** `CHANGELOG.md` `[Unreleased]` gets a `### Changed` entry. It says that for a 0.1.0-alpha.1 user, the
  Copyright finding is now category `rights` in text, JSON and CSV; that the `all-pii` preview now shows it as
  kept (for a PNG `Copyright` text chunk too); and that the library's `AllPii` strip keeps it. `README.md`: the
  "Using exif-audit" paragraph adds "copyright notices" to the list of what the scan reports, and the PNG
  paragraph (line 19) says `Artist` and `Author` are authorship and `Copyright` is rights.
- **No new dependency, no new public function.** `Category::Rights` is the only new public item (the enum is
  `#[non_exhaustive]`, so this is not a breaking change).

## Acceptance Criteria

- [ ] `categorize(kind, 0x8298)` is `Some(Rights)` for `IfdKind::Ifd0`, `Sub` and `Exif`, and the audit reports it
      so in both byte orders. 0x013B,
      0xA430, 0x9C9D, 0xA437 and 0xA438 are still `Authorship`. `Rights.severity()` is `Medium`,
      `Rights.name()` is `"rights"` and `Rights.is_block()` is false.
- [ ] The DEC-009 grid holds cell by cell for 16 rows. `Rights` is removed by `All` and `Everything` only. The
      nesting property still holds.
- [ ] The may-hold grid holds cell by cell. `Rights` is in `Thumbnail`, `Xmp`, `Iptc`, `Jumbf`,
      `EmbeddedImage` and `OtherSegment`, and not in `MakerNote` or `Icc`.
- [ ] No preview disposition changes for any committed audit seed or strip fixture under any of the five
      presets, except that a Copyright finding is `Kept` under `AllPii` (it was `Removed`).
- [ ] No audit's `max_severity()` changes on any committed seed or fixture.
- [ ] `strip(exif-full-le/be, AllPii)` returns bytes in which the 0x8298 entry and its value are byte-identical
      to the input, 0x013B is deleted and its value zeroed, and the self-check passes. The same holds for
      `Custom([Authorship])`.
- [ ] `strip(_, Custom([Rights]))` deletes 0x8298 and nothing else. It is partial, so the segment is kept.
- [ ] `strip(_, All)` and `strip(_, Everything)` still remove every Exif segment whole (`mode` is `Whole`).
      `Custom(old nine)` is now `Partial` and keeps Copyright.
- [ ] Artist and Copyright pointing at the same out-of-line bytes under `AllPii` is refused as `SharedBytes`.
      Under `All` the segment is still removed whole.
- [ ] ExifTool on the `all_pii` and `authorship` outputs prints `IFD0:Copyright` with the canary value, and
      no `IFD0:Artist`. On `all`, `everything` and `rights` it prints no Copyright.
- [ ] JSON, CSV and text reports of a Copyright-only file name the category `rights`.
- [ ] PNG: `text_keyword_category(b"Copyright")` and `(b"copyright")` are `Rights`; `COPYRIGHT` stays `FreeText`;
      `Artist`, `Author` stay `Authorship`. `strip(png, AllPii)` keeps a `Copyright` text chunk byte-identical
      and removes an `Author` one; `All` removes both. An `eXIf` holding Artist and Copyright under `AllPii` keeps
      Copyright (entry deleted in place for Artist, CRC patched) and the self-check passes.
- [ ] WebP: an `EXIF` chunk holding Artist and Copyright under `AllPii` keeps Copyright and the self-check
      passes; `All` removes the chunk whole. A `LIST/INFO` carrier is still removed by every preset.
- [ ] `strip_png_live` moves `Copyright` out of the `AllPii` list into an `All`/`Everything` list, and
      `strip_webp_live`'s judge maps `Copyright` to `Rights`; run live with `DIAMOND_EXIF_REQUIRE_ORACLE=1` and paste the summary.
- [ ] TIFF writer: `strip(tiff with Artist and Copyright in IFD0, AllPii)` keeps Copyright and deletes Artist.
- [ ] Every existing test passes. Any test that asserted Copyright under `Authorship`, or removed under
      `AllPii`, is changed in this spec and named in the handback.
- [ ] Fuzz smokes (60 s each) clean on **all six** targets whose harness models categories, keywords or EXIF9:
      `audit`, `audit_render`, `rewrite_jpeg`, `rewrite_tiff`, `rewrite_png`, `rewrite_webp` (the SPEC-023
      lesson: run every smoke whose model the change touches, not only one).

## Failing Tests

Written first and red before the code changes. Each row of a hand-typed table is typed by hand, never derived
from the code under test.

- **`src/audit/category.rs` unit tests**
  - `"copyright is rights and artist stays authorship"`: asserts the first AC row through `categorize` over
    `IfdKind::Ifd0`, `Sub` and `Exif`.
  - Update the hand-typed tables: 0x8298 row to `Rights`, `(Rights, Medium)` in the severity table, `Rights` in
    the ALL-order list (16).
- **`src/policy.rs` unit tests**
  - `REMOVED` grid becomes `[(Category, [bool; 5]); 16]` with `(Rights, [false, false, false, true, true])`.
  - The may-hold grid gains the `Rights` column per the DEC-009 amendment.
  - `"rights bit is distinct and the set is full"`: asserts the 16 bits are distinct and
    `StripSpec::all()` has `len() == 16`.
- **`tests/policy_preview.rs`**
  - Grid row for `Rights`. Add `"all_pii keeps a copyright finding"`: a file with Artist and Copyright gives
    `Removed` for Artist and `Kept` for Copyright under `AllPii`, and `Removed` for both under `All`.
- **`tests/strip.rs`**
  - `"all_pii keeps copyright byte identical"` (both byte orders): the AC row, including value-byte identity.
  - `"custom rights removes only copyright"`.
  - `"old exif9 custom set is now partial and keeps copyright"`.
  - `"artist and copyright sharing bytes is refused under all_pii"`: asserts `SharedBytes`. Under `All`, the
    same file is removed whole.
  - Update the `only(&["authorship"])` tests and the table from line 2314 so they expect Copyright kept.
- **`tests/strip_tiff.rs`**
  - `"tiff all_pii keeps copyright and deletes artist"`.
- **`tests/strip_oracle.rs`** (replay and live)
  - `exiftool_tag_sets_equal_the_declared_tables_per_policy_and_fixture` (line 384): update the hand-typed
    declared tables so `all_pii` and `authorship` declare Copyright kept, and add the `rights` policy, which
    declares it deleted. This test is what makes "ExifTool agrees" true.
  - **Red-proof** with the existing `red_proof` helper (line 529), next to the comparators at lines 552 to 612:
    `"comparator is red when copyright is removed under all_pii"` (a record with the Copyright line dropped
    from the `all_pii` section must fail, naming `IFD0:Copyright`).
  - **Library red-proof (verify, not committed):** the verifier reverts the `by_id` row (0x8298 back to
    `Authorship`) in a scratch copy and confirms the replay oracle test and the `tests/strip.rs` AllPii test both
    go red. Record the result in the verify handback.
- **`tests/report_formats.rs`**
  - Add `"rights"` to both category-name lists. Add `"copyright finding renders as rights"` across the three
    formats.
- **PNG and WebP tests**
  - `src/audit/category.rs` `text_keywords_match_the_measured_rule_row_by_row`: `Copyright`, `copyright` rows to
    `Rights`.
  - `tests/strip_png.rs`: the `copyright` text part's role to `Rights`; `EXIF10`; add `"png all_pii keeps the
    copyright text chunk and the exif copyright"`.
  - `tests/strip_webp.rs`: `EXIF10`; add `"webp all_pii keeps the exif copyright"`.
  - `tests/audit_png_webp.rs` line 157 and `tests/audit_png_webp_oracle.rs` `by_hand`: `Copyright` to `Rights`
    (the oracle test then shows ExifTool's printed `Copyright` lands in `rights`).
  - `tests/strip_png_live.rs`: split `AUTHORSHIP_TAGS`; its red check (`the_check_goes_red_on_an_unstripped_file`)
    must still require violations for every file.
- **`tests/fuzz_layout.rs`**
  - Update the category rows. Add the two new seeds to whatever seed-set assertions exist.

## Non-Goals

- A CLI option to choose categories (`--keep`, `--policy custom:…`). If it is wanted, it gets its own spec and a
  DEC-011 amendment.
- Editing XMP `dc:rights` or the IPTC CopyrightNotice. They follow their packets.
- Moving any other tag. Photographer (0xA437) and ImageEditor (0xA438) name people and stay `Authorship`.
- Changing any severity or the `--fail-on` default.
- Widening `StripSpec` past `u16`.
- Designing STAGE-007's SPEC-027. It will read the amended DEC-009 when it is designed.

## Notes for the Implementer

- Audit first, policy second, writer third. The writer must keep Copyright **because the audit no longer puts it
  in a category the policy holds**, through the existing entry-deletion path. Do not special-case 0x8298 in the
  writer. If `plan_block` refuses something unexpected on the fixtures under `AllPii`, report it in the handback;
  do not work around it.
- The fuzz models and `verify.rs`'s list are deliberately independent copies. Update each by hand from the
  DEC amendments, not by calling the library's function.
- Zero panics on untrusted input. Keep the five-lint deny set and `forbid(unsafe_code)`. Use `<<`-free bit
  literals (the lint set forbids `<<` on integers, which is why `bit` is a literal table).
- Re-recording needs ExifTool 13.55 on PATH. If it is missing, stop and say so in the handback. Never hand-edit
  a record.
- Branch from current `main` (PR #55 is merged), then run the grep in Context and list every
  hit in the handback with what you did to it.
- Budget: one builder, sonnet, about 120 tool calls. Cold verify on opus, one round; a second round only for
  High or Medium findings.

---

## Reflection

*Appended during **ship**. Three questions, short answers.*

1. **What would I do differently next time?**
   — <answer>

2. **Does any template, constraint, or decision need updating?**
   — <answer>

3. **Is there a follow-up spec I should write now before I forget?**
   — <answer>

4. **Where was the worst defect caught?** — one of `design` | `build` | `verify` | `ship` | `escaped` | `none`.
   — <answer>

5. **What can a user do now that they couldn't before?** — before → after, one sentence.
   — <answer>
