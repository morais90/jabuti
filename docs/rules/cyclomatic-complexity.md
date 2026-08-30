# `cyclomatic-complexity`

Reports a function with more independent execution paths than the limit.

**Default limit:** 10. **Default severity:** off.

See [the measure](../measures/cyclomatic-complexity.md) for how the number is calculated.

## Why it is off

Not because the number is wrong. Because in Rust it points at the wrong things.

Across 737,499 functions from 1,645 crates published on crates.io:

| p50 | p75 | p90 | p95 | p99 |
|---|---|---|---|---|
| 1 | 1 | 2 | 4 | 10 |

Three quarters of Rust functions score 1. They do not branch. A distribution that flat means the
threshold has to sit far out to fire at all, and when you look at what does land out there, most of
it is this:

```rust
fn normalized(method: &Method) -> &'static str {
    match method.as_str() {
        "GET" => "GET",
        "POST" => "POST",
        // eight more arms
    }
}
```

That scores 10. It is a lookup table, and it reads at a glance. Rust reaches for exhaustive matching
where other languages reach for a hash map, so the arms pile up as paths without piling up as
difficulty.

We checked this on a real codebase rather than assuming it. Of the functions this rule reported,
most were flat tables of exactly this shape. A rule that is mostly false alarms teaches people to
skim past everything jabuti prints, and that is a worse outcome than not having the rule.

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
