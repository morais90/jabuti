# Duplication

Finds regions of code that have the same structure as a region somewhere else.

Every other measure jabuti computes is a number about one unit. This one is different in kind. It is
a comparison, and a region of code only has an answer once you have looked at everything else.

## Comparing shape instead of text

The obvious way to find copied code is to look for repeated text. It works, and it stops working the
moment somebody renames a variable.

So jabuti compares structure. Each region of the syntax tree is reduced to a fingerprint built only
from the *kinds* of nodes it contains, in order. Names, literals and types never enter the
calculation. Two regions with the same fingerprint have the same shape, whatever they happen to call
things.

```rust
fn parse_header(input: &str) -> Option<String> {
    let mut parts = input.splitn(2, ':');
    let name = parts.next()?.trim().to_lowercase();
    if name.is_empty() {
        return None;
    }
    Some(name)
}

fn read_pair(line: &str) -> Option<String> {
    let mut pieces = line.splitn(2, ':');
    let key = pieces.next()?.trim().to_lowercase();
    if key.is_empty() {
        return None;
    }
    Some(key)
}
```

Those two functions share no identifier at all and jabuti reports them as copies, because they are.

The clone literature has a standard vocabulary for this. Identical copies are Type-1. Copies where
identifiers, literals and types were changed are Type-2. Both are what this measure detects. Type-3,
where a line or two was also added or removed, and Type-4, where the behaviour matches but the
structure does not, are outside what fingerprint comparison can see.

## Counting nodes, not lines

The size of a repeated region is reported in syntax nodes rather than lines.

Lines are a poor unit here because they are a formatting choice. The same logic written across
fifteen lines or squeezed into four is the same amount of duplication, and a measure that ranks one
above the other would be measuring the formatter. A node count is stable under reformatting, which
is the property that matters when the threshold has to hold across whole codebases.

The trade is that node counts are less intuitive than line counts. As a rough anchor, a small
function of five or six lines with a condition in it lands somewhere around 60 to 90 nodes.

## What is left out

Attributes and annotations do not take part. In Rust that means `#[derive(...)]` and friends; in
Kotlin, annotations and modifier lists.

The reason is a consequence of comparing shape. Because names and literals are ignored,
`#[case("a", 1)]` and `#[case("b", 2)]` produce the same fingerprint, so any two functions carrying
the same number of attributes would look alike no matter what their bodies did. Treating that as
duplication describes the metadata, not the code. Attributes are excluded from both the fingerprint
and the node count for the same reason they are excluded from the
[parameter count](parameters.md): they describe a declaration rather than form part of it.

## Only the widest region

A repeated function contains repeated statements, which contain repeated expressions. Reporting all
of them would bury the one finding you can act on under dozens you cannot.

When one repeated region sits inside another, only the outer one is reported. You are told about the
duplicated function, not about each of its lines.

## Honest limits

Fingerprints are 64-bit hashes and the text of the two regions is never compared, so a collision
would produce a report of two regions that are not actually alike. At the sizes involved this is
vanishingly unlikely, but it is a comparison of hashes rather than a proof of equality, and it is
worth knowing that.

Structure is all that is compared. Two functions with the same shape and genuinely unrelated
meanings will be reported, which is why the [`duplicate-block`](../rules/duplicate-block.md) rule
ships as a warning rather than an error.

Because this measure is a comparison, it needs to see the whole codebase to work. Scoping a run with
`--since` narrows which findings are *reported*, but jabuti still reads every file, because a copy
in the file you edited is only findable if the original is also in view.

## Further reading

Baxter, I. D., Yahin, A., Moura, L., Sant'Anna, M., Bier, L. (1998). *Clone Detection Using Abstract
Syntax Trees*. International Conference on Software Maintenance. The paper that established
fingerprinting subtrees as the way to do this.

Baker, B. S. (1995). *On Finding Duplication and Near-Duplication in Large Software Systems*.
Working Conference on Reverse Engineering. Introduced matching that is blind to identifier names,
which is what makes Type-2 detection possible.

Roy, C. K., Cordy, J. R. (2007). *A Survey on Software Clone Detection Research*. Queen's University
Technical Report 2007-541. The source of the Type-1 through Type-4 vocabulary used above.
