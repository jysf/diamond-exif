---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-003                     # stable, zero-padded, continuous across the repo
  status: shipped                   # proposed | active | shipped | cancelled | on_hold
  priority: high                    # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-05
shipped_at: 2026-11-06

# What part of the project's value thesis this stage advances.
# If you can't articulate value_contribution, the stage may be
# infrastructure-only — acceptable but flag it.
value_contribution:
  advances: "The first thing a user can run: exif-audit turns the verified TIFF/JPEG reader into a read-only privacy scan (what metadata is in these files, which of it is personal, what a strip policy would remove), the open-core funnel of the product shape."
  delivers:
    - "A pure-library audit model: container sniff, an inventory of every tag and located region, and PII findings by category (GPS, device identity, authorship, timestamps, thumbnail, makernote presence), with the category table recorded in a DEC"
    - "StripPolicy presets as data and a policy preview (what each preset would remove) with no writer involved"
    - "Text, JSON and CSV reports that stay well-formed on hostile input, with their own fuzz target"
    - "The exif-audit binary: scans files and directories, isolates per-file errors, deterministic output, CI exit codes"
  explicitly_does_not:
    - "Write, strip or rewrite any file (writer is M1c); the preview only reports"
    - "Read PNG, WebP or HEIC (a later stage); other containers are reported as unsupported, not as clean"
    - "Decode XMP, ICC, IPTC or C2PA contents, or parse MakerNotes (their presence is reported, serials inside them are not)"
    - "Ship exif-pro, a license gate, telemetry, or any network access"

# Orchestration cost — the spend that has no spec to attach to (roadmap:
# orchestration + framing cost attribution). Framing a stage, deciding the spec
# breakdown, and cross-spec steering all happen BEFORE/BETWEEN specs, so today
# they are invisible and recorded cost is systematically under-counted.
#
# THE ORCHESTRATOR FILLS THIS — not the human. At stage close, read your own
# session total (`/cost` in Claude Code; the `usage` object via API) and append
# one entry. Stage grain ONLY: do not try to split orchestration across specs —
# that is a division you cannot observe, so any per-spec number is invented.
# Warn-only, never a gate. A null here is honest; a guess is not. (DEC-013 §5)
orchestration_cost:
  sessions:
    - tokens_total: 1584655
      estimated_usd: 10.46
      recorded_at: 2026-11-06
      notes: "the orchestrator conversation after the STAGE-002 close commit (c3e9578, 2026-11-05T20:43:08-08:00): STAGE-003 framing follow-through, designs of SPEC-012..016 and SPEC-018 and DEC-008..011, dispatches, independent gate re-runs, follow-up rounds for the builders, delta-verify routing, ships, PRs 17-25 and their merges; recovered from transcript 1cd6e68e usage (input + output + cache_creation, de-duplicated by message id, cache_read excluded) at 6.60 USD/Mtok. Read at stage close; the closing turns after the read are not included"                      # - tokens_total: N
                                    #   estimated_usd: N
                                    #   recorded_at: YYYY-MM-DD
                                    #   notes: "framing + spec breakdown"
---

# STAGE-003: M1 exif-audit CLI

## What This Stage Is

Make the verified reader useful from a shell. `exif-audit PATH...` walks files and directories, sniffs each file's container (TIFF or JPEG; anything else is reported as unsupported, never as clean), builds an inventory of every tag and located metadata region, classifies what is personal (GPS, device identity and serial numbers, authorship, timestamps, an embedded thumbnail that may carry its own metadata, the mere presence of a MakerNote), previews what each strip policy would remove, and prints the result as text, JSON or CSV with exit codes a CI job can act on. The classification and the preview live in the pure library (no I/O); the binary adds only argument parsing, the directory walk and the exit-code contract. Nothing is written back.

## Why Now

STAGE-001 and STAGE-002 delivered a reader whose parity with ExifTool is shown, tolerant on hostile input, for the two containers uploads arrive in most. The audit CLI is the first deliverable named in the product shape and exercises that reader end to end on whole directories; it also forces the decisions the writer needs anyway (what counts as PII, what a strip policy means) before any byte is rewritten, so the writer stages inherit a settled policy instead of inventing one. PNG/WebP breadth does not unblock any of that.

## Success Criteria

- `exif-audit` on a file or directory emits inventory, PII findings and a policy preview as text, JSON and CSV, deterministically (sorted paths, stable field order), with documented exit codes (clean, findings above the threshold, usage or I/O error).
- No input makes the library or the binary panic; a bad file never aborts a directory run (per-file typed outcome); an unsupported container is reported as unsupported, not as clean.
- JSON and CSV output are well-formed for any input bytes, proven by a fuzz target over the renderers (constraint `fuzz-target-per-surface`), including tag values with control characters, quotes, invalid UTF-8 and very long strings.
- The inventory (group and tag id) equals ExifTool's on the hand-built fixtures, with the comparator red-proofed as before; any difference is a recorded divergence.
- The PII category table and the strip-policy presets are each recorded in a DEC before they are coded, and every preset is checked against fixtures (what it would remove is exactly what the table says).
- The library still has no I/O and no dependencies; the binary adds none either unless a DEC says otherwise; all CI jobs green on every PR, including a job that builds the binary.

