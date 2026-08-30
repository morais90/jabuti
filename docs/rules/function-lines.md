# `function-lines`

Reports a function that spans more lines than the limit.

**Default limit:** 60. **Default severity:** warning. This is the only rule enabled out of the box.

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

It is the 98th percentile of real code. Measured across 737,689 functions from 1,645 crates published
on crates.io, the distribution looks like this:

| p50 | p75 | p90 | p95 | p99 |
|---|---|---|---|---|
| 6 | 10 | 21 | 34 | 86 |

So the claim behind the default is narrow and checkable: this function is longer than roughly 98% of
the Rust written in public. It is not a claim that 60 lines is intrinsically wrong.

Setting it near 25, which is a common limit in other ecosystems, would land around the 92nd
percentile here and report one function in twelve. At that volume people stop reading the output,
which costs more than the rule is worth.

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
