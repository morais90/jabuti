# `function-lines`

Reports a function that spans more lines than the limit.

**Default limit:** 60 in Rust, 47 in Kotlin. **Default severity:** warning.

Both numbers are the same claim measured in each language; [Languages](../languages.md) explains why
they differ.

## What it means when it fires

```
src/handler.rs:120  warning  function-lines  handle_request  measured 71, limit 60
```

The function `handle_request` starts on line 120 and covers 71 lines, counting blanks and comments
inside it.

Length is a crude signal, and it is worth being honest that it is crude. It does not know whether
those 71 lines are one clear sequence or four tangled ones. What it does reliably tell you is that
someone reading this function cannot see all of it at once, and a reader who has to scroll loses
track of what came before.

That is usually the moment to ask whether the function is doing more than one thing. It often is.
Sometimes it is a long but genuinely linear sequence, such as building a large configuration
structure, and then the right answer is to leave it alone.

## Where 60 comes from

It is the 98th percentile of published Rust libraries. Measured across 737,689 functions from 1,645
crates published on crates.io, the distribution looks like this:

| p50 | p75 | p90 | p95 | p99 |
|---|---|---|---|---|
| 6 | 10 | 21 | 34 | 86 |

So the claim behind the default is narrow and checkable: this function is longer than roughly 98% of
the Rust published as libraries. It is not a claim that 60 lines is intrinsically wrong.

Setting it near 25, which is a common limit in other ecosystems, would land around the 92nd
percentile here and report one function in twelve. At that volume people stop reading the output,
which costs more than the rule is worth.

## Which population that percentile is of

crates.io is a registry of libraries, and a library has a shape of its own. Its median function is
six lines, because a library is largely small composed pieces, trait implementations and code written
by macros. Application code is not shaped that way, and the difference is large:

| Corpus | p50 | p90 | p98 |
|---|---|---|---|
| 1,645 crates published on crates.io | 6 | 21 | 59 |
| hyperswitch, a payments orchestrator | 12 | 45 | 122 |
| meilisearch, a search engine | 11 | 66 | 173 |

The whole distribution moves, not only the tail, so the same limit sits at a different place in each.
What that costs in practice, as the share of functions this rule reports:

| Corpus | Reported |
|---|---|
| 1,645 crates published on crates.io | 1.9% |
| hyperswitch | 6.8% |
| meilisearch | 11.4% |

We kept 60 rather than raising it. A limit placed where meilisearch's 98th percentile falls would let
a 173 line function through, and noticing that function is the entire point of the rule. What changes
is the claim rather than the number: 60 is the 98th percentile of published libraries, and an
application should expect this rule to fire two to six times more often than that.

Part of the gap is not quality. A function dispatching over thirty payment connectors is long for a
reason that a six line accessor never has to answer for. The rule tells you where to look; it does
not tell you that what you find is wrong.

## Changing it

```toml
[rules]
function-lines = { limit = 80, severity = "error" }
```

Raising it is reasonable if your project has a house style that produces longer functions, or if you
want to start further out and tighten later. Lowering it below about 35 will start reporting
ordinary code.

Turning it into an `error` is safe once you are running with `--since`, because a function that was
already long stays quiet until someone edits it.

## A caveat about tests

Integration tests legitimately run longer than production functions. They set up a scenario, do one
thing, and assert. The rule does not currently know the difference, so on a project with substantial
test suites the first findings tend to be there. Excluding them is one option:

```toml
exclude = ["tests/**"]
```

Though it is worth reading the findings before you silence them. A test that needs 90 lines of setup
is sometimes telling you about the code under test.

## Further reading

Alves, T.L., Ypma, C., Visser, J. (2010). *Deriving metric thresholds from benchmark data*.
International Conference on Software Maintenance.

Buse, R.P.L., Weimer, W.R. (2010). *Learning a Metric for Code Readability*. IEEE Transactions on
Software Engineering, 36(4). Includes evidence on which surface properties of code actually
correlate with people finding it hard to read.
