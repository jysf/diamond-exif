# License policy — an optional dependency gate

This is the sibling of the cost-capture gate (`docs/cost-tracking.md`): a
discipline that documentation can't keep, made mechanical with a check + a CI
job. It is **opt-in and per-language** — the template core ships no license tool,
because the right tool depends on your ecosystem.

Wire it up when your project pulls in third-party dependencies and you care about
what licenses end up in your shipped artifact (constraint `license-policy` in
`guidance/constraints.yaml`).

## Why

A statically-linked binary inherits the licenses of everything compiled into it.
A single copyleft (GPL/AGPL) dependency can force you to relicense the *whole*
artifact — which silently breaks an otherwise permissive (MIT / Apache-2.0)
project, including any closed-source or commercial use. License drift is invisible
until someone audits it. The fix is the same shape as every other gate here:
declare the policy, enforce it in CI, fail the build on a violation.

## The pattern (any language)

1. **Pick the allowed set.** Usually permissive-only: `MIT`, `Apache-2.0`,
   `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Zlib`, `0BSD`, `Unicode-3.0`.
2. **Choose the tool for your ecosystem** and run it locally (a `just` recipe)
   and in CI (a job that fails on a disallowed license):

   | Ecosystem | Tool | Check command |
   |---|---|---|
   | Rust | [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) | `cargo deny check licenses` |
   | Python | [pip-licenses](https://pypi.org/project/pip-licenses/) / pip-audit | `pip-licenses --fail-on 'GPL'` |
   | Node | [license-checker](https://www.npmjs.com/package/license-checker) | `license-checker --onlyAllow 'MIT;Apache-2.0;BSD;ISC'` |
   | Go | [go-licenses](https://github.com/google/go-licenses) | `go-licenses check ./...` |

3. **Allow narrow, named exceptions** — never widen the global allow-list for one
   transitive dependency. Record *why* the exception is safe (e.g. weak copyleft
   like LGPL does not relicense a dynamically-used library; a fuzz-only transitive
   isn't in the shipped binary).

## Rust worked example (cargo-deny)

This is the concrete version of the pattern. It is **not** part of the template
core — copy it into your repo only if the project is Rust.

`deny.toml` at the repo root:

```toml
# cargo-deny — license policy enforcement.
#
# This crate is distributed under a permissive license, so the binary stays
# widely embeddable, including in closed-source/commercial products. To keep that
# promise, every dependency must carry a permissive (or an explicitly excepted)
# license. Any GPL/AGPL — or any license not in `allow` and not excepted — fails
# `cargo deny check licenses` (and the CI `licenses` job). Run locally: `just deny`.

[graph]
# Evaluate the FULL feature graph so a copyleft dependency cannot slip in behind
# an off-by-default feature flag unnoticed.
all-features = true

[licenses]
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "Zlib",
    "0BSD",
    "Unicode-3.0",
]

# Per-crate exceptions: a license OUTSIDE `allow`, knowingly accepted for ONE
# named crate. Everything else still fails — including any NEW GPL/AGPL/LGPL dep.
# Document why each exception is safe.
exceptions = [
    # { name = "some-crate", allow = ["LGPL-3.0-or-later"] },
]

# Require high confidence when matching a license file's text to an SPDX id.
confidence-threshold = 0.9
```

A `just deny` recipe (in your project's `justfile`):

```just
# Enforce the permissive license policy over the dependency tree
# (install: cargo install cargo-deny). Same check the CI `licenses` job runs.
deny:
    cargo deny check licenses
```

A CI job (in `.github/workflows/ci.yml`):

```yaml
  licenses:
    name: license policy (cargo-deny)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2
        with:
          command: check licenses
```

Then point the `license-policy` constraint's `rationale` at this doc and bump its
severity to `blocking` once the gate is live.
