# jabuti documentation

If this is your first time here, start with [concepts](concepts.md). It explains the few terms these
pages use and how to read what `jabuti check` prints.

## Measures

A measure is a number about your code, or about how that code has changed over time. On its own it says nothing about whether that number is good
or bad. You never configure a measure directly. You configure the rules that read it.

| Measure | What it counts |
|---|---|
| [Lines](measures/lines.md) | How many lines are code, comment or blank |
| [Cyclomatic complexity](measures/cyclomatic-complexity.md) | How many different ways execution can flow through a piece of code |
| [Cognitive complexity](measures/cognitive-complexity.md) | How hard a piece of code is to follow when you read it |
| [Parameters](measures/parameters.md) | How many arguments a function declares |
| [Churn](measures/churn.md) | How many commits have touched a file |

## Rules

A rule is what turns a number into something worth your attention. Each page below tells you what
the rule is looking for, what it means when it fires, and when changing the default makes sense.

Most rules read one measure. [`hotspot`](rules/hotspot.md) reads two, which is where measures start
paying for themselves: neither change frequency nor complexity says much alone, and together they
say a lot.

| Rule | Limit | Severity |
|---|---|---|
| [`cognitive-complexity`](rules/cognitive-complexity.md) | 7 | warning |
| [`hotspot`](rules/hotspot.md) | 90 | warning |
| [`function-lines`](rules/function-lines.md) | 60 | warning |
| [`parameters`](rules/parameters.md) | 4 | warning |
| [`file-lines`](rules/file-lines.md) | 1000 | off |
| [`cyclomatic-complexity`](rules/cyclomatic-complexity.md) | 10 | off |
| [`churn`](rules/churn.md) | none | off |

The last three are switched off by default. Their pages explain why, and what you gain by turning
them on if your project wants them.

## External tools

jabuti also runs the linters your project already has, folding their findings into the same output.
[External tools](tools.md) covers how to see what is available and how to turn one on.

## Configuration

Create a `jabuti.toml` in the directory you run the command from:

```toml
exclude = ["generated/**"]

[rules]
function-lines = { limit = 60, severity = "error" }
file-lines = { limit = 800, severity = "warning" }
```

Severity can be `error`, which makes the run fail, `warning`, which reports the finding but still
passes, or `off`. Anything you leave out keeps its default, so you only write down what you want to
change.
