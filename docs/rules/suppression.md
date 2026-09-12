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

## Two open questions

Two questions this rule does not yet answer on its own:

Should an occurrence carrying a written reason be reported differently from a silent one? The rule
currently treats them the same.

Does an assertion to `any` belong in this rule or in one of its own, given that it silences the type
checker rather than the linter? It ships here for now, distinguished only by its message.

Both are calibration questions: the standing rate this rule produces over real corpora has not yet
been measured, and the record of that measurement in [`CALIBRATION.md`](../../CALIBRATION.md) is
where these two questions get their answer.

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
