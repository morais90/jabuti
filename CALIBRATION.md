# jabuti calibration record

This file records benchmark populations, distributions, revisions and validation probes.

## Method

Syntax metrics use integer arithmetic over every function jabuti can parse in each corpus. Percentiles
use nearest rank over the combined function population. A rule fires when its value is greater than its
limit, so the percentile value and the limit are not always the same number. Repository probes use the
rule's production path rather than a synthetic model.

The method follows Alves, Ypma and Visser, *Deriving metric thresholds from benchmark data* (ICSM
2010). A distribution describes a population, not an intrinsic boundary between good and bad code.

## Corpus summary

| Language | Population | Files measured | Functions | Measured |
|---|---|---:|---:|---|
| Rust | 1,645 crates published on crates.io | 45,361 | 737,689 | August 2026 |
| Kotlin | ten established projects | not retained as one aggregate | 54,933 | August 2026 |
| TypeScript | ten established projects | 38,869 | 286,924 | September 2026 |

## Syntax metric distributions

### Function lines

| Language | p50 | p75 | p90 | p95 | p98 | p99 | Default limit | Reported at default |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Rust | 6 | 10 | 21 | 34 | 59 | 86 | 60 | 1.9% |
| Kotlin | 7 | 14 | 23 | 32 | 47 | not retained | 47 | approximately 2% |
| TypeScript | 4 | 12 | 26 | 42 | 71 | 102 | 71 | 1.97% |

The Rust library population does not transfer to applications:

| Corpus | Kind | p50 | p90 | p98 | Reported at limit 60 |
|---|---|---:|---:|---:|---:|
| crates.io | libraries | 6 | 21 | 59 | 1.9% |
| hyperswitch | application | 12 | 45 | 122 | 6.8% |
| meilisearch | application | 11 | 66 | 173 | 11.4% |

The default remains 60 because moving it to the application p98 would permit functions between 122
and 173 lines.

### Cognitive complexity

| Language | p50 | p75 | p90 | p95 | p98 | p99 | Default limit | Reported at default |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Rust | 0 | 0 | 1 | 3 | 7 | 12 | 7 | 1.8% |
| Kotlin | 0 | 0 | 2 | 4 | 7 | 11 | 7 | approximately 2% |
| TypeScript | 0 | 1 | 5 | 10 | 18 | 28 | 18 | 1.98% |

A limit of 7 reports 6.6% of the TypeScript corpus. The third language invalidated the earlier
assumption that the Rust and Kotlin value transferred unchanged.

For the Rust and Kotlin validation set, the share reported at limit 7 was:

| Corpus | Kind | Reported |
|---|---|---:|
| crates.io | library | 1.8% |
| kotlinx.coroutines | library | 3.2% |
| okhttp | library | 2.4% |
| DuckDuckGo Android | application | 1.4% |
| hyperswitch | application | 2.1% |
| komga | application | 3.5% |
| Signal-Android | application | 4.3% |
| meilisearch | application | 6.5% |

### Cyclomatic complexity

| Language | p50 | p75 | p90 | p95 | p98 | p99 | Default limit | Reported at default |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Rust | 1 | 1 | 2 | 4 | not retained | 10 | 10 | below 1% |
| Kotlin | 1 | not retained | 3 | 4 | 6 | not retained | 10 | below 2% |
| TypeScript | 1 | 2 | 5 | 8 | 13 | 19 | 13 | 1.95% |

The rule remains off. Inspection of Rust findings showed that the tail was dominated by flat
exhaustive matches, which raise path count without raising reading difficulty.

### Parameters

| Language | p50 | p75 | p90 | p95 | p98 | p99 | Default limit | Reported at default |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Rust | 1 | 1 | 2 | 3 | 5 | 6 | 4 | 2.2% |
| Kotlin | 0 | 1 | 2 | 3 | 5 | 6 | 4 | approximately 2% |
| TypeScript | 1 | 2 | 3 | 4 | 5 | 6 | 4 | 2.69% |

