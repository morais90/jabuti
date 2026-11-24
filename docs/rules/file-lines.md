# `file-lines`

Reports a file with more lines than the limit.

**Default limit:** 1000.

This rule is off in the catalog and cannot be turned on from `jabuti.toml`. File length is still
measured and appears in `--format measures`.

## Why it is off

The rule belongs in the catalog because an agent extends the file it is in rather than opening a new
one, and a file of small functions can still be too large to hold in mind. Its limit has not been
calibrated yet, and a limit nobody can defend is worse than none: a large file of closely related
implementations can stay coherent while a smaller one doing unrelated jobs is hard to change, so the
number has to come from measured code. The rule stays off until it does, and
[`CALIBRATION.md`](../../CALIBRATION.md) will record the distribution it comes from.

Where the measure does earn its place is inside composite rules. A large module with many methods
and one complex central method is a real finding, and size is one of its terms. Reported that way,
the number contributes to something that discriminates.

## Further reading

Alves, T.L., Ypma, C., Visser, J. (2010). *Deriving metric thresholds from benchmark data*.
International Conference on Software Maintenance.
