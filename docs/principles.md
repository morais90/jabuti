# Principles

jabuti is opinionated on purpose. This page says where the opinions are, what backs each one, and
what it would take to change it. Read it if you want to know why a default sits where it does, why a
check you expected is missing, or why the output has the shape it has.

## What jabuti is

A single binary that gives an AI coding agent a deterministic, token cheap verdict on the code it
just wrote. Native sensors for metrics and git history, orchestrated market tools for everything
already solved. The value is the normalized output contract, the verdict scoped to a change, and a
format that fits in a context window.

## Facts, not judgement

jabuti produces facts and the agent reading them produces judgement. That division is what makes the
output safe to act on without checking it first.

The same input at the same version produces byte identical output. There are no learned models, no
wall clock, no random number generator and no network on the path that answers a check. Measures use
integer arithmetic, because float summation order changes under parallel reduction and parallelism
must never alter what is reported.

What you see of this in the output is that every finding points at the thing that caused it. There is
no aggregate score, because a number with no attributable cause cannot be acted on. `readability 0.63`
tells an agent nothing it can edit.

**Where the line on advice sits.** jabuti does not produce advice about your situation. It will not
read your function and tell you how to restructure it, because that answer is not reproducible and
deciding it is the caller's job. What a rule means, why the thing it reports is a problem and what the
usual fix looks like are none of those things. They are fixed properties of the rule, the same way its
threshold and its name are, and they read identically on every run. Text of that kind is data, and the
determinism principle has nothing to say against it.

## Every default is an opinion

A default is not a neutral place to start. It is the answer to "what is the best way to do this", and
answering that is what you came here for. A configuration you still have to go and research yourself
gave you nothing, because the research was the product.

So jabuti holds an opinion on four things, and each one is a decision somebody has to make anyway.

**Which rules run.** Off is an opinion too, not the absence of one. `cyclomatic-complexity` is off
because `cognitive-complexity` measures the same code better and two rules firing on one function is
noise. `churn` is off because no absolute limit transfers between repositories. Both are positions
with a reason, and each rule page carries the reason rather than the setting alone.

**What limit each rule carries.** A number placed at a percentile of a named population, with the
population named, because the same limit means different things in different languages and in
libraries against applications. [The calibration record](../CALIBRATION.md) holds the distributions
the numbers came from.

**Which tools your project should run.** A project with a Cargo manifest and no coverage report is
told it is missing coverage, rather than shown a neutral row saying coverage is unavailable.

**How each tool is invoked.** This is where the opinion is sharpest and least visible, and it is not
always about strictness. cargo-mutants shuffles its mutants by default, and jabuti runs it with
`--no-shuffle`, because a deterministic order is worth more here than the load balancing the shuffle
buys. A coverage run that does not ask for branch data throws away half of what the instrumentation
already collected. A linter invoked without the setting that enables its type aware rules is answering
an easier question than the one you think you asked. Every flag is a decision, and each one is written
down with its reason.

**What keeps this from being arbitrary.** An opinion here is measured rather than asserted, it carries
its reason and the date it was formed, and one line in `jabuti.toml` overrides it. Defaults that
improve are also why generated configuration writes only what differs from them. A file that pins
today's answers is a file that never receives a better one.

**The one place jabuti defers.** What your project explicitly configured, your project governs. Not
configuring something and configuring it weakly are different acts, and only the second is a choice
respected in silence. Where a project has actively switched a check off, the stricter answer is still
reported, at warning, under its own rule id, and the finding says the project chose otherwise.

## Tools are curated, not piped

Writing software stopped being the expensive part. What costs now is running it, and running it badly:
a tool installed with its defaults, most of its checks switched off, pointed at everything at once. A
codebase carrying four hundred warnings was once a rational trade, because clearing each one cost hours
of someone's attention. That trade no longer holds, and the tool configurations built around it do not
either.

So a tool jabuti runs is not a pipe. What jabuti decides is what to ask each ecosystem, and reading the
answer is the easy half.

**Turn on what the project left off.** Every linter ships its sharpest checks disabled. Clippy keeps
whole groups allow by default, typescript-eslint's type aware rules need a setting most projects skip
because it costs seconds, detekt holds rule sets back, and a TypeScript project with strict mode off
learns almost nothing from its own compiler. A curated set, per ecosystem and documented, is what turns
a tool from a formality into a measurement.

