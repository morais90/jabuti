# `duplicate-block`

Reports a region of code that also exists somewhere else.

**Default limit:** 120 nodes. **Default severity:** warning.

## The idea

Copied code is not wrong when it is written. It becomes wrong later, when one copy is fixed and the
others are not.

That is the actual cost, and it is worth being precise about it, because "duplication is bad" on its
own is the kind of advice that gets ignored. The problem is not the repetition. The problem is that
a change now has to be made in several places by someone who has no way of knowing how many places
there are. Studies of real systems have found that inconsistent changes to copies are a genuine and
recurring source of defects, and copied regions carry measurably more defects than the code around
them.

There is also a reason this rule sits near the top of jabuti's list rather than somewhere in the
middle. Analysis of hundreds of millions of changed lines has found duplicated blocks rising sharply
in codebases written with AI assistance, far faster than any other quality signal moved. Suggesting
a block of code is something a model does well. Noticing that the block already exists three files
over is something it has no way to do.

## What the finding says

```
src/parser.rs:12  warning  duplicate-block  143 nodes repeated at src/reader.rs:40 (limit 120)
```

Every occurrence is reported, and each one points at the others. So a block appearing in two places
produces two lines, each naming the other. That is deliberate: whichever file you happen to open,
the finding tells you where the rest of the family lives.

The number is a count of syntax nodes rather than lines, for reasons covered in
[the measure](../measures/duplication.md). Names and literals do not take part in the comparison, so
a copy where everything was renamed still counts as a copy.

## What to do with one

The reflex is to extract a shared function. Often that is right. It is not always right, and
reaching for it every time is how codebases end up with a utility module nobody can read.

Two questions usually settle it. Do the copies have to change together? If a fix to one is always a
fix to all, they are one thing and should be one thing. And are they alike for a reason, or by
coincidence? Two functions that look identical today because both happen to validate a two-field
form will drift apart the moment one form gains a field, and merging them now buys a coupling you
will have to undo.

Duplication in tests deserves its own note, since it is where this rule fires most often. Repetition
in a test is frequently the point: a test that reads top to bottom without sending you off to a
helper is easier to trust when it fails. Extracting shared setup is worth it when the setup is
genuinely shared, and not worth it when it only looks shared.

## Why it is a warning

Because the rule reports structure, and structure is evidence rather than proof. Two functions can
have the same shape and nothing to do with each other, and jabuti cannot tell the difference. That
is a reasonable thing to put in front of a person, and not a reasonable thing to fail a build on.

## Where the default comes from

At 120 nodes the rule reports around two findings per thousand lines on the codebases used to
calibrate it, and everything it reported there was a real copy.

Lowering it gets noisy quickly, and the noise is a specific and predictable kind. Below roughly 60
nodes you start catching the ordinary grammar of the language, the small arrangements that any two
pieces of code written in the same style will share, and those are not copies in any sense you can
act on.

## Changing it

```toml
[rules]
duplicate-block = { limit = 200, severity = "warning" }
```

Raising it leaves only substantial copies, which is a reasonable first setting on an existing
codebase where you want the rule on without a backlog appearing on day one. Lowering it toward 80
catches smaller repeats and is worth trying on a codebase you already keep clean.

Promoting it to `error` works best together with `--since`, so that the gate applies to copies being
introduced rather than to every copy already present.

## Requirements and limits

The rule compares files against each other, so jabuti reads every file in scope even when the run is
narrowed with `--since`. Only findings that touch changed lines are reported, but the search itself
has to see the whole picture, since a copy is only findable when its twin is also in view.

Copies are found across files and across languages jabuti supports, but never across a boundary it
cannot parse. A file with a syntax error contributes nothing to the comparison.

Type-3 clones, where a line was added or removed alongside the renaming, are not detected. Neither
are two implementations of the same behaviour written differently. Both are on the roadmap and both
need a different technique than fingerprint comparison.

## Further reading

Juergens, E., Deissenboeck, F., Hummel, B., Wagner, S. (2009). *Do Code Clones Matter?*
International Conference on Software Engineering. The study that pinned the cost on inconsistent
changes rather than on repetition itself.

Kapser, C., Godfrey, M. W. (2008). *"Cloning Considered Harmful" Considered Harmful: Patterns of
Cloning in Software*. Empirical Software Engineering. The counterweight, and the source of the
argument that some duplication is a reasonable engineering choice.

Baxter, I. D., Yahin, A., Moura, L., Sant'Anna, M., Bier, L. (1998). *Clone Detection Using Abstract
Syntax Trees*. International Conference on Software Maintenance.

GitClear (2026). *The AI Code Quality and Maintainability Gap*. Analysis of 623 million changed
lines, and the source of the finding on duplicated blocks growing in AI-assisted codebases.
