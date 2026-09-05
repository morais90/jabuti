# External tools

jabuti does not reimplement linters. It runs the ones your project already has and folds their
output into the same shape as everything else: one line per finding, one exit code, the same
`--since` scoping.

## Seeing what is available

```console
$ jabuti tools
clippy           enable with [tools.clippy] enabled = true
cargo-llvm-cov   enable with [tools.cargo-llvm-cov] enabled = true
```

Every tool has three independent states, and running requires all three.

| State | Meaning |
|---|---|
| Applicable | Your project has the marker file the tool needs, such as `Cargo.toml` for clippy |
| Available | The command answered when jabuti asked for its version |
| Enabled | Your `jabuti.toml` turned it on |

The output tells you which one is missing and what to do about it, so a tool never quietly does
nothing.

## Turning one on

```toml
[tools.clippy]
enabled = true
```

Findings then appear alongside the rest:

```console
$ jabuti check
1 error and 0 warnings across 28 files and 387 units.

src/external.rs:65  error  clippy/struct_field_names  field name starts with the struct's name
```

The identifier is `<tool>/<lint>`, which is what you use to adjust or silence it:

```toml
[rules]
"clippy/struct_field_names" = { severity = "off" }
```

## Installing missing tools

```console
$ jabuti tools install
Installed clippy.
Installed cargo-llvm-cov.
```

Successful lines follow registry order. When nothing needs installation, the command prints
`No tools need installation.`

The install action takes no tool names. It provisions every registered tool that is applicable and
unavailable, whether or not `[tools.*].enabled` is true. It does not enable a tool or write
`jabuti.toml`. Bare `jabuti tools` remains a read-only status listing, and `jabuti check` never
provisions or downloads anything.

Each registry entry carries a curated, exact sequence of programs and arguments. jabuti starts them
directly, never through a shell:

- Clippy: `rustup component add clippy`.
- `cargo-llvm-cov`: `cargo install cargo-llvm-cov --version 0.9.0 --locked`, then
  `rustup component add llvm-tools-preview`.

The rustup commands run in the project directory, so they install components for its active
toolchain. These commands change tool availability only; runtime analyzer settings still come from
the project. Tools that are already available or do not apply are left alone. If an installer cannot
start, exits unsuccessfully or leaves the tool unavailable, `jabuti tools install` stops and exits 2.

## Your configuration is the configuration

jabuti runs the tool in your project directory, so it reads your `clippy.toml`, your `[lints]`
section and your `#![allow]` attributes exactly as it would if you ran it yourself. A lint your
project has deliberately allowed stays allowed.

Severity comes from the tool. If clippy calls something an error in your project, jabuti reports an
error. jabuti never passes `-D warnings`, because doing so would replace your project's judgement
with ours.

You can still override any individual lint through `[rules]`, which is the same mechanism that
adjusts jabuti's own rules.

## Why they are off by default

Native measures read your source and finish in milliseconds. Clippy compiles your crate, which takes
seconds at best and considerably longer on a cold cache.

`jabuti check` is meant to be fast enough to run after every change, so anything that slow has to be
a decision you make rather than a surprise you discover. Turning it on is a line in a file that lives
in your repository, so the choice is shared with everyone working in it.

## Two kinds of output

A tool produces one of two things.

**Diagnostics.** Clippy prints findings, and jabuti folds each one into the report as
`clippy/<lint>`, next to its own rules. This is the shape of a linter: the tool already knows what
is wrong, and jabuti's job is to carry that verdict in the same line format, with the same `--since`
scoping and the same `[rules]` overrides.

**A file a rule reads.** `cargo-llvm-cov` prints nothing you see. It runs the project's tests under
instrumentation and writes an LCOV report to `target/jabuti/coverage.lcov`, and the native rule
[`uncovered-new-code`](rules/uncovered-new-code.md) reads that file to find new lines no test
reached. The tool knows how to produce coverage for one ecosystem; the rule knows what to make of it
for any language, because LCOV and JaCoCo XML say the same thing whoever wrote them. A project that
already produces coverage in its build can skip the tool and name its own report under `[coverage]`.

When jabuti itself runs inside an instrumented test run (its own suite under `cargo llvm-cov` is the
case that matters), the enclosing run has installed itself as the compiler wrapper for every child
process. A tool launched with that environment intact would call the wrapper, which compiles its
target under the wrapper again, without end. jabuti drops the inherited wrapper before launching any
tool, so a nested producer builds and tests the project it was pointed at, and nothing else.

Either way the tool is off by default, for the reason above, and turning it on is the same line in
`jabuti.toml`.

## Adding a tool

The registry holds clippy and `cargo-llvm-cov`. A tool is described by the marker that makes it
applicable, the command that probes it, the command that runs it, and what it produces: diagnostics
to fold into findings, or a file at a known path that a rule reads. Cargo-based tools share the
diagnostic format, so the next Rust linter is mostly a matter of declaring it.
