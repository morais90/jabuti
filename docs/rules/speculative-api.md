# `speculative-api`

Reports a public declaration that this change added and that nothing in the project references.

**Default severity:** warning. This rule has no limit; every unreferenced new public item is
reported. It needs `--since`, because "added" only means anything against an earlier revision, and
it says so rather than staying quiet when you leave the flag out.

## The idea

Code written by a model tends to arrive with more surface than the task needed: a helper made public
"in case", a struct with three constructors when one is called, a method that mirrors another one on
a sibling type because that type had it. None of it is wrong on its own line. It is API that nobody
asked for and that somebody now has to keep, and it is invisible in review because an unused public
function looks exactly like a used one.

A finding here is the moment to ask whether the item should exist:

```
src/tasks.rs:14  warning  speculative-api  pause  public, and nothing references it
```

Either something should call `pause`, or `pause` should go, or it is meant for callers outside this
repository, in which case the section below explains how the rule already knows that.

## What counts as public, and what counts as a reference

**Rust.** An item marked plain `pub`: a function, a struct, an enum, a union, a trait, a type alias, a
constant, a static, and a `pub fn` inside an `impl` block. `pub(crate)` and `pub(super)` items are
left to the compiler, which already reports them when nothing uses them.

**Kotlin.** A top-level class, object, function or type alias with no `private`, `internal` or
`protected` modifier, which is to say the default.

**A reference** is the item's name written anywhere in the project other than inside its own
declaration: a call, a type position, a method call through a receiver, a path segment, a use list,
a value passed or assigned, a name inside a macro. Every identifier counts, whatever position it
holds. Resolution is by name and not by scope, so two items sharing a name keep each other alive, and
a local variable that happens to share a name does too. That is the direction of error this rule
wants: a finding withheld costs a moment, a finding that deletes a live function costs the function.
The whole project is read for references, whatever paths the command was given, so the answer does
not change with how the command was scoped.

## What is deliberately not reported

**The exported surface of a library.** In a crate with a `lib.rs`, an item is the library's API when
the chain of modules above it is `pub mod` all the way to the root, when a `pub use` in that chain
names it, or when a `pub use path::*` or `pub use path::{self}` in that chain re-exports the module
file it lives in. The public methods of an exported type are part of that surface wherever their
`impl` block sits. Nothing inside the repository needs to call a library's API for it to be alive, so
those items are never reported. A binary crate exports nothing, so there every unreferenced public
item is a candidate, except `main`. A module written inline, `pub mod items { ... }` inside another
file, is not a file the walk can reach, so a glob over it is not followed.

**Anything an attribute or annotation marks.** `#[no_mangle]`, `#[tokio::main]`, `#[get("/x")]`,
`@RestController`, `@Scheduled`, `@Component`: each of these says a runtime, a framework or a linker
will call the item, and the rule cannot follow that. Rather than guess, it treats the item as unknown
and stays quiet. A short list of attributes that say nothing about callers is exempt from this:
`derive`, `allow`, `inline`, `cfg`, `repr`, `doc` and their relatives.

**Test code.** A file under a test directory, an item inside a `#[cfg(test)]` module, and anything a
test attribute marks belong to the test runner, which is a root the rule does not need to see.

**A name that appears in a file jabuti could not parse.** Unreadable files are listed at the end of
every report. If one of them mentions the name as a whole word, the item is unknown, not speculative.

**An item that already existed.** The rule compares the declarations of the changed file with the
same file at the base revision, so editing a file full of public functions reports nothing about
them. Renaming is the one case that reads as new, and a renamed item nobody references is worth the
line. When the base revision of a file cannot be parsed, the file is skipped rather than read as new.

## Kotlin libraries

Kotlin has no `lib.rs`, and public is the default, so the rule assumes a Kotlin project is an
application whose code is called from inside the repository or through an annotation. In a Kotlin
library, new public API is the point, and the rule will report it. Switch it off there:

```toml
[languages.kotlin.rules]
speculative-api = { severity = "off" }
```

## How often it fires

Measured commit by commit, each against its parent, counting declarations added at the top level of a
file (`pub` items in Rust, any class, object, function or alias in Kotlin):

| Project | Commits | New public items | Findings |
|---|---|---|---|
| meilisearch (Rust, library crates) | 60 | 90 | 0 |
| komga (Kotlin, Spring) | 25 | 746 | 0 |
| jabuti (Rust, library and binary) | 40 | 162 | 0 |

Ninety public items were added to meilisearch in those commits and every one was either called or
exported. That is the expected shape: a person adds a function because a call site needs it. The
rule exists for the other author, and a probe confirms it is awake: appending an unreferenced public
declaration to a file in either meilisearch or komga produces exactly one finding at that line.

## What it cannot see

The limits of the graph in [`docs/concepts.md`](../concepts.md) apply. A call reached only through a
type the rule does not resolve, through a trait, or through code a macro generates is not a reference,
and an item reached only that way is reported. In practice those items are almost always also named
somewhere plainly, which is why the measured count is zero rather than a stream of false positives,
but a project that relies heavily on generated call sites should expect some and can turn the rule
off.

## Changing it

```toml
[rules]
speculative-api = { severity = "off" }
```

Turning it off makes sense for a Kotlin library, or during a phase where a module is being built ahead
of its callers on purpose.

Promoting it to `error` fits an agent harness where the agent is expected to wire what it declares in
the same change. A human team will usually want the warning, since scaffolding ahead of use is a
legitimate way to work for a day.
