# `assertion`

Reports a test that runs code and checks nothing.

**Default severity:** warning. This rule has no limit; every unit marked as a test and carrying no
assertion is reported.

## The idea

Ask an agent for tests and this is the shape you get most often:

```typescript
it('creates a user', async () => {
  const user = await service.create({ email: 'ada@example.com' })
})
```

It runs, it passes, and it will keep passing on the day `create` starts returning the wrong thing,
because nothing in it makes a claim about what happened. It also reports coverage over every line
`create` touches, so the number most teams watch moves in the reassuring direction.

A test suite is the only part of a project that states what the code is supposed to do in a form a
machine can check. A test with no assertion states nothing and looks exactly like one that does.

## Who this is for

Standing in a codebase, the count says how much of the suite is decoration. Someone who inherited the
code has no other way to find that out short of reading every test, and it changes how far they
should trust a green build.

In a change, the reader is whoever reviews the tests that just arrived. This is the failure mode of
generated tests, which is the case jabuti exists for.

## What counts as a test

| Language | Marker |
|---|---|
| Rust | `#[test]` and its variants |
| Kotlin | `@Test`, `@ParameterizedTest`, `@RepeatedTest` |
| TypeScript | a closure passed to `it`, `test`, `it.only`, `it.skip`, `test.only` or `test.skip` |

TypeScript carries no declaration-level marker of its own; jabuti recognises the shape of the call
that wraps the closure instead. `describe(...)`'s own callback does not count, because it groups
tests rather than being one. `it.each`/`test.each`'s parameterised form is not recognised: the
callback there sits behind a curried call the rule does not follow, so a body with no assertion
inside one of those goes unreported.

## What counts as an assertion

| Language | Construction |
|---|---|
| Rust | `assert!`, `assert_eq!`, `assert_ne!`; `assert_cmd`'s `.assert()`; `insta`'s `assert_snapshot!` and its neighbours |
| Kotlin | `kotlin.test` and JUnit's `assertEquals`, `assertTrue`, `assertThrows` and their neighbours, plus `assertThat` |
| TypeScript | `expect(...)`, and `assert(...)` or any `assert.*` call from `node:assert` |

`Assertion` is concept-bound the same way [`error-masking`](error-masking.md) is, and matched by exact
name rather than a prefix, for the same reason a project's own `assertSomething` helper should not be
guessed at: a project with its own assertion helpers extends the vocabulary instead of living with
false positives. `.assert()` and the `insta` macros were added after jabuti's own test suite, which
leans on both, first showed how much a purely macro-shaped Rust vocabulary would have missed.

## A call to a helper in the same file counts, a call into another file does not

```rust
fn responds_with(value: &Response, status: u16) {
    assert_eq!(value.status(), status);
}

#[test]
fn creates_a_user() {
    let response = service.create(user());
    responds_with(&response, 201);
}
```

`creates_a_user` carries no assertion of its own, but it calls `responds_with`, declared in the same
file, and `responds_with` does. That counts. The check goes exactly one call deep, and only by plain
name: a method call, a closure held in a variable, or a helper declared in another file does not
count, because following any of those needs the symbol graph and this rule does not have one. That is
the conservative direction: a test that truly asserts only through a chain of helpers, or through a
helper from another file, is reported here as if it asserted nothing.

A test with a direct assertion is never checked against this fallback, so the common case pays no
cost for it.

## Rust's `#[should_panic]`

A Rust test carrying `#[should_panic]` asserts by definition: the panic is the check, and the rule
does not also require a body assertion. No equivalent is assumed for Kotlin or TypeScript. JUnit's
`assertThrows` and TypeScript's `expect(...).toThrow()` are already ordinary calls that this rule
already reads as assertions, so they need no special case.

## What the finding says

```
src/create_user.test.ts:3  warning  assertion  the test runs and asserts nothing
tests/report.rs:12  warning  assertion  creates_a_user  the test runs and asserts nothing
```

A named test carries its name as the subject. An anonymous TypeScript closure carries none, the same
way any other unnamed unit does.

## Why it is a warning

Because the one-hop rule undercounts on purpose, a real assertion reached through a longer chain of
helpers reports the same as a genuinely empty test. Precision on "does this project consider its test
suite complete" is not something shape can answer; precision on "does this specific unit, read on its
own terms, make a claim" is what the rule promises.

## Requirements and limits

There is no threshold to tune. Every unit marked as a test and found not to assert is reported. The
measured standing rate over three corpora lives in [`CALIBRATION.md`](../../CALIBRATION.md).

## Extending the concept vocabulary

A project that wraps its own assertion helper in a function or macro can extend the vocabulary by
canonical path:

```toml
[languages.typescript.concepts]
assertion = ["@mycorp/testing.checkThat"]
```

Direct and aliased imports resolve to that path. The configured binding extends the built-ins; it
does not replace them.

## Changing it

```toml
[rules]
assertion = { severity = "error" }
```

Promoting it to `error` is reasonable together with `--since`, which turns it into a gate on newly
written tests while leaving what already exists alone.

```toml
[languages.typescript.rules]
assertion = { severity = "off" }
```

Turning it off for one language is supported, and is the right move if a codebase leans on an
assertion style this rule cannot follow, such as a snapshot-testing library.
