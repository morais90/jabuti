# `function-lines`

Reports a function that spans more lines than the limit.

**Default limit:** 60 in Rust, 47 in Kotlin, 71 in TypeScript. **Default severity:** warning.

All three numbers are the same claim measured in each language; [Languages](../languages.md) explains
why they differ.

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

## Calibration

The default is selected separately for each language. [`CALIBRATION.md`](../../CALIBRATION.md)
records the benchmark populations, distributions, application comparison and resulting limits.

Part of the gap is not quality. A function dispatching over thirty payment connectors is long for a
reason that a six line accessor never has to answer for. The rule tells you where to look; it does
not tell you that what you find is wrong.

## Changing it

```toml
[rules]
function-lines = { limit = 80, severity = "error" }
```

Raising it is reasonable if your project has a house style that produces longer functions, or if you
want to start further out and tighten later. Lowering it increases the finding volume.

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
