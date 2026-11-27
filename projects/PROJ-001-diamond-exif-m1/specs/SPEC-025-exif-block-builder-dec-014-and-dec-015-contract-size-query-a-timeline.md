# SPEC-025 timeline

Architect appends as cycles are designed. Executors update status as
they go. Status markers: `[ ]` not started · `[~]` in progress · `[x]` complete · `[?]` blocked.

Cycle prompts live in `prompts/SPEC-025-<cycle>.md`.

## Instructions

- [x] frame — outline in STAGE-007's frame (2026-11-20); design gate on irradiance/DEC-072 cleared and byte-order conversion split out to SPEC-041 at the re-frame (2026-11-27, claude-opus-5-5)
- [x] design — ExifTool 13.55 measurements (offset-carrying sweep over 1,178 sources, writer and reader passes; unknown types; count 0; Exif next-word; layout placed at bases 8 and 1000), DEC-014 amendment and DEC-015 clarification of 2026-11-27, question `strip-offset-tags-incomplete` raised, spec, HANDOFF-092 (2026-11-27, claude-opus-5-5)
- [?] build — HANDOFF-092 (claude-sonnet-5-5), two parts A and B; waits on jysf accepting DEC-014 as amended
- [ ] verify
- [ ] ship
