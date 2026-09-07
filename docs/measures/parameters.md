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

An attribute on a parameter is not a parameter either, which matters in test code where macros
annotate arguments:

```rust
fn measured(#[allow(unused)] first: i32, second: i32) -> i32 {  // parameters = 2
    first + second
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

## Calibration

All three languages currently use a limit of 4. TypeScript's explicit `this` parameter describes the
receiver and is not counted. The measured distributions live in
[`CALIBRATION.md`](../../CALIBRATION.md).

## Further reading

Alves, T.L., Ypma, C., Visser, J. (2010). *Deriving metric thresholds from benchmark data*.
International Conference on Software Maintenance.
