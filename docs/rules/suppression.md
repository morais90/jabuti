# `suppression`

Reports a linter or type-checker diagnostic that was switched off instead of resolved.

**Default severity:** warning. This rule has no limit; every occurrence is reported.

## The idea

Every linter has a way of saying "not this one". Rust has `#[allow]`, Kotlin has `@Suppress`,
TypeScript has `@ts-ignore` and `@ts-nocheck`, and there is always the wider escape hatch of
asserting a value to `any`. Sometimes that is the right call. Often it is what happens when the
deadline is closer than the fix.

```rust
#[allow(clippy::too_many_arguments)]
fn build_report(store: &Store, from: Date, to: Date, tz: Tz, fmt: Format, locale: Locale, out: &mut dyn Write) -> Result<()> {
```

That line makes the build green and leaves no other trace. The only record that a standard was
waived is an attribute nobody will read again.

Nothing else reports this, and the reason is structural. The tool that would object is the tool that
was switched off, so a clean run is what a suppression looks like from the outside.

## Who this is for

Someone who inherited a codebase wants the standing count: how much of a declared standard is
actually in force. Either the check runs on that line or somebody turned it off, which is not a
matter of degree.

Someone reviewing a change wants only the new ones. A suppression that arrives in a diff is usually
the line that made the build green, and it sits among real edits where nobody looks at it twice.

## What counts

| Language | Construction |
|---|---|
| Rust | `#[allow(...)]`, `#![allow(...)]` |
| Kotlin | `@Suppress(...)` |
| TypeScript | `@ts-ignore`, `@ts-nocheck`, an `eslint-disable` comment (any variant), an assertion to `any` |

Detection is syntactic, the same way [`error-masking`](error-masking.md) reads shape rather than
types. The subject named in a finding is the lint the construct names when the syntax gives us one
(`dead_code`, `no-console`), and the marker itself otherwise (`ts-ignore`, `any`).

## Why test code is not left out

[`error-masking`](error-masking.md) leaves test code alone because an `unwrap()` inside a test is the
assertion, not a hidden failure. That argument does not transfer here: a suppressed lint inside a
test is still a check that was switched off rather than satisfied. `suppression` reports everywhere a
file is read, tests included.

## What the finding says

```
src/report.rs:1  warning  suppression  too_many_arguments  a linter diagnostic is suppressed instead of satisfied
src/widen.ts:4  warning  suppression  any  a type-checker diagnostic is suppressed instead of satisfied
```

The message names which check was switched off: a linter for `allow`, `Suppress`, `ts-ignore`,
`ts-nocheck` and `eslint-disable`, the type checker for an assertion to `any`.

## What to do with one

Not every occurrence is a defect. A lint disabled with a clear reason beside it is a different act
from a silent one, and the rule does not attempt to tell them apart: every occurrence is reported,
with no limit, whether or not it carries a written reason. Reading it is still worth the reader's
time, because the decision to waive a standard is exactly the kind of thing a diff should show rather
than hide.

## Why it is a warning

Because a suppression is sometimes the right call, and the rule cannot tell a reasoned waiver from a
convenient one by shape alone. Precision on "was this the right call" is not something syntax can
answer; precision on "was a check switched off here" is exact. That is what the rule promises, and no
more.

## Requirements and limits

There is no threshold to tune. Every matched occurrence is reported when the rule is enabled. The
measured standing rate lives in [`CALIBRATION.md`](../../CALIBRATION.md), together with the answer to
the two questions the idea above leaves open.

**Should a reasoned suppression be reported differently from a silent one?** Across four corpora, only
9% of occurrences carried an unambiguous reason on the same line as the construct. The other 91% are
exactly what this rule exists to surface, so it keeps reporting every occurrence regardless. A written
reason is the cheaper case for a reader to skip past, not a reason to build a second code path around.

**Does an assertion to `any` belong here or in a rule of its own?** It silences the type checker rather
than a linter, and the sampled occurrences read as proactive type-erasure rather than a response to a
diagnostic that already fired, which is the distinction the open question anticipated. At roughly a
fifth of TypeScript's suppression occurrences, the share is real but not dominant. `any` stays in this
rule for now, told apart only by its message; splitting it into a rule of its own is worth doing if a
future measurement finds the two constructs behaving differently in practice, not on the strength of
this one.

## Extending the concept vocabulary

`suppression` is concept-bound. A project that wraps its own suppression helper in a function or
macro can extend the vocabulary by canonical path:

```toml
[languages.typescript.concepts]
suppression = ["@mycorp/types.castAny"]
```

Direct and aliased imports resolve to that path. The configured binding extends the built-ins; it
does not replace them.

## Changing it

```toml
[rules]
suppression = { severity = "error" }
```

Promoting it to `error` is reasonable together with `--since`, which turns it into a gate on newly
introduced suppressions while leaving what already exists alone.

```toml
[languages.rust.rules]
suppression = { severity = "off" }
```

Turning it off for one language is supported, and is the right move if a codebase has a convention
this rule reads wrongly.
