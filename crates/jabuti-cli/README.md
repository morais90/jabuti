# jabuti-cli

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/morais90/jabuti/blob/main/LICENSE)

The `jabuti` binary. A deterministic verdict on the code your agent just wrote.

See the [project README](https://github.com/morais90/jabuti) for what it measures and why, and
[the documentation](https://github.com/morais90/jabuti/tree/main/docs) for each rule.

```console
$ jabuti check --since main
1 error and 0 warnings across 3 files and 14 units.

src/handler.rs:120  error  function-lines  handle_request  measured 71, limit 60
```

Exit codes separate the two failures that must never be confused: `0` passed, `1` a gate was
violated, `2` the tool itself broke and nothing was checked.

This crate is the shell, covering arguments, configuration, file discovery, rendering and exit
codes. The analysis lives in [`jabuti-core`](https://crates.io/crates/jabuti-core).

## Status

Early. One rule reports by default and two more are calculated but switched off.

## License

Apache-2.0.
