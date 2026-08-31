---
name: jabuti-test
description: The testing principles for this repository, each with what to do and what not to do — what a test must assert, where it lives, how a fixture is built, how the analyzer is prevented from failing silently, and how determinism is proven. Use before writing or changing any test, before adding a fixture or a snapshot, and when judging whether existing coverage is honest.
---

# Testing principles

Principles, not law. Each one carries the reason it exists, so a principle that stops paying for
itself can be dropped on purpose instead of quietly ignored. This file is meant to be argued with:
when the work disproves one, change it here, in the same pull request.

---

## 1. A test asserts the whole value

A test that checks the child count of a tree passes while every span inside it drifts. Asserting the
whole value means a new capture in the query breaks the test until someone looks at it.

**Not this**

```rust
let file = syntax::parse(source, &lang::RUST).expect("fixture parses");

assert_eq!(file.children.len(), 3);
assert_eq!(file.children[0].kind, UnitKind::Module);
```

**This**

```rust
let file = syntax::parse(source, &lang::RUST).expect("fixture parses");

insta::assert_snapshot!(outline(&file));
```

The snapshot carries names, kinds, spans and nesting at once. Anything the query starts or stops
capturing shows up as a diff.

---

## 2. No field escapes the pattern

`..` and `_` are how a field stops being checked without anyone deciding that. Name every field,
including the ones the test does not care about.

**Not this**

```rust
assert!(matches!(error, SyntaxError::Query(..)));
```

**This**

```rust
assert!(matches!(error, SyntaxError::Query(source) if source.to_string().contains("unit.")));
```

When a field genuinely cannot be compared, bind it away by name rather than eliding it, so adding a
field is a compile error rather than a silent pass.

---

## 3. The test covers the path production uses

Test what the binary runs, not what is convenient to construct. A metric test that assembles a
`Unit` by hand never touches the query — and the query is where the language knowledge lives, which
makes it the part most likely to be wrong.

**Not this**

```rust
let unit = Unit {
    name: Some("doubled".to_owned()),
    kind: UnitKind::Function,
    span: Span { start_line: 11, end_line: 14 },
    children: Vec::new(),
};

assert_eq!(cognitive_complexity(&unit), 3);
```

**This**

```rust
let file = parse_fixture("rust/cognitive/nesting.rs");

assert_eq!(cognitive_complexity(function(&file, "doubled")), 3);
```

The same rule applies upward: at least one test per sensor must run through the engine rather than
calling the sensor directly, because ordering, deduplication and scope live there.

---

## 4. The name states the behaviour, and the body earns it

A failing test should say what broke before anyone opens the file. And a name the body does not
assert is worse than a vague one, because it reads as covered.

**Not this**

```rust
#[test]
fn test_parse() { ... }
```

**This**

```rust
#[test]
fn a_closure_inside_a_method_nests_under_that_method() { ... }
```

The trap is subtler than naming. This body does not test nesting at all — it passes for any tree
that captured anything:

**Not this**

```rust
fn a_closure_inside_a_method_nests_under_that_method() {
    let file = parse_fixture("rust/units.rs");

    assert!(!file.children.is_empty());
}
```

**This**

```rust
fn a_closure_inside_a_method_nests_under_that_method() {
    let file = parse_fixture("rust/units.rs");

    let doubled = function(&file, "doubled");

    assert_eq!(kinds(&doubled.children), [UnitKind::Closure]);
}
```

---

## 5. Tests live under `tests/`

Nothing inline in `src/`. `jabuti-core` is published, so `pub` is a contract with the outside world,
and a suite that can only reach the public surface is a suite that keeps that surface honest.

This costs nothing and buys principle 3: `nest` is private, so the only way to test it is through
`parse`, which is the call production makes.

**Not this**

```
src/syntax.rs        implementation + #[cfg(test)] mod tests
```

**This**

```
src/syntax.rs                  implementation only
tests/syntax.rs                its tests
tests/common/mod.rs            shared scaffolding, as a directory module
tests/fixtures/rust/units.rs   the input
```

`tests/common.rs` as a plain file would be compiled as a test binary of its own. Always the
directory form.

---

## 6. The fixture is real code, not a caricature

A fixture more forgiving than real source hides the bug it exists to catch. A cognitive complexity
fixture built only from `if` proves nothing about `else if`, boolean chains, `match` or macros —
which is where every implementation of that metric goes wrong.

**Not this**

```rust
fn classify(value: i32) -> bool {
    if value > 0 {
        true
    } else {
        false
    }
}
```

**This**

```rust
fn classify(value: i32) -> Outcome {
    if value < 0 && value > MIN {
        return Outcome::Low;
    } else if matches!(value, 0) {
        return Outcome::Zero;
    }

    for step in 0..value {
        while step > 0 || value > MAX {
            return Outcome::High;
        }
    }

    Outcome::Normal
}
```

