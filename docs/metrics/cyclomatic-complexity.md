# Cyclomatic complexity

`jabuti/metrics/cyclomatic-complexity`

## What it measures

The number of linearly independent paths through a unit. McCabe defined it on the control flow
graph as `M = E − N + 2P`, where `E` is edges, `N` is nodes and `P` is connected components.

Every binary decision adds one node and two edges to that graph, a net `+1` to `E − N`. So the
metric can be counted from syntax instead of built from a graph, and the result is identical:

```
M = decision points + 1
```

The practical reading is the number of test cases needed to cover every branch at least once.

## What counts in Rust

Decision points come from `queries/rust/decisions.scm`. A capture named `@decision` adds one;
`@decision.discount` subtracts one.

| Construct | Effect |
|---|---|
| `if`, including `else if` and `if let` | +1 each |
| `while`, `for`, `loop` | +1 each |
| `&&`, `\|\|` | +1 each |
| `match` arm | +1 each |
| `match` arm guard | +1 each |
| `match` expression | −1 |
| plain `else` | 0 |

## Choices we made

**A `match` of N arms adds N−1, not N.** The arms contribute `+1` each and the `match` itself
contributes `−1`, which is what the discount capture is for. The reason is consistency: a two-arm
`match` and the equivalent `if`/`else` describe the same control flow, so they must score the same.
Counting every arm would make the `match` spelling more expensive than the `if` spelling of
identical logic. It also gives the right answer at the edges — a one-arm `match` adds nothing,
because it does not branch.

**A guard counts.** `Some(x) if x > 0 => …` can match its pattern and still fall through to the next
arm, which is a real path.

**The `?` operator does not count.** It is a branch, so strict McCabe would count it. We do not,
because it is a uniform early-return idiom that appears many times in ordinary Rust: counting it
would push routine functions past any useful threshold without indicating a decision anyone has to
reason about. The reading cost of error propagation is better captured by cognitive complexity.

## Attribution

A unit's complexity covers its own body and any closures inside it, but not nested units that are
measured separately — a function declared inside another function has its own score and does not
inflate its container. A file therefore always scores 1: everything in it belongs to something else.

## Threshold

The default maximum is **5**.

## What it does not measure

Nesting. These two score identically at 4:

```rust
if a { }              if a {
if b { }                  if b {
if c { }                      if c { }
                          }
                      }
```

That blindness is the reason cognitive complexity exists, and the reason this metric reports but
does not carry the gate on its own.

## Reference

McCabe, *A Complexity Measure*, IEEE Transactions on Software Engineering, 1976.
