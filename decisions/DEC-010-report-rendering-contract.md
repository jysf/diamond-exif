---
insight:
  id: DEC-010
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
  - "src/report/**"
  - "tests/report*.rs"
  - "fuzz/fuzz_targets/audit_render.rs"

tags:
  - report
  - json
  - csv
  - hostile-input
  - privacy
---

# DEC-010: How a report is written — what it shows, what it escapes, and the JSON and CSV shapes

## Decision

A report is built from audits by pure functions that return a `String` (`render_text`, `render_json`, `render_csv`). Everything in a report that did not come from this library's own constants is **hostile text**: file labels (paths), tag value excerpts, and failure details. The contract has five parts.

**1. Reports are value-free by default.** A privacy scan that prints serial numbers, GPS coordinates and owner names into a CI log spreads the data it was run to find. By default a report holds categories, severities, tag numbers, locations, counts, dispositions and health, and no value bytes. A **value excerpt** appears only when the caller sets `RenderOptions::values`. An excerpt is the entry's raw payload (borrowed from the input, never decoded): for an ASCII-typed entry the text up to the first NUL, otherwise the payload bytes, in both cases at most **64 bytes**, rendered as display-safe text (part 2) with an explicit `truncated` flag when cut. No value is decoded in this decision (no rationals to degrees, no dates): a decoded view is a later spec (question `report-value-decoding`).

**2. Display-safe text.** One function turns hostile bytes into text that is safe to print and to embed: `safe_text(bytes)`. Valid UTF-8 passes through except for **escaped** characters; invalid bytes and escaped characters are written as ASCII escapes. Escaped: backslash (`\\`), double quote (`\"`), every C0 control (U+0000 to U+001F, as `\n`, `\r`, `\t`, or `\xNN`), DEL, every C1 control (U+0080 to U+009F, as `\u{NNNN}`), the Unicode bidirectional controls (U+061C, U+200E, U+200F, U+202A to U+202E, U+2066 to U+2069) and the line and paragraph separators U+2028 and U+2029 (as `\u{NNNN}`), and each byte of an invalid UTF-8 sequence (`\xNN`). The result contains no control character, no unpaired bidi control and no raw quote or backslash, so a hostile name cannot inject terminal escape sequences, spoof a path with a right-to-left override, or break a quoted field. The same function is used for labels, excerpts and failure details in all three formats; the JSON and CSV layers then escape for their own syntax again (defence in depth, each layer fuzzed alone).

