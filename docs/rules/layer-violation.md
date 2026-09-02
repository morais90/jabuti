# `layer-violation`

Reports a file that depends on a layer its own layer is not allowed to depend on.

**Default severity:** warning. This rule has no limit; every crossing is reported. It does nothing
until you declare layers, and it says so when a layer you declared matches no file.

## The idea

An architecture is a set of rules about what may depend on what. Domain code should not know how it
is stored; a web handler should not reach into the database directly; a shared kernel should depend
on nothing. Every codebase has some version of this, written in a document or held in the heads of
the people who were there at the start.

The rules erode one import at a time. Nobody decides to couple the domain to the database; someone
needs to persist a value from inside a domain function, and the shortest path is to call the
repository from there. It works, the tests pass, and the boundary is gone.

This rule turns the document into something that runs:

```toml
[layers]
domain = { paths = ["src/domain/**"], depends_on = [] }
application = { paths = ["src/application/**"], depends_on = ["domain"] }
infrastructure = { paths = ["src/infrastructure/**"], depends_on = ["domain", "application"] }
```

Each layer names the files it contains and the layers it may depend on. Anything else is a crossing:

```
src/domain/book.rs:4  warning  layer-violation  domain may not depend on infrastructure (src/infrastructure/db.rs)
```

The finding sits on the line that made the dependency, so the fix is one click away from the report.

## Why this is a permission list

Two shapes are possible: name what each layer may depend on, or name what it may not. Naming what
is allowed is stricter, because a layer nobody wrote a rule for depends on nothing else by default,
and that is the shape you want when the point is to hold a boundary. Naming what is forbidden is
easier to switch on in an existing codebase and catches only what someone remembered to forbid.

The permission shape was chosen because the cost of the stricter reading is paid once, when you
write the configuration, and the cost of the looser one is paid every time a crossing nobody
anticipated goes unreported.

## What is not a violation

**A dependency inside one layer.** A layer is free to depend on itself.

**A file in no layer.** Files that match no declared layer are unconstrained, in both directions.
That is what makes the rule adoptable: you declare the boundaries you care about and leave the rest
alone, rather than having to classify every file before the rule can run.

**An indirect dependency.** Only the direct edge is checked. If `application` may depend on `domain`
and `infrastructure` may depend on `application`, then `infrastructure` reaching `domain` through
`application` is exactly how layers are supposed to work.

## Writing the paths

Layer paths use the same patterns as `exclude`. A pattern that names a directory matches only the
directory itself, so `src/domain` matches nothing you want and `src/domain/**` matches every file
under it. Because that mistake is easy to make and silent, a layer that matches no file is reported
on stderr rather than left empty.

## Together with `--since`

Under `--since`, a crossing is reported only when the line that makes it is part of the change. A
codebase with a hundred existing violations can switch the rule on today and hear about them one at
a time, as each is touched, rather than all at once.

The same line often carries two findings under `--since`: [`new-dependency`](new-dependency.md)
says the dependency did not exist before, and this rule says it is not allowed. They are different
facts and both are worth knowing.

## Changing it

```toml
[rules]
layer-violation = { severity = "error" }
```

Promoting it to `error` is the natural setting once the layers are trusted, since a crossing is a
defect by your own declaration. It is a warning by default so that switching the rule on in an
existing codebase reports rather than fails, and `--since` keeps that report short.

## Further reading

Martin, R. C. (2017). *Clean Architecture*. Prentice Hall. Chapter 22 states the dependency rule
that this configuration expresses: source code dependencies point inward, toward policy.

Evans, E. (2003). *Domain-Driven Design*. Addison-Wesley. Chapter 4 on layered architecture, and
the argument for keeping the domain layer free of infrastructure concerns.
