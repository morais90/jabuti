# Lines

Counts how many lines a piece of code occupies, and splits them into three kinds: code, comment and
blank.

## How a line is classified

Every line falls into exactly one of the three, so the parts always add up to the whole.

A line is **blank** when it holds nothing but whitespace. It is a **comment** when everything on it
belongs to a comment. Otherwise it is **code**.

That last sentence has a consequence worth spelling out. A line that ends with a trailing note
counts as code, not as a comment:

```rust
let doubled = value * 2; // this line counts as code
```

The reasoning is that you still have to read and understand the statement. The comment came along
for the ride.

## Why this comes from the parser

Counting comments by searching for lines that begin with a comment marker seems simpler, and it is,
right up until it is wrong. Here is a line from a real Rust project:

```rust
*last = next_base64url_symbol_with_the_same_significant_bits(*last);
```

A pattern that treats a leading `*` as part of a comment block, which is a common shortcut, reads
that as a comment. It is a pointer dereference.

Every language has a version of this problem. A `#` inside a Python string, a `//` inside a
JavaScript URL, a `--` inside a SQL literal. jabuti asks the parser which spans of the file are
comments, so a line counts as a comment only when the language itself says so. Checking a real
project, a text search reported ten comment lines where jabuti reported nine. The extra one was that
dereference.

## Where the number is used

Two rules read this measure:

- [`function-lines`](../rules/function-lines.md) counts every line a function spans
- [`file-lines`](../rules/file-lines.md) counts every line in a file

Both include blanks and comments inside the span, because the question they ask is how much there is
to scroll past, not how many statements there are.

## What real code looks like

Limits are calibrated per language, because the same number means different things in different
places. A twenty line function is unremarkable in one language and unusual in another.

**Rust**, measured across 1,645 crates published on crates.io, covering 45,361 files and 737,689
functions:

| | p50 | p75 | p90 | p95 | p99 |
|---|---|---|---|---|---|
| Lines per function | 6 | 10 | 21 | 34 | 86 |
| Lines per file | 129 | 350 | 918 | 1677 | 4578 |

Half of all Rust functions are six lines or shorter. Borrowing a limit from a language where
functions are typically three times longer would report one function in twelve here, which is enough
noise that people stop reading the output.

**Kotlin**, measured across 54,933 functions in ten established projects:

| | p50 | p75 | p90 | p95 | p98 |
|---|---|---|---|---|---|
| Lines per function | 7 | 14 | 23 | 32 | 47 |

The medians are close, but Rust has the longer tail, which is why the two limits differ.

## Further reading

Alves, T.L., Ypma, C., Visser, J. (2010). *Deriving metric thresholds from benchmark data*.
International Conference on Software Maintenance. The method behind reading a threshold off a
measured distribution instead of choosing a round number.
