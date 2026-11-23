# `cognitive-complexity`

Reports a function that is harder to follow than the limit allows.

**Default limit:** 7 in Rust and Kotlin, 18 in TypeScript.

See [the measure](../measures/cognitive-complexity.md) for how the score is built.

## What it means when it fires

```
tests/state_container.rs:98  cognitive-complexity  derive_account_id  measured 13, limit 7
```

Unlike a length finding, this one is usually specific about what to look at. A high score comes from
depth, so the function almost always contains a loop wrapping conditionals, or conditionals wrapping
conditionals. Finding the deepest part of the function is normally finding the problem.

Here is a real example that scored 13:

```rust
for line in stdout.lines() {
    if let Some(rest) = line.strip_prefix("window Uid:") {
        assert!(...);
    }
    if let Some(rest) = line.strip_prefix("window Gid:") {
        assert!(...);
    }
    // three more of the same shape
}
```

Five conditionals inside one loop. Each is simple on its own, but a reader has to hold five prefixes
and five expectations at once. The fix that suggests itself, a table of prefix to expected value
driving one loop, is usually the fix the score is pointing at.

## Calibration

Rust and Kotlin use 7; TypeScript uses 18. [`CALIBRATION.md`](../../CALIBRATION.md) records the
populations, distributions and report rates.

## Why this rule is on

Cognitive complexity charges for nesting and discounts flat dispatch, so its finding points more
directly at reading effort than a path count. This is why it is enabled while
[`cyclomatic-complexity`](cyclomatic-complexity.md) is not.

## Further reading

Campbell, G.A. (2018). *Cognitive Complexity: A new way of measuring understandability*.

Muñoz Barón, M., Wyrich, M., Wagner, S. (2020). *An Empirical Validation of Cognitive Complexity as
a Measure of Source Code Understandability*. Empirical Software Engineering and Measurement.
