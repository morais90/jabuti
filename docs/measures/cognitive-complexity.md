# Cognitive complexity

Measures how hard a piece of code is to follow when you read it.

## Why it exists

[Cyclomatic complexity](cyclomatic-complexity.md) counts how many ways execution can flow through
code. That answers a testing question. It does not answer the question most people are actually
asking, which is whether the code is hard to understand, and the gap between those two shows up
immediately:

```rust
if a { }              if a {
if b { }                  if b {
if c { }                      if c { }
                          }
                      }
```

Cyclomatic complexity scores both of these at 4. Anyone who has read code knows they are not the
same. Cognitive complexity scores the left one 3 and the right one 6, because it charges for depth.

## The three rules

G. Ann Campbell proposed the measure in 2018 around three ideas.

**Ignore shorthands.** A construct that collapses several statements into one readable form is free.
Reading it is a single act, so it should not be billed as several.

**Add one for every break in the flow.** Anything that stops you reading top to bottom costs one:
a conditional, a loop, a jump.

**Add the nesting level when a break sits inside another.** An `if` at the top of a function costs 1.
The same `if` inside a loop costs 2. Inside a loop inside a conditional, 3.

That third rule is the whole point. Depth is what makes code hard to hold in your head, and it is
exactly what path counting ignores.

## Worked example

```rust
fn classify(items: &[Item]) -> Summary {
    let mut total = 0;

    for item in items {                       // +1   at depth 0
        if item.active {                      // +2   at depth 1
            if item.value > 0 && item.ready { // +3   at depth 2
                total += item.value;          // +1   for the run of &&
            } else {                          // +1   for the else
                total -= 1;
            }
        }
    }

    Summary { total }                         // cognitive complexity = 8
}
```

The same function scores 5 on cyclomatic complexity. The entire difference is depth.

## The details that matter

**`else` and `else if` cost one, with no depth charge.** You are already inside the conditional you
started reading, so the alternative branch is a continuation rather than a new level. It still
interrupts the flow, so it still costs one.

**A run of the same logical operator costs one, not one per operator.** `a && b && c` costs 1,
because reading a uniform chain is a single act. `a && b || c` costs 2, because mixing operators
forces you to work out what binds to what.

**A whole `match` costs one, however many arms it has.** This is the first rule at work. Dispatching
on a value is one thing to understand, and a table with twenty entries is no harder to read than one
with three:

```rust
fn normalized(method: &Method) -> &'static str {
    match method.as_str() {    // +1, and that is all
        "GET" => "GET",
        "POST" => "POST",
        // eight more arms
    }
}
```

Cyclomatic complexity scores that function 10. Cognitive complexity scores it 1, and 1 is the honest
answer.

**A closure raises the depth without costing anything itself.** Writing a closure is not a break in
flow, but code inside one is a level deeper for the reader.

## Attribution

A function's score covers its own body and any closures inside it. A function declared inside
another function is scored on its own and does not add to its container.

## What we do not measure yet

Campbell's specification adds one for each function in a recursive cycle, and one for a jump to a
label. Neither is implemented. Recursion needs call resolution rather than syntax alone, and labelled
jumps are rare enough in Rust that the omission has not cost anything so far. Both are noted here
rather than left for you to discover.

## What real code looks like

**Rust**, measured across 737,689 functions in 1,645 crates published on crates.io:

| p50 | p75 | p90 | p95 | p98 | p99 |
|---|---|---|---|---|---|
| 0 | 0 | 1 | 3 | 7 | 12 |

Four functions in five score zero, meaning they contain no branching at all. That shape is worth
knowing: when a function does score, it is already unusual.

**Kotlin**, across 54,933 functions in ten established projects:

| p50 | p75 | p90 | p95 | p98 | p99 |
|---|---|---|---|---|---|
| 0 | 0 | 2 | 4 | 7 | 11 |

Those two distributions are almost identical, in languages that share very little syntax. It is some
evidence that the measure tracks something about programs rather than about a grammar.

## Further reading

Campbell, G.A. (2018). *Cognitive Complexity: A new way of measuring understandability*. The paper
that defines the measure, including the reasoning behind each increment.

Muñoz Barón, M., Wyrich, M., Wagner, S. (2020). *An Empirical Validation of Cognitive Complexity as
a Measure of Source Code Understandability*. Empirical Software Engineering and Measurement. An
independent study testing whether the measure actually tracks how long people take to understand
code.
