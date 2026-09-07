# `error-masking`

Reports code that removes an error path without handling it.

**Default severity:** warning. This rule has no limit; every occurrence is reported.

## The idea

Masking an error is not the same as writing a bug. It is a decision, usually made in a hurry, that a
particular failure is not worth facing right now. What makes it expensive is that the decision leaves
no trace, and the failure it hid still happens.

```rust
let config = load_config().unwrap();
```

If the file is missing, this does not report that the config was not found at a particular path. It
reports a panic on this line. The cause was known right here, and it was thrown away right here.

The silent version costs more:

```rust
let _ = save_state();
```

Nothing panics. The state was not saved, the program carries on, and the symptom appears somewhere
else much later with no visible connection to this line. You end up investigating from scratch
something the code already knew.

Studies of exception handling in production systems have been finding the same picture for a long
time: a large share of handlers do nothing meaningful with the failure they catch, and swallowed
exceptions are a recurring source of defects that are hard to diagnose precisely because the
evidence was discarded at the point where it existed.

## Why this rule sits near the top of the list

Analysis of hundreds of millions of changed lines found error-masking constructs growing sharply in
codebases written with AI assistance, faster than almost anything else measured.

The mechanism is worth understanding, because it explains why review does not catch it. Handling an
error properly requires knowing what the caller should do about it. Abort? Fall back? Retry? That is
knowledge about the product, not about the snippet. A model completing a function rarely has it, and
it does have a very concrete pressure: the code has to compile. `.unwrap()` compiles.

It is the path of least resistance, and it works. That is why the construct spreads rather than
being caught: every single occurrence looks harmless on its own.

## What counts

In Rust:

| Construction | What it removes |
|---|---|
| `.unwrap()`, `.expect(...)` | Turns the failure into a panic, dropping the error type |
| `let _ = f();` | Discards the whole result, with no panic and no log |
| `.ok()` | Converts to an option, dropping the cause |
| `Err(_) => {}` | Matches the failure and does nothing |

In Kotlin:

| Construction | What it removes |
|---|---|
| `!!` | The direct equivalent of `unwrap` |
| `catch (e: Exception) { }` | Catches the failure and does nothing |
| `runCatching { }.getOrNull()` | Same effect as `.ok()` |

In TypeScript:

| Construction | What it removes |
|---|---|
| `catch { }` | Catches a thrown value and does nothing |
| `promise.catch(() => { })` | Converts a rejected promise into a successful one with no value |

Detection is syntactic. jabuti reads the shape of the code, not its types, so it does not know
whether `.ok()` was called on a `Result` or on something else that happens to have that method. This
is what keeps the rule fast enough to run on every change.

## Why test code is left out

An `unwrap()` in a test is not a masked error. It is the assertion. Writing the handled version
would make the test worse, because a test that quietly returns on failure is a test that cannot fail.

jabuti recognises test code by path and by language markers. Rust skips `tests/`, `benches/` and
`examples/`; Kotlin skips test source sets; TypeScript skips `test`, `tests`, `__tests__`, `*.test.ts`
and `*.spec.ts`. Rust and Kotlin declarations marked as tests are also skipped wherever they live,
because a path rule cannot reach a test function inside an ordinary production file.

Production code beside a test module is still read. Only the test parts are quiet.

## What the finding says

```
src/handler.rs:42  warning  error-masking  unwrap  the failure becomes a panic
```

The construct is named as it appears in your code, and the consequence describes what was removed.
There are only three consequences, one per kind of masking, so the line stays cheap to read and cheap
to parse.

## What to do with one

Not every occurrence is a defect, and the rule is honest about that.

```rust
let pattern = Regex::new(r"^\d+$").unwrap();
```

That argument is a literal the author wrote. If it fails to compile, the program is broken in a way
no handling would fix. This is an assertion about an invariant, not a hidden failure. The same is
true of a panic in `main`, where stopping is the correct behaviour.

The question worth asking at each one is simple: can this fail because of something outside the
program, such as a file, a network, a user, or a clock? If yes, the failure will happen eventually
and someone will have to diagnose it without the information this line discarded. If no, the
construct is documenting an invariant and it is fine.

## Why it is a warning

Because it reports shape, and shape cannot tell an invariant from a real error path. Precision is
medium by construction, and recall is high, which is a reasonable thing to show a person and an
unreasonable thing to fail a build on by default.

The rule earns most of its value scoped to a change. Two new masked errors in a diff is a question
worth asking. Four hundred in a codebase you inherited is a fact you cannot act on.

## Requirements and limits

There is no threshold to tune. Every matched occurrence is reported when the rule is enabled. The
measured production rate and the test-code exclusion study live in
[`CALIBRATION.md`](../../CALIBRATION.md).

Detection is per file, so unlike [`duplicate-block`](duplicate-block.md) this rule can be configured
per language.

## Extending the concept vocabulary

`error-masking` is concept-bound. Each language query tags the syntax above with one of three
effects: panic, discard or swallow. The rule consumes those tags and contains no language names.

A project API that intentionally drops an error can be added by canonical path:

```toml
[languages.typescript.concepts]
error-discard = ["@mycorp/errors.discard"]
```

Direct and aliased imports resolve to that path. The configured binding extends the built-ins; it
does not replace the empty-handler checks.

## Changing it

```toml
[rules]
error-masking = { severity = "error" }
```

Promoting it to `error` is reasonable together with `--since`, which turns it into a gate on newly
introduced masking while leaving what already exists alone.

```toml
[languages.kotlin.rules]
error-masking = { severity = "off" }
```

Turning it off for one language is supported, and is the right move if a codebase has a convention
this rule reads wrongly.

## Further reading

Cabral, B., Marques, P. (2007). *Exception Handling: A Field Study in Java and .NET*. European
Conference on Object-Oriented Programming. Surveyed handlers across a large body of real code and
found most of them doing nothing useful with the failure.

Ebert, F., Castor, F., Serebrenik, A. (2015). *An exploratory study on exception handling bugs in
Java programs*. Journal of Systems and Software. Traces defects back to how failures are caught and
discarded.

GitClear (2026). *The AI Code Quality and Maintainability Gap*. Analysis of 623 million changed
lines, and the source of the growth figure quoted above.
