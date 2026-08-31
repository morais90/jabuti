# Languages

jabuti reads Rust and Kotlin. A file is matched by its extension: `.rs` for Rust, `.kt` and `.kts`
for Kotlin.

## Limits are calibrated per language

The same number means different things in different languages, so a threshold measured on one does
not simply carry over to another. Every limit here comes from measuring real code in that language
and taking the value that reports roughly the worst 2%.

| Rule | Rust | Kotlin |
|---|---|---|
| `function-lines` | 60 | 47 |
| `cognitive-complexity` | 7 | 7 |
| `parameters` | 4 | 4 |

The corpora behind those numbers are 737,689 functions from 1,645 crates published on crates.io, and
54,933 functions from ten established Kotlin projects.

The result is worth noticing. Only function length actually needed a different limit, and the
difference is real: Rust has a longer tail, so the same 2% report rate sits at 60 lines there and 47
in Kotlin. Cognitive complexity and parameter count landed on identical numbers in both, which is
some evidence that they measure something about programs rather than about a particular syntax.

## Setting a limit for one language

Anything under `[rules]` applies everywhere. A `[languages.<name>.rules]` section overrides it for
that language alone:

```toml
[rules]
cognitive-complexity = { limit = 10 }

[languages.kotlin.rules]
function-lines = { limit = 80 }
```

That configuration relaxes cognitive complexity for the whole project and function length for Kotlin
only, leaving Rust on its own default.

## What each language contributes

Support for a language is a grammar plus three query files describing what counts as a unit, what
counts as a comment and what counts as a decision, plus a small table for the cognitive complexity
walk. No analysis code is written per language.

That claim was tested rather than assumed. Adding Kotlin needed one change to shared code: the way
the alternative branch of a conditional is located. Rust wraps it in a node with a name; Kotlin leaves
it as an unnamed child. The fix made the shared algorithm simpler, since it now finds the branch by
position rather than by a name only one grammar uses.

## A limitation worth knowing about

Around 2.8% of files in the Kotlin corpus could not be parsed by the grammar we use, and jabuti
reports each one on stderr rather than measuring it. One cause we traced is soft keywords: an
expression like `where?.let { ... }` uses `where` as an identifier, which the grammar reads as the
keyword.

Rejecting those files is the right behaviour, since a number computed over a misparsed tree is worse
than no number. It does mean the Kotlin calibration is drawn from the files that parse, and those may
be slightly simpler than the ones that do not.

Rust files in the equivalent corpus parsed without exception.

## Adding another language

The work is a grammar crate, three query files, a cognitive table, and a corpus to calibrate against.
The last of those is the part that takes real time, and it is not optional: a language shipped with
another language's limits would be reporting a number nobody can defend.
