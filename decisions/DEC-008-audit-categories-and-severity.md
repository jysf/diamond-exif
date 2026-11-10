---
insight:
  id: DEC-008
  type: decision
  confidence: 0.75
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

created_at: 2026-11-05
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope:
  - "src/audit/**"
  - "tests/audit*.rs"

tags:
  - audit
  - pii
  - policy
  - classification
---

# DEC-008: What the audit calls personal data — categories, severity, and the tag table

## Decision

The audit reports **findings**, each in exactly one **category** with a fixed **severity**. Findings come from three places and nowhere else: a tag in the table below (in any non-GPS, non-Interop IFD), any tag in a GPS IFD, and a located metadata region. Everything else the reader sees is **inventory only** (listed, never a finding). The table is code (`src/audit/category.rs`); this DEC is its normative statement and the code is tested against it row by row.

**Categories and severity**

| Category | Severity | What it means |
|---|---|---|
| `Gps` | High | where the picture was taken: every tag in a GPS IFD (IfdKind::Gps), whatever its id |
| `DeviceId` | High | identifies one physical device: serial numbers, `ImageUniqueID` |
| `Authorship` | Medium | names a person or rights holder |
| `FreeText` | Medium | free text the author typed (can hold anything) |
| `Thumbnail` | Medium | an embedded thumbnail, which can keep metadata the main image lost |
| `MakerNote` | Medium | vendor block, **present but not parsed** (serials may be inside) |
| `Xmp` | Medium | an XMP packet or extended-XMP part (not decoded) |
| `Iptc` | Medium | an IPTC / Photoshop-resource block (not decoded) |
| `Jumbf` | Medium | a JUMBF box (C2PA lives here; not decoded, not identified as C2PA) |
| `Timestamps` | Low | when the picture was taken or edited, and timezone offsets |
| `CameraModel` | Low | make, model, lens make and model |
| `Software` | Low | what edited or produced the file |
| `Icc` | Low | an embedded colour profile (a fingerprint at most) |

Severity order is `Low < Medium < High`. A report's severity is the maximum over its findings; no findings is "none", which is **not** the same as "clean" when the file was unsupported or its metadata unreadable (the audit model carries that separately).

**Tag table** (non-GPS, non-Interop IFDs; the id decides, the IFD kind does not, because ExifTool names these tags by id wherever they appear)

| Id | Tag | Category |
|---|---|---|
| 0x010E | ImageDescription | FreeText |
| 0x010F, 0x0110 | Make, Model | CameraModel |
| 0x0131 | Software | Software |
| 0x0132 | DateTime | Timestamps |
| 0x013B | Artist | Authorship |
| 0x8298 | Copyright | Authorship |
| 0x9003, 0x9004 | DateTimeOriginal, DateTimeDigitized | Timestamps |
| 0x9010, 0x9011, 0x9012 | OffsetTime, OffsetTimeOriginal, OffsetTimeDigitized | Timestamps |
| 0x9290, 0x9291, 0x9292 | SubSecTime, SubSecTimeOriginal, SubSecTimeDigitized | Timestamps |
| 0x9286 | UserComment | FreeText |
| 0x927C | MakerNote | MakerNote |
| 0xA420 | ImageUniqueID | DeviceId |
| 0xA430 | CameraOwnerName | Authorship |
| 0xA431, 0xA435 | BodySerialNumber, LensSerialNumber | DeviceId |
| 0xA433, 0xA434 | LensMake, LensModel | CameraModel |
| 0xC62F | CameraSerialNumber (DNG) | DeviceId |
| 0x9C9B, 0x9C9C, 0x9C9E, 0x9C9F | XPTitle, XPComment, XPKeywords, XPSubject | FreeText |
| 0x9C9D | XPAuthor | Authorship |
| 0xA436 | ImageTitle | FreeText |
| 0xA437 | Photographer | Authorship |
| 0xA438 | ImageEditor | Authorship |
| 0xA439 | CameraFirmware | Software |
| 0xA43A | RAWDevelopingSoftware | Software |
| 0xA43B, 0xA43C | ImageEditingSoftware, MetadataEditingSoftware | Software |
| 0x02BC (700) | XMP (TIFF tag) | Xmp |
| 0x83BB (33723), 0x8649 (34377) | IPTC-NAA, Photoshop resources (TIFF tags) | Iptc |
| 0x8773 (34675) | InterColorProfile (TIFF tag) | Icc |
| 0x0201 in an `Ifd1` | JPEGInterchangeFormat (thumbnail) | Thumbnail |

