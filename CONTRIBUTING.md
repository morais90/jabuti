# Contributing to jabuti

jabuti gives an AI coding agent a deterministic verdict on the code it just wrote. Before changing
anything here, read [the principles](docs/principles.md). They say what the tool is opinionated about
and why, and most review comments on a first contribution trace back to one of them.

## Opening a work item

Work is tracked as issues, one per problem. A problem is answered by one rule, so an idea that lands
on a problem already answered is a refinement of that rule rather than a second rule beside it, and an
idea that names no family is not yet a rule.

An issue is complete when the problem is stated, not when the solution is decided.

Open with the problem and a short piece of code that shows it. Someone who has never seen this project
should understand what goes wrong before meeting any vocabulary from it. Say why nothing already
reports it, because if an existing linter does, the work is curating that linter rather than writing a
rule.

Then say who learns something they did not already have, and whether the finding is about a change,
about the repository, or about both. The author of a change and the person who inherited a codebase do
not know the same things, and a finding that restates what its own reader wrote on purpose changes
nothing however cheap it is to detect.

Then say how we would answer it: the rule or tool, which class of input it reads, whether the detection
is ours or delegated to a curated tool, and what is still open. Cite primary literature where a source
is needed, and do not cite another tool as the source of a threshold, a counting rule or a definition.
Where the justification is structural rather than published, say so and name the measurement that will
settle it.

Close with a `Done when` checklist, and write one only where the answer is already decided. An item
waiting on a measurement gets the experiment as its `Done when`, with the number and the recorded
decision as the deliverable, and the rule becomes a second issue if the number says yes. Inventing a
solution before the work starts is the planning nobody asked for.

Where the issue is a reasonable place for someone new to start, say so at the end, say why, and point
at a rule already in the tree that shows the mechanism working. Add the `good first issue` label.
Leave both off where it is not true.

The family is a label rather than a heading, and so is the kind of work: `rule`, `tool`, `capability`,
`experiment` or `infra`. An issue blocked by another opens with a `Blocked by #N` line.

## Definition of done

Every delivery meets all of these. They are the common ruler, so an issue's own `Done when` lists only
what is specific to it.

- `just check` is clean
- every measure has a page under `docs/measures/`, every rule a page under `docs/rules/`, and every
  rule at least one fixture
- every rule page names the family the rule belongs to and the domain its finding falls under, so a
  reader arriving by either vocabulary lands in the right place
- every rule carries its succinct explanation, with `why`, `fix` and a per language `example`
- every default carries its reason: why a rule is on or off, why a limit sits where it sits, and why
  each flag in a tool's invocation is there
- every threshold states which percentile it sits at, and for which language
- the change is reviewed before it is committed, and the findings are answered inside the same change
- the work is committed atomically, in conventional commits, once and not again to repair itself

A surviving mutant is not a warning. It means either a test is missing or the code it changed is dead,
and both belong to the work that produced it rather than to the work that follows.

## Adding a rule

The path is the same every time, and skipping a step is what makes a rule unmaintainable rather than
what makes it late.

1. Open the issue and settle the family, the problem and the reader.
2. Declare the language specifics as data. Node kinds and concepts belong in the language tables and in
   `.scm` queries, never in an imperative walker.
3. Write a conformance fixture per language it applies to. A fixture states its expected value and the
   derivation that produced it, which is the one place in the repository where an annotation is the
   specification rather than a comment.
4. Calibrate the limit if the rule carries one, per language, and record the distribution in the
   calibration data. A threshold measured on one corpus does not transfer to another language.
5. Write the measure page and the rule page. The measure page explains the number. The rule page
   explains the limit, the severity, and what a reader should make of a finding.
6. Write the succinct explanation that ships in the output, which is a separate artifact from the rule
   page and written for a different reader.
7. Report the rate the rule produces over the corpora before it ships. A rule that fires on a fifth of a
   corpus is not shipped whatever its reputation.

A rule that needs a limit is only worth building if the distribution has a tail long enough to place one
on. Measure first.

With `--since`, every finding is trimmed in one place to the lines the change added, whatever rule
raised it. A rule whose finding belongs somewhere else, such as a check that was deleted or a caller the
change broke without touching, would be dropped there without a word. The first such rule declares
where its findings anchor in the catalog, and the trim learns to respect it, before the rule ships.

## Style

Write every artifact in English: identifiers, documentation, CLI help, package descriptions, issues and
commit messages.

Do not write comments, in any file type. Code, configuration and workflows explain themselves through
naming and structure, and a comment means a name is wrong or a unit does too much. Test fixtures are the
exception, for the reason given above.

Do not use em dashes. A comma, a full stop or a pair of parentheses reads more naturally.

Prefer named intermediates over combinator chains in analysis code. Traversal and metric algorithms are
intrinsically unpleasant to read, and left alone they become the part of the codebase nobody will touch.

Write documentation for the person using jabuti, not about how the project is built. Explain a measure so
a reader of any level follows it, and back it with literature rather than assertion.

## Maintainability

Keep language specifics in declarative tables and tree-sitter queries. Extract with `.scm` queries, and
hand write traversal only where the algorithm carries state across the walk.

When the dogfooding gate flags our own implementation, fix the implementation rather than raising the
threshold.

## Extensibility

Keep the three extension axes distinct: a new rule on an existing sensor, a new sensor, a new language.

Keep each context in its own module tree, shaped like a crate of its own: `code`, `graph`, `history`,
`tools` and `crossings` in the core. A context owns its rules, its measures, its language tables, its queries and its
tests, and reaches only the kernel (`catalog`, `model`, `policy`, `report`, `lang`, `syntax`, `diff`), never another
context. A test in each crate holds that boundary. Composition happens in `check` of the binary, so a
context can grow, be replaced or become a subcommand without touching the others. `check` reads as the
run itself, plan, examine, judge, trim and order, and it asks the catalog rather than naming a rule; the
one place in it that may name a rule is `check/crossings`.

A rule that crosses two contexts, the way `hotspot` crosses history with complexity, belongs to neither.
It lives in `crossings`, the one context allowed to reach the others, and no context reaches back into
it.

Everything that touches the world lives in `inputs` of the binary: walking the project, asking git,
running a tool, reading a coverage report, resolving which file belongs to which layer. The rest of the
binary receives what `inputs` loaded, the core never touches the world at all, and a boundary test holds
both.

Treat rule ids as public API. Deprecate with an alias, and never rename silently.

Extend in tree and declaratively. There is no dynamic plugin loading and there will not be.

## Scope

Build a mechanism at the third instance, not the first. Add no abstraction for single use code, no
unrequested configurability, and no handling for impossible states.

Touch only what the task requires. Leave adjacent code, comments and formatting alone. Remove the
bindings your change orphaned, and leave pre-existing dead code alone while mentioning it.

## Measuring what things cost

Every delivery that changes how much work a run does measures it on the corpora and records the number.
A decision about what to run needs data about what things cost, and that data exists only if it is
collected while the work happens rather than reconstructed afterwards.
