# `new-dependency`

Reports a file that now depends on another file it did not depend on before.

**Default severity:** warning. This rule has no limit; every new dependency is reported. It needs
`--since`, because "new" only means anything against an earlier revision, and it says so rather than
staying quiet when you leave the flag out.

## The idea

Most of what makes a system hard to change is not inside any one file, it is the web of connections
between them. That web grows one edge at a time, and each edge on its own looks reasonable. Nobody
decides to couple the payment code to the reporting code; someone needs a formatter, imports it, and
the coupling arrives as a side effect of a five line change.

A finding here is not a defect. It is a fact you would otherwise have to go looking for:

```
src/report.rs:2  warning  new-dependency  now depends on src/git.rs
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

## What counts as a dependency

Whatever names something the other file declares, wherever it is written. That is more than the
import list: a path spelled out where it is used, a path relative to a module in scope, a path inside
a macro, and in Kotlin a bare name from the same package, which needs no import at all.
[`docs/concepts.md`](../concepts.md) describes the graph this reads and, just as importantly, what it
cannot see.

## What is deliberately not reported

**A file this change created.** Every one of its dependencies is new by definition, so reporting them
would bury the signal under the noise of ordinary new code.

**A dependency that only moved.** The rule compares the set of files a file reaches, so rewriting the
same dependency in another form is not a finding.

## How often it fires

Measured over the last 62 commits of meilisearch that touched Rust, counting dependencies added to
files that already existed:

| | new dependencies |
|---|---|
| median | 0 |
| mean | 1.5 |
| 90th percentile | 3 |
| most in one commit | 30 |

Forty-five of those 62 commits introduced none at all. It is a rule that stays quiet, which is why it
ships on: when it does say something, it is worth the line it costs.

## Changing it

```toml
[rules]
new-dependency = { severity = "off" }
```

Turning it off is reasonable on a codebase in early construction, where nearly every commit is
supposed to be wiring things together and the finding tells you nothing you did not intend.

Promoting it to `error` only makes sense next to a written rule about what may depend on what. On its
own it would fail a build for the ordinary act of using something, which is not a defect.
