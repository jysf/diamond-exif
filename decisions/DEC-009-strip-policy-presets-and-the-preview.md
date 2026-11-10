---
insight:
  id: DEC-009
  type: decision
  confidence: 0.7
  audience:
    - developer
    - agent

agent:
  id: claude-opus-5-5
  session_id: null

project:
  id: PROJ-001
repo:
  id: diamond-exif

created_at: 2026-11-06
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope:
  - "src/policy.rs"
  - "src/audit/**"
  - "tests/policy*.rs"

tags:
  - policy
  - strip
  - audit
  - preview
---

# DEC-009: What each strip preset removes, and what the preview promises

## Decision

A strip policy is **a set of DEC-008 categories**. Nothing else: no tag lists, no ids. The five presets are nested sets of categories, and a custom policy is any set. The preview says, for every finding of an audit, what the policy does to it. The preset contents are code (`src/policy.rs`) and this DEC is their normative statement; the code is tested against it cell by cell.

**The presets** (a cell is "removed"; blank is "kept"):

| Category | `Gps` | `GpsAndDevice` | `AllPii` | `All` | `Everything` |
|---|---|---|---|---|---|
| `Gps` | removed | removed | removed | removed | removed |
| `DeviceId` | | removed | removed | removed | removed |
| `Authorship` | | | removed | removed | removed |
| `FreeText` | | | removed | removed | removed |
| `Timestamps` | | | removed | removed | removed |
| `CameraModel` | | | removed | removed | removed |
| `Thumbnail` | | | removed | removed | removed |
| `MakerNote` | | | removed | removed | removed |
| `Software` | | | | removed | removed |
| `Xmp` | | | | removed | removed |
| `Iptc` | | | | removed | removed |
| `Icc` | | | | | removed |
| `Jumbf` | | | | | removed |

Each preset contains the one before it (`Gps` ⊂ `GpsAndDevice` ⊂ `AllPii` ⊂ `All` ⊂ `Everything`), and `Everything` is every category. Three choices in the table are decisions, not transcription of design section 5:

1. **`AllPii` removes `Thumbnail` and `MakerNote`.** Design section 5 lists tags (Artist, Copyright, UserComment, DateTimeOriginal, Make and Model, GPS, device ids) and says "everything a privacy auditor would flag". DEC-008 makes an embedded thumbnail and a vendor block Medium findings for the reason that each can keep what the main image lost (a thumbnail of the uncropped picture; serials inside a MakerNote). A preset named for personal data that leaves both would defeat itself. They sit inside the Exif structure the writer rewrites anyway, so removing them whole is a deletion of one IFD entry or one IFD, not a new kind of edit.
2. **`AllPii` keeps `Xmp` and `Iptc`; `All` removes them.** They are separate segments, and design section 7 edits XMP by locating known properties, not by dropping the packet. Dropping a whole packet is `All`'s meaning (design section 5: "All EXIF/XMP/IPTC"). Until the writer can edit inside a packet, `AllPii` leaves it and the preview says so (below).
3. **`All` keeps `Icc` and `Jumbf`** (design section 5: ICC profiles and C2PA manifests are preserved by requirement); `Everything` is the explicit opt-in that removes them. `Software` is removed by `All` and not by `AllPii`, matching DEC-008's reasoning that it fingerprints a toolchain but identifies no person.

`Gps` and `GpsAndDevice` keep the thumbnail and the MakerNote even though either can hold GPS or serials. That is the design's meaning ("GPS block only", "GPS + device identification"), and the preview reports the consequence instead of the policy silently widening (next section).

**Dispositions.** For each finding the preview gives exactly one of:

- `Removed`: the finding's category is in the policy's set.
- `Kept`: it is not, and nothing the policy targets can hide inside it.
- `KeptUnread`: it is not removed, the finding is a **block** category (not decoded: `Thumbnail`, `MakerNote`, `Xmp`, `Iptc`, `Jumbf`, `Icc`), and the block **may hold** a category the policy does remove. This is the honest-limit finding: the library cannot say the block is free of what the policy was chosen to remove.

The **may-hold relation** (data, in `src/policy.rs`; a block category and the categories it can contain):

| Block | May hold |
|---|---|
| `Thumbnail` | `Gps`, `DeviceId`, `Authorship`, `FreeText`, `Timestamps`, `CameraModel`, `Software` (a thumbnail JPEG may carry its own Exif) |
| `MakerNote` | `Gps`, `DeviceId`, `CameraModel`, `Timestamps`, `Software` |
| `Xmp` | `Gps`, `DeviceId`, `Authorship`, `FreeText`, `Timestamps`, `CameraModel`, `Software` |
| `Iptc` | `Authorship`, `FreeText`, `Timestamps` |
| `Jumbf` | `Gps`, `DeviceId`, `Authorship`, `FreeText`, `Timestamps`, `CameraModel`, `Software` (a C2PA manifest carries assertions, including Exif ones) |
| `Icc` | nothing (DEC-008: "a fingerprint at most"; keeping it under `All` is the stated requirement) |

A non-block category (the tag categories) is never `KeptUnread`: a kept tag is simply `Kept`.

