<div align="center">

# jabuti

**A deterministic verdict on the code your agent just wrote.**

[![CI](https://github.com/morais90/jabuti/actions/workflows/ci.yml/badge.svg)](https://github.com/morais90/jabuti/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

*Jabuti não sobe em árvore.* A tortoise does not climb trees. If one is up there, somebody put it
there. In Brazil the word became shorthand for whatever got slipped in where it does not belong,
which is exactly what this tool goes looking for.

</div>

> **Status: early but usable.** `jabuti check` reads Rust and Kotlin, reports eight rules and
> computes three more that are held back, and can fold in the linters a project already runs.

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
$ jabuti check
1 error and 0 warnings across 42 files and 378 units.

src/handler.rs:120  error  function-lines  handle_request  measured 71, limit 60
```

A clean run costs one line:

```console
$ jabuti check
No findings across 42 files and 378 units.
```

Every finding names its severity, the rule, where it is, what was measured and what the limit was,
which is enough to act on without asking a follow-up question. There is no advice in the output:
the rule name carries the meaning, and deciding what to do about it is the caller's job.

Exit codes separate the two failures an agent must never confuse: `0` passed, `1` a gate was
violated, `2` the tool itself broke. Output is byte-identical across runs, so the same input on the same
version produces the same bytes and the result can be diffed, cached and trusted.

## Scoping to a change

A run over a whole repository reports code nobody just wrote, and on a mature codebase that is most
of it. Handing an agent 800 findings for twelve changed lines is worse than handing it nothing: it
cannot tell which ones it caused, so it learns to ignore the output.

```console
$ jabuti check --since main
```

Only changed files are analysed, and within them only findings overlapping a changed line are
reported. Uncommitted edits and untracked files count as changed, because that is the state an agent
is usually in.

This is also what makes an absolute threshold adoptable. A legacy function that was already over the
limit stays quiet until someone touches it, so a project can turn the gate on today rather than
after a cleanup it will never schedule.

## Asking what a change reaches

A gate answers whether the code is acceptable. The other useful question is what it is wired to, and
an agent needs that answer before it edits rather than after:

```console
$ jabuti graph impact --since main
1 file changed, 5 files reached.

crates/jabuti-cli/src/git.rs
  crates/jabuti-cli/src/churn.rs
  crates/jabuti-cli/src/graph.rs
  crates/jabuti-cli/src/main.rs
  crates/jabuti-cli/src/scan.rs
  crates/jabuti-cli/src/since.rs
```

Nothing is judged and nothing fails. It is the same dependency graph read in the other direction, as
context rather than a verdict. The graph also holds a boundary when you declare one: name the layers
and what each may depend on, and a crossing is reported at the line that made it.

The dependencies are found wherever they are written, not only in the import list. This repository
is its own example: no file contains `use crate::git`, yet two of them call `crate::git::run` on the
line that uses it, and in Kotlin a file needs no import at all to use its own package. Reading only
imports would miss both, and a missing dependency is the expensive kind of mistake here.
[`docs/concepts.md`](docs/concepts.md) says what the graph can and cannot see.

## Thresholds are measured, not asserted

A threshold nobody can defend gets disabled the first time it is wrong. Ours are drawn from the
distribution of real code: 1,645 crates published on crates.io, 45,361 files, 737,689 functions.

`function-lines` is capped at 60 and `cognitive-complexity` at 7, both at the 98th percentile. The
claim each makes is that the code it points at is unusual by the standard of published Rust, and
that is a claim we can show.

The same measurement is why two rules ship switched off. Cyclomatic complexity scores 1 for three
quarters of all Rust functions, and what lands above any threshold that fires is dominated by flat
exhaustive matches, which are lookup tables that read at a glance. File length is too dispersed for
a single number to separate healthy from unhealthy. Both are still computed, because their signal
survives inside composites even though it does not survive alone.

We checked rather than assumed. On one real project cyclomatic complexity flagged six functions and
four were lookup tables. At the same percentile, cognitive complexity flagged one function, and that
function genuinely was the hardest to follow in the repository.

Each rule and its calibration is written up under [`docs/`](docs).

## Installing

```console
$ just install
```

Configuration is optional. When a `jabuti.toml` sits in the working directory it adjusts limits and
severities:

```toml
exclude = ["generated/**"]

[rules]
function-lines = { limit = 60, severity = "error" }
```

Severity is `error` (fails the gate), `warning` (reported, exit 0) or `off`. Nothing defaults to
`error`, because without diff scoping an absolute gate fails on the first legacy file it meets.
Opting in is the project's decision, not ours.

## How it works

Two families of analysis sit behind one output contract.

**Native sensors** are built here: structural metrics over [tree-sitter](https://tree-sitter.github.io/),
and process metrics mined from git history. This is where the market is weakest. Every language
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
lets a threshold change without touching analysis code, and what lets a composite like
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
extension means adding data in-tree, never loading arbitrary plugins, which would buy flexibility
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

Contributions are welcome, including disagreement with the principles above. They are written down
so they can be argued with.

Before opening a pull request, `just check` must pass in full. That includes mutation testing with
zero survivors: a surviving mutant means either a test is missing or the code it changed is dead,
and both are worth knowing. Commits follow [Conventional Commits](https://www.conventionalcommits.org/).

## License

Apache-2.0. See [`LICENSE`](LICENSE), and [`NOTICE`](NOTICE) for the bundled MIT dependencies.
