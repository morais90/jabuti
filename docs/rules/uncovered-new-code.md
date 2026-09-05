# `uncovered-new-code`

Reports a stretch of consecutive lines that this change added and that no test executed.

**Default severity:** off. This rule has no limit; every uncovered stretch of new code is reported,
once, at its first line. It ships switched off because it cannot run without a coverage report, and
producing one costs a full instrumented test run. It needs `--since`, because "added" only means
anything against an earlier revision, and when it is set to `error` without the flag it says so
rather than staying quiet.

## The idea

A test suite is the only part of a project that says what the code is supposed to do in a form a
machine can check. New code that no test reaches has made no such statement. It may be right, but
nothing in the repository would notice if it stopped being right, and the moment that is cheapest
to fix is the change that added it, while the author still knows what the code is for.

A finding here names the lines and nothing else:

```
src/tasks.rs:41  warning  uncovered-new-code  5 lines run by no test
```

Five consecutive new lines, starting at line 41, that the tests never executed. Either a test should
reach them, or the lines are handling a case that cannot happen, or the tests that exist run under a
configuration the coverage report did not see. All three are worth knowing.

## What coverage does and does not prove

Coverage proves execution. A line marked covered was run by some test; it does not mean any test
asserted anything about what that line did. A test that calls a function and drops the result covers
the whole function. Inozemtseva and Holmes measured this directly on five large Java projects and
found that, once suite size is accounted for, coverage is not strongly correlated with how many
faults a suite detects (Inozemtseva and Holmes, "Coverage Is Not Strongly Correlated with Test Suite
Effectiveness", ICSE 2014). Mutation testing, which changes the code and asks whether any test
notices, answers the question coverage stands in for.

So this rule makes the one claim coverage can back: no test reaches this line. For code that already
existed that claim is weak, because uncovered code that has worked for years is mostly fine. For code
added in this change it is the cheapest honest signal there is, and it is worth the line it costs.

## Where the report comes from

The rule reads a coverage report. It does not produce one, because producing one means running the
project's tests under instrumentation, which takes seconds on a small project and minutes on a large
one, and belongs to the build rather than to a check that runs after every edit.

**Point at a report you already have.** A project that produces coverage in its build, in CI or
locally, names the file in `jabuti.toml`:

```toml
[coverage]
report = "target/coverage/lcov.info"
```

The path is relative to the project root. A file ending in `.lcov` or `.info` is read as LCOV, the
interchange format `cargo-llvm-cov`, `grcov` and the `lcov` tool itself write and most other runners
can export. A file ending in `.xml` is read as a JaCoCo XML report, which is what JaCoCo and Kover
write.

**Or let a tool produce it.** For a Rust project, the `cargo-llvm-cov` tool in the registry runs the
tests under instrumentation and writes `target/jabuti/coverage.lcov`, and the rule reads that file
with no further configuration:

```toml
[tools.cargo-llvm-cov]
enabled = true
```

Like every tool, it is off by default and jabuti does not install it. `jabuti tools` says whether it
is applicable, available and enabled, and what to do about whichever is missing.
[`docs/tools.md`](../tools.md) explains the three states.

## What is deliberately not reported

**A line the report does not instrument.** A blank line, a closing brace, a signature, a type
declaration: the coverage tool records no hit count for these, and a line with no hit count is not
uncovered, it is not a line that can be executed. Two instrumented lines with an uninstrumented line
between them belong to the same stretch, which is why a finding says how many lines run by no test
rather than how many lines it spans.

**A file absent from the report.** A file the report never mentions was not compiled or not
instrumented in the run that produced the report. That is unknown, not uncovered, and the rule
stays quiet rather than reporting every line of a file it knows nothing about.

**Test code.** A file under a test directory is what runs the tests, not what they test.

**A line that existed before the change.** The rule compares the file against `--since`, and only a
line the change added can be reported. Editing a function with poor coverage reports nothing about
the lines that were already there.

## A stale report is not read

A report written before the last change to a file describes code that no longer exists. Reading it
anyway would mark every new line uncovered, because the tests that ran had never seen those lines,
and the rule would report a wall of findings that are all false. So when the report is older than
any changed file the rule reads nothing and says why:

```
jabuti: uncovered-new-code skipped: target/jabuti/coverage.lcov is older than src/tasks.rs
```

Regenerate the report, or enable the tool that produces it, and the rule runs on the next check.
When the rule is on and no report is named or produced at all, it says that too:

```
jabuti: uncovered-new-code needs a coverage report; set [coverage] report or enable a tool that produces one
```

## Changing it

Turning it on with a report the build already produces:

```toml
[coverage]
report = "target/coverage/lcov.info"

[rules]
uncovered-new-code = { severity = "warning" }
```

A Kotlin project using Kover points at the XML report Kover writes:

```toml
[coverage]
report = "build/reports/kover/report.xml"

[rules]
uncovered-new-code = { severity = "warning" }
```

A Rust project with nothing producing coverage yet turns on the tool instead of naming a report:

```toml
[tools.cargo-llvm-cov]
enabled = true

[rules]
uncovered-new-code = { severity = "warning" }
```

Promoting it to `error` fits an agent harness where every change is expected to arrive with the test
that exercises it, and where a build already produces a fresh report on every run. A human team will
usually want the warning, since a change that lands its test in the next commit is a legitimate way
to work for an afternoon.