Every construct the language spec assigns a rule to needs a fixture exercising it. The fixture set
is the coverage claim.

---

## 7. A construct the analyzer does not understand fails loudly

Silent skipping is the worst failure available to this project: the caller believes they were
checked and they were not. tree-sitter always returns a tree, so a fixture with a syntax error still
parses — and every metric computed over it is measured on rubble.

**This**, in `tests/common/mod.rs`, used by every test that parses

```rust
pub fn parse_fixture(relative: &str) -> Unit {
    let source = read_fixture(relative);
    let tree = tree_of(&source);

    assert!(
        !tree.root_node().has_error(),
        "fixture {relative} does not parse cleanly"
    );

    syntax::parse(&source, &lang::RUST).expect("fixture parses")
}
```

The same rule at the language level: a rule that cannot run for the current language is reported as
unavailable, never omitted. A test asserts the availability report matches the rule registry.

---

## 8. The fixture states its expected value and how it was derived

For metrics, the fixture is the specification. A number in a test body with no derivation is a
number nobody can check, and the first person to disagree with it has to rebuild the reasoning from
scratch.

This is the one place comments belong in this repository — the annotation is test data, not
commentary.

**Not this**

```rust
assert_eq!(cognitive_complexity(function(&file, "classify")), 7);
```

**This**, with the fixture carrying the derivation

```rust
fn classify(value: i32) -> Outcome {
    if value < 0 && value > MIN {   // +1 if, +1 boolean sequence
        return Outcome::Low;
    } else if value == 0 {          // +1 else if, no nesting penalty
        return Outcome::Zero;
    }

    for step in 0..value {          // +1 for
        while step > 0 {            // +2 while at nesting 1
            return Outcome::High;
        }
    }

    Outcome::Normal                 // cognitive = 7
}
```

Where our reading departs from the published specification, the fixture names the deviation and
`docs/metrics/<rule>.md` explains it.

---

## 9. A test depends on nothing outside its fixture

Ambient state makes a suite that passes on the author's machine and fails in CI, or worse, passes in
both while measuring the wrong thing. No test reads the repository's own git history, the real
`HOME`, the network, or the clock.

**Not this**

```rust
let churn = churn_of(Path::new("."));
```

**This**

```rust
let repo = TempRepo::new();
repo.commit("src/lib.rs", "fn a() {}");
repo.commit("src/lib.rs", "fn a() { b() }");

assert_eq!(churn_of(repo.path()).get("src/lib.rs"), Some(&2));
```

A suite that sleeps is a suite people stop running. When timing enters through external tool
orchestration, drive it with an injected clock rather than wall time.

---

## 10. Determinism is asserted, not assumed

Byte-identical output is the property this project sells. A property that is claimed but not tested
is a property that is already broken somewhere.

**This**, over a fixture tree containing several files

```rust
#[test]
fn output_is_byte_identical_across_orderings_and_thread_counts() {
    let forward = analyze(&fixture_tree(), Order::Given, Threads(1));
    let shuffled = analyze(&fixture_tree(), Order::Reversed, Threads(8));

    assert_eq!(forward, shuffled);
}
```

The usual culprits are `HashMap` iteration reaching output, parallel reduction over floats, and
paths rendered from a non-deterministic walk. This test is what catches them.

---

## 11. A snapshot is read before it is accepted

`insta` makes accepting trivial, which makes it trivial to enshrine a bug. A snapshot accepted
without reading asserts whatever the code happened to produce.

Review with `cargo insta review` and read the diff. `INSTA_UPDATE=always` is for creating a snapshot
you are about to read, never for making a red suite green.

A snapshot whose diff cannot be explained is a failing test, not a stale one.

---

## 12. Core behaviour is tested in the core

`cargo mutants` runs only the tests belonging to the package a mutation lives in. A function in
`jabuti-core` exercised solely through a CLI integration test is, as far as the mutation gate is
concerned, untested: every mutation of it survives and nobody notices.

So a public function in the core needs a test in the core, even when the CLI already covers it end to
end. The CLI test proves the wiring; the core test proves the behaviour.

---

## 13. The suite is measured by what it kills, not by what it covers

Coverage proves a line executed. It does not prove anything was asserted about it — a test that calls
`parse` and drops the result covers the whole module. `cargo mutants` changes the code and asks
whether any test notices, which is the question coverage was standing in for.

**This**

```
just mutants
```

A surviving mutant is a specific, actionable statement: this change to production code broke nothing.
Either the test that should have caught it is missing, or the code it mutated is dead.

Run it scoped to the change rather than over the whole tree — the same clean-as-you-code rule the
product itself applies:

```
cargo mutants --no-shuffle --in-diff pull-request.diff
```

`--no-shuffle` is not optional here. cargo-mutants randomises mutant order by default, and this
project does not ship tools whose output changes between identical runs.
