# Provenance ledger

**Every ported file, algorithm and decoder in `exif-core` gets a row here,
recording where it came from and under what licence.** The row is written
before, or when, the file lands (DEC-003).

The licence a crate *declares* and the provenance its code *carries* are
different things, and `cargo deny` only sees the former. This ledger is what
makes the permissive (MIT OR Apache-2.0) claim defensible rather than merely
asserted.

## The rule

Ranked best to worst. Prefer the highest available:

1. **A published specification** — zero contamination (e.g. TIFF 6.0, the Exif
   specification, the DNG specification).
2. **A published paper** — the algorithm, not someone's code.
3. **Our own prior code** — e.g. irradiance, same author. Code copied from
   irradiance and then edited is class 3, derived from class 1 (DEC-003).
4. **Reading a permissively-licensed implementation** — allowed, must be
   recorded, and the source licence's terms must be honoured.
5. **Reading a copyleft implementation** — **not permitted.** If a behaviour is
   only available this way, it does not ship in the default build; it needs its
   own decision.

"I read it years ago and reimplemented from memory" is class 5, not class 3.
Record it honestly.

### ExifTool

ExifTool (Perl, Artistic/GPL) may be RUN as an oracle but never READ to resolve a format question: running a tool imposes nothing on this crate, while reading copyleft source is a class-5 violation. It is never linked and never vendored.

## The ledger

| Module / algorithm | Source | Source licence | Provenance class | Notes |
|---|---|---|---|---|
