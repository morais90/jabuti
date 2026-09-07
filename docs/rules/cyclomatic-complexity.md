# `cyclomatic-complexity`

Reports a function with more independent execution paths than the limit.

**Default limit:** 10 in Rust and Kotlin, 13 in TypeScript. **Default severity:** off.

See [the measure](../measures/cyclomatic-complexity.md) for how the number is calculated.

## Why it is off

Cyclomatic complexity counts independent paths but does not charge for nesting. A flat dispatch table
and deeply tangled control flow can therefore receive the same value. That makes the number useful as
an input and unreliable as a default verdict.

Rust and Kotlin carry a default limit of 10; TypeScript carries 13. The rule remains available when a
project wants a path-count backstop. [`CALIBRATION.md`](../../CALIBRATION.md) records the distributions
and the inspection that led to the default severity.

## What to use instead

Cognitive complexity is the metric designed for the question you are probably asking, which is how
hard the function is to follow rather than how many paths it has. It charges for nesting and treats
a whole `match` as a single decision, so the table above scores 1 instead of 10.

Until that lands, the honest answer is that this rule is more useful as a number feeding other
rules than as a gate on its own. It is still calculated, so nothing is lost by leaving it off.

## Turning it on

It behaves better in code with real branching logic, such as a parser or a state machine, and worse
in code that dispatches on an enumeration.

```toml
[rules]
cyclomatic-complexity = { limit = 15, severity = "warning" }
```

Starting at 15 rather than 10 avoids most of the flat tables while still catching functions with
genuinely tangled control flow. Scoping to changed code with `--since` helps a lot here too, since
the false alarms tend to be in stable code that nobody is editing.

## Further reading

McCabe, T.J. (1976). *A Complexity Measure*. IEEE Transactions on Software Engineering, SE-2(4).

Shepperd, M. (1988). *A critique of cyclomatic complexity as a software metric*. Software Engineering
Journal.
