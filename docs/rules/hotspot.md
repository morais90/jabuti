# `hotspot`

Notes a file that is both complex and frequently changed. This is context: it is shown beside the
findings and never fails the run.

**Default limit:** 90.

## The idea

Complexity on its own is a weak signal, and change frequency on its own is not a signal at all. A
file can be tangled and completely stable, in which case nobody is paying for it. A file can change
every day and be trivial, in which case nobody is struggling with it.

The combination is different. Code that is hard to understand *and* keeps needing changes is where
time disappears and where mistakes get made, because every change is a change someone had to reason
through.

This is the oldest idea in the catalog that is not about a single number, and the evidence behind it
is the strongest. Studies pairing version control history with code quality across production
codebases keep finding the same thing: the intersection is what predicts cost, not either half.

## What the number means

Both halves are ranked within your repository rather than measured against an absolute scale, and
the score is the **lower of the two rankings**.

```
src/syntax.rs:1  hotspot  measured 92, limit 90
```

That reads as: this file is above the 92nd percentile on both change frequency and complexity,
compared with the other files jabuti analysed here.

Taking the lower of the two is what makes it a conjunction. A file in the busiest 1% but with median
complexity scores around 50, because 50 is the lower half of the pair. Only a file that is high on
both ends up with a high score.

## Why it is ranked and not measured

A commit count does not transfer between repositories. Twelve commits is a lot in a project three
months old and nothing in one that has been running a decade, so no absolute number works for both.
Ranking sidesteps that entirely: being in the busiest tenth of your own repository means the same
thing everywhere.

## What to do with one

A hotspot does not point at a specific thing to fix. It points at a place worth spending attention
on, which is why it is reported as context.

A useful next step is to look at the file's other findings. A hotspot that is also over the cognitive
complexity limit tells you exactly which function to start with. A hotspot with no other findings is
usually a file that has grown a lot of small, correct, unrelated pieces, and splitting it is the
change that pays.

## Requirements and limits

It needs a git repository. Outside one, jabuti says so on stderr and carries on without it rather
than failing the run.

The ranking always covers the whole repository, because a ranking computed over the three files you
happened to change would be meaningless. A check that judges a change shows it for the files the
change touched, which is when it is worth knowing: you are working in a place where time goes.
`--all` shows it for every file that ranks above the limit.

A repository with very few files cannot produce a meaningful ranking, and a single file never scores
above zero.

## It never fails a run

A hotspot is not a defect, and failing a build because a file is in a busy part of the repository
would punish people for working where the work is.

## Further reading

Tornhill, A., Borg, M. (2022). *Code Red: The Business Impact of Code Quality*. International
Conference on Technical Debt. Pairs change frequency with code quality across 39 production
codebases.

Rahman, F., Devanbu, P. (2013). *How, and why, process metrics are better*. International Conference
on Software Engineering.
