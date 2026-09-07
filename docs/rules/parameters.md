# `parameters`

Reports a function that declares more arguments than the limit.

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

## Calibration

Rust, Kotlin and TypeScript currently use 4. An explicit TypeScript `this` parameter identifies the
receiver and is excluded. [`CALIBRATION.md`](../../CALIBRATION.md) records the measured distributions
and report rates.

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
