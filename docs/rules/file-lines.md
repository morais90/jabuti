# `file-lines`

Reports a file with more lines than the limit.

**Default limit:** 1000. **Default severity:** off.

## Why it is off

The measurement did not support turning it on. Across 45,361 files from 1,645 crates published on
crates.io:

| p50 | p75 | p90 | p95 | p99 |
|---|---|---|---|---|
| 129 | 350 | 918 | 1677 | 4578 |

A limit of 400, which sounds sensible and was very nearly the default, would report close to a
quarter of all files. To report as rarely as `function-lines` does, the limit would have to sit
around 2500 lines, and a rule that only fires above 2500 is not saying anything useful.

The deeper problem is that file length does not separate healthy from unhealthy on its own. A
900 line file made of small, closely related implementations is fine. A 300 line file doing four
unrelated jobs is not. Length cannot tell those apart, and no single number will.

Where the measure does earn its place is inside composite rules. A large module with many methods
and one complex central method is a real finding, and size is one of its terms. Reported that way,
the number contributes to something that discriminates.

## Turning it on

There is a reasonable case for it as a backstop, catching the genuinely extreme file rather than
grading everything:

```toml
[rules]
file-lines = { limit = 800, severity = "warning" }
```

At 800 you are looking at roughly the largest 10% of files, which is a lot for a warning you intend
to act on and about right for something you check occasionally.

If you enable it, pair it with `exclude` for anything generated. Generated files are frequently the
longest in a repository and there is nothing to do about them.

```toml
exclude = ["**/generated/**", "**/*.pb.rs"]
```

## Further reading

Alves, T.L., Ypma, C., Visser, J. (2010). *Deriving metric thresholds from benchmark data*.
International Conference on Software Maintenance.