## Scope

### In scope
- Container sniff for TIFF and JPEG; unsupported-container outcome
- Inventory and PII findings from `Container` and `JpegMetadata`; the category table (DEC)
- `StripPolicy` presets as data; policy preview from a `FileAudit`
- Text, JSON and CSV renderers (pure, in the library or a module shared with the fuzz target)
- The `exif-audit` binary: argument parsing, directory walk (symlink and size-cap policy), per-file isolation, exit codes, `--json` / `--csv`, README usage, `just dev`
- Inventory parity against ExifTool on fixtures (group + tag id), with red-proofs

### Explicitly out of scope
- Any write, strip, rewrite or in-place edit
- PNG, WebP, HEIC readers; XMP, ICC, IPTC, C2PA decoding; MakerNote parsing
- `exif-pro`, license gates, telemetry, networking, a TUI
- Recursing into an embedded thumbnail JPEG (its presence is a finding; scanning inside it is an open question, `audit-thumbnail-recursion`)

## Spec Backlog

Ordered list of specs composing this stage. Add specs as you identify
them. Update status as specs progress.

Format: `- [status] SPEC-ID (cycle) — one-line summary`

- [x] SPEC-012 (shipped on 2026-11-05) — [M] Audit model: container sniff, inventory, PII categories and findings (pure library); DEC for the category table
- [x] SPEC-013 (shipped on 2026-11-06) — [L] Inventory parity against ExifTool: group + tag-id sets on fixtures, comparator red-proofed
- [x] SPEC-014 (shipped on 2026-11-06) — [M] StripPolicy presets as data and the policy preview; DEC for preset semantics
- [x] SPEC-015 (shipped on 2026-11-06) — [L] Text, JSON and CSV renderers and the `audit_render` fuzz target (well-formed on hostile input)
- [x] SPEC-017 (shipped on 2026-11-06) — [M] Bound Container memory: a total entry budget for the tolerant walk (found by the SPEC-012 build: a 789 KB TIFF parses cleanly to 33M entries, ~950 MB)
- [x] SPEC-018 (shipped on 2026-11-06) — [M] Incremental report assembly: ReportBuilder with output equal to the one-shot renderers (so the binary holds one input file at a time)
- [x] SPEC-016 (shipped on 2026-11-06) — [L] The `exif-audit` binary: arguments, directory walk, per-file isolation, exit codes, CLI integration tests, CI build job

**Count:** 7 shipped / 0 active / 0 pending

## Design Notes

- **Where things live.** Classification, inventory and preview are pure functions over `&[u8]` results in `exif_core` (the library has no I/O, constraint `no-io-in-library`). The binary owns `std::fs`, argument parsing and exit codes. Renderers build `String`s and take no writer, so the fuzz target can call them without the binary.
- **No dependencies.** Argument parsing is hand-written (a small, fixed flag set) and JSON/CSV are written by hand with an escaping function that is itself fuzzed; a clap/serde dependency would need a DEC under `no-new-top-level-deps-without-decision`, and none is expected.
- **Honest limits are findings.** A MakerNote is present but unparsed, so serials inside it cannot be listed: the report says "MakerNote present (not parsed)" rather than implying a clean file. Unsupported containers and unreadable files are outcomes, not silent skips.
- **Policy before code.** The PII category table and the strip-policy preset contents are DECs written at the design of their specs, before any code, as DEC-001 was for the tolerant walk. The preview must be derivable from the table so the writer stages cannot drift from what the audit promised.
- **Multi-Exif.** Open question `several-exif-segments` (merged Exif view) is decided in the first spec: the audit must report findings from every Exif block, not only the primary.
- **Oracle.** ExifTool is run for inventory parity (`-G1 -a -H`-style listing of group and tag id); it is never read or linked. Any new hostile shape is probed alone under the time limit first.

## Dependencies

### Depends on
- STAGE-002 (shipped) — the JPEG reader, located regions, and the oracle with JPEG records
- STAGE-001 (shipped) — the TIFF engine, ExifTool oracle and red-proof apparatus
- External: ExifTool 13.55 as the oracle (run, never read or linked)

### Enables
- The writer and strip stages (M1c/M1d) — a settled category table and preset semantics, and a preview to verify writes against
- PNG/WebP breadth — every new container plugs into the same audit model
- `exif-pro` (M2) — `audit_directory` is this stage's library function plus a license gate

## Stage-Level Reflection

*Filled in when status moves to shipped. Run Prompt 1c (Stage Ship) in
FIRST_SESSION_PROMPTS.md to draft this.*