**Restrict what it looks at.** The two moves pull in opposite directions on purpose: a broader set of
questions asked of less code. Scope to the change, and where a coverage report exists, to the lines a
test actually reaches. That is what keeps a broader rule set from producing a wall nobody reads, and it
is what lets an analysis that used to belong in a nightly job fit inside a single turn.

**What it costs.** A curated set is a maintenance commitment. Lints are added, deprecated and renamed,
and a curation left to rot is worse than none. It is versioned with the binary, so a release that
changes what is asked says so, and a verdict on unchanged code can move between versions for that
reason and no other.

**Where this stands today.** clippy is the only orchestrated linter, and it runs with your project's
configuration and nothing added. The curated sets described here are the standard this project holds
itself to when it integrates a tool, not a description of what ships, and each one arrives with the
release that adds it.

**When the rule is jabuti's instead.** Curation says how somebody else's tool gets run. The other half
is knowing when not to. A rule belongs to jabuti when it needs two inputs that no single tool holds.
The coverage runner knows which lines ran and only jabuti knows which are new. `hotspot` crosses history
with complexity. `layer-violation` crosses the reference graph with a boundary you declared. Whether a
pipeline step is pinned to a mutable tag is one input and belongs to the tool that lints pipelines.
Whether the pipeline actually runs the gate your repository configures is two, and belongs here.

## What a run is allowed to cost

**Scheduling may change, output may not.** Whatever gets skipped, reused, parallelised or deferred, the
report is the one a naive run over everything would have produced. An optimisation that changes what is
reported is not an optimisation, it is a different tool. This is the determinism principle applied to
the order work happens in, and it is what makes the inside of a run safe to rebuild.

**Yield never reaches the output.** Ordering rules by how often they fire is scheduling. Skipping the
ones that rarely fire is a different verdict wearing the word optimisation, and the temptation is
obvious enough to be worth naming.

## Families

Every rule belongs to a family, and the family names the way a change fails rather than the area of
software it touches. It is how the rule catalog is organised, and it is what tells you whether a rule
that is missing is one nobody built yet or one that would be a duplicate of something else.

| Family | What goes wrong |
|---|---|
| `completeness` | The change is reported as done and is not |
| `silent-failure` | The tests are green and the failure happens elsewhere |
| `silenced` | The check that would have caught it was removed or switched off |
| `shape` | The code grew where it should have been restructured |
| `structure` | Relationships between files left what the project declared |
| `contract` | The change breaks a promise made to code outside this repository |
| `supply` | What the project depends on, how it got in, or what leaked out |
| `convention` | The linter is off, or it is on and nobody scoped it |
| `cost` | Where maintenance accumulates |

Only `shape` and `cost` carry a limit, which is why calibration exists for them and for nothing else.
In every other family a finding is a fact, and a threshold on it would mean nothing.

The obvious alternative is to organise by security, performance, scalability and maintainability, which
is the vocabulary most readers already speak. It is not the spine here for two reasons. Sorted that way
the buckets come out lopsided, with almost everything under maintainability and nothing under
scalability, and the thin ones stay thin because measuring execution needs a workload and a clock, which
determinism rules out. And a domain is an attribute rather than a parent: `blocking-in-async` is
reliability, performance and scalability at once. So each rule page names the domain its finding falls
under, and the family is what the catalog is built on.

## Where the analysis stops

jabuti reads syntax, the reference graph between files, and git history. It does not resolve the type of
a value, and it does not track a value through a function.

That boundary is what keeps the tool fast and reproducible, and it is also why some rules you might
expect are absent rather than unfinished. Anything that turns on knowing the type behind a receiver,
which includes most of the classic object oriented smells, needs a compiler frontend. Anything that
turns on following a handle from where it is acquired to where it is released needs data flow, and most
of the rules that want it need it across functions as well.

jabuti also keeps no state of its own between runs. A tool it orchestrates may reach the network and may
cache on disk, and jabuti reads whatever is there. An index that jabuti maintained would be the first
thing in the output that a reader could not inspect, and a partially stale one breaks byte identical
output in a way that is hard to prove fixed.

## What gets built next

Work is ordered by what the reader learns that they did not already have, with the reader named. A
finding that restates something its own reader wrote on purpose changes nothing, however cheap it is to
detect, and the author of a change and the person who inherited a codebase do not know the same things.
The cheap and invisible comes before the expensive and obvious for the same reason.

What is queued, and what was considered and set aside, lives in
[the issue tracker](https://github.com/morais90/jabuti/issues) rather than in this repository.
