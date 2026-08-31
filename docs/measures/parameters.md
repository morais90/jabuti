# Parameters

Counts how many arguments a function or closure declares.

## What counts

Every declared parameter, and nothing else. A method's receiver is not counted, because the caller
never passes it:

```rust
impl Holder {
    fn method(&self, first: i32, second: i32) -> i32 {  // parameters = 2
        first + second
    }
}
```

Closures are counted the same way as functions, on themselves:

```rust
fn outer() -> i32 {                                     // parameters = 0
    let add = |first: i32, second: i32| first + second; // parameters = 2

    add(1, 2)
}
```

## Why it is worth measuring

Arity is one of the few surface properties of a function that reliably says something about its
design. A function that needs many separate values is usually either doing several jobs at once, or
missing a type that should be holding those values together.

The second case is the more interesting one. When the same three or four arguments keep appearing
together across different signatures, they are describing something that has no name yet. Giving it
one usually simplifies every function that was passing the pieces around.

## Where the number is used

The [`parameters`](../rules/parameters.md) rule reports functions that declare more than the limit.

## What real code looks like

**Rust**, measured across 737,689 functions in 1,645 crates published on crates.io:

| p50 | p75 | p90 | p95 | p98 | p99 |
|---|---|---|---|---|---|
| 1 | 1 | 2 | 3 | 5 | 6 |

Roughly 43% of functions take no arguments at all, and the median is one. Anything past three is
already unusual.

**Kotlin**, across 54,933 functions in ten established projects:

| p50 | p75 | p90 | p95 | p98 | p99 |
|---|---|---|---|---|---|
| 0 | 1 | 2 | 3 | 5 | 6 |

Close enough to Rust that both use the same limit.

## Further reading

Alves, T.L., Ypma, C., Visser, J. (2010). *Deriving metric thresholds from benchmark data*.
International Conference on Software Maintenance.
