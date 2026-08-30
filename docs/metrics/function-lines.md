# Function lines

`function-lines`

## What it measures

Every line a function spans, from its signature to its closing brace, including the blank and
comment lines inside it. Not a count of statements — a count of how much there is to read before
reaching the end.

## Attribution

A function is measured over its own span. A function declared inside another is measured on itself
and also counts toward its container, because its lines really are part of what a reader has to
scroll past.

## Threshold

The default maximum is **60**, and this is the only rule enabled by default.

It was calibrated, not chosen. Measuring 737,689 functions across 1,645 crates published on
crates.io gives this distribution:

| p50 | p75 | p90 | p95 | p99 | max |
|---|---|---|---|---|---|
| 6 | 10 | 21 | 34 | 86 | 4100 |

A limit of 60 sits at roughly the 98th percentile: it flags about 2% of real functions. The
threshold therefore makes a claim we can defend — *this function is longer than 98% of the Rust
written in public* — rather than asserting that some number is intrinsically bad.

The calibration matters more than it looks. Thresholds borrowed from other ecosystems commonly sit
near 25 lines, which lands around the 92nd percentile here and would flag one function in twelve.
Rust functions are short: half of them are six lines or fewer.

## Known limitation

The corpus includes generated code, which lengthens the tail. It does not move the percentiles the
threshold is drawn from, but it inflates the maximum.

Test functions legitimately run longer than production ones, and the threshold does not currently
know the difference. Until it does, a project with long integration tests should expect findings
there first.
