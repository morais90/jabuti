# `file-lines`

Reports a file with more lines than the limit.

**Default limit:** 1000.

This rule is off in the catalog and cannot be turned on from `jabuti.toml`. File length is still
measured and appears in `--format measures`.

## Why it is off

File length does not separate healthy from unhealthy code on its own. A large file made of small,
closely related implementations can remain coherent; a smaller file doing unrelated jobs can be hard
to change. No single line boundary distinguishes those cases.

The distribution that informed the limit and the decision to leave the rule off lives in
[`CALIBRATION.md`](../../CALIBRATION.md).

Where the measure does earn its place is inside composite rules. A large module with many methods
and one complex central method is a real finding, and size is one of its terms. Reported that way,
the number contributes to something that discriminates.

## Further reading

Alves, T.L., Ypma, C., Visser, J. (2010). *Deriving metric thresholds from benchmark data*.
International Conference on Software Maintenance.
