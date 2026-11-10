---
# A SPIKE is a BOUNDED EXPLORATION — the phase before you know the shape.
# Two modes, one discipline:
#   mode: question — a timeboxed investigation. Code is evidence, usually thrown away.
#   mode: build    — a vibe-coding session. Code is the deliverable, you intend to keep it.
# See AGENTS.md "Spike lane" and docs/decisions/DEC-012.
#
# Collapsed cycle: spike -> land.
#   spike — explore. NO spec, NO failing tests, NO DEC required. Speed IS the value.
#   land  — MANDATORY. The entire point of the lane: write down what you learned,
#           emit the DECs the exploration already made, decide the code's fate.
#
# There is deliberately NO verify step: a spike has no acceptance criteria, so an
# "independent verify" would have nothing to check (DEC-012). The timebox and the
# mandatory land step are the disciplines that replace it.

task:
  id: SPIKE-001
  type: spike                      # epic | story | task | bug | chore | patch | spike
  cycle: land  # spike | land  (collapsed from a spec's 5)
  blocked: false
  priority: medium

spike:
  question: On real-world JPEG and TIFF files (phone, camera, scanner), where does exif-audit's inventory disagree with ExifTool, and which differences would break a writer?
                                   # REQUIRED. A spike with no question is just coding.
                                   # For mode: build this is legitimately loose
                                   # ("is a local standup tool worth having?").
                                   # Loose is fine. Absent is not.
  mode: question                   # question | build
  timebox: 1 session
                                   # REQUIRED. Exceeded means STOP and land it as
                                   # `inconclusive` — not extend. Extending twice
                                   # means it isn't a spike, it's an unframed project.
  outcome: answered                  # set at LAND. Never leave null on a landed spike —
                                   #   answered     — the question got an answer
                                   #   inconclusive — timebox hit, no answer (a real result)
                                   #   graduated    — the code becomes real work (see below)
                                   #   discarded    — the code is thrown away (also a win)
  landed_at: 2026-11-09                  # YYYY-MM-DD, stamped at land

project:
  id: PROJ-001               # OPTIONAL (null) — a spike may PRECEDE any project
                                   # (that's the point). Back-link at land if one exists.
  # No `stage:` — a spike attaches to the repo, not a stage.
repo:
  id: diamond-exif

agents:
  explorer: claude-sonnet-5-5  # who ran the spike (tier_map.build; DEC-005)
  created_at: 2026-11-06

references:
  decisions: []                    # DECs EMITTED AT LAND (not during — that's the point)

# Cost is ADVISORY for spikes — `just cost-audit` does NOT gate them (DEC-012 v1).
# A spike is often pre-project and deliberately cheap; a cost gate on exploration
# is exactly the friction that would make people skip the artifact.
cost:
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0.00
    session_count: 0
---

# SPIKE-001: On real-world JPEG and TIFF files (phone, camera, scanner), where does exif-audit's inventory disagree with ExifTool, and which differences would break a writer?

## Question

Every parity result so far (178 inputs for the inventory, the value and JPEG records) comes from hand-built fixtures and fuzz seeds. On **real** JPEG and TIFF files from phones, cameras and scanners, where does `exif-audit`'s view (inventory, findings, statuses) disagree with ExifTool's, and which of those differences would make a **writer** produce a broken or leaky file (a wrong offset inside a MakerNote, a thumbnail or sub-IFD the reader misses, an Exif block the reader reports differently)?

It matters because the next stage is the JPEG and TIFF writer (strip by segment, then by IFD entry). A writer is only as correct as the reader under it, and real files are where reader gaps live (vendor MakerNotes, multi-picture JPEGs, tiled and multi-page TIFFs, Exif in odd places). The answer decides what the writer's design must account for, and which reader gaps are fixed first.

## Timebox

`1 session`. At the timebox, stop and land with what was measured (a partial answer over N files is an `answered` with a stated coverage; no answer is `inconclusive`). No second extension.

## Method and ground rules

- **Corpus source: decided with the user before anything is read** (a folder of their own photos, public sample images, or both). Nothing in the corpus is committed to the repo, copied into it, or sent anywhere. Files are read in place or in a scratch directory outside the repo.
- **Privacy:** the corpus may hold GPS and serial numbers. The spike never prints or records tag *values*; findings and logs name files by an index and a short hash, and tags by `(group, id)` only. `--values` is not used.
- **Compare** per file: `exif-audit --json --policy none --inventory` (this binary) against `exiftool -a -u -G1 -H -s` (run, never read), with the SPEC-013 comparator's rules (drop `-` ids and file-level groups; known pinned divergence classes recognised). Classify each new difference: ours-only, theirs-only, mis-grouped, status (`ok`/`issues`/`unreadable`) disagreement, crash or timeout.
- **Writer-relevance pass** over the differences: for each class, say whether a strip by segment or by IFD entry would be wrong because of it, and what the writer design must do.
- **Cost:** advisory (spikes are not cost-gated). Sub-agents may do the comparison runs; the interpretation stays with the architect.

