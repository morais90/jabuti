# `parameters`

Reports a function or closure that declares more arguments than the limit.

**Default limit:** 4. **Default severity:** warning.

## What it means when it fires

```
src/partners/mod.rs:60  warning  parameters  new  measured 6, limit 4
```

There are two common causes, and they have different fixes.

The function may be doing more than one thing, with each job needing its own inputs. Splitting it
usually makes both halves take fewer arguments than the original took in total.

More often, several of those arguments belong together and have no type yet. A constructor taking a
host, a port, a timeout and a retry count is describing a connection setting. Once that type exists,
the signature shrinks and every other place passing those four values around shrinks with it.

Both readings point at the same thing from different directions, which is why the count is worth
knowing even though it says nothing about what the function does.

## Where 4 comes from

It is close to the 98th percentile of real code. Measured across 737,689 functions from 1,645 crates
published on crates.io:

| p50 | p75 | p90 | p95 | p98 | p99 |
|---|---|---|---|---|---|
| 1 | 1 | 2 | 3 | 5 | 6 |

A limit of 4 reports about 2.3% of functions, in the same band as the other rules that are on by
default.

## When to change it

```toml
[rules]
parameters = { limit = 6, severity = "warning" }
```

Raising it is reasonable in code where wide signatures are deliberate. Performance-sensitive internal
functions sometimes take many arguments specifically to avoid building a structure on a hot path, and
that is a trade-off the author made on purpose rather than an oversight.

Lowering it below 3 will start reporting ordinary code, since the 95th percentile is only 3.

## Further reading

Alves, T.L., Ypma, C., Visser, J. (2010). *Deriving metric thresholds from benchmark data*.
International Conference on Software Maintenance.
