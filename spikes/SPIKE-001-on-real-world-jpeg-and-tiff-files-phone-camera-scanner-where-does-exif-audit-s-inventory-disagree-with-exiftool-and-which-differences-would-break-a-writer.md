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
  cycle: spike                     # spike | land  (collapsed from a spec's 5)
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
  outcome: null                    # set at LAND. Never leave null on a landed spike —
                                   #   answered     — the question got an answer
                                   #   inconclusive — timebox hit, no answer (a real result)
                                   #   graduated    — the code becomes real work (see below)
                                   #   discarded    — the code is thrown away (also a win)
  landed_at: null                  # YYYY-MM-DD, stamped at land

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
    estimated_usd: 0
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

*Free-form. Written during the spike, for you. No structure required — this is
the one place in the template with no conventions to follow. What you tried,
what surprised you, dead ends worth remembering.*

---

## Land

*The mandatory terminal step. A spike that is never landed is exactly the failure
this lane exists to prevent — undocumented decisions leaking into production.*

### Answer

What did you learn? Answer the `## Question` directly, in a few sentences. If the
outcome is `inconclusive`, say what would make it answerable next time.

### Decisions this exploration already made

The spike made real choices without writing them down — that's allowed *during*,
and settled *here*. For each one that **still constrains what you build next**,
emit a `DEC-*` and list it below with honest confidence.

Not archaeology: only the load-bearing ones. If the rationale is genuinely lost,
`confidence: 0.4` with a note is the truthful record, not a failure.

- `DEC-NNN` — <one line> (confidence: X.X)

### What happens to the code

Pick one and complete its block.

**`discarded`** — the code is thrown away.
- What's worth keeping in writing instead: <one or two lines>
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
- What this unblocks or changes: <one or two lines>

### What's next

- [ ] <the concrete next action — frame a project, write a spec, drop the idea>

### Land checklist

- [ ] `spike.outcome` set (never leave it null — `just validate` fails on this)
- [ ] `spike.landed_at` stamped
- [ ] `project.id` back-linked if a project exists now
- [ ] `just advance-cycle SPIKE-NNN land`
- [ ] `just archive-spike SPIKE-NNN`
