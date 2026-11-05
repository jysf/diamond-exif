---
# Hand-crafted red-proof fixture for SPEC-016 (AC1/AC2). NOT a real spec.
#
# Every required spec key below is present and line-greppable with valid values,
# so the OLD grep-only `validate` (required-keys check, no parse) reports it
# "valid required front-matter". The `value_link:` scalar opens a double-quoted
# string, wraps onto further lines, and embeds an UNESCAPED quote — the exact
# shape `handback-sync` wrote into the shipped-then-archived artifact and the
# live HANDOFF-039 break (`did not find expected key`). So a real YAML parser
# rejects it. grep-satisfiable, parse-broken: the one case the parse gate adds.
#
# This fixture lives under tests/oracle-fixtures/ (outside the parse gate's
# projects/ + decisions/ scope), so it never trips the live `just validate`.
# scripts/validate-red-proof.sh copies it into a throwaway repo to exercise the
# gate. Keep the break intact — repairing it defeats the proof.
task:
  id: SPEC-900
  type: story
  cycle: build
  complexity: S
project:
  id: PROJ-001
  stage: STAGE-005
repo:
  id: diamond-exif
value_link: "this note opens a quoted scalar and wraps onto
  a second line, then embeds "an unescaped quote" which closes
  the scalar early and leaves trailing words no parser can place"
---
# SPEC-900: unparseable-front-matter fixture

If a YAML parser can read the front matter above, this fixture has stopped
testing what it exists to test — see scripts/validate-red-proof.sh and SPEC-016.
