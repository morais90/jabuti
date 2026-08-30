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

## Status: off by default

The rule is computed but does not report, and the reason is the section below rather than any doubt
about the implementation.

Measuring 737,499 functions across 1,645 crates published on crates.io gives this distribution:

| p50 | p75 | p90 | p95 | p99 | max |
|---|---|---|---|---|---|
| 1 | 1 | 2 | 4 | 10 | 1586 |

Three quarters of all Rust functions score 1: they do not branch at all. A limit of 5 sits near the
95th percentile and a limit of 10 at the 99th — and inspecting what lands in that top percentile
shows it is dominated by flat exhaustive matches:

```rust
fn normalized(method: &Method) -> &'static str {   // scores 10
    match method.as_str() {
        "GET" => "GET",
        "POST" => "POST",
        ...
    }
}
```

That is a lookup table written as a `match`, and it reads at a glance. Rust reaches for exhaustive
matching where other languages reach for a map, so on idiomatic Rust this rule is mostly noise at
any threshold that fires at all. Publishing it would teach whoever reads our output to stop reading
it, which costs more than the rule is worth.

The measure stays. It is an input to composites — complexity crossed with change frequency, or with
size and state access — where the signal survives. It reports as part of something that
discriminates, not on its own.

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
