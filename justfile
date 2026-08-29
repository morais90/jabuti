mutant_jobs := "4"

default:
    @just --list

fmt:
    cargo +nightly fmt --all

test:
    cargo nextest run --workspace --all-targets
    cargo mutants --no-shuffle --jobs {{ mutant_jobs }}

check:
    cargo +nightly fmt --all --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo nextest run --workspace --all-targets
    cargo mutants --no-shuffle --jobs {{ mutant_jobs }}
    cargo deny check

hooks:
    pre-commit install --install-hooks
    pre-commit install --hook-type commit-msg