## TypeScript corpus revisions

The command was `jabuti check . --format measures` using tree-sitter-typescript 0.23.2. Named arrow
functions assigned to variables, fields or object members count as functions. Anonymous callbacks
remain attributed to their containing function.

| Project | Revision | Measured files | Unreadable | Functions | Function lines p98 | Cognitive p98 | Cyclomatic p98 | Parameters p98 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| oclif | `c4a1e077d395` | 163 | 0 | 840 | 76 | 21 | 15 | 3 |
| tRPC | `66d054454a33` | 690 | 1 | 2,304 | 140 | 26 | 15 | 3 |
| Zod | `7a00236683c7` | 463 | 4 | 2,349 | 112 | 41 | 28 | 4 |
| Nest | `39fbddae5128` | 1,826 | 2 | 4,966 | 43 | 8 | 7 | 4 |
| Effect | `2a30248b6eb7` | 1,304 | 210 | 10,887 | 67 | 16 | 12 | 4 |
| pnpm | `97554102d012` | 3,123 | 5 | 27,018 | 88 | 16 | 12 | 4 |
| Prisma | `dd846dcc8f06` | 4,392 | 30 | 15,771 | 63 | 14 | 11 | 4 |
| typescript-eslint | `1288fcd15164` | 2,297 | 257 | 3,354 | 187 | 21 | 18 | 4 |
| Visual Studio Code | `3d7cfab6d77d` | 12,908 | 30 | 167,841 | 72 | 20 | 14 | 6 |
| TypeScript | `1f70213d4922` | 11,703 | 1,086 | 51,594 | 39 | 12 | 11 | 5 |

The scan discovered 40,494 `.ts` files and rejected 1,625. Many rejected files are intentionally
invalid compiler and parser fixtures. Current Effect and TypeScript source also exposes grammar gaps,
so unreadable files remain part of the product output rather than being removed from the record.

## Grammar coverage probes

The Kotlin grammar comparison used five projects:

| Project | Files measured | Files unreadable |
|---|---:|---:|
| DuckDuckGo Android | 5,874 | 5 |
| Signal-Android | 4,180 | 18 |
| komga | 534 | 17 |
| okhttp | 610 | 7 |
| kotlinx.coroutines | 1,066 | 16 |

The alternative Kotlin grammar parsed four of seven newer constructs that the selected grammar
rejects, but rejected 80 Signal-Android files where the selected grammar rejected 18. Neither grammar
was uniformly better.

## Rule validation probes

### `duplicate-block`

At 120 syntax nodes the calibration projects produced about two findings per thousand lines. Manual
inspection classified the findings as real Type-2 copies. Below roughly 60 nodes, ordinary grammar
shapes began to dominate.

### `error-masking`

The Rust crate registry, two large Kotlin projects and two smaller Rust projects produced roughly two
to four findings per thousand lines of production code. Between 73% and 87% of all matching
constructs were in test code, which is why path and declaration test markers are excluded.

### `new-dependency`

Across 62 meilisearch commits that touched Rust, 45 introduced no dependency. The mean was 1.5 new
dependencies, p90 was 3 and the maximum was 30.

### `speculative-api`

| Project | Commits | New public items | Findings |
|---|---:|---:|---:|
| meilisearch | 60 | 90 | 0 |
| komga | 25 | 746 | 0 |
| jabuti | 40 | 162 | 0 |

Appending one synthetic unreferenced public declaration to meilisearch or komga produced exactly one
finding at that declaration.

## Reference graph probes

The share of resolved edges that an import-only graph missed was:

| Source of edge | Rust | Kotlin |
|---|---:|---:|
| Path written at its use site | 23% in hyperswitch, 26% in jabuti | not applicable |
| Path written inside a macro | 3.3% to 5.0% | not applicable |
| Bare same-package name | not applicable | 13% in komga, 16% in DuckDuckGo, 20% in Signal-Android |
