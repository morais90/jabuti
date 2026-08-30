# `churn`

Reports a file that has been touched by more commits than the limit.

**Default limit:** none useful. **Default severity:** off.

See [the measure](../measures/churn.md) for what is counted and why history is worth reading.

## Why it is off

Every other rule here has a limit drawn from a distribution, so the claim it makes is that the code
is unusual by some measurable standard. Churn has no such number.

The count depends on how old and how busy the repository is, not on the code. Twelve commits is a
lot in a young project and nothing in an old one, and no single limit is right for both. Shipping a
default here would mean shipping a number nobody can defend, which is the thing we have avoided
everywhere else.

## Using it anyway

Within one repository the measure is perfectly meaningful, and if you know your own history a limit
is easy to pick. Look at your busiest files first:

```console
$ git log --format= --numstat | awk -F'\t' 'NF==3 {c[$3]++} END {for (f in c) print c[f], f}' | sort -rn | head
```

Then set the limit somewhere past the bulk of that list:

```toml
[rules]
churn = { limit = 40, severity = "warning" }
```

Reported on its own, this tells you which files are moving. That is genuinely useful when you are new
to a codebase and want to know where the action is, and much less useful as a gate, since a file
changing often is not a defect.

## What it is really for

Change frequency earns its place when it is combined with something else. A file that is complex and
rarely touched costs nobody anything. The same file changing every week is where time and mistakes
accumulate, and that combination is a much sharper signal than either half.

That composite is the next thing being built. Until it exists, this rule is here so the number is
available and so its definition is settled.

## Further reading

Rahman, F., Devanbu, P. (2013). *How, and why, process metrics are better*. International Conference
on Software Engineering.

Tornhill, A., Borg, M. (2022). *Code Red: The Business Impact of Code Quality*. International
Conference on Technical Debt.