## What's out of bounds

A spike may **not**:

- Ship user-facing behavior (that's a spec — or a patch, for shipped behavior).
- Be built upon before it lands. Land it first.
- Be a way to skip the cycle on work you already understand. If you can write
  acceptance criteria, you have a spec, not a spike.

---

## Log

Corpus: 16 files in one local folder (read in place, nothing copied; tag values never printed or recorded; files named here by short hash only): 11 JPEG, 5 DNG, 1 PNG, plus two non-image files that were not scanned. The JPEGs came from five kinds of source by their MakerNote signature or lack of one: three Nikon bodies' output, one Fuji, one Leica, one iPhone-class, and five with no MakerNote (a minimal 280 to 298 byte Exif, or Exif plus XMP and ICC). The DNGs were four without a MakerNote and one Leica. Tools: the release `exif-audit` (`--json --policy none --inventory`, values off) against `exiftool -a -u -G1 -H -s` 13.55, compared with the SPEC-013 rules by a throw-away script (scratchpad, not in the repo; ExifTool output reduced to group and id in memory and never written down).

**Inventory comparison.** 15 of 15 TIFF/JPEG/DNG files: no ExifTool entry missing from ours (0 theirs-only); every audit `clean`; no crash, timeout or stderr from either tool. Ours-only in 8 files, always the same two ids: `ExifIFD 0x927C` (MakerNote, 7 files) and `IFD0 0xC4A5` (PrintIM, 1 Fuji file): ExifTool expands those values into its own groups (`Nikon`, `FujiFilm`, `Apple`, `Leica`, `PrintIM`) and prints no line for the tag itself: the same "directory-valued" class SPEC-013 recorded for the pointer tags. The PNG is `unsupported` as designed, and ExifTool lists 46 EXIF-family entries plus XMP in it. Speed: ours 7 to 32 ms per file (an 85 MB DNG in 32 ms), ExifTool 45 to 95 ms.

**Structures a writer meets that the inventory does not show** (from segment walks, ExifTool's MPF/preview extraction, and MakerNote signatures; structure only):

- Exif APP1 segments of 65,468, 65,534 and 65,534 bytes: at the 64 KB ceiling (the three big-MakerNote files).
- Multi-Picture Format: 4 of 11 JPEGs have an APP2 `MPF` segment and 1 to 2 extra JPEG images inside the file (387 KB to 915 KB). The Nikon extras are preview JPEGs, two of the three with their own Exif; the iPhone extras are depth and HDR gain-map images carrying XMP and an ICC profile.
- Preview JPEGs inside MakerNotes: the Nikon and Fuji files carry a PreviewImage (20 to 96 KB), the Nikon ones with their own Exif; the Leica files carry a PreviewImage and, in the DNG, a 1.6 MB JpgFromRaw.
- Proprietary APP10 segments with signature `AROT` (718 and 1,022 bytes) in 2 of 11 JPEGs (the iPhone-class and one more); the audit has no category or region kind for them, so no policy can name them.
- A 313-byte non-Exif trailer after the last EOI in 1 of 11 JPEGs.
- IFD1 thumbnails (6 files): none carries metadata of its own.
- MakerNote layout: signatures `FUJIFILM`, `Nikon`, `Apple iOS`, `LEICA`; none starts with a TIFF header at byte 0.

**MakerNote relocation experiment.** For each JPEG with a MakerNote, copy the note to the end of the Exif block, point the 0x927C entry at the copy, zero the old bytes, and compare ExifTool's whole non-file view. Result: Nikon (11.5 KB) and Apple (1.7 KB) identical (the only changes were the MPF image offsets, which move with the file size); Fuji, Leica and the two larger Nikon notes could not be tested because the grown block would exceed 64 KB.

## Land

*The mandatory terminal step. A spike that is never landed is exactly the failure
this lane exists to prevent — undocumented decisions leaking into production.*

### Answer

On these 15 real TIFF, JPEG and DNG files the audit's inventory matches ExifTool's exactly except for two directory-valued tags (MakerNote 0x927C and PrintIM 0xC4A5, where ExifTool prints the expanded vendor group instead), with every file read cleanly and no crash, timeout or stderr: the reader showed no gap that would make a writer wrong **on the inventory**. The answer to the second half of the question is that the inventory is the wrong lens for a writer: real files contain five structures the audit and the DEC-009 preview do not model, and a strip that follows the preview would leave them behind or break them: (1) Multi-Picture Format secondary images, (2) preview JPEGs inside MakerNotes, (3) proprietary APP10 `AROT` segments, (4) Exif blocks at the 64 KB limit, (5) a non-Exif trailer. Coverage: 15 files, 5 JPEG source kinds, 2 raw sources, 1 PNG; not covered: Canon and Sony (the classic absolute-offset MakerNotes), Samsung and Pixel motion photos, Photoshop/Lightroom-edited files with APP13 and extended XMP, non-DNG TIFFs, HEIC and WebP.

**What the writer's design must therefore do** (carried into the writer stage's frame):
- Delete entries **in place** and **scrub**: removing an IFD entry must zero its out-of-line value bytes and any dead space, otherwise the GPS values stay in the file; do not relocate MakerNotes (relocation worked for Nikon and Apple notes but is untested for the rest).
- Never grow an Exif block (several are already at the ceiling); `All`/`Everything` remove whole segments.
- Treat MPF as a unit: either drop the MPF segment together with the secondary images or process each embedded JPEG; resizing the primary's header segments moves the secondary images' absolute offsets.
- Decide, per policy, what happens to unrecognised APPn segments and trailers.
- Make the audit report all of these first, so the preview can be true (see the follow-up below).

**Follow-ups** (recorded as questions, nothing built here): the audit should report MPF/embedded images, unrecognised APPn segments (APP10 `AROT`) and trailer data, which is a DEC-008 amendment (new categories) and the natural first spec of the writer stage; the SPEC-013 comparator's `DIRECTORY_VALUED` set should learn 0x927C and 0xC4A5 for real-world files; more file types are needed (below).

**More file types wanted** (for a second, smaller pass before or inside the writer stage): a Canon and a Sony JPEG (absolute-offset MakerNotes, the classic relocation hazard); a Samsung or Pixel motion-photo JPEG (large trailer, embedded video); a Photoshop- or Lightroom-edited JPEG (APP13/IPTC, extended XMP); a plain multi-page, tiled or strip TIFF scan (not DNG); an iPhone HEIC and a WebP (for the later stage).

### Decisions this exploration already made

The spike made real choices without writing them down — that's allowed *during*,
and settled *here*. For each one that **still constrains what you build next**,
emit a `DEC-*` and list it below with honest confidence.

Not archaeology: only the load-bearing ones. If the rationale is genuinely lost,
`confidence: 0.4` with a note is the truthful record, not a failure.

- None emitted at land. The writer-design implications (in-place delete plus scrub, never grow an Exif block, MPF as a unit, no MakerNote relocation) are not yet decided choices: they are carried into the writer stage's frame, where the first of them becomes a DEC before any writer code. The comparator rule used (drop `-` ids and file-level groups, treat directory-valued tags as ExifTool-expanded) is the SPEC-013 rule unchanged.

### What happens to the code

Pick one and complete its block.

**`discarded`** — the code is thrown away.
- What's worth keeping in writing instead: the log and answer above. The comparison and structure scripts lived in the session scratchpad (none entered the repo); the corpus stayed where it was and nothing from it was copied or recorded.
- *(A discarded spike is a win: you bought an answer cheaply.)*

**`graduated`** — the code becomes real work. **The five-item contract (DEC-012):**
- [ ] `.repo-context.yaml` describes what now exists
- [ ] `AGENTS.md` carries the real tech stack, commands, conventions
- [ ] `guidance/toolchain-brief.md` filled in from what this spike learned the
      hard way (most valuable here — a spike generates exactly this friction)
- [ ] Retroactive `DEC-*` for load-bearing choices only, honest confidence
- [ ] A project brief framed around **what comes next** — this spike is prior art
      in `Dependencies → Depends on`, not the subject of the project

  **Do NOT retro-write specs for code that already works.** A spec directs work
  that hasn't happened yet; written after the fact it describes behavior the
  tests already assert, and nobody reads it.

**`answered` / `inconclusive`** — the question resolved (or didn't); no code
graduates.
- What this unblocks or changes: the writer stage can be framed on evidence: its first spec is an audit-coverage spec (MPF/embedded images, unrecognised APPn, trailers) so the preview can be true, then segment-level strip, then in-place IFD deletion with scrub.

### What's next

- [x] Frame the writer stage (M1c/M1d, JPEG and TIFF), starting from the audit-coverage spec and the six implications in the Answer; ask the user for the extra file types listed in the Answer.

### Land checklist

- [x] `spike.outcome` set (never leave it null — `just validate` fails on this)
- [x] `spike.landed_at` stamped
- [x] `project.id` back-linked if a project exists now
- [ ] `just advance-cycle SPIKE-NNN land`
- [ ] `just archive-spike SPIKE-NNN`
