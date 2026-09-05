---
name: jabuti-code
description: Development principles for the jabuti project. Load before writing or editing any code, test, config or documentation in this repository.
---

# jabuti development principles

## Style

- Write every artifact in English: identifiers, documentation, CLI help, package descriptions, commit messages.
- Do not write comments, in any file type. Code, configuration and workflows explain themselves through naming and structure; a comment means a name is wrong or a unit does too much.
- Test fixtures are the exception: a fixture states its expected value and the derivation producing it. That annotation is the specification.
- Prefer named intermediates over combinator chains in analysis code.
- Justify a rule on its own merits. Do not cite external tools as the source of a threshold, a
  counting rule or a definition; cite primary literature when a source is needed.
- Write documentation for the person using jabuti, not about how the project is built. Explain a
  measure so a reader of any level follows it, and back it with literature rather than assertion.
- Do not use em dashes. A comma, a full stop or a pair of parentheses reads more naturally.

## Determinism

jabuti produces facts; the agent consuming it produces judgement.

- Same input and version yields byte-identical output.
- No learned models. A score with no attributable cause is not actionable.
- No wall clock, RNG or network on the fast path.
- `HashMap` iteration order reaching output is a bug. Use `BTreeMap` or sort before rendering.
- Use integer arithmetic for metrics. Parallelism must never alter output.
- Emit no prose advice. A finding is a rule, a location, a measured value and a threshold.

## Maintainability

- Keep language specifics in declarative tables and tree-sitter queries, never in imperative walkers.
- Extract with `.scm` queries. Hand-write traversal only where the algorithm carries state across the walk.
- When the dogfooding gate flags our own implementation, fix the implementation rather than raising the threshold.

## Extensibility

- Keep the three extension axes distinct: a new rule on an existing sensor, a new sensor, a new language.
- Keep each context in its own module tree, shaped like a crate of its own: `code`, `graph`, `history`
  and `tools` in both crates. A context owns its rules, its measures, its language tables, its
  queries and its tests, and reaches only the kernel (`model`, `policy`, `report`, `lang`, `syntax`),
  never another context. A test in each crate holds that boundary. Composition happens in the
  kernel of the binary, so a context can grow, be replaced or become a subcommand without touching
  the others.
- Give every measure a page under `docs/measures/` and every registered `RuleId` a page under
  `docs/rules/` plus at least one fixture. A measure page explains the number; a rule page explains
  the limit, the severity and what a reader should make of a finding.
- Calibrate a limit per language. The same number means different things in different languages, so
  a threshold measured on one corpus does not transfer.
- Treat rule IDs as public API. Deprecate with an alias; never rename silently.
- Extend in-tree and declaratively. No dynamic plugin loading.

## Scope

- Build a mechanism at the third instance, not the first.
- Add no abstraction for single-use code, no unrequested configurability, no handling for impossible states.
- Touch only what the task requires. Leave adjacent code, comments and formatting alone.
- Remove bindings your change orphaned; leave pre-existing dead code and mention it instead.
