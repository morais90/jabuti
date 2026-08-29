<div align="center">

# jabuti

**A deterministic verdict on the code your agent just wrote.**

[![CI](https://github.com/williandmorais/jabuti/actions/workflows/ci.yml/badge.svg)](https://github.com/williandmorais/jabuti/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

*Jabuti não sobe em árvore.* A tortoise does not climb trees — if one is up there, somebody put it
there. In Brazil the word became shorthand for whatever got slipped in where it does not belong,
which is exactly what this tool goes looking for.

</div>

> **Status: early.** The syntax layer and line counting work for Rust. The metric, sensor, policy
> and CLI layers are being built in that order, and there is no usable binary yet. The output shown
> below is the contract being built toward, not something you can run today.

## Why this exists

AI coding agents do not fail loudly. They fail by degrees, and the degrees are measurable.

GitClear's analysis of [623 million changes between 2023 and 2026](https://www.gitclear.com/the_ai_code_quality_maintainability_gap)
found duplicated blocks up **81%**, refactoring activity down from 21% to **3.8%**, error-masking
constructs up **47%**, and cross-file reuse down **35%**. Separately, a study of
[39 production codebases](https://arxiv.org/abs/2203.04374) found that unhealthy code carries
**15 times** more defects and takes twice as long to change.

None of that is mysterious. All of it is mechanically detectable. What is missing is a tool shaped
for the consumer: existing analyzers are built for a human reading a dashboard about a whole
repository, while an agent needs an answer about the twelve lines it just changed, in under a
second, in a form small enough to read.

## What it looks like

```console
$ jabuti check --since main
src/handler.rs:120-186  handle_request  cognitive=31 (max 15)  nesting=6 (max 4)
src/handler.rs:204-231  parse_body      duplicate-introduced (matches src/handler.rs:120-147)
2 findings on changed code
```

A clean run costs one line:

```console
$ jabuti check --since main
ok  3 files, 14 units, 0 findings
```

Exit codes separate the two failures an agent must never confuse: `0` passed, `1` a gate was
violated, `2` the tool itself broke. Output is byte-identical across runs — same input, same
version, same bytes — so it can be diffed, cached and trusted as a verification signal.

## How it works

Two families of analysis sit behind one output contract.

**Native sensors** are built here: structural metrics over [tree-sitter](https://tree-sitter.github.io/),
and process metrics mined from git history. This is where the market is weakest — every language
has its own complexity tool with its own definition, and almost nothing exposes churn, hotspots or
refactoring ratio outside a paid dashboard.

**Orchestrated sensors** are consolidated tools such as clippy and cargo-deny. jabuti provisions,
invokes and normalizes them into the same finding model. It does not reimplement them; clippy alone
contributes hundreds of Rust lints for free.

Language knowledge lives in declarative tree-sitter queries rather than imperative walkers, so a new
language is a set of query files instead of a new analyzer:

```scheme
(function_item name: (identifier) @name) @unit.function
(closure_expression) @unit.closure
```

Sensors only ever measure. Turning a measurement into a verdict happens in one place, which is what
lets a threshold change without touching analysis code — and what lets a composite like
*churn × complexity* read from two unrelated sensors with no special case.

## Principles

Three constraints shape every decision, and each one has a mechanism that fails the build when it
is violated.

**Determinism.** jabuti produces facts; the agent consuming them produces judgement. No learned
models, no wall clock, no network on the fast path, and no unordered iteration reaching output. A
score with no attributable cause is not actionable, so every finding names the thing that caused it.

**Maintainability.** Traversal and metric algorithms are unpleasant to read and become the code
nobody touches. Language specifics stay in declarative data, the algorithm is written once, and the
fixtures carry the reasoning where it can be verified.

**Extensibility.** The catalog will keep growing, so adding to it has to stay a bounded operation
for years. Rule identifiers are public API, every rule ships with documentation and a fixture, and
extension means adding data in-tree — never loading arbitrary plugins, which would buy flexibility
by giving up the other two principles.

The long form lives in [`.claude/skills/jabuti-code`](.claude/skills/jabuti-code/SKILL.md) and
[`.claude/skills/jabuti-test`](.claude/skills/jabuti-test/SKILL.md).

## Building

Requires a stable Rust toolchain, plus nightly for `rustfmt` (import grouping is a nightly option).

```console
$ just check    # format, lint, test, mutation testing, license policy
$ just test     # tests and mutation testing
$ just fmt      # apply formatting
$ just hooks    # install pre-commit hooks
```

## Contributing

Contributions are welcome, including disagreement with the principles above — they are written down
so they can be argued with.

Before opening a pull request, `just check` must pass in full. That includes mutation testing with
zero survivors: a surviving mutant means either a test is missing or the code it changed is dead,
and both are worth knowing. Commits follow [Conventional Commits](https://www.conventionalcommits.org/).

## License

Apache-2.0. See [`LICENSE`](LICENSE), and [`NOTICE`](NOTICE) for the bundled MIT dependencies.