**3. JSON.** One document, UTF-8, `schema_version` 1, keys in the fixed order the spec lists, no duplicate keys, no number but a non-negative integer, no `null` except where the spec says. Strings are escaped per RFC 8259 (`"`, `\`, every U+0000 to U+001F as `\uXXXX`); because of part 2, an input never contributes a raw control character. Field names and enumerated values are API: they are pinned by a test, and a change bumps `schema_version`. The library adds no JSON dependency.

**4. CSV.** RFC 4180: header row, CRLF line ends, a field is quoted iff it contains a comma, quote, CR or LF (quotes doubled). One row per finding, plus one row for every file that has no finding (so an unsupported, failed or clean file is visible, never absent); every row has the same columns. **Formula injection guard:** a field that comes from hostile input (label, excerpt, failure detail) and begins with `=`, `+`, `-`, `@`, TAB or CR is prefixed with a single quote `'` before CSV quoting (OWASP guidance for spreadsheet import). The guard changes the data, so it is documented in the header comment of the render function and pinned by a test; JSON and text do not apply it.

**5. File status is a first-class field, and "ok" is earned.** Each file has exactly one `status`:

| `status` | When |
|---|---|
| `failed` | the caller could not read the file (the binary's I/O failure, a size cap, not a regular file); there is no audit |
| `unsupported` | the container is not one the library reads (the audit's `ContainerKind::Unsupported`) |
| `unreadable` | a supported container whose metadata could not be read (`failed_blocks > 0`) |
| `issues` | readable, but the readers reported issues or the inventory was truncated (`clean == false` otherwise) |
| `ok` | `health().clean` |

`ok` is emitted only for a clean audit. A report never prints "no personal data" for any file; "no findings" is printed as a count of zero beside the status, and a policy preview's residual numbers are printed only after `reliable` (DEC-009 amendment). Per-file `status` and the run summary supply every fact the binary needs for its exit codes (SPEC-016 decides the codes).

**What a finding line shows.** Container-independent: category, severity, the location (`ifd` kind and index, tag number in hex, field type and count for a tag; region kind and length for a region), the Exif block index, whether it lies after the scan, and, with a policy, the disposition (`removed`, `kept`, `kept_unread`) worded as DEC-009 requires: `kept` means "not targeted by this policy", `kept_unread` means "may hold what this policy removes". A thumbnail finding also shows its range and the `starts_with_ffd8` flag. Tag names are not shown (question `report-tag-names`): the category and the tag number identify it.

## Context

The renderers are a surface in their own right: a tag value, a path or an error message is attacker-controlled bytes that end up inside a JSON document read by a CI job, a CSV opened in a spreadsheet, and text printed to a terminal. Each is a known injection channel (invalid JSON that breaks the CI parser, formula injection in a spreadsheet, terminal escape and bidi spoofing). Deciding the escaping and the privacy default once, in a DEC, keeps three renderers and a fuzz harness from each inventing a rule. The value-free default follows from the audit's own purpose and from STAGE-003's honest-limits principle.

## Amendment (2026-11-06, from the SPEC-015 build)

Four clarifications from the build's doubts; only the first changes behaviour.

1. **The summary's residual number needs a reliable file.** `Summary::residual_findings` is `Some(n)` only when a policy is set and at least one audited file is reliable (`n` sums the reliable files only); otherwise it is `None` (JSON `null`, and the text says the residual is not available). `Some(0)` beside zero reliable files read like "nothing remains", which DEC-009's amendment forbids.
2. **DEL is written `\x7f`**, like the other C0 controls without a short form. TAB and CR are escaped by `safe_text` (`\t`, `\r`) before the CSV layer sees them, so the formula guard's TAB and CR triggers do not fire through `render_csv`; they stay in the guard as defence in depth for any field that reaches the CSV layer another way.
3. **`label` and failure `detail` are caller-supplied, not file content.** The library does not bound their length (output is linear in them) and prints the detail whatever `values` says, because the detail is the caller's own error text. The binary must pass only a path and an OS error message, never bytes read from the file. The fuzz target feeds file bytes into the detail on purpose, to exercise the escaping.
4. **A `Custom` policy is named only "custom"** in every format. Two different custom sets are not told apart in a report; the binary exposes presets only, so nothing produces this today (question `report-custom-policy-identity`).

## Amendment 2 (2026-11-06, from the SPEC-015 verify)

Two rules tightened; the first closes a bypass the verifier demonstrated.

1. **CSV quotes on semicolon too.** A field is quoted iff it contains a comma, a double quote, CR, LF **or a semicolon**. A spreadsheet in a semicolon locale splits an unquoted `a;=1+cmd|...` into a cell that starts with `=`, which the single-quote guard would not have seen; quoting keeps it one cell. (A leading space before a trigger and the full-width `＝` are text in spreadsheets and are not guarded.)
2. **`safe_text` also escapes invisible and format characters**, as `\u{NNNN}`: U+00AD, U+200B to U+200D, U+2060, U+FEFF, U+206A to U+206F, U+FFF9 to U+FFFB and the tag characters U+E0000 to U+E007F. `photo\u{200B}.jpg` must not look like `photo.jpg`. Homoglyphs (different letters that look alike) are not addressed: they are valid text.

## Alternatives considered

- **Print values by default.** Rejected: a CI log is the wrong place for a serial number or coordinates; the opt-in costs one flag.
- **Decode values (dates, rationals, GPS degrees) in the report.** Rejected for this spec: it is a second parsing surface with its own correctness proof (SPEC-007 decoders exist but are not report-tuned); excerpts carry the raw payload faithfully.
- **Lossy U+FFFD for invalid UTF-8.** Rejected as the only rule: two different hostile names would render the same; `\xNN` keeps them apart and is ASCII-safe. (JSON never sees a raw invalid byte, so no U+FFFD is needed there either.)
- **A JSON or CSV crate.** Rejected: `no-new-top-level-deps-without-decision`; the writers are small, and the independent checkers in the tests and the fuzz harness are what make them trustworthy. Revisit if the format grows.
- **JSON Lines instead of one document.** Rejected for v1: a single document with a summary is easier to consume in CI; streaming is a binary concern if directories grow very large.
- **No CSV injection guard.** Rejected: audit tools are routinely run on untrusted directories and their CSV opened in spreadsheets.

## Consequences

- **Positive:** all three outputs are safe for any input bytes by construction and by fuzz; a report cannot say "ok" for a file the library could not read; value exposure is a conscious flag.
- **Negative:** the CSV guard alters hostile fields beginning with those characters (documented); excerpts are raw, so a rational GPS value prints as bytes; JSON `schema_version` makes every field rename a version bump.
- **Neutral:** `RenderOptions` is the only configuration; exit codes and file walking stay in the binary.

## Validation

Revisit when a consumer needs streaming output, when a decoded value view is specified, when a spreadsheet or terminal injection shape is found that `safe_text` lets through, or when SPEC-016 shows a status the five above cannot express.

## References

- Related specs: SPEC-015 (implements), SPEC-016 (consumes status and summary), SPEC-012, SPEC-014
- Related decisions: DEC-008 (categories, severity), DEC-009 (dispositions, `reliable`)
- External: RFC 8259 (JSON), RFC 4180 (CSV), OWASP "CSV Injection"
