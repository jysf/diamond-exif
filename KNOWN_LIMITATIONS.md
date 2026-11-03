# Known Limitations

Things the hardening pass on 2026-04-20 explicitly left unfixed. Each
has a reason — none of these is a surprise bug.

## No `just new-project` command

Creating `PROJ-002`, `PROJ-003`, etc. requires manually creating the
directory and copying `projects/_templates/project-brief.md` into
`projects/PROJ-NNN-<slug>/brief.md`. In practice Claude does this
during the PROJECT BRIEF step (see `GETTING_STARTED.md`), so the gap
rarely shows up in the documented flow.

If you add this command later, give it the same shape as `new-stage`:
accept a title, next-id a PROJ ID, copy the template, substitute
`PROJ-XXX` and `__TODAY__`.

## Stage-backlog editing is partly automated

As of v0.5.19, `archive-spec` **does** perform the mechanical, low-risk part of
the backlog update: it flips the spec's `- [ ] SPEC-NNN` line to
`- [x] … (shipped on DATE)` and recomputes the `**Count:**` line, scoped to the
`## Spec Backlog` section (falling back to a printed hint if the spec isn't
listed). What stays manual is the *judgment-laden* part — writing/reordering
backlog summaries and promoting "(not yet written)" bullets — which a script
that gets it wrong is worse than one that doesn't try. `new-spec` still does not
add a scaffolded spec to the backlog (the architect curates that list).

## `just init` is interactive-only

The recipe uses `read` to ask which variant to use. This works in a
normal terminal but breaks in CI, piped shells, or any non-TTY
context. The intended use (one human clicks "Use this template" and
runs `just init` once) doesn't hit this, so it's left as-is.

## No cross-platform CI

`just test` runs locally on whoever runs it. There's no GitHub Actions
workflow that exercises both macOS and Linux. The scripts have
`uname = Darwin` branches for `stat` and for `sed -i`, and those are
tested under `just test` on whichever OS runs the test — but drift
between the two branches is possible.

## Scripts assume 3-digit zero-padded IDs

`next_id` formats as `%03d`. If someone manually creates `SPEC-0001`
or `SPEC-10000`, behavior is undefined. The projected worst case at
normal scale is one project with >999 specs, which would mean the
stage-and-spec hierarchy is being misused.

## `get_active_project` uses a lexical-first, status-blind heuristic

When multiple `PROJ-*` directories exist (not counting the example),
`get_active_project` picks the lexically first one **regardless of its
`status`** — so a *shipped* project can stay "active" and a newer one is
invisible to `just status` and default `new-spec`/`new-stage` resolution.
Override with `export ACTIVE_PROJECT=PROJ-NNN-slug`, or pass the project id
explicitly to `new-stage`/`new-spec` (which now resolve deterministically and
hard-error on an ambiguous glob — v0.5.19). A status-aware default (prefer the
highest-numbered `active`/`proposed` project) is a candidate improvement, not yet
made because it would change the selection every command sees.

Note: `STAGE-*`/`SPEC-*` **numbering** is repo-wide and continuous (v0.5.20), so
it is unaffected by which project is "active" — new ids always continue from the
global maximum.

## Templates exist in two copies (one per variant)

`claude-only/` and `claude-plus-agents/` share most files. Changing a
shared file means editing it in both places. No symlink or generation
system exists. The user chose not to fix this in the hardening pass;
it's a maintainability hazard to watch over time.
