# `file-lines`

Reports a file with more lines than the limit.

**Default limit:** 1000. **Default severity:** off.

## Why it is off

File length does not separate healthy from unhealthy code on its own. A large file made of small,
closely related implementations can remain coherent; a smaller file doing unrelated jobs can be hard
to change. No single line boundary distinguishes those cases.

The default limit remains available as a project-selected backstop. The distribution that informed
the disabled default lives in [`CALIBRATION.md`](../../CALIBRATION.md).

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

At 800 the rule is a broad backstop rather than a focused finding, so it is best used occasionally or
scoped to changed code.

If you enable it, pair it with `exclude` for anything generated. Generated files are frequently the
longest in a repository and there is nothing to do about them.

```toml
exclude = ["**/generated/**", "**/*.pb.rs"]
```

## Further reading

Alves, T.L., Ypma, C., Visser, J. (2010). *Deriving metric thresholds from benchmark data*.
International Conference on Software Maintenance.