- **Did we deliver the outcome in "What This Stage Is"?**
  Yes. `exif-audit PATH...` is a binary a person or CI job runs: it sniffs each file (TIFF and JPEG are read; anything else is reported `unsupported`, never clean), builds an inventory of every tag and located region, classifies findings by DEC-008 category, previews what each of five strip presets would remove (DEC-009, with `KeptUnread` for blocks the library cannot read), and prints text, JSON or CSV (DEC-010) with exit codes 0/1/2/3/4 (DEC-011). Inventory parity with ExifTool is shown over 178 distinct inputs with every difference pinned; the pure library has no I/O and no dependencies; the binary adds none. Honest narrowing: values are raw excerpts only when asked (no decoding); MakerNote serials, XMP, IPTC, ICC and C2PA are located, not read; the Linux-only paths and the new CI release job had never run on the dev host when the last PR was opened (they run for the first time in PR 25).

- **How many specs did it actually take?**
  Seven against the six framed: SPEC-012 audit model, SPEC-013 inventory parity, SPEC-014 presets and preview, SPEC-015 renderers, SPEC-016 binary, plus two added mid-stage: SPEC-017 (the total entry budget, found by the SPEC-012 build: a 789 KB TIFF parsed cleanly to 33M entries) and SPEC-018 (the incremental `ReportBuilder`, found at the SPEC-016 design: a directory scan would otherwise hold every file's bytes alive). All seven shipped with verify verdict approved; first-pass verifies that returned a punch list: SPEC-016 (a FIFO swap race that hung the run) and SPEC-018 (test-strength gaps); SPEC-015 was approved with findings that took three builder follow-ups and a delta verify that itself returned a punch list (a timing test that flaked under load) before the final approval. Metered build and verify cost 4,483,854 tokens (about $29.58) across the 7 specs, plus 1,584,655 tokens (about $10.46) of orchestration; 6,068,509 tokens (about $40.04) in total. Per-spec estimates ran under for six of seven (3.60M estimated vs 4.48M actual, about 25 percent under; SPEC-016 alone was over-estimated, 800k vs 666k).

- **How many outlines survived unchanged?**
  4 of 5 framed outlines kept their scope (012, 013, 014, 015); SPEC-016 kept its goal but lost its memory model to SPEC-018 and gained DEC-011's walk and race rules; SPEC-017 and SPEC-018 were not outlines at frame time.

- **What changed between starting and shipping?**
  Reading hostile bytes safely turned out to be the work: two library bounds and a builder were added because the first design of each spec let memory scale with the input, and the cold verifiers found real defects in every spec's tests or code (a CSV semicolon bypass of the formula guard, invisible-character name spoofing, a FIFO hang) that the builds' own tests had not.

- **Lessons that should update AGENTS.md, templates, or constraints?**
  - Put a security and race review of the surface into the design, not only into verify: the escaping gaps (SPEC-015) and the check-then-open window (SPEC-016) were both found by the verifier's own attacks; add "list the hostile shapes and the races for each I/O or escaping surface" to the spec design checklist (below bar, recorded as the watch signals below).
  - A guard is only as strong as what its inputs and plants vary: seeds must reach every class (SPEC-014, SPEC-015), plants must vary in kind and not only in length (SPEC-018). Both are watch signals.
  - A timing assertion must be a ratio of the same code at two sizes, never an absolute bound (SPEC-013 weakness, SPEC-015 flake); applied in SPEC-018 and SPEC-016.
  - Cumulative sub-agent token notices change between a first and a final figure: record only after the final notification (cost fixes were needed for SPEC-013, SPEC-015 and SPEC-016).
  - The measured table in a design is a prediction for the build to re-measure (SPEC-013 again: Class R, the ICC sizes).

- **Signals dispositioned at this close?**
  Walked all `type: lesson` signals owned by this stage close. `flag-default-explicitness` (N=3, at its bar) is **codified**: the spec template's Outputs section already requires each flag's accepted values and its default, and SPEC-016 and DEC-011 applied it (every default stated and pinned by `usage_text_names_every_default`); no recurrence this stage. Left `watch`: `fuzz-guard-needs-seeds-for-every-class` (N=2: SPEC-014, SPEC-015), `equality-guard-plants-must-vary-in-kind` (N=1: SPEC-018), `check-then-open-needs-a-safe-open` (N=1: SPEC-016). Project-close signals (`measure-exit-status-without-a-pipe`, `divergence-entries-must-pin-our-values`, `run-ci-only-gates-locally`, the process-debt items) are not owned by this close and were not touched; `run-ci-only-gates-locally` was applied again (the build briefs named the CI-only scripts) with no recurrence.

- **Should any spec-level reflections be promoted to stage-level lessons?**
  - "Design the attack list with the surface": recorded above as below-bar notes tied to the three watch signals; revisit at project close.
  - Carried forward (not blockers): questions `cli-symlink-swap-window`, `cli-stdin-input`, `cli-follow-symlinks`, `cli-large-trees`, `cli-fail-on-residual`, `report-value-decoding`, `report-tag-names`, `report-custom-policy-identity`, `strip-may-hold-gaps`, `preview-unclassified-entries`, `gps-preset-leaves-unread-blocks`, `inventory-oracle-low-findings`, plus the earlier SBYTE selector, several-exif-segments, C2PA-in-JUMBF, audit-thumbnail-recursion, DEC-008 amendment candidates and the upstream `decisions-audit.sh` parsing.
