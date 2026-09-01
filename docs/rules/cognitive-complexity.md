# `cognitive-complexity`

Reports a function that is harder to follow than the limit allows.

**Default limit:** 7. **Default severity:** warning.

See [the measure](../measures/cognitive-complexity.md) for how the score is built.

## What it means when it fires

```
tests/state_container.rs:98  warning  cognitive-complexity  derive_account_id  measured 13, limit 7
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

## Where 7 comes from

It is the 98th percentile of published Rust libraries. Measured across 737,689 functions from 1,645
crates published on crates.io:

| p50 | p75 | p90 | p95 | p98 | p99 |
|---|---|---|---|---|---|
| 0 | 0 | 1 | 3 | 7 | 12 |

Four Rust functions in five score zero. A limit of 7 reports about 1.8% of functions, which is the
same rate [`function-lines`](function-lines.md) is calibrated to.

## This limit travels further than the others

A percentile drawn from libraries does not automatically describe an application, and for
[`function-lines`](function-lines.md) the difference is large. For this rule it is small. The share
of functions reported at a limit of 7:

| Corpus | Kind | Reported |
|---|---|---|
| 1,645 crates published on crates.io | library | 1.8% |
| kotlinx.coroutines | library | 3.2% |
| okhttp | library | 2.4% |
| DuckDuckGo Android | application | 1.4% |
| hyperswitch | application | 2.1% |
| komga | application | 3.5% |
| Signal-Android | application | 4.3% |
| meilisearch | application | 6.5% |

Two languages, libraries and applications, and the rate stays between 1.4% and 6.5%. Over the same
eight corpora, function length ranges from 1.1% to 11.4%.

That is worth more than it looks. A threshold that holds its meaning across populations is measuring
something about programs; one that moves with the kind of project is partly measuring the kind of
project. It is a second reason to trust this rule over the ones it sits beside.

## Why this is the rule worth trusting

We checked what it reports rather than assuming. On one real project, cyclomatic complexity flagged
six functions and four of them were flat lookup tables that read at a glance. On the same project at
the same percentile, cognitive complexity flagged one function, and that function genuinely was the
hardest to follow in the repository.

That difference is the reason this rule is on by default and
[`cyclomatic-complexity`](cyclomatic-complexity.md) is not.

## Changing it

```toml
[rules]
cognitive-complexity = { limit = 7, severity = "error" }
```

This is the rule most worth promoting to `error`, especially alongside `--since`. Scoped to changed
code, the claim becomes "you touched this and it is harder to follow than 98% of published Rust",
which is specific enough to act on and rare enough not to be in the way.

Raising it to 10 or 12 is reasonable if you are adopting it on an existing codebase and want to start
with the worst cases. Below about 5 you will start reporting ordinary code, since the 95th percentile
is only 3.

## Further reading

Campbell, G.A. (2018). *Cognitive Complexity: A new way of measuring understandability*.

Muñoz Barón, M., Wyrich, M., Wagner, S. (2020). *An Empirical Validation of Cognitive Complexity as
a Measure of Source Code Understandability*. Empirical Software Engineering and Measurement.
