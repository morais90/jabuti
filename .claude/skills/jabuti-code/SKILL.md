---
name: jabuti-code
description: Development principles for the jabuti project. Load before writing or editing any code, test, config or documentation in this repository.
---

# jabuti development principles

The rules are in the repository, and those files are the source. Read both before writing or editing
anything here.

- `CONTRIBUTING.md` at the repository root: style, the definition of done, how a rule is added,
  maintainability, extensibility and scope.
- `docs/principles.md`: what the tool is opinionated about, determinism, the families, and where the
  analysis stops.

Everything below is what those two pages do not carry, because it applies while writing code rather
than to the shape of the product.

## Determinism in the code

`HashMap` iteration order reaching output is a bug. Use `BTreeMap`, or sort before rendering.

## Before committing

Run `/code-review` while the change is still uncommitted, and answer the findings inside the same
change. A change that was reviewed after it was committed gets repaired by a second commit, which the
definition of done rules out.