An IFD1 without `0x0201` but with strip offsets (`0x0111`) is also a thumbnail (an uncompressed one): `Thumbnail` on the `0x0111` entry. Pointer tags (`0x8769`, `0x8825`, `0xA005`, `0x014A`) are structure, not findings: the GPS pointer's finding is the GPS IFD's own tags. (A pointer-tag id that appears *inside* a GPS IFD is a `Gps` finding like every other tag there: "every tag in a GPS IFD" wins over the pointer exemption, which applies to the IFDs the table covers.) ExifTool also names some ids this table leaves as inventory only (for example `0x013C` HostComputer, `0x010D` DocumentName, `0xC614` UniqueCameraModel, `0xC65D` RawDataUniqueID, `0xA432` LensInfo); adding any of them is an amendment, not a bug, and `0xC65D` and `0x013C` are the first candidates (identifiers a privacy scan would want).

Regions found by the JPEG layer map: `Xmp` and `ExtendedXmp` to `Xmp`, `Iptc` to `Iptc`, `Icc` to `Icc`, `Jumbf` to `Jumbf`.

## Context

The strip policies (design section 5: `Gps`, `GpsAndDevice`, `AllPii`, `All`, `Everything`) and every report depend on "what is personal". If the table is implied by code in two places (the audit and, later, the writer) they drift and a report promises something a strip does not do. Writing it down once, as a decision with ids, makes the audit, the policy preview (SPEC-014) and the writer (M1c/M1d) all derive from one source. The table follows design section 5's description (GPS; serials and unique IDs; Artist, Copyright, UserComment, DateTimeOriginal, Make and Model) and extends it only where a standard tag plainly belongs (offset times and sub-second times leak the timezone and clock; owner name; XP* Windows tags; the Exif 3.0 title/editor tags).

Honest limits are part of the decision: a `MakerNote` finding says "vendor block present, not parsed" because serial numbers commonly live inside it and are not listed; XMP, IPTC and JUMBF are found, not read. No category claims the file is free of anything it cannot see.

## Amendment (2026-11-05, from the SPEC-012 build)

The Exif 3.0 ids originally written here for `0xA437`-`0xA439` were wrong by one place (I wrote them from memory). The SPEC-012 live check against ExifTool 13.55 measured: `0xA437` Photographer, `0xA438` ImageEditor, `0xA439` CameraFirmware, `0xA43A` RAWDevelopingSoftware, `0xA43B` ImageEditingSoftware, `0xA43C` MetadataEditingSoftware. The table above is corrected; `0xA43A` and the shifted rows are the only change (Photographer joins `Authorship`; firmware and the software tags are `Software`). The check is why the table now carries names measured from the reference tool and not from memory. Nine other rows differ from ExifTool only in the display name (for example `0x0132` is `ModifyDate` there, `DateTime` here); the category is unaffected and the test pins those rows as known naming differences.

## Alternatives considered

- **Per-IFD-kind tables.** Rejected: ExifTool and the standards name these tags by id regardless of IFD; per-kind tables would multiply rows without changing a result.
- **A finding per category rather than per tag.** Rejected: the policy preview and the future writer need the exact entries; grouping is a rendering concern.
- **Treating `Software` and `Timestamps` as High.** Rejected: they fingerprint, they do not locate or identify; a CI threshold of "fail on High" should mean location or device identity.
- **Parse MakerNotes for serials now.** Rejected for M1 (vendor offset quirks; design section 6): reported as present-not-parsed.

## Consequences

- **Positive:** one source for audit, preview and writer; each row is testable and checkable against ExifTool's own tag names; severity gives CI a meaningful threshold.
- **Negative:** the table will be incomplete for exotic vendor tags (they stay inventory-only until added); a wrong id would mislabel a tag, so SPEC-012 checks every row against ExifTool's tag name.
- **Neutral:** adding a row is a DEC amendment plus one table line plus one test row; `Category` is `#[non_exhaustive]`.

## Validation

Revisit when a real-world file shows personal data in a tag the table lists as inventory-only, when ExifTool names a listed id as something else (SPEC-012's check fails), or when the writer stages need a category the table lacks.

## References

- Related specs: SPEC-012 (implements), SPEC-013 (inventory parity), SPEC-014 (policy presets derive from this table)
- Related decisions: DEC-005 (pointer tags), DEC-006 (regions)
- External: Exif 2.32 / 3.0 tag tables (ids), TIFF 6.0 tags 700, 33723, 34377, 34675; design `docs/exif-core-design.md` section 5

## Amendment (2026-11-09, DEC-012)

Two categories are added: `EmbeddedImage` (Medium) and `OtherSegment` (Medium), both block categories; a `COM` segment is a `FreeText` finding. The table above is otherwise unchanged. See DEC-012.
