# jabuti-cli

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/williandmorais/jabuti/blob/main/LICENSE)

The `jabuti` binary — a deterministic verdict on the code your agent just wrote.

See the [project README](https://github.com/williandmorais/jabuti) for what it measures and why.

```console
$ jabuti check --since main
src/handler.rs:120-186  handle_request  cognitive=31 (max 15)  nesting=6 (max 4)
1 finding on changed code
```

Exit codes separate the two failures an agent must never confuse: `0` passed, `1` a gate was
violated, `2` the tool itself broke.

This crate is the shell — arguments, configuration, file discovery, rendering and exit codes. The
analysis lives in [`jabuti-core`](https://crates.io/crates/jabuti-core).

## Status

Early. There is no usable binary yet; the command above is the contract being built toward.

## License

Apache-2.0.
