# Languages

jabuti reads Rust, Kotlin and TypeScript. A file is matched by its extension: `.rs` for Rust, `.kt`
and `.kts` for Kotlin, and `.ts` for TypeScript.

## What a language is, and what it is not

A language here is four things at once: a grammar, a tree of units, measures over those units, and a
limit calibrated on a population of real code. All four have to be present, which is why the list is
three names long and grows slowly.

Plenty of what a repository holds has none of that. A workflow file, a container definition, a
deployment manifest and a lockfile carry no units and no measure, and there is no distribution to place
a limit on. They are still worth reading, and a rule that reads one belongs to a second class rather
than to this list: documents jabuti reads and does not measure, where the rule reads structure instead
of counting anything.

Nothing in this release reads a document. `Cargo.toml` is looked at only to decide whether a Cargo tool
applies to your project. What matters meanwhile is what the numbers mean. Language coverage counts
languages, and a document is never folded into it, because a figure mixing the two would claim more is
checked than is.

## Per-language defaults

The same number means different things in different languages, so the built-in limits are selected
per language where the measured distributions differ.

| Rule | Rust | Kotlin | TypeScript |
|---|---|---|---|
| `function-lines` | 60 | 47 | 71 |
| `cognitive-complexity` | 7 | 7 | 18 |
| `parameters` | 4 | 4 | 4 |
| `cyclomatic-complexity` (off by default) | 10 | 10 | 13 |

[`CALIBRATION.md`](../CALIBRATION.md) records the benchmark populations, distributions and dates
behind those defaults.

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

## Rule availability

Each language reports its extensions, grammar version and native-rule coverage through
`jabuti languages`. Universal rules are available across supported languages. Concept-bound rules are
available where that language maps its syntax and APIs to the required concepts. Language-specific
rules name their supported languages instead of disappearing silently.

For `error-masking`, Rust's `.unwrap()`, Kotlin's `!!` and an empty TypeScript `catch` carry the same
error-masking vocabulary. Project wrappers can be added without replacing built-in bindings:

```toml
[languages.typescript.concepts]
error-discard = ["@mycorp/errors.discard"]
```

An imported alias is resolved through the reference facts before it is compared with that path.
Resolution covers direct, aliased and namespace imports in the common case. It does not perform type
inference or follow a re-export chain.

## TypeScript grammar limits

TypeScript support covers `.ts`. TSX uses a distinct grammar and `.tsx` is not claimed as TypeScript
source in this release.

Files using syntax outside grammar 0.23.2 are reported as unreadable and contribute no measures.
The measured compatibility results live in [`CALIBRATION.md`](../CALIBRATION.md).

## Kotlin grammar limits

The Kotlin grammar we use was published in January 2025 and has not moved since. Kotlin has. Seven
constructs in current use are missing from it, and a file containing any one of them cannot be read
at all:

| Construct | Example | Introduced by |
|---|---|---|
| Guard in a `when` branch | `is Empty if active -> clear()` | Kotlin 2.1 |
| Multi-dollar string | `$$"${application.version}"` | Kotlin 2.2 |
| Collection literal in an annotation | `@Constraint(validatedBy = [])` | annotation arguments |
| Destructured lambda parameter with a type | `map { (_, second): Pair<K, V> -> second }` | destructuring |
| Annotated accessor on a property with no declared type | `val shapes @Composable get() = ...` | Compose |
| Annotated function type | `content: @Composable (() -> Unit)` | Compose |
| Soft keyword used as an identifier | `where?.let { ... }` | the language's own keyword rules |

Rejecting a file is the safe behavior, since a number computed over a tree with syntax errors would
be misleading. Every unreadable file is named in agent, JSON and SARIF output and counted in the JSON
summary. [`CALIBRATION.md`](../CALIBRATION.md) records the compatibility measurements and the grammar
comparison.

## Which version of a language

None in particular, and that is deliberate.

We parse rather than compile, so our only exposure to a language version is whether the grammar can
read the syntax. Grammars are additive: one that understands Rust 2024 reads Rust 2015 without
effort. There is no code here that supports an old version, so there is nothing to deprecate and no
support window worth declaring.

What that leaves us owing you is different, and checkable:

```console
$ jabuti languages
kotlin     .kt .kts     grammar 1.1.0    15/15 native rules
rust       .rs          grammar 0.24.2   15/15 native rules
typescript .ts          grammar 0.23.2   15/15 native rules
```

The grammar version is what actually determines whether your syntax parses, so it is the number to
quote when something does not. And when a file cannot be read, jabuti says so in its own output, and
says where the trouble starts rather than only that it failed:

```console
$ jabuti check .
No findings across 41 files and 682 units.

1 file was not measured, so nothing above accounts for it.
src/broken.rs  unreadable syntax from line 6
```

External tools are the place where versions do need a policy, since their output formats and lint
names change under us. There the rule is the current stable release and at most one before it.
