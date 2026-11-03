# Reviewer brief — design fit

You are reviewing for **one question: is this the right shape for *this*
codebase?** Assume it works — someone else checked that. You are asking whether
it belongs.

Read `/decisions/INDEX.md`, `guidance/constraints.yaml`, and the spec's
`references.decisions` before the diff. You cannot judge fit without knowing
what has already been decided.

## Look for

- **Duplication of something that already exists.** The most common real
  finding, and the hardest to see from inside the work.
- **A recorded decision this contradicts.** If a DEC governs these paths
  (`just decisions-audit --changed`), does the change honour it? If it
  deliberately departs, is there a new DEC saying so?
- **A pattern foreign to the repo** — a new way of doing something the codebase
  already does another way. Novelty is a cost even when the new way is better.
- **Coupling that will be expensive later**: a module reaching across a boundary
  the rest of the code respects.
- **A load-bearing choice with no DEC.** If someone will ask "why is it like
  this?" in six months, the answer should exist.

## Deliberately ignore

Whether the tests pass, whether criteria are met, formatting, and anything a
linter could decide. If your finding could be a lint rule, it belongs in a lint
rule.

## Your verdict

`approved` · `punch-list` · `rejected`

**Reject only for:** duplicating an existing capability, contradicting an active
decision without superseding it, or a load-bearing choice that needs a DEC and
has none.

"I would have done it differently" is **not** a finding. Say why *this* codebase
makes the chosen way wrong, or approve it.
