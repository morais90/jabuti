# `cyclomatic-complexity`

Reports a function with more independent execution paths than the limit.

**Default limit:** 10 in Rust and Kotlin, 13 in TypeScript.

This rule is off in the catalog and cannot be turned on from `jabuti.toml`. The measure is still
calculated and appears in `--format measures`.

See [the measure](../measures/cyclomatic-complexity.md) for how the number is calculated.

## Why it is off

Cyclomatic complexity counts independent paths but does not charge for nesting. A flat dispatch table
and deeply tangled control flow can therefore receive the same value. That makes the number useful as
an input and unreliable as a default verdict.

Rust and Kotlin carry a default limit of 10; TypeScript carries 13.
[`CALIBRATION.md`](../../CALIBRATION.md) records the distributions and the inspection that led to
leaving the rule off.

## What to use instead

Cognitive complexity is the metric designed for the question you are probably asking, which is how
hard the function is to follow rather than how many paths it has. It charges for nesting and treats
a whole `match` as a single decision, so the table above scores 1 instead of 10.

This rule is more useful as a number feeding other rules than as a gate on its own. It is still
calculated, so nothing is lost by leaving it off.

## Further reading

McCabe, T.J. (1976). *A Complexity Measure*. IEEE Transactions on Software Engineering, SE-2(4).

Shepperd, M. (1988). *A critique of cyclomatic complexity as a software metric*. Software Engineering
Journal.
