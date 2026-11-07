---
insight:
  id: DEC-011
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

created_at: 2026-11-07
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope:
  - "src/bin/exif-audit/**"
  - "tests/cli*.rs"
  - "fuzz/fuzz_targets/cli_args.rs"

tags:
  - cli
  - exit-codes
  - walk
  - symlinks
  - ci
---

# DEC-011: The `exif-audit` command line — options and defaults, the walk, and the exit codes

## Decision

`exif-audit [OPTIONS] [--] PATH...` scans the named files and directories, prints one report (DEC-010) to stdout, and exits with a code a CI job can act on. It writes nothing but that report, reads nothing but the named paths, and keeps no state. This DEC fixes the options with their defaults, the walk, the failure wording, and the exit codes. It answers question `audit-exit-code-contract`.

**1. Options and defaults** (every option's default is stated here and in `--help`; a flag whose default is unstated is a defect):

| Option | Values | Default | Meaning |
|---|---|---|---|
| `--format F` | `text`, `json`, `csv` | `text` | report format |
| `--json`, `--csv` | (none) | off | shorthand for `--format json` / `--format csv`; combining two format selections is a usage error |
| `--policy P` | `gps`, `gps-and-device`, `all-pii`, `all`, `everything`, `none` | `all-pii` | which strip policy the report previews (DEC-009); `none` omits the preview |
| `--fail-on S` | `none`, `low`, `medium`, `high` | `high` | exit 1 when any file has a finding of at least this severity; `none` never exits 1 |
| `--strict` | (none) | off | count `unsupported` and `issues` files as incomplete (exit 3) |
| `--values` | (none) | off | include a raw value excerpt (DEC-010 part 1) |
| `--inventory` | (none) | off | list every inventory entry, not only findings |
| `--max-size N` | integer bytes with optional `K`, `M`, `G` suffix (powers of 1024), at least 1 | `256M` | largest file read; a larger file is reported `failed` |
| `-h`, `--help` | | | usage on stdout, exit 0 |
| `-V`, `--version` | | | `exif-audit <crate version>` on stdout, exit 0 |

Options take their value as the next argument or as `--name=value`. `--` ends options (so a path may start with `-`). At least one PATH is required. A lone `-` is a usage error (stdin is not read). Giving the same option twice is a usage error. Unknown options and bad values are usage errors whose message is fixed ASCII text (the offending argument is shown through `safe_text`).

**2. Exit codes.** Exactly one code, chosen in this order:

| Code | When |
|---|---|
| `2` | usage error (nothing was scanned; message on stderr, stdout empty) |
| `4` | the report could not be written to stdout (including a closed pipe); no partial claim of success |
| `3` | the audit is **incomplete**: at least one file is `failed` or `unreadable`, or, with `--strict`, `unsupported` or `issues` |
| `1` | complete, and at least one file has a finding of severity at least `--fail-on` |
| `0` | complete, and no file reaches the threshold (including: no files at all, and `--fail-on none`) |

`--help` and `--version` exit 0 and scan nothing. A Rust panic would exit 101; the library and the binary are panic-free (a 101 is a bug, and the integration tests assert it never happens on the committed seeds). Incomplete outranks findings because a CI job must never read "1: findings" as "everything else was fine"; the report on stdout carries every per-file fact either way. An unsupported file (for example a PNG) is **never** counted as clean: it is printed with status `unsupported`, and it fails the run only under `--strict`. The threshold compares against each audit's `max_severity()` over all of its findings, not against the previewed policy's residual.

**3. The walk.** Arguments are processed in the order given. A file argument is one entry. A directory argument is walked recursively, **entries sorted by name** (the platform's `OsString` order), depth-first, at most **64** levels deep. Hidden files and directories are scanned (a privacy scan must not skip them). The label of an entry is the argument as written, joined with the path below it by the platform separator; paths are never canonicalised, made absolute, or case-folded, so output is the same wherever the command is run from.

- **Symlinks.** A path named on the command line is followed (the user named it). A symlink *found while walking a directory* is **not** followed and is reported as `failed` with reason `not_regular_file` and detail `symlink not followed`. Nothing is silently skipped.
- **Special files.** A FIFO, socket, device or anything that is not a regular file is reported `failed` / `not_regular_file` / `not a regular file`, and is never opened (opening a FIFO can block forever). The regular-file check is repeated on the open handle (a path can change between the check and the open).
- **Size.** A regular file larger than `--max-size` is reported `failed` / `too_large` / `larger than --max-size`; at most `max-size + 1` bytes are read to find out. A file of exactly `max-size` bytes is read.
- **Read only what is needed.** The first 12 bytes are read and sniffed (the library's `sniff`). A container the library does not read (anything but TIFF and JPEG) is reported from those bytes alone (`unsupported`) without reading the rest of the file; a TIFF or JPEG is read whole and audited.
- **Depth.** A directory more than 64 levels below its argument is reported as one `failed` / `other` entry `directory depth limit`, and not entered.
- **Missing or unreadable paths** are `failed` entries, not usage errors, so one bad path does not hide the others.

**4. Failure wording is fixed.** The `detail` of every `failed` entry is one of a closed list of ASCII strings chosen from the `io::ErrorKind` or the condition: `not found`, `permission denied`, `larger than --max-size`, `not a regular file`, `symlink not followed`, `directory depth limit`, `read error`, `i/o error`. The operating system's own message is never used (it varies by platform and can embed path text), so the output is identical across hosts. `FailReason` maps: not found, permission denied, read error and i/o error to `unreadable`; the size case to `too_large`; the two not-regular cases to `not_regular_file`; the depth case to `other`. DEC-010 amendment 1 item 3 holds: label and detail never contain file *contents*.

**5. Output discipline.** The report is the only thing on stdout, written once at the end (assembled with the incremental builder, SPEC-018, so memory is the size of the report, not the sum of the input files). stderr carries usage errors only. There is no progress output, colour, timestamp, absolute path, locale-dependent text or environment-dependent text; running twice on the same tree gives identical bytes. One file is held in memory at a time.

**6. Not decided here** (questions): reading a file list from stdin, a `--follow-symlinks` option, parallel scanning, a `--fail-on-residual` threshold, bounding a report for very large trees.

## Context

CI jobs depend on exit codes, and the STAGE-003 principle that an unsupported or unreadable file is never counted as clean has to survive contact with a shell. The decisions that bite are the ones a script author cannot see: what happens with a symlink loop or a FIFO, whether a path named with a hostile name breaks the report, which of "findings" and "could not read" wins, and whether the same command prints the same bytes on two machines. Writing them once, with defaults, keeps the binary, its tests and the README from disagreeing.

## Alternatives considered

- **Exit 1 for findings, 2 for everything else.** Rejected: usage mistakes, unreadable files and a failed write are different problems for a CI author.
- **Findings outrank incomplete.** Rejected: "1" would then mean "findings, and maybe the scan was also partial". Incomplete first makes 0 and 1 trustworthy.
- **Unsupported files fail the run by default.** Rejected: a mixed directory (images plus text files) would always fail. `--strict` is the explicit opt-in.
- **Skip non-image files by extension.** Rejected: extensions lie, and a privacy scan should judge by content (the 12-byte sniff makes it cheap).
- **Follow symlinks inside walks.** Rejected for v1: loops, escapes from the tree and double counting; reporting them is the safe default.
- **Use the OS error text in `detail`.** Rejected: platform-dependent, not deterministic.
- **Default `--fail-on` low or medium.** Rejected: nearly every camera file has a timestamp or a model (Low) and many have an Artist or a MakerNote (Medium), so a default CI gate would always fail. `high` (GPS or a device serial) is the useful default; the user tightens it.
- **A CLI-parsing dependency (clap).** Rejected: `no-new-top-level-deps-without-decision`; the grammar above is small and fuzzed.

## Consequences

- **Positive:** a CI job can gate on `exit 0/1/3` knowing what each means; output is reproducible; hostile paths cannot break the report; large trees cost memory proportional to the report, one input file at a time.
- **Negative:** a default of `--fail-on high` means a file with only an Artist or a MakerNote passes unless the user lowers the threshold; a very large tree still produces a report held in memory (question `cli-large-trees`).
- **Neutral:** the pure parts (argument parsing, the exit-code function) are separate modules of the binary so tests and a fuzz target can reach them.

## Validation

Revisit when a CI user needs a different precedence between 1 and 3, when stdin input or symlink following is requested, when a tree is large enough that the in-memory report matters, or when the first real-world run shows a default surprising people.

## References

- Related specs: SPEC-016 (implements), SPEC-018 (incremental report builder), SPEC-015
- Related decisions: DEC-010 (report, statuses, failure wording rules), DEC-009 (policies, `reliable`), DEC-008 (severities)
- Questions: `audit-exit-code-contract` (answered here)
