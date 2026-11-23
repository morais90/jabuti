# jabuti documentation

If this is your first time here, start with [concepts](concepts.md). It explains the few terms these
pages use and how to read what `jabuti check` prints.

jabuti reads Rust, Kotlin and TypeScript. [Languages](languages.md) covers what each one contributes,
which rules are available and why their limits differ.

[Principles](principles.md) answers the other kind of question: why a default sits where it does, why
a check you expected is missing, and why the output has the shape it has.

[The calibration record](../CALIBRATION.md) holds the populations, distributions and dates that every
default limit was drawn from, and says what a percentile in it does and does not claim.

## Output formats

`jabuti check` writes a grouped, colored report to a terminal and compact agent output anywhere else.
[Other output shapes](concepts.md#other-shapes-of-output) cover both, and the JSON findings, raw
measures and SARIF for programmatic consumers.

## Measures

A measure is a number about your code, or about how that code has changed over time. On its own it says nothing about whether that number is good
or bad. Rules read measures and compare them against the limits jabuti sets.

| Measure | What it counts |
|---|---|
| [Lines](measures/lines.md) | How many lines are code, comment or blank |
| [Cyclomatic complexity](measures/cyclomatic-complexity.md) | How many different ways execution can flow through a piece of code |
| [Cognitive complexity](measures/cognitive-complexity.md) | How hard a piece of code is to follow when you read it |
| [Parameters](measures/parameters.md) | How many arguments a function declares |
| [Churn](measures/churn.md) | How many commits have touched a file |
| [Duplication](measures/duplication.md) | Which regions of code have the same structure as a region elsewhere |

## Rules

A rule is what turns a number into something worth your attention. Each page below tells you what
the rule is looking for, what it means when it fires, and why its limit sits where it does.

Most rules read one measure. [`hotspot`](rules/hotspot.md) reads two, which is where measures start
paying for themselves: neither change frequency nor complexity says much alone, and together they
say a lot.

Every rule belongs to a [family](principles.md#families), which names the way a change fails rather
than the area of software it touches. Reading the catalog by family is usually faster than reading it
by name.

| Rule | Family | Limit | Runs |
|---|---|---|---|
| [`cognitive-complexity`](rules/cognitive-complexity.md) | `shape` | 7 | always |
| [`duplicate-block`](rules/duplicate-block.md) | `shape` | 120 nodes | always |
| [`error-masking`](rules/error-masking.md) | `silent-failure` | none | always |
| [`function-lines`](rules/function-lines.md) | `shape` | 60 | always |
| [`parameters`](rules/parameters.md) | `shape` | 4 | always |
| [`suppression`](rules/suppression.md) | `silenced` | none | always |
| [`assertion`](rules/assertion.md) | `completeness` | none | always |
| [`layer-violation`](rules/layer-violation.md) | `structure` | none | when layers are declared |
| [`speculative-api`](rules/speculative-api.md) | `shape` | none | on a change |
| [`uncovered-new-code`](rules/uncovered-new-code.md) | `completeness` | none | on a change, with a coverage report |
| [`file-lines`](rules/file-lines.md) | `shape` | 1000 | off |
| [`cyclomatic-complexity`](rules/cyclomatic-complexity.md) | `shape` | 10 | off |

Every finding is something to fix, and any finding fails the run. Rules marked "on a change" compare
against an earlier revision, so they skip `--all` and repositories without git. The last two are off
and cannot be turned on; their measures still appear in `--format measures`, and their pages explain
why.

Two rules report context instead of findings. Context is shown beside the findings and never fails
the run, because it describes something worth knowing that is not a defect on its own.

| Rule | Family | Limit | Runs |
|---|---|---|---|
| [`hotspot`](rules/hotspot.md) | `cost` | 90 | with git; on a change, for the files it touched |
| [`new-dependency`](rules/new-dependency.md) | `structure` | none | on a change |

## External tools

jabuti also runs the linters your project already has, folding their findings into the same output.
[External tools](tools.md) covers how to see what is available and how to turn one on.

## Configuration

Create a `jabuti.toml` at the root of the project. It is found from any directory below it within
the same repository, and every path shown, every `exclude` pattern and every layer path is relative to
the directory holding it. A path given on the command line that lies outside that project is refused,
and one that does not exist is an error rather than an empty report:

```toml
exclude = ["generated/**"]

[coverage]
report = "target/coverage/lcov.info"

[layers]
domain = { paths = ["src/domain/**"], depends_on = [] }
```

The file holds what only the project knows: `exclude`, which tools run under `[tools.<name>]`, the
coverage report under `[coverage]`, the layers under `[layers]`, and the project's own wrappers under
`[languages.<name>.concepts]`. Which rules run and where their limits sit are fixed by jabuti. A
`[rules]` or `[languages.<name>.rules]` table stops the run with exit `2` and says why.
