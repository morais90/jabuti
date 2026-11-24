# `error-masking`

Reports code that removes an error path without handling it.

This rule has no limit; every occurrence is reported.

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

## Why writing into memory is left out

A string held in memory always accepts text. Formatting into one can only fail when a value refuses
to format itself, which is a bug in that value rather than a condition to handle, yet the language
still returns a result that code has to unwrap. jabuti leaves that unwrap out when it can see that
the target is a string in memory: the nearest declaration of the name before the write gives it that
type, or builds it with a constructor that makes one, or it is a parameter of that type. A string
that arrives some other way, such as a field or a value returned from another function, is still
reported, because the declaration that would prove it is out of sight. Writing to a file or a
terminal can fail and is always reported.

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
src/handler.rs:42  error-masking  unwrap  the failure becomes a panic
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
invariant belongs somewhere the compiler can hold it, such as a value built once at startup or a type
that cannot be empty. The rule reports the line either way, because it cannot tell the two apart.

## Why it fails a change

The rule reports shape, and shape cannot tell an invariant from a real error path. Precision is
medium by construction, and recall is high.

The scope keeps that workable. A check reports only what the change made worse, so the
question is asked of the masked errors a diff added. Two new masked errors in a diff is a question
worth asking. Four hundred in a codebase you inherited is a fact you cannot act on, and a check on a
change never shows them.

## Requirements and limits

There is no threshold. Every matched occurrence is reported. The measured production rate and the
test-code exclusion study live in [`CALIBRATION.md`](../../CALIBRATION.md).

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

## Further reading

Cabral, B., Marques, P. (2007). *Exception Handling: A Field Study in Java and .NET*. European
Conference on Object-Oriented Programming. Surveyed handlers across a large body of real code and
found most of them doing nothing useful with the failure.

Ebert, F., Castor, F., Serebrenik, A. (2015). *An exploratory study on exception handling bugs in
Java programs*. Journal of Systems and Software. Traces defects back to how failures are caught and
discarded.

GitClear (2026). *The AI Code Quality and Maintainability Gap*. Analysis of 623 million changed
lines, and the source of the growth figure quoted above.
