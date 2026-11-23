# SPEC-033 timeline

Architect appends as cycles are designed. Executors update status as
they go. Status markers: `[ ]` not started · `[~]` in progress · `[x]` complete · `[?]` blocked.

Cycle prompts live in `prompts/SPEC-033-<cycle>.md`.

## Instructions

- [x] design — spec, failing-test list, DEC-008/009/013 amendments, HANDOFF-053 (2026-11-21, claude-opus-5-5)
- [x] design revision — PNG and WebP sites, DEC-020/021 amendments, depends on SPEC-038 (2026-11-22, claude-opus-5-5)
- [x] build — HANDOFF-053 (claude-sonnet-5-5): https://github.com/jysf/diamond-exif/pull/57, 37156982 tokens (transcript), 2026-11-23
- [x] verify — HANDOFF-085 (claude-opus-5-5, cold): ✅ APPROVED at 43eb634, three Low findings (no fix round), PR #57, tokens in HANDOFF-085 handback (transcript), 2026-11-23
- [x] ship — L1–L3 fixed, costs re-summed, STAGE-009 closed (2026-11-23, claude-opus-5-5)
