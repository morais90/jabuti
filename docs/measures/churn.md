# Churn

Counts how many commits have touched a file.

This is the first measure that does not come from reading the code. It comes from the history of the
repository, which turns out to say things about a file that the file itself cannot.

## Why history is worth reading

Static measures describe what the code is. History describes what has happened to it, and for
predicting where the next defect will appear, the second turns out to be the stronger signal.

The finding is not new and it is fairly robust. Studies comparing metrics computed from source
against metrics computed from version control have repeatedly found the version control ones ahead,
with change frequency in particular outperforming complexity for predicting fault density.

The intuition is not complicated. Complex code that nobody has touched in three years is not costing
anyone anything. Complex code that changes every week is where the time goes, and where the mistakes
land.

## What counts

One commit that touched the file counts once, however many lines it changed. A commit that reformats
two thousand lines and a commit that fixes a typo both count as one.

That choice is deliberate. Counting changed lines instead would let a single reformatting pass
dominate the measure, and reformatting says nothing about how often a file is actually being
reasoned about.

Renames are not followed. A file that was moved starts its history over, and until rename following
is implemented the number for a recently moved file will be lower than the truth.

## What it does not tell you on its own

An absolute number of commits does not transfer between repositories. Twelve commits is a lot in a
project three months old and unremarkable in one that has been running for a decade. There is no
percentile we can quote here the way we can for the source measures, because the distribution
depends on the age and activity of the repository rather than on the language.

This is why the [`churn`](../rules/churn.md) rule is off by default, and why the measure is more
useful as one half of a comparison than as a threshold. A file being in the busiest tenth of *this*
repository is meaningful. Twelve commits is not.

Two things would make it directly thresholdable, and both are on the roadmap: restricting the count
to a window anchored to the latest commit, which normalises for repository age, and combining it with
a complexity measure, which is where the signal actually lives.

## Further reading

Rahman, F., Devanbu, P. (2013). *How, and why, process metrics are better*. International Conference
on Software Engineering. The comparison that established process metrics as the stronger predictor.

Tornhill, A., Borg, M. (2022). *Code Red: The Business Impact of Code Quality*. International
Conference on Technical Debt. Combines change frequency with code health across 39 production
codebases.
