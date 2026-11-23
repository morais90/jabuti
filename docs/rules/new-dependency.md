# `new-dependency`

Notes a file that now depends on another file it did not depend on before. This is context: it is
shown beside the findings and never fails the run.

Every new dependency is noted. "New" needs an earlier revision to compare against, so the rule
runs only when a check judges a change, never under `--all` or outside git.

## The idea

Most of what makes a system hard to change is not inside any one file, it is the web of connections
between them. That web grows one edge at a time, and each edge on its own looks reasonable. Nobody
decides to couple the payment code to the reporting code; someone needs a formatter, imports it, and
the coupling arrives as a side effect of a five line change.

Each note is a fact you would otherwise have to go looking for:

```
src/report.rs:2  new-dependency  now depends on src/git.rs
```

The report module used to be self-contained and now runs git. That may be exactly right, and it may
be the moment someone should have passed the value in instead.

## Why this is worth a rule of its own

Reviewing a diff shows you the lines that changed. It does not show you that the change altered the
shape of the system, because the shape is not in the diff, it is in the difference between two
graphs. So a new coupling reads as one more `use` line among twenty and passes without comment.

This matters more with a model writing the code. Reaching for what is available is the path of least
resistance, and a model has no sense of which side of a boundary it is standing on. Passing a value
in as an argument requires knowing that the boundary exists and is worth keeping. Importing does not.

## Why it does not fail the run

A new dependency is not a defect by itself. Sometimes it is the right design, and nothing in the
code says which case this is. Failing the run on every new import would leave an agent one way out,
which is not to write the dependency even when it is right.

A dependency is a defect when the project has said so. Declare the layers, and a dependency that
crosses one the wrong way is reported by [`layer-violation`](layer-violation.md), which fails the run.

## What counts as a dependency

Whatever names something the other file declares, wherever it is written. That is more than one
import shape: a Rust path may be written at its use site or inside a macro, Kotlin may use a bare name
from the same package, and TypeScript may spell a relative module without an extension or with the
`.js` extension it will have after compilation. [`docs/concepts.md`](../concepts.md) describes the
graph this reads and, just as importantly, what it cannot see.

## What is deliberately not noted

**A file this change created.** Every one of its dependencies is new by definition, so reporting them
would bury the signal under the noise of ordinary new code.

**A dependency that only moved.** The rule compares the set of files a file reaches, so rewriting the
same dependency in another form is not a finding.

## Calibration

The rule has no numerical threshold: any dependency added to an existing file is one note. The
commit-history probe used to validate its noise level lives in
[`CALIBRATION.md`](../../CALIBRATION.md).