**Preview facts.** Beyond the per-finding dispositions the preview exposes the three counts, `residual_findings` (= kept + kept-unread), the highest severity among the residual findings, and `reliable`: whether the underlying audit was clean and its findings complete. `residual_findings == 0` supports "nothing the audit found would remain" **only when `reliable` is true**; the preview, like the audit, never turns an unreadable, unsupported or truncated file into a clean one. When the audit's findings were truncated (DEC-008 bounds), the residual severity is the audit's own maximum severity (the dropped findings' categories are unknown, so nothing is assumed removed).

**What the preview does not say.** It does not cover inventory-only entries (tags with no category): whether `All` or `Everything` removes them depends on the container (an Exif segment of a JPEG goes whole; structural tags of a bare TIFF must stay) and is the writer's decision, recorded in the writer's spec. It does not say how a block is removed (whole packet, or properties inside it). It writes nothing.

## Context

The strip policies and every report depend on one statement of what a preset means (DEC-008's Context says why a single source matters). The audit already classifies every finding into a category. If the preset contents live as categories, the preview is a lookup, the writer (M1c/M1d) takes the same sets, and nothing can drift: adding a category is a DEC amendment that must place it in every column. Writing the contents down before any writer exists is the same discipline as DEC-001 and DEC-008.

The can't-parse blocks are the hard part of the policy question. A preview that said "GpsAndDevice removes the device identity" for a file whose MakerNote holds a serial would be false, and a CI job trusting it would pass a leaking file. `KeptUnread` makes that case a visible, countable outcome instead of a silent one.

## Amendment (2026-11-06, from the SPEC-014 verify)

Three clarifications; no table cell and no code changes.

1. **`Icc` is kept by decision, not because it is empty.** An ICC profile header carries a creation date, device manufacturer and model, a profile creator and free-text tags, so "nothing the policy targets can hide inside it" overstates. `may_hold(Icc)` stays empty because `All` keeps ICC by stated requirement (design section 5) and `Everything` is the explicit opt-in that removes it; marking it `KeptUnread` under every preset would flag every file with a colour profile and make the signal useless. A renderer must therefore describe `Kept` for a block as "not targeted by this policy", never as "contains none of it".
2. **Read `reliable` first.** When only the inventory cap is hit, the findings are complete but the preview still reports the conservative residual severity (rule 4), so `AllPii` can show `residual_findings == 0` beside a `High` residual severity. This is intended: a caller (SPEC-015, SPEC-016) checks `reliable()` before reading either number.
3. **Known gaps in the may-hold table, for `Custom` policies only.** No preset's outcome changes, because the missing pairs fall in cells where the block is already removed or already `KeptUnread`: a MakerNote can hold `Authorship` (vendor owner names); IPTC can hold `Software` (OriginatingProgram) and textual locations; a C2PA manifest can carry a thumbnail and XMP; a thumbnail can carry XMP or ICC. They are recorded as question `strip-may-hold-gaps` and amended with the first writer spec or the first user report.

## Alternatives considered

- **Presets as tag-id lists.** Rejected: it duplicates DEC-008 in a second place; the first drift would make a report promise something a strip does not do.
- **Widen `Gps` and `GpsAndDevice` to remove the thumbnail, the MakerNote, XMP and JUMBF whenever they might hold the target.** Rejected as a default: it turns "GPS only" into "most of the metadata", contradicts design section 5, and removes C2PA manifests under a GPS preset. The user who wants that has `AllPii`/`All` or a `Custom` set; the preview tells the others what was left.
- **Keep `Thumbnail` and `MakerNote` out of `AllPii` too (pure design-section-5 tag list).** Rejected: see choice 1. Revisit if a fixture or a real file shows a user expecting the thumbnail to survive `AllPii`.
- **Two-state dispositions (removed / kept).** Rejected: it cannot express the unparsed-block limit, which is the main honest-limits requirement of STAGE-003.
- **Preview over inventory items (every tag) instead of findings.** Rejected for M1: the unclassified entries have no category, so the policy as a category set cannot speak about them; see "What the preview does not say".

## Consequences

- **Positive:** presets, preview and the future writer share one category vocabulary; the nesting is a testable property; the unparsed-block limit is visible and countable (a CI exit-code rule can use `KeptUnread`).
- **Negative:** `KeptUnread` is conservative by construction (a vendor block that holds nothing still shows as kept-unread under `Gps`), and a user who wants a GPS-only strip that is also thumbnail-safe must choose `Custom`. `Everything` and `All` cannot be previewed for unclassified tags until the writer exists.
- **Neutral:** `StripPolicy` and `Category` are `#[non_exhaustive]`; a new category requires a DEC-008 row, a DEC-009 column and one test cell.

## Validation

Revisit when the writer's first spec shows a preset cannot be implemented as stated (for example an XMP edit that cannot remove properties without dropping the packet), when thumbnail or MakerNote parsing lets `KeptUnread` become `Kept` or `Removed` for part of a block, or when a user's file shows a preset leaving something the table says it removes.

## References

- Related specs: SPEC-012 (the category table), SPEC-014 (implements), SPEC-015 (renders dispositions), SPEC-016 (exit codes), M1c/M1d (writer)
- Related decisions: DEC-008 (categories, severity), DEC-006 (regions)
- External: design `docs/exif-core-design.md` sections 5 and 7

## Amendment (2026-11-09, DEC-012)

The grid gains two rows, `EmbeddedImage` and `OtherSegment`, removed only by `Everything` (so `KeptUnread` under the other four presets); `may_hold` gains the same two rows (every category except itself) and `MakerNote` may hold `EmbeddedImage`. All other cells are unchanged. See DEC-012.
