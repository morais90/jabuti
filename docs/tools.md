# External tools

jabuti does not reimplement linters. It runs the ones your project already has and folds their
output into the same shape as everything else: one line per finding, one exit code, the same
`--since` scoping.

## Seeing what is available

```console
$ jabuti tools
clippy     enable with [tools.clippy] enabled = true
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

## Nothing is installed for you

If a tool is applicable but not available, jabuti tells you the command to install it and carries on
without it. It does not download or install anything itself.

For clippy that is not a limitation, it is correctness. Clippy is a component of your Rust toolchain
and has to match your compiler version. A version pinned by us independently of your toolchain would
be the wrong version.

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

## Adding a tool

The registry currently holds clippy. A tool is described by the marker that makes it applicable, the
command that probes it, the command that runs it, and how to read its output. Cargo-based tools all
share the diagnostic format, so the next Rust tool is mostly a matter of declaring it.
