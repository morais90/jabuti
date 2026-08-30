# File lines

`file-lines`

## What it measures

Every line in a file, including blanks and comments.

## Status: off by default

This rule is computed but does not report. The reason is calibration, and it is worth recording
because the measurement is what settled it.

Across 45,361 files from 1,645 crates published on crates.io:

| p50 | p75 | p90 | p95 | p99 | max |
|---|---|---|---|---|---|
| 129 | 350 | 918 | 1677 | 4578 | 134643 |

A threshold of 400 — which looks reasonable, and which we very nearly shipped — would flag close to
a quarter of all files. To reach the same 2% report rate the other rules are calibrated for, the
limit would have to sit near 2500 lines, which is not a quality threshold in any useful sense.

The distribution is simply too dispersed for a single number to separate healthy from unhealthy. A
900-line file of small cohesive implementations is fine; a 300-line file doing four unrelated jobs
is not, and file length cannot tell them apart.

## Where its signal actually lives

The measure is still worth having. File length is an input to composite detectors — a large module
with many methods and one complex central method is a meaningful finding, and file length is one of
its terms. It reports as part of something that discriminates, rather than on its own.

## Enabling it

Set a limit and a severity if a project wants it:

```toml
[rules]
file-lines = { limit = 800, severity = "warning" }
```
